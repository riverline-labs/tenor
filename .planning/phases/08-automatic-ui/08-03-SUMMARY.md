---
phase: 08-automatic-ui
plan: 03
subsystem: ui
tags: [react, typescript, codegen, theming, css]

# Dependency graph
requires:
  - phase: 08-02
    provides: generate.rs orchestrator with all components ready for theme updates
provides:
  - theme.rs: dedicated theme generation module with contract_hue, hsl_to_hex, derive_palette, emit_theme
  - theme.ts: richer theme with primaryDark, textPrimary, textSecondary, shadows, breakpoints, xxl spacing, full border-radius, heading font
  - styles.css: global CSS reset with responsive layout (sidebar, main-content, card, media queries)
  - Updated components: all generated TypeScript references theme.colors.textPrimary/textSecondary (not old text/textMuted)
  - Custom --theme flag: per-color JSON overrides merged over contract-derived defaults
affects:
  - Generated UI applications (all contracts get the new richer theme)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Theme module separation: theme.rs extracted from generate.rs with full unit test coverage"
    - "Color merge strategy: custom JSON provides per-color overrides; missing keys fall back to contract-derived defaults"
    - "CSS reset separate from inline styles: styles.css handles global reset/layout; components use inline styles for component-level styling"

key-files:
  created:
    - crates/cli/src/ui/theme.rs
  modified:
    - crates/cli/src/ui/mod.rs
    - crates/cli/src/ui/generate.rs
    - crates/cli/src/ui/templates.rs
    - crates/cli/src/ui/components.rs

key-decisions:
  - "[08-03] theme.rs extracted from generate.rs: dedicated module with full unit test coverage (6 tests)"
  - "[08-03] Custom theme uses per-color merge (not full replacement): pass custom JSON with primary color, all other colors use contract-derived defaults"
  - "[08-03] textPrimary/textSecondary rename: matches theme.ts output keys; old text/textMuted names removed from all generated TypeScript"
  - "[08-03] sidebar color removed from theme: Layout now uses theme.colors.surface (white) for sidebar background"
  - "[08-03] styles.css emitted as standalone global reset: imported in main.tsx, complements per-component inline styles"

patterns-established:
  - "Theme module test pattern: unit tests for hash function, HSL conversion, full output keys, custom overrides, contract ID in comment"
  - "Color key consistency invariant: theme.rs must define exactly the keys that components and templates reference"

requirements-completed: [UI-THEME-01, UI-THEME-02, UI-THEME-03]

# Metrics
duration: 5min
completed: 2026-02-27
---

# Phase 8 Plan 3: Automatic UI — Professional Theme and Responsive Layout Summary

**Dedicated theme.rs module generates contract-derived color palettes (HSL from djb2 hash), emitting theme.ts with richer structure (primaryDark, textPrimary/textSecondary, shadows, breakpoints, xxl spacing, full border-radius) and supporting per-color custom overrides via --theme JSON.**

## Performance

- **Duration:** 5 min
- **Started:** 2026-02-27T20:24:00Z
- **Completed:** 2026-02-27T20:28:51Z
- **Tasks:** 5 (1 theme module, 1 global CSS, 1 component updates, 1 wiring verify, 1 quality gates)
- **Files created/modified:** 5

## Accomplishments

- `theme.rs`: Dedicated theme generation module with:
  - `contract_hue()` — djb2 hash of contract_id → hue 0-359
  - `hsl_to_hex()` — standard HSL-to-RGB conversion
  - `derive_palette()` — builds primary/primaryLight/primaryDark/secondary/accent from hue
  - `emit_theme()` — generates full theme.ts with per-color custom override merging
  - 6 unit tests covering hue uniqueness, hex format validation, all required keys, custom overrides, contract ID in comment
- Richer `theme.ts` structure: added `primaryDark`, renamed `text`→`textPrimary` and `textMuted`→`textSecondary`, added `shadows`, `breakpoints`, `xxl` spacing, `full` border-radius, `heading` font
- `templates.rs`: Added `global_css()` generating `styles.css` with CSS reset, base styles, responsive layout (sidebar/main-content/card), and 768px media query
- `main.tsx` now imports `'./styles.css'` so the reset is always applied
- All components updated: `textMuted`→`textSecondary`, `text`→`textPrimary`, `sidebar`→`surface` across 35+ occurrences
- `--theme` custom override verified: pass `{"colors": {"primary": "#ff0000"}}` to override just primary while keeping all other contract-derived colors
- 96/96 conformance tests pass, all workspace tests pass, clippy clean

