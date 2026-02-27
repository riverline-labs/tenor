---
phase: 08-automatic-ui
plan: 01
subsystem: ui
tags: [react, vite, typescript, cli, codegen]

# Dependency graph
requires:
  - phase: 07-sdks
    provides: TypeScript codegen patterns (CodegenBundle, type mappers, to_pascal_case etc.)
  - phase: 05-trust
    provides: CLI subcommand pattern established for connect, trust commands
provides:
  - tenor ui CLI command that generates complete React project from any valid contract
  - TenorClient TypeScript class with all 9 executor HTTP endpoints
  - Contract-derived types.ts (entity state unions, fact interface, persona union, operation/flow metadata)
  - Contract-derived theme.ts (HSL color palette from contract_id hash)
  - Static templates: package.json, tsconfig.json, vite.config.ts, index.html, main.tsx, App.tsx, Layout.tsx
  - Stub components (11) and stub hooks (3) ready for Plan 08-02 to fill in
affects:
  - 08-02 (component implementations will fill the stubs generated here)
  - 08-03 (any further UI pipeline plans)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "UI generation reuses CodegenBundle and typescript helpers from tenor-codegen crate"
    - "Contract-to-hue mapping: djb2 hash % 360 -> HSL color palette"
    - "Stub pattern: components emit placeholder div with name, hooks return null state"
    - "Two-tier template structure: static templates.rs + dynamic generate.rs orchestrator"

key-files:
  created:
    - crates/cli/src/ui/mod.rs
    - crates/cli/src/ui/api_client.rs
    - crates/cli/src/ui/templates.rs
    - crates/cli/src/ui/generate.rs
  modified:
    - crates/cli/src/main.rs

key-decisions:
  - "[08-01] Ui subcommand output flag renamed from --output to --out to avoid clap global arg conflict with the global --output (OutputFormat) flag"
  - "[08-01] types.ts uses plain string for Decimal/Money/Date/DateTime (not branded types) for simpler UI usage"
  - "[08-01] theme.ts uses djb2 hash of contract_id to derive HSL hue, then builds primary/secondary/accent palette"
  - "[08-01] generate_ui_project uses vec![] macro to build file list (avoids clippy::vec_init_then_push)"
  - "[08-01] Stub hooks always return null type — Plan 08-02 will replace with real API integration"

patterns-established:
  - "CLI module pattern: ui/mod.rs mirrors connect/mod.rs structure (UiOptions struct, cmd_ui entry)"
  - "Template functions in templates.rs return String; orchestrator in generate.rs calls them"
  - "write_file helper: takes &Path and &str content, returns Result<PathBuf, String>"

requirements-completed: [UI-CLI-01, UI-SCAFFOLD-01, UI-API-01, UI-PROJ-01]

# Metrics
duration: 7min
completed: 2026-02-27
---

# Phase 8 Plan 1: Automatic UI — CLI Command and React Project Scaffold Summary

**`tenor ui <contract>` generates a complete React/Vite/TypeScript project with TenorClient API class (9 executor endpoints), contract-derived types.ts and theme.ts, Layout component, 11 stub components, and 3 stub hooks ready for Plan 08-02.**

## Performance

- **Duration:** 7 min
- **Started:** 2026-02-27T00:33:34Z
- **Completed:** 2026-02-27T00:40:54Z
- **Tasks:** 6 (Tasks 1-5 implemented in single pass, Task 6 quality gates)
- **Files modified:** 5

## Accomplishments
- `tenor ui` CLI command with all documented flags (--out, --api-url, --contract-id, --theme, --title)
- Complete TenorClient TypeScript class with 9 executor HTTP endpoints per Section 19 spec
- Contract-derived type generation: entity state unions, fact interface, persona union, operation/flow metadata
- Contract-derived color theme using djb2 hash of contract_id -> HSL palette
- Static project templates: package.json, tsconfig.json, vite.config.ts, main.tsx, App.tsx with React Router
- Layout component with sidebar navigation, header with persona selector
- 11 stub components + 3 stub hooks, ready for Plan 08-02 to fill in
- 96/96 conformance tests still pass, clippy clean

## Task Commits

Tasks 1-6 implemented in single atomic commit:

1. **Tasks 1-6: tenor ui command, module, TenorClient, templates, generator, quality gates** - `af302bf` (feat)

## Files Created/Modified

- `crates/cli/src/main.rs` - Added Ui variant to Commands enum with --out/--api-url/--contract-id/--theme/--title flags; added match arm dispatching to ui::cmd_ui
- `crates/cli/src/ui/mod.rs` - UiOptions struct, cmd_ui entry point: loads .tenor or .json, parses CodegenBundle, derives contract_id/title, loads custom theme, calls generate_ui_project
- `crates/cli/src/ui/api_client.rs` - emit_api_client: generates TenorClient TypeScript class with all 9 executor endpoints
- `crates/cli/src/ui/templates.rs` - Static template functions: package_json, tsconfig_json, vite_config, index_html, main_tsx, app_tsx, layout_tsx, stub_component, stub_hook
- `crates/cli/src/ui/generate.rs` - generate_ui_project orchestrator: creates directory tree, writes all files, emit_types (contract-derived), emit_theme (hue-derived palette)

## Decisions Made

- `--output` renamed to `--out` in the Ui subcommand to avoid clap global arg naming conflict with the global `--output OutputFormat` flag (mirrors `tenor connect --out` pattern)
- types.ts uses plain `string` for Decimal/Money/Date/DateTime types (not branded types from codegen) for simpler UI form/display usage
- theme.ts uses djb2 hash of contract_id for hue derivation — deterministic, reproducible color palette per contract

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Renamed Ui --output to --out to fix clap global arg conflict**
- **Found during:** Task 1 smoke test (after initial implementation)
- **Issue:** Ui subcommand's `output: PathBuf` field with `--output` flag conflicted with the global `--output: OutputFormat` arg, causing a runtime panic: "Mismatch between definition and access of output"
- **Fix:** Changed `#[arg(long, default_value = "./tenor-ui")]` + `output: PathBuf` to `#[arg(long = "out", default_value = "./tenor-ui")]` + `out_dir: PathBuf` to avoid the name collision
- **Files modified:** crates/cli/src/main.rs
- **Verification:** `tenor ui --help` displays --out flag; smoke test generates project successfully
- **Committed in:** af302bf (task commit)

**2. [Rule 1 - Bug] Fixed clippy::vec_init_then_push in generate.rs**
- **Found during:** Task 6 (quality gates)
- **Issue:** `let mut files = Vec::new(); files.push(...);` pattern flagged by clippy -D warnings
- **Fix:** Replaced with `vec![]` macro initialization collecting from iterator
- **Files modified:** crates/cli/src/ui/generate.rs
- **Verification:** `cargo clippy --workspace -- -D warnings` clean
- **Committed in:** af302bf (task commit)

---

**Total deviations:** 2 auto-fixed (1 naming bug, 1 clippy lint)
**Impact on plan:** Both necessary. No scope creep.

## Issues Encountered
- clap global `--output` flag conflicts with subcommand field of same name — resolved by renaming to `--out` (consistent with `tenor connect --out`)

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- `tenor ui <contract>` fully functional and generates complete React project skeleton
- All 24 generated files in place (package.json, tsconfig.json, vite.config.ts, index.html, main.tsx, App.tsx, api.ts, types.ts, theme.ts, 1 full Layout component, 11 stub components, 3 stub hooks)
- Plan 08-02 can directly implement the stub components with real logic
- Generated api.ts TenorClient exports `client` singleton ready for use in components

---
*Phase: 08-automatic-ui*
*Completed: 2026-02-27*
