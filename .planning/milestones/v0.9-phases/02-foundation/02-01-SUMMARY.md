---
phase: 02-foundation
plan: 01
subsystem: infra
tags: [cargo-workspace, rust, elaborator, refactoring]

# Dependency graph
requires:
  - phase: 01-spec-completion
    provides: "Frozen v1.0 spec, 47-test conformance suite, elaborator source"
provides:
  - "Cargo workspace with tenor-core library and tenor-cli binary crates"
  - "Per-pass elaboration modules (pass1-pass6) as separate Rust files"
  - "Shared AST types in tenor-core::ast"
  - "Public elaborate() API via tenor-core::elaborate"
affects: [02-foundation, 03-cli-evaluator, 04-static-analysis]

# Tech tracking
tech-stack:
  added: [cargo-workspace]
  patterns: [per-pass-module-decomposition, ast-type-extraction, pub-use-reexport]

key-files:
  created:
    - crates/core/Cargo.toml
    - crates/core/src/lib.rs
    - crates/core/src/ast.rs
    - crates/core/src/elaborate.rs
    - crates/core/src/pass1_bundle.rs
    - crates/core/src/pass2_index.rs
    - crates/core/src/pass3_types.rs
    - crates/core/src/pass4_typecheck.rs
    - crates/core/src/pass5_validate.rs
    - crates/core/src/pass6_serialize.rs
    - crates/core/src/error.rs
    - crates/core/src/lexer.rs
    - crates/core/src/parser.rs
    - crates/cli/Cargo.toml
    - crates/cli/src/main.rs
    - crates/cli/src/runner.rs
    - crates/cli/src/tap.rs
    - crates/cli/src/ambiguity/mod.rs
    - crates/cli/src/ambiguity/api.rs
    - crates/cli/src/ambiguity/compare.rs
    - crates/cli/src/ambiguity/fixtures.rs
    - crates/cli/src/ambiguity/prompt.rs
    - crates/cli/src/ambiguity/report.rs
  modified:
    - Cargo.toml
    - Cargo.lock

key-decisions:
  - "Short directory names under crates/ (core, cli) while Cargo package names use tenor- prefix (tenor-core, tenor-cli)"
  - "AST types extracted to ast.rs with pub use re-exports from parser.rs for backward compatibility"
  - "Thin elaborate.rs orchestrator delegates to pass modules rather than re-exporting pass internals"
  - "Old elaborator/ directory fully removed (not just renamed) after conformance verification"

patterns-established:
  - "Per-pass module convention: pass{N}_{name}.rs files in tenor-core"
  - "Workspace dependency management: shared dependencies declared in workspace root"
  - "CLI-core separation: core logic in library crate, CLI commands in binary crate"

requirements-completed: [FNDN-01, FNDN-02, FNDN-03]

# Metrics
duration: ~35min
completed: 2026-02-21
---

# Phase 2 Plan 01: Cargo Workspace + Pass Module Extraction Summary

**Monolithic elaborator decomposed into Cargo workspace with tenor-core (6 per-pass modules + AST + orchestrator) and tenor-cli binary -- 47/47 conformance tests pass unchanged**

## Performance

- **Duration:** ~35 min
- **Started:** 2026-02-21T19:56:33Z
- **Completed:** 2026-02-21T20:33:10Z
- **Tasks:** 2
- **Files modified:** 27

## Accomplishments
- Extracted monolithic `elaborate.rs` (2067 lines) into 6 per-pass modules plus thin orchestrator
- Created Cargo workspace with `tenor-core` library and `tenor-cli` binary crates
- All 47 conformance tests pass unchanged through the new workspace structure
- Removed old `elaborator/` directory entirely -- single source of truth in `crates/`
- 16 unit tests in tenor-cli pass (ambiguity module, prompt builder, verdict comparator)

## Task Commits

Each task was committed atomically:

1. **Task 1: Create Cargo workspace and extract tenor-core pass modules** - `6170494` (feat)
2. **Task 2: Create tenor-cli binary crate and verify conformance** - `34595dd` (feat)

