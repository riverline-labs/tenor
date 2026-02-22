use serde::{Deserialize, Serialize};

/// An elaboration error. Matches the expected-error.json format exactly.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ElabError {
    pub pass: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub construct_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub construct_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    pub file: String,
    pub line: u32,
    pub message: String,
}

impl ElabError {
    pub fn new(
        pass: u8,
        construct_kind: Option<&str>,
        construct_id: Option<&str>,
        field: Option<&str>,
        file: &str,
        line: u32,
        message: impl Into<String>,
    ) -> Self {
        ElabError {
            pass,
            construct_kind: construct_kind.map(str::to_owned),
            construct_id: construct_id.map(str::to_owned),
            field: field.map(str::to_owned),
            file: file.to_owned(),
            line,
            message: message.into(),
        }
    }

    pub fn lex(file: &str, line: u32, message: impl Into<String>) -> Self {
        ElabError::new(0, None, None, None, file, line, message)
    }

    pub fn parse(file: &str, line: u32, message: impl Into<String>) -> Self {
        ElabError::new(0, None, None, None, file, line, message)
    }

    /// Serialize to JSON matching the expected-error.json format.
    /// The format always includes all fields (null for missing), not skip_serializing_if.
    pub fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "construct_id":   self.construct_id,
            "construct_kind": self.construct_kind,
            "field":          self.field,
            "file":           self.file,
            "line":           self.line,
            "message":        self.message,
            "pass":           self.pass,
        })
    }
}
