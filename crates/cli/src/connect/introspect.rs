//! Schema introspection for Source declarations.
//!
//! When a Source has a `schema_ref` field, this module fetches and parses
//! the external schema to extract endpoints, field types, and structure.
//! Supports OpenAPI 3.x (JSON), GraphQL SDL, and SQL DDL.

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

/// An introspected external schema.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ExternalSchema {
    /// The format of the schema document.
    pub format: SchemaFormat,
    /// Endpoints / entry points extracted from the schema.
    pub endpoints: Vec<Endpoint>,
}

/// The schema document format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaFormat {
    OpenApi3,
    GraphQl,
    SqlDdl,
}

/// An endpoint extracted from an external schema.
#[derive(Debug, Clone)]
pub struct Endpoint {
    /// HTTP method (GET, POST, etc.) or operation name.
    pub method: String,
    /// Path or operation identifier (e.g. "/orders/{id}/balance").
    pub path: String,
    /// Path parameters (e.g. ["id"]).
    pub parameters: Vec<String>,
    /// Response fields with their types.
    pub response_fields: Vec<SchemaField>,
}

/// A field extracted from an external schema.
#[derive(Debug, Clone)]
pub struct SchemaField {
    /// Dot-delimited path within the response (e.g. "balance.amount").
    pub path: String,
    /// The field's type as reported by the schema.
    pub field_type: ExternalType,
}

/// Type information from an external schema, mapped to a simplified type system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalType {
    Integer,
    Number,
    String,
    Boolean,
    Object,
    Array,
    Unknown(String),
}

impl fmt::Display for ExternalType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExternalType::Integer => write!(f, "integer"),
            ExternalType::Number => write!(f, "number"),
            ExternalType::String => write!(f, "string"),
            ExternalType::Boolean => write!(f, "boolean"),
            ExternalType::Object => write!(f, "object"),
            ExternalType::Array => write!(f, "array"),
            ExternalType::Unknown(s) => write!(f, "unknown({})", s),
        }
    }
}

/// Introspect an external schema given the protocol and schema reference.
///
/// Auto-detects format based on file extension first, then falls back
/// to protocol-based dispatch.
pub fn introspect_schema(protocol: &str, schema_path: &Path) -> Result<ExternalSchema, String> {
    // Try auto-detection first based on file extension
    let ext = schema_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    match ext {
        "graphql" | "gql" => introspect_graphql(schema_path),
        "sql" => introspect_sql_ddl(schema_path),
        "json" => {
            // Could be OpenAPI -- check content
            introspect_openapi(schema_path)
        }
        _ => {
            // Fall back to protocol hint
            match protocol {
                "http" => introspect_openapi(schema_path),
                "graphql" => introspect_graphql(schema_path),
                "database" => introspect_sql_ddl(schema_path),
                "grpc" => Err("gRPC proto introspection not yet implemented".to_string()),
                _ => Err(format!(
                    "schema introspection not supported for protocol '{}'",
                    protocol
                )),
            }
        }
    }
}

