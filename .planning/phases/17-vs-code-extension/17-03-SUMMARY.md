---
phase: 17-vs-code-extension
plan: 03
subsystem: lsp
tags: [lsp, navigation, goto-definition, find-references, hover, completions, document-symbols]

# Dependency graph
requires:
  - phase: 17-02
    provides: LSP server with diagnostics and semantic tokens
provides:
  - Go-to-definition for all construct reference types
  - Find-all-references across workspace
  - Document symbols for outline/breadcrumb navigation
  - Hover information with construct type and summary
  - Context-aware completions for keywords, references, and types
  - ProjectIndex for workspace-wide construct indexing
affects: [17-04, 17-05]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "ProjectIndex pattern: parse all .tenor files, cache declarations/references/symbols/summaries"
    - "Context-aware completions: scan backwards from cursor to determine construct/field context"
    - "Workspace root extraction from InitializeParams with fallback from workspace_folders to root_uri"

key-files:
  created:
    - crates/lsp/src/navigation.rs
    - crates/lsp/src/hover.rs
    - crates/lsp/src/completion.rs
  modified:
    - crates/lsp/src/server.rs
    - crates/lsp/src/lib.rs

key-decisions:
  - "ProjectIndex rebuilt on every file save for simplicity over incremental updates"
  - "Hover uses markdown code blocks with tenor language tag for syntax highlighting"
  - "Completions detect context by scanning backwards for construct/field keywords with brace depth tracking"

patterns-established:
  - "Navigation index pattern: HashMap<(kind, id), Location> for declarations, HashMap<(kind, id), Vec<Location>> for references"
  - "ConstructSummary pattern: pre-computed hover text stored alongside index for O(1) hover lookup"

requirements-completed: [DEVX-02]

# Metrics
duration: 13min
completed: 2026-02-23
---

# Phase 17 Plan 03: LSP Navigation Features Summary

**Go-to-definition, find-all-references, hover, completions, and document symbols for Tenor contracts via ProjectIndex workspace indexing**

## Performance

- **Duration:** 13 min
- **Started:** 2026-02-23T04:27:11Z
- **Completed:** 2026-02-23T04:40:11Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- Full LSP navigation: go-to-definition jumps to any construct declaration (facts, entities, operations, flows, personas, types, rules, systems)
- Find-all-references lists all usages of a construct across the entire workspace including the declaration
- Document symbols provide outline/breadcrumb navigation with construct type and detail
- Hover shows construct summary in markdown (type info, states, effects, personas, etc.)
- Context-aware completions: construct keywords at top level, field keywords in bodies, fact/entity/persona/operation names in relevant contexts, built-in and declared types

## Task Commits

Each task was committed atomically:

1. **Task 1: Go-to-definition, find-all-references, document symbols** - `bef38c0` (feat)
2. **Task 2: Hover and completions** - `2c00029` (feat)

## Files Created/Modified
- `crates/lsp/src/navigation.rs` - ProjectIndex with declarations, references, symbols, summaries; go-to-definition, find-references, document-symbols; word-at-position, construct/field context detection
- `crates/lsp/src/hover.rs` - Hover provider using ProjectIndex summaries with markdown rendering
- `crates/lsp/src/completion.rs` - Context-aware completion provider with construct/field/reference/type completions
- `crates/lsp/src/server.rs` - Added 5 new capabilities (definition, references, document_symbol, hover, completion), workspace root extraction, ProjectIndex lifecycle, request dispatch for all new methods
- `crates/lsp/src/lib.rs` - Added navigation, hover, completion module declarations

## Decisions Made
- Rebuild full ProjectIndex on every file save rather than incremental updates -- simpler and sufficient for typical .tenor project sizes
- Hover renders construct details in markdown code blocks with `tenor` language tag
- Completion context detection uses backwards scan with brace-depth tracking to determine whether cursor is at top level, in a construct body, or within a specific field block
- ProjectIndex stores pre-computed ConstructSummary for O(1) hover lookup rather than re-parsing on demand

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All LSP navigation features working: goto-definition, find-all-references, hover, completions, document symbols
- ProjectIndex infrastructure ready for future features (e.g., rename, code actions)
- Ready for Phase 17 Plan 04 (packaging/testing)

---
*Phase: 17-vs-code-extension*
*Completed: 2026-02-23*
