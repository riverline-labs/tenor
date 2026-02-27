//! Schema introspection for Source declarations.
//!
//! When a Source has a `schema_ref` field, this module fetches and parses
//! the external schema to extract endpoints, field types, and structure.
//! Currently supports OpenAPI 3.x (JSON).

use std::fmt;
use std::path::Path;

/// An introspected external schema.
#[derive(Debug, Clone)]
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
/// The `schema_path` can be a local file path or a URL. For now, only local
/// files are supported. URLs in schema_ref are treated as local path hints
/// (the user can pass the actual file via --environment).
pub fn introspect_schema(protocol: &str, schema_path: &Path) -> Result<ExternalSchema, String> {
    match protocol {
        "http" | "graphql" => introspect_openapi(schema_path),
        "database" => {
            // Database schemas via SQL DDL are not yet supported
            Err("database schema introspection not yet implemented".to_string())
        }
        "grpc" => Err("gRPC proto introspection not yet implemented".to_string()),
        _ => Err(format!(
            "schema introspection not supported for protocol '{}'",
            protocol
        )),
    }
}

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

/// Build an ExternalSchema for a source with no schema_ref (degraded mode).
/// Produces a minimal schema based on protocol metadata.
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
}
