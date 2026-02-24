//! Tenor Language Server Protocol implementation for IDE integration.
//!
//! Provides diagnostics on save, semantic token highlighting,
//! agent capabilities extraction, and document state management
//! for open files. Connects to editors via the `tenor lsp` CLI
//! subcommand over stdio.

pub mod agent_capabilities;
pub mod completion;
pub mod diagnostics;
pub mod document;
pub mod hover;
pub mod navigation;
pub mod semantic_tokens;
pub mod server;

/// Run the LSP server over stdio. This is the public entry point
/// called by `tenor lsp`.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    server::run()
}