/// Detect the schema format and introspect automatically.
///
/// Detection logic:
/// - `.json` extension + contains `"openapi"` key -> OpenAPI
/// - `.graphql` or `.gql` extension -> GraphQL SDL
/// - `.sql` extension -> SQL DDL
/// - Otherwise -> try each parser in order (OpenAPI, GraphQL, SQL), return first success
pub fn detect_and_introspect(schema_path: &Path) -> Result<ExternalSchema, String> {
    let ext = schema_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    match ext {
        "json" => {
            // Check if it looks like OpenAPI
            let content = std::fs::read_to_string(schema_path).map_err(|e| {
                format!(
                    "could not read schema file '{}': {}",
                    schema_path.display(),
                    e
                )
            })?;
            if content.contains("\"openapi\"") {
                return introspect_openapi(schema_path);
            }
            Err(format!(
                "JSON file '{}' does not appear to be an OpenAPI document",
                schema_path.display()
            ))
        }
        "graphql" | "gql" => introspect_graphql(schema_path),
        "sql" => introspect_sql_ddl(schema_path),
        _ => {
            // Try each parser in order
            if let Ok(schema) = introspect_openapi(schema_path) {
                return Ok(schema);
            }
            if let Ok(schema) = introspect_graphql(schema_path) {
                return Ok(schema);
            }
            if let Ok(schema) = introspect_sql_ddl(schema_path) {
                return Ok(schema);
            }
            Err(format!(
                "could not detect schema format for '{}'",
                schema_path.display()
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// OpenAPI 3.x introspection
// ---------------------------------------------------------------------------

/// Parse an OpenAPI 3.x JSON document and extract endpoints with response shapes.
fn introspect_openapi(schema_path: &Path) -> Result<ExternalSchema, String> {
    let content = std::fs::read_to_string(schema_path).map_err(|e| {
        format!(
            "could not read schema file '{}': {}",
            schema_path.display(),
            e
        )
    })?;

    let doc: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("invalid JSON in '{}': {}", schema_path.display(), e))?;

    // Verify it's OpenAPI 3.x
    let version = doc.get("openapi").and_then(|v| v.as_str()).unwrap_or("");
    if !version.starts_with("3.") {
        return Err(format!(
            "unsupported OpenAPI version '{}' (expected 3.x)",
            version
        ));
    }

    let paths = doc.get("paths").and_then(|p| p.as_object());
    let paths = match paths {
        Some(p) => p,
        None => {
            return Ok(ExternalSchema {
                format: SchemaFormat::OpenApi3,
                endpoints: vec![],
            })
        }
    };

    let mut endpoints = Vec::new();

    for (path_str, path_item) in paths {
        let path_obj = match path_item.as_object() {
            Some(o) => o,
            None => continue,
        };

        for (method, operation) in path_obj {
            // Skip non-method keys like "parameters", "summary"
            if !matches!(
                method.as_str(),
                "get" | "post" | "put" | "patch" | "delete" | "head" | "options"
            ) {
                continue;
            }

            let parameters = extract_path_parameters(path_str);

            let response_fields = extract_response_fields(operation, &doc);

            endpoints.push(Endpoint {
                method: method.to_uppercase(),
                path: path_str.clone(),
                parameters,
                response_fields,
            });
        }
    }

    Ok(ExternalSchema {
        format: SchemaFormat::OpenApi3,
        endpoints,
    })
}

/// Extract path parameter names from an OpenAPI path template.
/// E.g. "/orders/{id}/items/{item_id}" -> ["id", "item_id"]
fn extract_path_parameters(path: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut in_brace = false;
    let mut current = String::new();

    for ch in path.chars() {
        match ch {
            '{' => {
                in_brace = true;
                current.clear();
            }
            '}' => {
                if in_brace && !current.is_empty() {
                    params.push(current.clone());
                }
                in_brace = false;
            }
            _ => {
                if in_brace {
                    current.push(ch);
                }
            }
        }
    }

    params
}

/// Extract response fields from an OpenAPI operation's 200 response schema.
fn extract_response_fields(
    operation: &serde_json::Value,
    doc: &serde_json::Value,
) -> Vec<SchemaField> {
    let mut fields = Vec::new();

    // Look at responses.200.content.application/json.schema
    let schema = operation
        .get("responses")
        .and_then(|r| r.get("200").or_else(|| r.get("201")))
        .and_then(|r| r.get("content"))
        .and_then(|c| c.get("application/json"))
        .and_then(|j| j.get("schema"));

    let schema = match schema {
        Some(s) => resolve_ref(s, doc),
        None => return fields,
    };

    flatten_schema_fields(&schema, doc, "", &mut fields);

    fields
}

/// Resolve a $ref pointer within the OpenAPI document.
fn resolve_ref<'a>(schema: &'a serde_json::Value, doc: &'a serde_json::Value) -> serde_json::Value {
    if let Some(ref_str) = schema.get("$ref").and_then(|r| r.as_str()) {
        // Only handle #/components/schemas/... style refs
        if let Some(stripped) = ref_str.strip_prefix("#/") {
            let parts: Vec<&str> = stripped.split('/').collect();
            let mut current = doc;
            for part in &parts {
                current = match current.get(part) {
                    Some(v) => v,
                    None => return schema.clone(),
                };
            }
            return current.clone();
        }
    }
    schema.clone()
}

/// Flatten a JSON schema's properties into dot-delimited field paths.
fn flatten_schema_fields(
    schema: &serde_json::Value,
    doc: &serde_json::Value,
    prefix: &str,
    fields: &mut Vec<SchemaField>,
) {
    let schema = resolve_ref(schema, doc);

    let schema_type = schema.get("type").and_then(|t| t.as_str()).unwrap_or("");

    match schema_type {
        "object" => {
            if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
                for (name, prop_schema) in props {
                    let full_path = if prefix.is_empty() {
                        name.clone()
                    } else {
                        format!("{}.{}", prefix, name)
                    };

                    let resolved = resolve_ref(prop_schema, doc);
                    let prop_type = resolved.get("type").and_then(|t| t.as_str()).unwrap_or("");

                    // Add the field itself
                    fields.push(SchemaField {
                        path: full_path.clone(),
                        field_type: parse_external_type(prop_type),
                    });

                    // Recurse into objects (but not too deep)
                    if prop_type == "object" && prefix.matches('.').count() < 3 {
                        flatten_schema_fields(&resolved, doc, &full_path, fields);
                    }
                }
            }
        }
        _ => {
            if !prefix.is_empty() {
                fields.push(SchemaField {
                    path: prefix.to_string(),
                    field_type: parse_external_type(schema_type),
                });
            }
        }
    }
}

/// Map an OpenAPI type string to our simplified ExternalType.
fn parse_external_type(t: &str) -> ExternalType {
    match t {
        "integer" => ExternalType::Integer,
        "number" => ExternalType::Number,
        "string" => ExternalType::String,
        "boolean" => ExternalType::Boolean,
        "object" => ExternalType::Object,
        "array" => ExternalType::Array,
        "" => ExternalType::Unknown("unspecified".to_string()),
        other => ExternalType::Unknown(other.to_string()),
    }
}

// ---------------------------------------------------------------------------
// GraphQL SDL introspection
// ---------------------------------------------------------------------------

/// A parsed GraphQL type definition.
#[derive(Debug)]
struct GqlTypeDef {
    name: String,
    fields: Vec<GqlField>,
}

/// A field within a GraphQL type definition.
#[derive(Debug)]
struct GqlField {
    name: String,
    type_name: String,
    is_list: bool,
}

/// Parse a GraphQL SDL file and extract endpoints with field types.
///
/// This is a simple line-by-line parser that handles:
/// - `type X { field: Type }` syntax
/// - Non-null markers (`!`) are stripped
/// - List types (`[Type]`) are recognized
/// - `input`, `enum`, `interface`, `union`, `directive`, `schema` blocks are ignored
fn introspect_graphql(schema_path: &Path) -> Result<ExternalSchema, String> {
    let content = std::fs::read_to_string(schema_path).map_err(|e| {
        format!(
            "could not read schema file '{}': {}",
            schema_path.display(),
            e
        )
    })?;

    let type_defs = parse_graphql_types(&content)?;

    if type_defs.is_empty() {
        return Err(format!(
            "no type definitions found in '{}'",
            schema_path.display()
        ));
    }

    // Build a map for nested object resolution
    let type_map: BTreeMap<&str, &GqlTypeDef> =
        type_defs.iter().map(|t| (t.name.as_str(), t)).collect();

    let mut endpoints = Vec::new();

    for type_def in &type_defs {
        let method = match type_def.name.as_str() {
            "Query" => "QUERY",
            "Mutation" => "MUTATION",
            "Subscription" => continue, // Skip subscriptions
            _ => continue,              // Non-root types are only used for nesting
        };

        for field in &type_def.fields {
            let mut response_fields = Vec::new();
            let field_type = map_graphql_scalar(&field.type_name);

            if field_type == ExternalType::Unknown("object".to_string()) || field.is_list {
                // This field references another type -- flatten its fields
                flatten_graphql_fields(&field.type_name, &type_map, "", &mut response_fields, 0);

                // If we got nested fields, the root is an object/array
                if response_fields.is_empty() {
                    // Scalar-like unknown type
                    response_fields.push(SchemaField {
                        path: field.name.clone(),
                        field_type: if field.is_list {
                            ExternalType::Array
                        } else {
                            ExternalType::Unknown(field.type_name.clone())
                        },
                    });
                }
            } else {
                response_fields.push(SchemaField {
                    path: field.name.clone(),
                    field_type,
                });
            }

            endpoints.push(Endpoint {
                method: method.to_string(),
                path: field.name.clone(),
                parameters: vec![],
                response_fields,
            });
        }
    }

    Ok(ExternalSchema {
        format: SchemaFormat::GraphQl,
        endpoints,
    })
}

/// Flatten fields from a GraphQL object type into dot-path notation.
fn flatten_graphql_fields(
    type_name: &str,
    type_map: &BTreeMap<&str, &GqlTypeDef>,
    prefix: &str,
    fields: &mut Vec<SchemaField>,
    depth: usize,
) {
    if depth > 3 {
        return; // Depth limit to prevent infinite recursion
    }

    let type_def = match type_map.get(type_name) {
        Some(t) => t,
        None => return,
    };

    for field in &type_def.fields {
        let full_path = if prefix.is_empty() {
            field.name.clone()
        } else {
            format!("{}.{}", prefix, field.name)
        };

        let scalar_type = map_graphql_scalar(&field.type_name);

        if scalar_type == ExternalType::Unknown("object".to_string()) {
            // This is a nested object type -- recurse
            fields.push(SchemaField {
                path: full_path.clone(),
                field_type: if field.is_list {
                    ExternalType::Array
                } else {
                    ExternalType::Object
                },
            });
            flatten_graphql_fields(&field.type_name, type_map, &full_path, fields, depth + 1);
        } else {
            fields.push(SchemaField {
                path: full_path,
                field_type: if field.is_list {
                    ExternalType::Array
                } else {
                    scalar_type
                },
            });
        }
    }
}

/// Parse GraphQL SDL content into type definitions.
fn parse_graphql_types(content: &str) -> Result<Vec<GqlTypeDef>, String> {
    let mut type_defs = Vec::new();
    let mut lines = content.lines().peekable();

    while let Some(line) = lines.next() {
        let trimmed = line.trim();

        // Skip comments and empty lines
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Skip blocks we don't care about
        if trimmed.starts_with("input ")
            || trimmed.starts_with("enum ")
            || trimmed.starts_with("interface ")
            || trimmed.starts_with("union ")
            || trimmed.starts_with("directive ")
            || trimmed.starts_with("scalar ")
            || trimmed.starts_with("schema ")
            || trimmed.starts_with("extend ")
        {
            // Skip until closing brace
            if trimmed.contains('{') {
                let mut brace_depth =
                    trimmed.matches('{').count() as i32 - trimmed.matches('}').count() as i32;
                while brace_depth > 0 {
                    match lines.next() {
                        Some(l) => {
                            brace_depth +=
                                l.matches('{').count() as i32 - l.matches('}').count() as i32;
                        }
                        None => break,
                    }
                }
            }
            continue;
        }

        // Match `type TypeName {` or `type TypeName implements Interface {`
        if let Some(rest) = trimmed.strip_prefix("type ") {
            // Extract the type name (before `{`, `implements`, or whitespace)
            let name = rest
                .split(['{', ' '])
                .next()
                .unwrap_or("")
                .trim()
                .to_string();

            if name.is_empty() {
                continue;
            }

            // Find the opening brace
            let has_open_brace = trimmed.contains('{');
            if !has_open_brace {
                // Brace might be on the next line
                while let Some(next) = lines.peek() {
                    if next.trim().is_empty() || next.trim().starts_with('#') {
                        lines.next();
                        continue;
                    }
                    break;
                }
                if let Some(next) = lines.peek() {
                    if !next.contains('{') {
                        continue; // No opening brace found, skip
                    }
                    lines.next(); // consume the brace line
                }
            }

            // Parse fields until closing brace
            let mut fields = Vec::new();
            let mut brace_depth: i32 = 1;

            // Account for closing brace on same line as opening
            if has_open_brace && trimmed.contains('}') {
                // Single-line type definition: type X { field: Type }
                let inner = extract_between_braces(trimmed);
                for field_str in inner.split(',') {
                    if let Some(f) = parse_graphql_field(field_str.trim()) {
                        fields.push(f);
                    }
                }
                type_defs.push(GqlTypeDef { name, fields });
                continue;
            }

            for line in lines.by_ref() {
                let field_trimmed = line.trim();

                brace_depth += field_trimmed.matches('{').count() as i32
                    - field_trimmed.matches('}').count() as i32;

                if brace_depth <= 0 {
                    break;
                }

                // Skip comments and empty lines within the type body
                if field_trimmed.is_empty() || field_trimmed.starts_with('#') {
                    continue;
                }

                if let Some(f) = parse_graphql_field(field_trimmed) {
                    fields.push(f);
                }
            }

            type_defs.push(GqlTypeDef { name, fields });
        }
    }

    Ok(type_defs)
}

/// Extract content between the first `{` and last `}`.
fn extract_between_braces(s: &str) -> &str {
    let start = s.find('{').map(|i| i + 1).unwrap_or(0);
    let end = s.rfind('}').unwrap_or(s.len());
    if start < end {
        &s[start..end]
    } else {
        ""
    }
}

/// Parse a single GraphQL field line like `fieldName: Type` or `fieldName(args): Type`.
fn parse_graphql_field(line: &str) -> Option<GqlField> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') || line == "{" || line == "}" {
        return None;
    }

    // Remove inline comments
    let line = line.split('#').next().unwrap_or(line).trim();

    // Find the colon separating name from type
    // Handle arguments: `fieldName(arg: Type): ReturnType`
    let colon_pos = if let Some(paren_pos) = line.find('(') {
        // Skip past the closing paren to find the colon
        let after_paren = line[paren_pos..].find(')')?;
        let after_close = paren_pos + after_paren + 1;
        line[after_close..].find(':').map(|p| p + after_close)
    } else {
        line.find(':')
    };

    let colon_pos = colon_pos?;

    let name_part = if let Some(paren_pos) = line.find('(') {
        if paren_pos < colon_pos {
            line[..paren_pos].trim()
        } else {
            line[..colon_pos].trim()
        }
    } else {
        line[..colon_pos].trim()
    };

    if name_part.is_empty() {
        return None;
    }

    let type_part = line[colon_pos + 1..].trim();
    // Strip trailing commas or semicolons (shouldn't be common, but defensive)
    let type_part = type_part.trim_end_matches([',', ';']);
    let type_part = type_part.trim();

    // Parse the type: strip `!`, detect `[...]`
    let (type_name, is_list) = parse_graphql_type_ref(type_part);

    Some(GqlField {
        name: name_part.to_string(),
        type_name,
        is_list,
    })
}

