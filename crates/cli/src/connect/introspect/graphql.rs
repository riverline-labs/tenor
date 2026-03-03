//! GraphQL SDL introspection.

use std::collections::BTreeMap;
use std::path::Path;

use super::{Endpoint, ExternalSchema, ExternalType, SchemaField, SchemaFormat};

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
pub(crate) fn introspect_graphql(schema_path: &Path) -> Result<ExternalSchema, String> {
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

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_malformed_graphql_sdl_syntax_error() {
        let dir = tempfile::tempdir().unwrap();
        let schema_path = dir.path().join("bad.graphql");
        std::fs::write(
            &schema_path,
            r#"
typ Query {
    broken field without colon
}
"#,
        )
        .unwrap();

        let result = introspect_graphql(&schema_path);
        assert!(result.is_err(), "malformed GraphQL SDL should fail");
        let err = result.unwrap_err();
        assert!(
            err.contains("no type definitions found"),
            "error should mention no type definitions, got: {}",
            err
        );
    }
}
