//! OpenAPI 3.x introspection.

use std::path::Path;

use super::{parse_external_type, Endpoint, ExternalSchema, SchemaField, SchemaFormat};

/// Parse an OpenAPI 3.x JSON document and extract endpoints with response shapes.
pub(crate) fn introspect_openapi(schema_path: &Path) -> Result<ExternalSchema, String> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connect::introspect::ExternalType;

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

    #[test]
    fn test_malformed_openapi_json_truncated() {
        let dir = tempfile::tempdir().unwrap();
        let schema_path = dir.path().join("bad.json");
        std::fs::write(&schema_path, r#"{"openapi": "3.0.0", "paths": {"#).unwrap();

        let result = introspect_openapi(&schema_path);
        assert!(result.is_err(), "truncated JSON should fail");
        let err = result.unwrap_err();
        assert!(
            err.contains("invalid JSON"),
            "error should mention invalid JSON, got: {}",
            err
        );
    }

    #[test]
    fn test_invalid_openapi_structure_missing_version() {
        let dir = tempfile::tempdir().unwrap();
        let schema_path = dir.path().join("no_version.json");
        let json = serde_json::json!({
            "info": { "title": "Test", "version": "1.0" },
            "paths": { "/test": { "get": {} } }
        });
        std::fs::write(&schema_path, serde_json::to_string(&json).unwrap()).unwrap();

        let result = introspect_openapi(&schema_path);
        assert!(result.is_err(), "missing openapi field should fail");
        let err = result.unwrap_err();
        assert!(
            err.contains("unsupported OpenAPI version"),
            "error should mention unsupported version, got: {}",
            err
        );
    }

    #[test]
    fn test_empty_openapi_schema_no_paths() {
        let dir = tempfile::tempdir().unwrap();
        let schema_path = dir.path().join("empty_paths.json");
        let json = serde_json::json!({
            "openapi": "3.0.0",
            "info": { "title": "Empty API", "version": "1.0" },
            "paths": {}
        });
        std::fs::write(&schema_path, serde_json::to_string_pretty(&json).unwrap()).unwrap();

        let result = introspect_openapi(&schema_path);
        assert!(result.is_ok(), "empty paths should succeed");
        let schema = result.unwrap();
        assert_eq!(schema.format, SchemaFormat::OpenApi3);
        assert!(
            schema.endpoints.is_empty(),
            "empty paths should yield no endpoints"
        );
    }
}