/// Parse a GraphQL type reference, handling `!`, `[...]`.
/// Returns (base_type_name, is_list).
fn parse_graphql_type_ref(type_str: &str) -> (String, bool) {
    let s = type_str.trim();
    // Strip all `!` (non-null markers)
    let s = s.replace('!', "");
    let s = s.trim();

    if s.starts_with('[') && s.ends_with(']') {
        let inner = &s[1..s.len() - 1];
        let inner = inner.replace('!', "");
        (inner.trim().to_string(), true)
    } else {
        (s.to_string(), false)
    }
}

/// Map a GraphQL scalar name to ExternalType.
fn map_graphql_scalar(type_name: &str) -> ExternalType {
    match type_name {
        "String" => ExternalType::String,
        "Int" => ExternalType::Integer,
        "Float" => ExternalType::Number,
        "Boolean" => ExternalType::Boolean,
        "ID" => ExternalType::String,
        _ => ExternalType::Unknown("object".to_string()), // Assume non-scalar is object type
    }
}

// ---------------------------------------------------------------------------
// SQL DDL introspection
// ---------------------------------------------------------------------------

/// Parse SQL DDL (CREATE TABLE statements) and extract table structures.
///
/// This is a simple regex-free line-based parser that handles:
/// - `CREATE TABLE table_name (col1 type1, col2 type2, ...);`
/// - Case-insensitive SQL keywords
/// - Common type aliases across Postgres, MySQL, SQLite
/// - Parenthesized precision like `DECIMAL(10,2)`, `VARCHAR(255)`
fn introspect_sql_ddl(schema_path: &Path) -> Result<ExternalSchema, String> {
    let content = std::fs::read_to_string(schema_path).map_err(|e| {
        format!(
            "could not read schema file '{}': {}",
            schema_path.display(),
            e
        )
    })?;

    let tables = parse_sql_tables(&content)?;

    if tables.is_empty() {
        return Err(format!(
            "no CREATE TABLE statements found in '{}'",
            schema_path.display()
        ));
    }

    let endpoints = tables
        .into_iter()
        .map(|table| {
            let response_fields = table
                .columns
                .into_iter()
                .map(|col| SchemaField {
                    path: col.name,
                    field_type: map_sql_type(&col.sql_type),
                })
                .collect();

            Endpoint {
                method: "QUERY".to_string(),
                path: table.name,
                parameters: vec![],
                response_fields,
            }
        })
        .collect();

    Ok(ExternalSchema {
        format: SchemaFormat::SqlDdl,
        endpoints,
    })
}

