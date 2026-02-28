//! SQL DDL introspection.

use std::path::Path;

use super::{Endpoint, ExternalSchema, ExternalType, SchemaField, SchemaFormat};

/// Parse SQL DDL (CREATE TABLE statements) and extract table structures.
///
/// This is a simple regex-free line-based parser that handles:
/// - `CREATE TABLE table_name (col1 type1, col2 type2, ...);`
/// - Case-insensitive SQL keywords
/// - Common type aliases across Postgres, MySQL, SQLite
/// - Parenthesized precision like `DECIMAL(10,2)`, `VARCHAR(255)`
pub(crate) fn introspect_sql_ddl(schema_path: &Path) -> Result<ExternalSchema, String> {
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

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_malformed_sql_ddl_no_tables() {
        let dir = tempfile::tempdir().unwrap();
        let schema_path = dir.path().join("bad.sql");
        std::fs::write(
            &schema_path,
            r#"
SELECT * FROM nonexistent;
INSERT INTO nowhere VALUES (1, 2, 3);
ALTER TABLE phantom ADD COLUMN ghost INT;
"#,
        )
        .unwrap();

        let result = introspect_sql_ddl(&schema_path);
        assert!(result.is_err(), "SQL with no CREATE TABLE should fail");
        let err = result.unwrap_err();
        assert!(
            err.contains("no CREATE TABLE"),
            "error should mention no CREATE TABLE, got: {}",
            err
        );
    }
}
