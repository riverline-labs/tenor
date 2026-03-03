//! Document state management for open files in the editor.

use std::collections::HashMap;
use std::path::PathBuf;

/// Tracks which documents are currently open in the editor.
pub struct DocumentState {
    documents: HashMap<String, DocumentInfo>,
}

/// Information about a single open document.
pub struct DocumentInfo {
    /// File system path for this document.
    pub path: PathBuf,
    /// Editor-reported version number.
    pub version: i32,
    /// Latest content from the editor (for semantic tokens).
    pub content: String,
}

impl Default for DocumentState {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentState {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
        }
    }

    /// Track a newly opened document.
    pub fn open(&mut self, uri: &str, path: PathBuf, version: i32, content: String) {
        self.documents.insert(
            uri.to_owned(),
            DocumentInfo {
                path,
                version,
                content,
            },
        );
    }

    /// Update content for an already-open document.
    pub fn change(&mut self, uri: &str, version: i32, content: String) {
        if let Some(doc) = self.documents.get_mut(uri) {
            doc.version = version;
            doc.content = content;
        }
    }

    /// Remove a closed document from tracking.
    pub fn close(&mut self, uri: &str) {
        self.documents.remove(uri);
    }

    /// Get information about an open document.
    pub fn get(&self, uri: &str) -> Option<&DocumentInfo> {
        self.documents.get(uri)
    }
}