/// A parsed SQL table definition.
#[derive(Debug)]
struct SqlTable {
    name: String,
    columns: Vec<SqlColumn>,
}

/// A parsed SQL column.
#[derive(Debug)]
struct SqlColumn {
    name: String,
    sql_type: String,
}

/// Parse SQL DDL content into table definitions.
fn parse_sql_tables(content: &str) -> Result<Vec<SqlTable>, String> {
    let mut tables = Vec::new();

    // Normalize: collapse the entire content into a single string with whitespace simplified.
    // Then find CREATE TABLE ... (...) patterns.
    let normalized = content
        .lines()
        .map(|line| {
            // Remove SQL single-line comments
            let line = line.split("--").next().unwrap_or(line);
            line.trim()
        })
        .collect::<Vec<_>>()
        .join(" ");

    // Find each CREATE TABLE statement
    let upper = normalized.to_uppercase();
    let mut search_pos = 0;

    while let Some(ct_pos) = upper[search_pos..].find("CREATE TABLE") {
        let abs_pos = search_pos + ct_pos;
        let after_create = abs_pos + "CREATE TABLE".len();

        // Handle optional IF NOT EXISTS
        let rest_upper = &upper[after_create..];
        let rest_orig = &normalized[after_create..];
        let (name_start_upper, name_start_orig) =
            if rest_upper.trim_start().starts_with("IF NOT EXISTS") {
                let skip = rest_upper.find("IF NOT EXISTS").unwrap() + "IF NOT EXISTS".len();
                (&rest_upper[skip..], &rest_orig[skip..])
            } else {
                (rest_upper, rest_orig)
            };

        // Extract table name
        let name_trimmed = name_start_orig.trim_start();
        let name_end = name_trimmed
            .find(|c: char| c == '(' || c.is_whitespace())
            .unwrap_or(name_trimmed.len());
        let table_name = name_trimmed[..name_end].trim();

        // Strip optional schema prefix and quoting
        let table_name = strip_sql_quoting(table_name);
        let table_name = table_name
            .rsplit('.')
            .next()
            .unwrap_or(&table_name)
            .to_string();
        let table_name = strip_sql_quoting(&table_name);

        if table_name.is_empty() {
            search_pos = after_create;
            continue;
        }

        // Find the opening paren
        let _ = name_start_upper; // used for position tracking
        let paren_search = &normalized[after_create..];
        let open_paren = match paren_search.find('(') {
            Some(p) => after_create + p,
            None => {
                search_pos = after_create;
                continue;
            }
        };

        // Find matching closing paren
        let close_paren = match find_matching_paren(&normalized, open_paren) {
            Some(p) => p,
            None => {
                search_pos = after_create;
                continue;
            }
        };

        let columns_str = &normalized[open_paren + 1..close_paren];
        let columns = parse_sql_columns(columns_str);

        tables.push(SqlTable {
            name: table_name,
            columns,
        });

        search_pos = close_paren + 1;
    }

    Ok(tables)
}

