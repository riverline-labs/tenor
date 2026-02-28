//! Schema introspection for Source declarations.
//!
//! When a Source has a `schema_ref` field, this module fetches and parses
//! the external schema to extract endpoints, field types, and structure.
//! Supports OpenAPI 3.x (JSON), GraphQL SDL, and SQL DDL.

mod graphql;
mod openapi;
mod sql;

use std::fmt;
use std::path::Path;

use graphql::introspect_graphql;
use openapi::introspect_openapi;
use sql::introspect_sql_ddl;

/// An introspected external schema.
#[derive(Debug, Clone)]
pub struct ExternalSchema {
    /// The format of the schema document.
    #[allow(dead_code)] // Structural metadata; used by Debug output and future format-aware logic
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
    Unknown(std::string::String),
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
// Degraded schema (no schema_ref)
// ---------------------------------------------------------------------------

/// Build an ExternalSchema for a source with no schema_ref (degraded mode).
/// Produces a minimal schema based on protocol metadata.
#[allow(dead_code)] // Kept for connect workflows with sources that have no schema_ref
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
