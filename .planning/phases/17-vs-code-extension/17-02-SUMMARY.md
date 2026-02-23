---
phase: 17-vs-code-extension
plan: 02
subsystem: editor-tooling
tags: [lsp, vscode, semantic-tokens, diagnostics, lsp-server, lsp-types, vscode-languageclient]

# Dependency graph
requires:
  - phase: 17-01
    provides: VS Code extension scaffold with TextMate grammar and TypeScript build pipeline
provides:
  - LSP server in tenor-lsp crate with check-on-save diagnostics via elaborate()
  - Semantic token provider with 12 token types for construct-aware highlighting
  - Document state management for open file tracking
  - `tenor lsp` CLI subcommand for stdio-based LSP server
  - VS Code language client connecting to tenor lsp over stdio
affects: [17-03-PLAN, 17-04-PLAN, 17-05-PLAN]

# Tech tracking
tech-stack:
  added: [lsp-server 0.7, lsp-types 0.97, vscode-languageclient ^9.0.0]
  patterns: [synchronous LSP server with crossbeam channels, lsp-types Uri (not Url) for v0.97, delta-encoded semantic tokens]

key-files:
  created:
    - crates/lsp/src/server.rs
    - crates/lsp/src/diagnostics.rs
    - crates/lsp/src/document.rs
    - crates/lsp/src/semantic_tokens.rs
  modified:
    - crates/lsp/Cargo.toml
    - crates/lsp/src/lib.rs
    - crates/cli/Cargo.toml
    - crates/cli/src/main.rs
    - Cargo.toml
    - editors/vscode/package.json
    - editors/vscode/src/extension.ts

key-decisions:
  - "lsp-types 0.97 uses Uri (not Url) -- uri_to_path strips file:// prefix manually since fluent_uri has no to_file_path()"
  - "Synchronous LSP server using lsp-server crate -- no async runtime, matches project architecture"
  - "Semantic tokens use lexer-based position finding with construct names from load_bundle for classification"
  - "12 semantic token types covering keyword, type, variable, property, enumMember, function, class, namespace, string, number, comment, operator"
  - "Best-effort semantic tokens: parse errors are swallowed, tokens degrade gracefully on incomplete files"

patterns-established:
  - "LSP request handling: match on method string, deserialize params, compute result, serialize response"
  - "uri_to_path: strip file:// prefix for cross-platform URI-to-path conversion"
  - "Semantic token position calculation: build line offset table, search for token text in source"

requirements-completed: [DEVX-01, DEVX-02, DEVX-03]

# Metrics
duration: 11min
completed: 2026-02-23
---

# Phase 17 Plan 02: LSP Server with Diagnostics and Semantic Tokens Summary

**Synchronous LSP server using lsp-server crate providing check-on-save diagnostics and 12-type semantic token highlighting, connected to VS Code via vscode-languageclient**

## Performance

- **Duration:** 11 min
- **Started:** 2026-02-23T04:12:24Z
- **Completed:** 2026-02-23T04:23:41Z
- **Tasks:** 2
- **Files created:** 4
- **Files modified:** 7

## Accomplishments
- LSP server starts via `tenor lsp` over stdio, responds to initialize with full capability declaration
- Diagnostics published on didOpen and didSave, cleared on didClose -- first-failing-pass behavior via elaborate()
- Semantic tokens classify construct keywords (declaration modifier), type names, fact refs, entity refs, persona refs, operation refs, field labels, and operators
- VS Code extension connects to tenor LSP server via vscode-languageclient with configurable binary path

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement LSP server core with diagnostics** - `90060f2` (feat)
2. **Task 2: Add semantic tokens and wire VS Code language client** - `348d349` (feat)

## Files Created/Modified
- `crates/lsp/src/server.rs` - LSP server main loop with initialize handshake, request/notification dispatch, semantic token handling
- `crates/lsp/src/diagnostics.rs` - Converts ElabError to LSP Diagnostic with 0-indexed line mapping
- `crates/lsp/src/document.rs` - Document state management tracking open files with content and version
- `crates/lsp/src/semantic_tokens.rs` - Token provider with lexer-based position finding and construct-aware classification
- `crates/lsp/src/lib.rs` - Re-exports server::run as public API
- `crates/lsp/Cargo.toml` - Dependencies: tenor-core, tenor-analyze, lsp-server, lsp-types, serde, serde_json
- `crates/cli/Cargo.toml` - Added tenor-lsp dependency
- `crates/cli/src/main.rs` - Added `Lsp` subcommand variant calling tenor_lsp::run()
- `Cargo.toml` - Added lsp-server and lsp-types workspace dependencies
- `editors/vscode/package.json` - Added vscode-languageclient, semanticTokenScopes, tenor.path/checkOnType config
- `editors/vscode/src/extension.ts` - Language client connecting to `tenor lsp` over stdio

## Decisions Made
- Used lsp-types 0.97 which uses `Uri` (not `Url`) backed by `fluent_uri` -- required manual URI-to-path conversion since there's no `to_file_path()` method
- Synchronous LSP server via lsp-server crate (same approach as rust-analyzer) -- no tokio/async runtime, matches project's synchronous architecture
- Semantic tokens are best-effort: parse errors are swallowed so highlighting degrades gracefully on incomplete files
- 12 semantic token types registered covering all Tenor constructs with semantic meaning
- Token position finding uses a line-offset table and substring search in the character array for accuracy

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] lsp-types 0.97 uses Uri instead of Url**
- **Found during:** Task 1
- **Issue:** Plan specified `lsp_types::Url` but lsp-types 0.97 renamed it to `lsp_types::Uri` backed by `fluent_uri`
- **Fix:** Changed all Url references to Uri, implemented manual `uri_to_path()` using string prefix stripping since `fluent_uri::Uri` has no `to_file_path()` method
- **Files modified:** crates/lsp/src/server.rs
- **Verification:** Build and clippy pass, LSP server responds correctly to initialize request
- **Committed in:** 90060f2 (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** API change in dependency required adapter code. No scope creep.

## Issues Encountered
- Pre-existing flaky test `elaborate_invalid_source_returns_400` in serve_integration.rs fails intermittently when all tests run concurrently (port conflicts between parallel server tests). Passes consistently when run in isolation or with `--test-threads=1`. Not related to this plan's changes.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- LSP server is ready for additional capabilities (go-to-definition, hover, etc. in Plan 03+)
- Semantic tokens provide foundation for richer highlighting as construct awareness grows
- VS Code extension has working language client for all future LSP features

---
*Phase: 17-vs-code-extension*
*Completed: 2026-02-23*
