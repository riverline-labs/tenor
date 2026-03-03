//! Elaboration-to-diagnostic conversion with first-failing-pass stop.
//!
//! Calls `tenor_core::elaborate()` and converts `ElabError` into
//! `lsp_types::Diagnostic`. Because `elaborate()` returns on the first
//! error, diagnostics naturally stop at the first failing pass --
//! no cascading downstream errors are shown.

use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};
use std::path::Path;

/// Elaborate the file at `file_path` and return any diagnostics.
///
/// On success: empty vec (no diagnostics).
/// On `ElabError`: a single diagnostic at the error's line.
/// On other errors (file not found, etc.): a diagnostic at line 0.
pub fn compute_diagnostics(file_path: &Path) -> Vec<Diagnostic> {
    match tenor_core::elaborate::elaborate(file_path) {
        Ok(_) => Vec::new(),
        Err(e) => {
            // ElabError line is 1-indexed; LSP positions are 0-indexed.
            let line = if e.line > 0 { e.line - 1 } else { 0 };
            vec![Diagnostic {
                range: Range::new(Position::new(line, 0), Position::new(line, u32::MAX)),
                severity: Some(DiagnosticSeverity::ERROR),
                source: Some("tenor".to_string()),
                message: e.message,
                ..Default::default()
            }]
        }
    }
}
