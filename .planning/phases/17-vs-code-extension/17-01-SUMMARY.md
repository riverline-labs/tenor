---
phase: 17-vs-code-extension
plan: 01
subsystem: editor-tooling
tags: [vscode, textmate-grammar, syntax-highlighting, typescript]

# Dependency graph
requires:
  - phase: none
    provides: standalone extension scaffold
provides:
  - VS Code extension package at editors/vscode/ with working TypeScript build pipeline
  - TextMate grammar (source.tenor) covering all Tenor DSL constructs
  - Language configuration with bracket matching, code folding, and auto-indent
  - Extension entry point with activate/deactivate lifecycle
affects: [17-02-PLAN, 17-03-PLAN, 17-04-PLAN, 17-05-PLAN]

# Tech tracking
tech-stack:
  added: [typescript ~5.7, "@types/vscode ^1.85.0"]
  patterns: [TextMate grammar scoping, VS Code extension scaffold]

key-files:
  created:
    - editors/vscode/package.json
    - editors/vscode/tsconfig.json
    - editors/vscode/.vscodeignore
    - editors/vscode/.gitignore
    - editors/vscode/src/extension.ts
    - editors/vscode/syntaxes/tenor.tmLanguage.json
    - editors/vscode/language-configuration.json
  modified: []

key-decisions:
  - "TextMate grammar uses 13 repository pattern groups (comments, import, construct-declaration, keywords, operators, strings, numbers, booleans, built-in-types, field-labels, step-types, built-in-functions, punctuation)"
  - "Standard TextMate scopes mapped to Tenor constructs (keyword.declaration, keyword.control, support.type, entity.name.type, etc.) -- no custom theme required"
  - "Persona uses separate pattern without lookahead for brace (persona declarations have no body block)"
  - "Unicode operators (logical, arrow, quantifiers) matched alongside ASCII equivalents"

patterns-established:
  - "Extension scaffold: editors/vscode/ with src/ -> out/ TypeScript build pipeline"
  - "TextMate grammar repository pattern: separate named groups for DRY reuse"

requirements-completed: [DEVX-01]

# Metrics
duration: 3min
completed: 2026-02-23
---

# Phase 17 Plan 01: VS Code Extension Scaffold + Syntax Highlighting Summary

**TextMate grammar with 13 pattern groups covering all Tenor DSL constructs, plus VS Code extension scaffold with TypeScript build pipeline**

## Performance

- **Duration:** 3 min
- **Started:** 2026-02-23T04:04:27Z
- **Completed:** 2026-02-23T04:07:01Z
- **Tasks:** 2
- **Files created:** 8

## Accomplishments
- VS Code extension package scaffolded with working TypeScript compile pipeline (src/ -> out/)
- TextMate grammar provides syntax highlighting for all Tenor DSL constructs using standard scopes
- Language configuration enables bracket matching, code folding on construct blocks, and auto-indent
- Grammar covers construct declarations, control keywords, Unicode/ASCII operators, strings, numbers (including money literals), booleans, built-in types, field labels, flow step types, and built-in functions

## Task Commits

Each task was committed atomically:

1. **Task 1: Scaffold VS Code extension package** - `299d78a` (feat)
2. **Task 2: Create TextMate grammar and language configuration** - `36bf6c8` (feat)

## Files Created/Modified
- `editors/vscode/package.json` - Extension manifest with language contribution, grammar path, and activation events
- `editors/vscode/tsconfig.json` - TypeScript config targeting ES2022 with Node16 modules
- `editors/vscode/.vscodeignore` - Excludes src/ts from packaged extension
- `editors/vscode/.gitignore` - Excludes out/ and node_modules/
- `editors/vscode/src/extension.ts` - Extension entry point with activate/deactivate and output channel
- `editors/vscode/syntaxes/tenor.tmLanguage.json` - TextMate grammar for Tenor DSL syntax highlighting
- `editors/vscode/language-configuration.json` - Bracket matching, folding markers, auto-indent rules
- `editors/vscode/package-lock.json` - npm lockfile for reproducible builds

## Decisions Made
- TextMate grammar uses 13 repository pattern groups to keep patterns DRY and maintainable
- Standard TextMate scopes (keyword.declaration, keyword.control, support.type, entity.name.type, etc.) ensure the grammar works with any VS Code color theme without custom theme definitions
- Persona construct uses a separate pattern without lookahead for braces, since persona declarations are single-line (no body block)
- Unicode operators are matched alongside their ASCII equivalents in the same scope
- Money literals matched as currency-code + amount pattern (e.g., `USD 100.00`)
- Flow step types (OperationStep, BranchStep, HandoffStep, Terminal, Terminate, Compensate) given support.class scope for distinct coloring

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Extension scaffold is ready for LSP client integration (Plan 02)
- Grammar provides immediate visual value for .tenor file editing
- All subsequent VS Code extension plans (diagnostics, commands, snippets) build on this foundation

## Self-Check: PASSED

All 8 created files verified on disk. Both task commits (299d78a, 36bf6c8) verified in git log.

---
*Phase: 17-vs-code-extension*
*Completed: 2026-02-23*