/// Find the matching closing parenthesis, handling nested parens.
fn find_matching_paren(s: &str, open_pos: usize) -> Option<usize> {
    let mut depth = 0;
    for (i, ch) in s[open_pos..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open_pos + i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Strip SQL quoting: `"name"` or `` `name` `` or `[name]` -> `name`.
fn strip_sql_quoting(name: &str) -> String {
    let name = name.trim();
    if (name.starts_with('"') && name.ends_with('"'))
        || (name.starts_with('`') && name.ends_with('`'))
        || (name.starts_with('[') && name.ends_with(']'))
    {
        name[1..name.len() - 1].to_string()
    } else {
        name.to_string()
    }
}

/// Parse column definitions from the content between CREATE TABLE parentheses.
fn parse_sql_columns(columns_str: &str) -> Vec<SqlColumn> {
    let mut columns = Vec::new();

    // Split by comma, but respect parenthesized precision (e.g., DECIMAL(10,2))
    let parts = split_sql_columns(columns_str);

    for part in parts {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }

        let upper = trimmed.to_uppercase();

        // Skip constraints: PRIMARY KEY, UNIQUE, CHECK, FOREIGN KEY, CONSTRAINT, INDEX
        if upper.starts_with("PRIMARY KEY")
            || upper.starts_with("UNIQUE")
            || upper.starts_with("CHECK")
            || upper.starts_with("FOREIGN KEY")
            || upper.starts_with("CONSTRAINT")
            || upper.starts_with("INDEX")
            || upper.starts_with("KEY ")
        {
            continue;
        }

        // Parse: column_name type_name [rest...]
        let tokens: Vec<&str> = trimmed.splitn(3, |c: char| c.is_whitespace()).collect();
        if tokens.len() < 2 {
            continue;
        }

        let col_name = strip_sql_quoting(tokens[0]);

        // The type might include precision: VARCHAR(255), DECIMAL(10,2)
        // Reconstruct the type from tokens[1] onward, stopping at keywords
        let type_str = tokens[1..].join(" ");
        let sql_type = extract_sql_base_type(&type_str);

        if !col_name.is_empty() && !sql_type.is_empty() {
            columns.push(SqlColumn {
                name: col_name,
                sql_type,
            });
        }
    }

    columns
}

/// Split column definitions by comma, respecting parenthesized expressions.
fn split_sql_columns(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0;

    for ch in s.chars() {
        match ch {
            '(' => {
                depth += 1;
                current.push(ch);
            }
            ')' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                parts.push(current.clone());
                current.clear();
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if !current.trim().is_empty() {
        parts.push(current);
    }

    parts
}

/// Extract the base SQL type name, stripping precision and trailing keywords.
fn extract_sql_base_type(type_str: &str) -> String {
    let type_str = type_str.trim();
    let upper = type_str.to_uppercase();

    // Handle DOUBLE PRECISION as a two-word type
    if upper.starts_with("DOUBLE PRECISION") || upper.starts_with("DOUBLE  PRECISION") {
        return "DOUBLE PRECISION".to_string();
    }

    // Handle CHARACTER VARYING
    if upper.starts_with("CHARACTER VARYING") {
        return "VARCHAR".to_string();
    }

    // Take the first token (the type name)
    let first_token_end = type_str
        .find(|c: char| c.is_whitespace() || c == '(')
        .unwrap_or(type_str.len());

    type_str[..first_token_end].to_uppercase()
}

/// Map a SQL type string to ExternalType.
fn map_sql_type(sql_type: &str) -> ExternalType {
    let upper = sql_type.to_uppercase();
    match upper.as_str() {
        "VARCHAR" | "TEXT" | "CHAR" | "CHARACTER" | "NVARCHAR" | "NCHAR" | "NTEXT" | "CLOB"
        | "TINYTEXT" | "MEDIUMTEXT" | "LONGTEXT" => ExternalType::String,

        "INTEGER" | "INT" | "BIGINT" | "SMALLINT" | "TINYINT" | "MEDIUMINT" | "SERIAL"
        | "BIGSERIAL" | "SMALLSERIAL" | "INT2" | "INT4" | "INT8" => ExternalType::Integer,

        "DECIMAL" | "NUMERIC" | "REAL" | "FLOAT" | "DOUBLE PRECISION" | "DOUBLE" | "FLOAT4"
        | "FLOAT8" | "MONEY" => ExternalType::Number,

        "BOOLEAN" | "BOOL" | "BIT" => ExternalType::Boolean,

        "TIMESTAMP" | "TIMESTAMPTZ" | "DATE" | "TIME" | "TIMETZ" | "DATETIME" | "DATETIME2"
        | "SMALLDATETIME" | "INTERVAL" => ExternalType::String,

        "JSON" | "JSONB" => ExternalType::Object,

        "BYTEA" | "BLOB" | "BINARY" | "VARBINARY" | "IMAGE" => {
            ExternalType::Unknown("binary".to_string())
        }

        "UUID" => ExternalType::String,

        "ARRAY" => ExternalType::Array,

        _ => ExternalType::Unknown(sql_type.to_string()),
    }
}

// ---------------------------------------------------------------------------
// Degraded schema (no schema_ref)
// ---------------------------------------------------------------------------

/// Build an ExternalSchema for a source with no schema_ref (degraded mode).
/// Produces a minimal schema based on protocol metadata.
#[allow(dead_code)]
pub fn degraded_schema(source: &tenor_interchange::SourceConstruct) -> ExternalSchema {
    let endpoints = match source.protocol.as_str() {
        "http" => {
            if let Some(base_url) = source.fields.get("base_url") {
                vec![Endpoint {
                    method: "GET".to_string(),
                    path: format!("{}/{{...}}", base_url),
                    parameters: vec![],
                    response_fields: vec![],
                }]
            } else {
                vec![]
            }
        }
        "database" => {
            vec![Endpoint {
                method: "QUERY".to_string(),
                path: "{table}.{column}".to_string(),
                parameters: vec![],
                response_fields: vec![],
            }]
        }
        _ => vec![],
    };

    ExternalSchema {
        format: SchemaFormat::OpenApi3, // placeholder
        endpoints,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_path_parameters() {
        assert_eq!(
            extract_path_parameters("/orders/{id}/items/{item_id}"),
            vec!["id", "item_id"]
        );
        assert_eq!(extract_path_parameters("/health"), Vec::<String>::new());
        assert_eq!(extract_path_parameters("/{org}/repos"), vec!["org"]);
    }

    #[test]
    fn test_parse_external_type() {
        assert_eq!(parse_external_type("integer"), ExternalType::Integer);
        assert_eq!(parse_external_type("number"), ExternalType::Number);
        assert_eq!(parse_external_type("string"), ExternalType::String);
        assert_eq!(parse_external_type("boolean"), ExternalType::Boolean);
        assert_eq!(
            parse_external_type(""),
            ExternalType::Unknown("unspecified".to_string())
        );
    }

    #[test]
    fn test_introspect_openapi_minimal() {
        let dir = tempfile::tempdir().unwrap();
        let schema_path = dir.path().join("api.json");
        let schema = serde_json::json!({
            "openapi": "3.0.0",
            "info": { "title": "Test", "version": "1.0" },
            "paths": {
                "/orders/{id}": {
                    "get": {
                        "parameters": [],
                        "responses": {
                            "200": {
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "type": "object",
                                            "properties": {
                                                "balance": { "type": "integer" },
                                                "status": { "type": "string" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
        std::fs::write(&schema_path, serde_json::to_string_pretty(&schema).unwrap()).unwrap();

        let result = introspect_openapi(&schema_path).unwrap();
        assert_eq!(result.format, SchemaFormat::OpenApi3);
        assert_eq!(result.endpoints.len(), 1);

        let ep = &result.endpoints[0];
        assert_eq!(ep.method, "GET");
        assert_eq!(ep.path, "/orders/{id}");
        assert_eq!(ep.parameters, vec!["id"]);
        assert_eq!(ep.response_fields.len(), 2);

        let balance = ep
            .response_fields
            .iter()
            .find(|f| f.path == "balance")
            .unwrap();
        assert_eq!(balance.field_type, ExternalType::Integer);
    }

    // -----------------------------------------------------------------------
    // GraphQL SDL tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_introspect_graphql_basic() {
        let dir = tempfile::tempdir().unwrap();
        let schema_path = dir.path().join("schema.graphql");
        std::fs::write(
            &schema_path,
            r#"
type Order {
    id: ID!
    balance: Int!
    status: String
    total: Float
}

type Query {
    order(id: ID!): Order
    health: Boolean
}

type Mutation {
    cancelOrder(id: ID!): Order
}

enum OrderStatus {
    ACTIVE
    CANCELLED
}

input CreateOrderInput {
    amount: Int!
}
"#,
        )
        .unwrap();

        let result = introspect_graphql(&schema_path).unwrap();
        assert_eq!(result.format, SchemaFormat::GraphQl);

        // Should have 2 Query endpoints + 1 Mutation endpoint
        let queries: Vec<_> = result
            .endpoints
            .iter()
            .filter(|e| e.method == "QUERY")
            .collect();
        let mutations: Vec<_> = result
            .endpoints
            .iter()
            .filter(|e| e.method == "MUTATION")
            .collect();

        assert_eq!(queries.len(), 2, "expected 2 query endpoints");
        assert_eq!(mutations.len(), 1, "expected 1 mutation endpoint");

        // Check the order query has nested fields from Order type
        let order_ep = queries.iter().find(|e| e.path == "order").unwrap();
        assert!(
            !order_ep.response_fields.is_empty(),
            "order endpoint should have response fields"
        );

        // Check that Order fields are flattened
        let field_names: Vec<_> = order_ep
            .response_fields
            .iter()
            .map(|f| f.path.as_str())
            .collect();
        assert!(field_names.contains(&"id"), "should contain 'id' field");
        assert!(
            field_names.contains(&"balance"),
            "should contain 'balance' field"
        );
        assert!(
            field_names.contains(&"status"),
            "should contain 'status' field"
        );

        // Check field types
        let balance = order_ep
            .response_fields
            .iter()
            .find(|f| f.path == "balance")
            .unwrap();
        assert_eq!(balance.field_type, ExternalType::Integer);

        let status = order_ep
            .response_fields
            .iter()
            .find(|f| f.path == "status")
            .unwrap();
        assert_eq!(status.field_type, ExternalType::String);

        // The health query should have a scalar Boolean field
        let health_ep = queries.iter().find(|e| e.path == "health").unwrap();
        assert_eq!(health_ep.response_fields.len(), 1);
        assert_eq!(
            health_ep.response_fields[0].field_type,
            ExternalType::Boolean
        );
    }

    #[test]
    fn test_introspect_graphql_nested_objects() {
        let dir = tempfile::tempdir().unwrap();
        let schema_path = dir.path().join("nested.graphql");
        std::fs::write(
            &schema_path,
            r#"
type Money {
    amount: Float!
    currency: String!
}

type Order {
    id: ID!
    balance: Money!
}

type Query {
    order: Order
}
"#,
        )
        .unwrap();

        let result = introspect_graphql(&schema_path).unwrap();
        let ep = &result.endpoints[0];
        assert_eq!(ep.path, "order");

        let field_paths: Vec<_> = ep.response_fields.iter().map(|f| f.path.as_str()).collect();

        // Should have: id (String from ID), balance (Object from Money), balance.amount, balance.currency
        assert!(field_paths.contains(&"id"));
        assert!(field_paths.contains(&"balance"));
        assert!(field_paths.contains(&"balance.amount"));
        assert!(field_paths.contains(&"balance.currency"));

        // balance should be Object type
        let balance = ep
            .response_fields
            .iter()
            .find(|f| f.path == "balance")
            .unwrap();
        assert_eq!(balance.field_type, ExternalType::Object);

        // balance.amount should be Number (Float)
        let amount = ep
            .response_fields
            .iter()
            .find(|f| f.path == "balance.amount")
            .unwrap();
        assert_eq!(amount.field_type, ExternalType::Number);
    }

    #[test]
    fn test_introspect_graphql_list_types() {
        let dir = tempfile::tempdir().unwrap();
        let schema_path = dir.path().join("lists.graphql");
        std::fs::write(
            &schema_path,
            r#"
type Query {
    orders: [Order!]!
    tags: [String]
}

type Order {
    id: ID!
    name: String
}
"#,
        )
        .unwrap();

        let result = introspect_graphql(&schema_path).unwrap();
        let orders_ep = result
            .endpoints
            .iter()
            .find(|e| e.path == "orders")
            .unwrap();

        // orders returns a list of Order, so it should have nested fields
        let field_paths: Vec<_> = orders_ep
            .response_fields
            .iter()
            .map(|f| f.path.as_str())
            .collect();
        assert!(field_paths.contains(&"id"));
        assert!(field_paths.contains(&"name"));

        let tags_ep = result.endpoints.iter().find(|e| e.path == "tags").unwrap();
        assert_eq!(tags_ep.response_fields.len(), 1);
        assert_eq!(tags_ep.response_fields[0].field_type, ExternalType::Array);
    }

    #[test]
    fn test_graphql_scalar_type_mapping() {
        assert_eq!(map_graphql_scalar("String"), ExternalType::String);
        assert_eq!(map_graphql_scalar("Int"), ExternalType::Integer);
        assert_eq!(map_graphql_scalar("Float"), ExternalType::Number);
        assert_eq!(map_graphql_scalar("Boolean"), ExternalType::Boolean);
        assert_eq!(map_graphql_scalar("ID"), ExternalType::String);
        assert_eq!(
            map_graphql_scalar("CustomType"),
            ExternalType::Unknown("object".to_string())
        );
    }

    #[test]
    fn test_parse_graphql_type_ref() {
        assert_eq!(
            parse_graphql_type_ref("String"),
            ("String".to_string(), false)
        );
        assert_eq!(
            parse_graphql_type_ref("String!"),
            ("String".to_string(), false)
        );
        assert_eq!(
            parse_graphql_type_ref("[String]"),
            ("String".to_string(), true)
        );
        assert_eq!(
            parse_graphql_type_ref("[String!]!"),
            ("String".to_string(), true)
        );
        assert_eq!(
            parse_graphql_type_ref("[Order!]!"),
            ("Order".to_string(), true)
        );
    }

    // -----------------------------------------------------------------------
    // SQL DDL tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_introspect_sql_ddl_basic() {
        let dir = tempfile::tempdir().unwrap();
        let schema_path = dir.path().join("schema.sql");
        std::fs::write(
            &schema_path,
            r#"
CREATE TABLE orders (
    id SERIAL PRIMARY KEY,
    customer_name VARCHAR(255) NOT NULL,
    amount DECIMAL(10,2),
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP NOT NULL,
    metadata JSONB
);

CREATE TABLE line_items (
    id BIGSERIAL PRIMARY KEY,
    order_id INTEGER REFERENCES orders(id),
    description TEXT,
    quantity INT NOT NULL,
    price NUMERIC(12,4)
);
"#,
        )
        .unwrap();

        let result = introspect_sql_ddl(&schema_path).unwrap();
        assert_eq!(result.format, SchemaFormat::SqlDdl);
        assert_eq!(result.endpoints.len(), 2, "expected 2 table endpoints");

        // Check orders table
        let orders = result
            .endpoints
            .iter()
            .find(|e| e.path == "orders")
            .unwrap();
        assert_eq!(orders.method, "QUERY");

        let col_names: Vec<_> = orders
            .response_fields
            .iter()
            .map(|f| f.path.as_str())
            .collect();
        assert!(col_names.contains(&"id"));
        assert!(col_names.contains(&"customer_name"));
        assert!(col_names.contains(&"amount"));
        assert!(col_names.contains(&"is_active"));
        assert!(col_names.contains(&"created_at"));
        assert!(col_names.contains(&"metadata"));

        // Check types
        let id_field = orders
            .response_fields
            .iter()
            .find(|f| f.path == "id")
            .unwrap();
        assert_eq!(id_field.field_type, ExternalType::Integer);

        let name_field = orders
            .response_fields
            .iter()
            .find(|f| f.path == "customer_name")
            .unwrap();
        assert_eq!(name_field.field_type, ExternalType::String);

        let amount_field = orders
            .response_fields
            .iter()
            .find(|f| f.path == "amount")
            .unwrap();
        assert_eq!(amount_field.field_type, ExternalType::Number);

        let active_field = orders
            .response_fields
            .iter()
            .find(|f| f.path == "is_active")
            .unwrap();
        assert_eq!(active_field.field_type, ExternalType::Boolean);

        let timestamp_field = orders
            .response_fields
            .iter()
            .find(|f| f.path == "created_at")
            .unwrap();
        assert_eq!(timestamp_field.field_type, ExternalType::String);

        let json_field = orders
            .response_fields
            .iter()
            .find(|f| f.path == "metadata")
            .unwrap();
        assert_eq!(json_field.field_type, ExternalType::Object);

        // Check line_items table
        let items = result
            .endpoints
            .iter()
            .find(|e| e.path == "line_items")
            .unwrap();
        assert_eq!(items.response_fields.len(), 5);
    }

    #[test]
    fn test_introspect_sql_ddl_if_not_exists() {
        let dir = tempfile::tempdir().unwrap();
        let schema_path = dir.path().join("conditional.sql");
        std::fs::write(
            &schema_path,
            r#"
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY,
    email VARCHAR(255),
    age INT
);
"#,
        )
        .unwrap();

        let result = introspect_sql_ddl(&schema_path).unwrap();
        assert_eq!(result.endpoints.len(), 1);
        assert_eq!(result.endpoints[0].path, "users");
        assert_eq!(result.endpoints[0].response_fields.len(), 3);
    }

    #[test]
    fn test_introspect_sql_ddl_constraints_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let schema_path = dir.path().join("constraints.sql");
        std::fs::write(
            &schema_path,
            r#"
CREATE TABLE accounts (
    id INT,
    name TEXT,
    balance DECIMAL(10,2),
    CONSTRAINT pk_accounts PRIMARY KEY (id),
    UNIQUE (name),
    CHECK (balance >= 0)
);
"#,
        )
        .unwrap();

        let result = introspect_sql_ddl(&schema_path).unwrap();
        let accounts = &result.endpoints[0];
        // Only id, name, balance -- not the constraint lines
        assert_eq!(
            accounts.response_fields.len(),
            3,
            "constraints should be skipped"
        );
    }

    #[test]
    fn test_sql_type_mapping() {
        // String types
        assert_eq!(map_sql_type("VARCHAR"), ExternalType::String);
        assert_eq!(map_sql_type("TEXT"), ExternalType::String);
        assert_eq!(map_sql_type("CHAR"), ExternalType::String);

        // Integer types
        assert_eq!(map_sql_type("INTEGER"), ExternalType::Integer);
        assert_eq!(map_sql_type("INT"), ExternalType::Integer);
        assert_eq!(map_sql_type("BIGINT"), ExternalType::Integer);
        assert_eq!(map_sql_type("SMALLINT"), ExternalType::Integer);
        assert_eq!(map_sql_type("SERIAL"), ExternalType::Integer);
        assert_eq!(map_sql_type("BIGSERIAL"), ExternalType::Integer);

        // Number types
        assert_eq!(map_sql_type("DECIMAL"), ExternalType::Number);
        assert_eq!(map_sql_type("NUMERIC"), ExternalType::Number);
        assert_eq!(map_sql_type("REAL"), ExternalType::Number);
        assert_eq!(map_sql_type("FLOAT"), ExternalType::Number);
        assert_eq!(map_sql_type("DOUBLE PRECISION"), ExternalType::Number);

        // Boolean
        assert_eq!(map_sql_type("BOOLEAN"), ExternalType::Boolean);
        assert_eq!(map_sql_type("BOOL"), ExternalType::Boolean);

        // Date/time as strings
        assert_eq!(map_sql_type("TIMESTAMP"), ExternalType::String);
        assert_eq!(map_sql_type("TIMESTAMPTZ"), ExternalType::String);
        assert_eq!(map_sql_type("DATE"), ExternalType::String);
        assert_eq!(map_sql_type("TIME"), ExternalType::String);

        // JSON
        assert_eq!(map_sql_type("JSON"), ExternalType::Object);
        assert_eq!(map_sql_type("JSONB"), ExternalType::Object);

        // UUID
        assert_eq!(map_sql_type("UUID"), ExternalType::String);
    }

    #[test]
    fn test_sql_extract_base_type() {
        assert_eq!(extract_sql_base_type("VARCHAR(255)"), "VARCHAR");
        assert_eq!(extract_sql_base_type("DECIMAL(10,2)"), "DECIMAL");
        assert_eq!(extract_sql_base_type("INT"), "INT");
        assert_eq!(
            extract_sql_base_type("DOUBLE PRECISION"),
            "DOUBLE PRECISION"
        );
        assert_eq!(extract_sql_base_type("CHARACTER VARYING"), "VARCHAR");
        assert_eq!(extract_sql_base_type("TEXT NOT NULL"), "TEXT");
    }

    #[test]
    fn test_strip_sql_quoting() {
        assert_eq!(strip_sql_quoting("\"my_table\""), "my_table");
        assert_eq!(strip_sql_quoting("`my_table`"), "my_table");
        assert_eq!(strip_sql_quoting("[my_table]"), "my_table");
        assert_eq!(strip_sql_quoting("plain"), "plain");
    }

    // -----------------------------------------------------------------------
    // Auto-detection tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_detect_and_introspect_graphql_by_extension() {
        let dir = tempfile::tempdir().unwrap();
        let schema_path = dir.path().join("api.graphql");
        std::fs::write(
            &schema_path,
            r#"
type Query {
    version: String
}
"#,
        )
        .unwrap();

        let result = detect_and_introspect(&schema_path).unwrap();
        assert_eq!(result.format, SchemaFormat::GraphQl);
    }

    #[test]
    fn test_detect_and_introspect_gql_extension() {
        let dir = tempfile::tempdir().unwrap();
        let schema_path = dir.path().join("api.gql");
        std::fs::write(
            &schema_path,
            r#"
type Query {
    version: String
}
"#,
        )
        .unwrap();

        let result = detect_and_introspect(&schema_path).unwrap();
        assert_eq!(result.format, SchemaFormat::GraphQl);
    }

    #[test]
    fn test_detect_and_introspect_sql_by_extension() {
        let dir = tempfile::tempdir().unwrap();
        let schema_path = dir.path().join("schema.sql");
        std::fs::write(&schema_path, "CREATE TABLE test (id INT, name TEXT);\n").unwrap();

        let result = detect_and_introspect(&schema_path).unwrap();
        assert_eq!(result.format, SchemaFormat::SqlDdl);
    }

    #[test]
    fn test_detect_and_introspect_openapi_json() {
        let dir = tempfile::tempdir().unwrap();
        let schema_path = dir.path().join("api.json");
        let schema = serde_json::json!({
            "openapi": "3.0.0",
            "info": { "title": "Test", "version": "1.0" },
            "paths": {}
        });
        std::fs::write(&schema_path, serde_json::to_string_pretty(&schema).unwrap()).unwrap();

        let result = detect_and_introspect(&schema_path).unwrap();
        assert_eq!(result.format, SchemaFormat::OpenApi3);
    }

    #[test]
    fn test_introspect_schema_extension_override() {
        // Even with protocol "http", a .graphql file should use the GraphQL parser
        let dir = tempfile::tempdir().unwrap();
        let schema_path = dir.path().join("schema.graphql");
        std::fs::write(
            &schema_path,
            r#"
type Query {
    ping: Boolean
}
"#,
        )
        .unwrap();

        let result = introspect_schema("http", &schema_path).unwrap();
        assert_eq!(result.format, SchemaFormat::GraphQl);
    }

    #[test]
    fn test_introspect_schema_sql_extension_override() {
        // Even with protocol "http", a .sql file should use the SQL parser
        let dir = tempfile::tempdir().unwrap();
        let schema_path = dir.path().join("schema.sql");
        std::fs::write(&schema_path, "CREATE TABLE items (id INT, name TEXT);\n").unwrap();

        let result = introspect_schema("http", &schema_path).unwrap();
        assert_eq!(result.format, SchemaFormat::SqlDdl);
    }

    #[test]
    fn test_introspect_schema_protocol_fallback() {
        // Without a recognized extension, protocol hint is used
        let dir = tempfile::tempdir().unwrap();
        let schema_path = dir.path().join("schema.txt");
        std::fs::write(&schema_path, "CREATE TABLE items (id INT, name TEXT);\n").unwrap();

        let result = introspect_schema("database", &schema_path).unwrap();
        assert_eq!(result.format, SchemaFormat::SqlDdl);
    }
}