## Task Commits

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Theme generation module | e392efa | theme.rs (new), mod.rs, generate.rs |
| 2 | Global CSS and responsive styles | 541745b | templates.rs, generate.rs |
| 3 | Update components to use new theme names | 4eec5c4 | components.rs, templates.rs |
| 4 | Wire theme into pipeline (verified, no new code) | — | — |
| 5 | Quality gates (fmt cleanup) | d6c3519 | theme.rs |

## Files Created/Modified

- `crates/cli/src/ui/theme.rs` — New: dedicated theme generation module with unit tests (193 lines)
- `crates/cli/src/ui/mod.rs` — Added `mod theme;`
- `crates/cli/src/ui/generate.rs` — Removed inline theme functions; now delegates to `theme::emit_theme`; added `styles.css` to emitted files
- `crates/cli/src/ui/templates.rs` — Added `global_css()` function; updated `main_tsx()` to import styles.css; renamed color references
- `crates/cli/src/ui/components.rs` — Updated 35+ color references: textMuted→textSecondary, text→textPrimary

## Decisions Made

- `theme.rs` extracted from `generate.rs` as a dedicated module — theme generation has grown to include unit tests, color merging logic, and palette derivation; separation keeps generate.rs focused on orchestration
- Custom theme uses per-color merge strategy — `{"colors": {"primary": "#ff0000"}}` overrides just `primary`; all other colors use contract-derived defaults. This is more useful than full replacement.
- Sidebar background changed from `theme.colors.sidebar` (which no longer exists) to `theme.colors.surface` — layout sidebar is white (same as card surfaces), consistent with card-based design
- `styles.css` as standalone global reset — imported once in main.tsx; components use inline styles for component-level styling. Clear separation of concerns.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Theme pipeline already wired (Task 4 was verify-only)**
- **Found during:** Task 4 review
- **Issue:** Plan 08-01 had already implemented the `--theme` flag loading and `custom_theme` passing to `generate_ui_project`. Task 4 of 08-03 instructed to "wire theme into generation pipeline" but this was already done.
- **Fix:** Verified the existing wiring works correctly via smoke tests (`--theme` flag produces custom primary color while keeping other defaults). No code changes needed.
- **Impact:** Task 4 committed as no-op (tested, not coded). All plan outcomes achieved.

---

**Total deviations:** 1 (Task 4 pre-completed by 08-01; verified rather than re-implemented)
**Impact on plan:** No scope changes; all plan requirements satisfied.

## Self-Check

### Files Created

- [x] `/Users/bwb/src/riverline/tenor/crates/cli/src/ui/theme.rs` exists

### Generated UI Verification

- [x] `/tmp/test-ui-08-03/src/theme.ts` — contains `primaryDark`, `textPrimary`, `textSecondary`, `shadows`, `breakpoints`
- [x] `/tmp/test-ui-08-03/src/styles.css` — contains responsive layout rules, card class, media query
- [x] `/tmp/test-ui-08-03/src/main.tsx` — imports `'./styles.css'`
- [x] Two contracts produce different primaries: `integration_escrow` → `#d22dbf`, `fact_basic` → `#d22d90`
- [x] `--theme '{"colors":{"primary":"#ff0000"}}'` → theme.ts shows `primary: '#ff0000'`

### Commits

- [x] e392efa — feat(08-03): extract theme generation into dedicated theme.rs module
- [x] 541745b — feat(08-03): add global CSS reset and responsive layout styles
- [x] 4eec5c4 — feat(08-03): update all components to use new theme color names
- [x] d6c3519 — chore(08-03): apply cargo fmt formatting to theme.rs

## Self-Check: PASSED

All created files exist. All 4 commits verified. Generated UI project smoke-tested successfully with 25 files generated (up from 24 in 08-02, adding styles.css). Theme customization verified with --theme flag.

---
*Phase: 08-automatic-ui*
*Completed: 2026-02-27*