## Files Created/Modified
- `Cargo.toml` - Workspace root with members crates/core and crates/cli
- `crates/core/Cargo.toml` - Library crate with serde, serde_json dependencies
- `crates/core/src/lib.rs` - Module declarations for all 12 modules
- `crates/core/src/ast.rs` - Extracted shared AST types (RawConstruct, RawType, RawExpr, etc.)
- `crates/core/src/error.rs` - ElabError type with serialization
- `crates/core/src/lexer.rs` - Tokenizer (Token, Spanned, lex function)
- `crates/core/src/parser.rs` - Parser with pub use re-exports from ast.rs
- `crates/core/src/elaborate.rs` - Thin orchestrator calling pass1-pass6 in order
- `crates/core/src/pass1_bundle.rs` - Pass 0+1: parsing and bundle assembly
- `crates/core/src/pass2_index.rs` - Pass 2: construct indexing
- `crates/core/src/pass3_types.rs` - Pass 3: type environment construction
- `crates/core/src/pass4_typecheck.rs` - Pass 4: type checking and resolution
- `crates/core/src/pass5_validate.rs` - Pass 5: structural validation
- `crates/core/src/pass6_serialize.rs` - Pass 6: JSON interchange serialization
- `crates/cli/Cargo.toml` - Binary crate depending on tenor-core
- `crates/cli/src/main.rs` - CLI entry point (run, elaborate, ambiguity commands)
- `crates/cli/src/runner.rs` - Conformance suite runner
- `crates/cli/src/tap.rs` - TAP v14 output formatter
- `crates/cli/src/ambiguity/` - Ambiguity testing module (6 files)

## Decisions Made
- **Short directory names:** Used `crates/core/` and `crates/cli/` (not `crates/tenor-core/` or `crates/tenor-cli/`) per user override, while keeping `tenor-core` and `tenor-cli` as Cargo package names
- **AST extraction pattern:** Types moved from parser.rs to dedicated ast.rs, with `pub use` re-exports from parser.rs to avoid breaking internal callers
- **Orchestrator pattern:** elaborate.rs became a thin ~15-line orchestrator calling pass modules sequentially, rather than re-exporting all pass internals
- **Full removal of elaborator/:** Removed old directory entirely after verifying conformance, rather than keeping as reference

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Created placeholder CLI crate for workspace compilation**
- **Found during:** Task 1 (workspace creation)
- **Issue:** Cargo requires all workspace members to exist before building any member. CLI crate didn't exist yet.
- **Fix:** Created minimal placeholder `crates/cli/Cargo.toml` and `src/main.rs` so `cargo build -p tenor-core` could proceed.
- **Files modified:** `crates/cli/Cargo.toml`, `crates/cli/src/main.rs`
- **Verification:** `cargo build -p tenor-core` succeeded
- **Committed in:** 6170494 (Task 1 commit)

**2. [Rule 1 - Bug] Fixed redundant glob import in parser.rs**
- **Found during:** Task 1 (compilation warnings)
- **Issue:** Both `pub use crate::ast::{...}` (named re-exports) and `use crate::ast::*` (glob import) caused unused import warnings
- **Fix:** Removed the glob import, keeping only the `pub use` re-exports for backward compatibility
- **Files modified:** `crates/core/src/parser.rs`
- **Verification:** Clean compilation with no warnings in parser.rs
- **Committed in:** 6170494 (Task 1 commit)

**3. [Rule 3 - Blocking] Added missing serde dependency to CLI crate**
- **Found during:** Task 2 (ambiguity module compilation)
- **Issue:** Ambiguity module files use `serde::{Deserialize, Serialize}` directly but serde was not in CLI's Cargo.toml dependencies
- **Fix:** Added `serde = { workspace = true }` to `crates/cli/Cargo.toml`
- **Files modified:** `crates/cli/Cargo.toml`
- **Verification:** `cargo build --workspace` succeeded
- **Committed in:** 34595dd (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (1 bug, 2 blocking)
**Impact on plan:** All auto-fixes necessary for compilation and correctness. No scope creep.

## Issues Encountered
- Plan referenced `crates/tenor-core/` and `crates/tenor-cli/` directory names, but user override specified short names (`crates/core/`, `crates/cli/`). Applied short names consistently throughout.
- Plan referenced `tenor-core = { path = "../tenor-core" }` in CLI Cargo.toml but actual path is `../core`. Adjusted accordingly.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Cargo workspace structure is established and ready for additional crate members (eval, analyze, codegen, lsp)
- `tenor-core` public API exposes `elaborate()`, typed AST, and all pass modules
- Plan 02-02 (public API design, stub crates) can proceed immediately
- CLAUDE.md build instructions will need updating to reflect new workspace commands

## Self-Check: PASSED

- All 19 key files: FOUND
- Commit 6170494 (Task 1): FOUND
- Commit 34595dd (Task 2): FOUND
- Old elaborator/ directory: REMOVED (git-tracked files removed in Task 2 commit, build artifacts cleaned separately)
- Conformance: 47/47 pass

---
*Phase: 02-foundation*
*Completed: 2026-02-21*
