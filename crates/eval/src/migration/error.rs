use std::fmt;

/// Errors that can occur during migration analysis and execution.
#[derive(Debug)]
pub enum MigrationError {
    /// Error from the diff engine.
    Diff(super::diff::DiffError),
    /// Deserialization error.
    Deserialize(String),
    /// Analysis error.
    Analysis(String),
    /// State mismatch for an entity instance.
    StateMismatch {
        entity_id: String,
        instance_id: String,
        expected: String,
        found: String,
    },
    /// Storage error.
    Storage(String),
    /// Incompatible migration.
    Incompatible(String),
}

impl fmt::Display for MigrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MigrationError::Diff(e) => write!(f, "diff error: {}", e),
            MigrationError::Deserialize(msg) => write!(f, "deserialization error: {}", msg),
            MigrationError::Analysis(msg) => write!(f, "analysis error: {}", msg),
            MigrationError::StateMismatch {
                entity_id,
                instance_id,
                expected,
                found,
            } => write!(
                f,
                "state mismatch for entity '{}' instance '{}': expected '{}', found '{}'",
                entity_id, instance_id, expected, found
            ),
            MigrationError::Storage(msg) => write!(f, "storage error: {}", msg),
            MigrationError::Incompatible(msg) => write!(f, "incompatible migration: {}", msg),
        }
    }
}

impl std::error::Error for MigrationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            MigrationError::Diff(e) => Some(e),
            _ => None,
        }
    }
}

impl From<super::diff::DiffError> for MigrationError {
    fn from(e: super::diff::DiffError) -> Self {
        MigrationError::Diff(e)
    }
}
