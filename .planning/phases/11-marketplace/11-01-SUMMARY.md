---
phase: 11-marketplace
plan: 01
subsystem: cli
tags: [tar, gzip, toml, template, packaging, marketplace]

requires:
  - phase: 10-hosted-platform
    provides: "Hosted platform with deployment infrastructure"

provides:
  - "tenor-template.toml manifest format with TemplateManifest struct"
  - "tenor pack CLI command producing .tenor-template.tar.gz archives"
  - "pack_template() that elaborates contract and bundles interchange JSON"
  - "unpack_template() for archive extraction (used by future tenor install)"

affects:
  - 11-marketplace (Plans 02-05: publish, search, install, deploy)

tech-stack:
  added:
    - flate2 = "1" (gzip compression)
    - tar = "0.4" (tar archive creation/extraction)
  patterns:
    - "Template directory layout: tenor-template.toml + contract/ + optional examples/screenshots/README.md"
    - "Archive format: tar.gz with bundle.json (pre-elaborated interchange)"
    - "--out flag naming (avoids clap global --output conflict with OutputFormat)"

key-files:
  created:
    - crates/cli/src/template/manifest.rs
    - crates/cli/src/template/pack.rs
    - crates/cli/src/template/mod.rs
    - crates/cli/tests/template_pack_e2e.rs
  modified:
    - crates/cli/src/main.rs
    - crates/cli/Cargo.toml

key-decisions:
  - "[11-01] Pack --output renamed to --out: avoids clap global arg naming conflict with OutputFormat --output"
  - "[11-01] unpack_template() marked #[allow(dead_code)]: public API for future tenor install (Phase 11 Plan 03)"
  - "[11-01] Semver validation is manual (no semver dep): check MAJOR.MINOR.PATCH prefix, allow pre-release suffix"
  - "[11-01] bundle.json added to archive root (not contract/): clearly separates source from elaborated artifact"
  - "[11-01] SHA-256 computed over archive bytes (not manifest): integrity check covers all archive contents"
  - "[11-01] sha256_hex uses format!(':02x') iterator (no hex crate): avoids new dep, sha2 already in workspace"

requirements-completed: [TPL-01, TPL-02, TPL-03, PKG-01, PKG-02, QLT-01]

duration: 10min
completed: 2026-02-27
---

# Phase 11 Plan 01: Template Manifest and Pack Command Summary

**`tenor-template.toml` manifest format defined and `tenor pack` produces `.tenor-template.tar.gz` archives with pre-elaborated interchange bundle using flate2/tar**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-02-27T19:57:04Z
- **Completed:** 2026-02-27T20:06:46Z
- **Tasks:** 5 (Tasks 1-4 new code, Task 5 quality gates)
- **Files modified:** 5 (4 new, 1 modified main.rs + Cargo.toml)

## Accomplishments

- `tenor-template.toml` manifest format with serde Deserialize/Serialize and validate() method
- `tenor pack [dir] [--out <path>]` CLI command that elaborates contract and creates a distributable archive
- Pack/unpack round-trip verified by 6 integration tests (valid, roundtrip, missing manifest, invalid contract, custom output, validation)
- All quality gates pass: fmt, build, 96/96 conformance, all tests, clippy clean

## Task Commits

1. **Task 1: Define TemplateManifest types** - `e2fe8b0` (feat)
2. **Task 2: Implement pack_template** - `cf3c629` (feat)
3. **Task 3: Wire up template module and Pack CLI subcommand** - `8d91fba` (feat)
4. **Task 4: End-to-end tests** - `2c2eb5d` (test)
5. **Task 5: Quality gates** - folded into task commits (no additional changes needed)

## Files Created/Modified

- `crates/cli/src/template/manifest.rs` - TemplateManifest, TemplateManifestFile, validate(), read_manifest(), archive_filename()
- `crates/cli/src/template/pack.rs` - pack_template(), unpack_template(), archive creation helpers, sha256_hex()
- `crates/cli/src/template/mod.rs` - Module root, cmd_pack dispatch function
- `crates/cli/tests/template_pack_e2e.rs` - 6 integration tests using assert_cmd + flate2/tar
- `crates/cli/src/main.rs` - Pack variant in Commands enum, match arm, mod template
- `crates/cli/Cargo.toml` - Added flate2 = "1", tar = "0.4" (deps + dev-deps)

## Decisions Made

- `--output` flag renamed to `--out` (consistent with other commands like `tenor sign --out`) to avoid clap global arg conflict with the `OutputFormat` `--output` flag
- `unpack_template()` kept in pack.rs with `#[allow(dead_code)]` — it's the implementation for the future `tenor install` command (Phase 11 Plan 03)
- Semver validation implemented without the `semver` crate: manually check MAJOR.MINOR.PATCH prefix and allow pre-release/build metadata suffix
- `bundle.json` placed at archive root (not inside `contract/`) to clearly separate source from pre-elaborated artifact
- SHA-256 computed via `sha2::Sha256::digest` with manual hex formatting — avoids adding `hex` crate

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Renamed --output to --out on Pack subcommand**
- **Found during:** Task 4 (integration test run)
- **Issue:** Clap panicked with "Mismatch between definition and access of `output`" — the `--output` flag on `Pack` conflicted with the global `--output OutputFormat` flag
- **Fix:** Changed `#[arg(long)]` `output: Option<PathBuf>` to `#[arg(long = "out", name = "pack_out")]` `out: Option<PathBuf>`, updated tests to use `--out`
- **Files modified:** crates/cli/src/main.rs, crates/cli/tests/template_pack_e2e.rs
- **Verification:** `cargo test -p tenor-cli --test template_pack_e2e` — all 6 tests pass
- **Committed in:** 8d91fba (Task 3 commit)

**2. [Rule 1 - Bug] Fixed invalid test contract DSL syntax**
- **Found during:** Task 4 (tests failing with elaboration error)
- **Issue:** Test's `minimal_tenor_contract()` used pseudo-DSL (` -> ` in entity transitions, `approve(...)` shorthand) instead of real Tenor syntax
- **Fix:** Rewrote to valid Tenor: `fact is_active { type: Bool ... }` + `rule { stratum: 0; when:; produce: verdict }`
- **Files modified:** crates/cli/tests/template_pack_e2e.rs
- **Verification:** All 6 integration tests pass
- **Committed in:** 2c2eb5d (Task 4 commit)

**3. [Rule 1 - Bug] Removed unused `std::io::Write as _` import and glob re-exports**
- **Found during:** Task 5 (clippy -D warnings)
- **Issue:** `use std::io::Write as _` in create_archive() was unused; `pub use manifest::*` and `pub use pack::*` in mod.rs produced unused import warnings
- **Fix:** Removed unused import; removed glob re-exports from mod.rs (binary crate, not library)
- **Files modified:** crates/cli/src/template/pack.rs, crates/cli/src/template/mod.rs
- **Verification:** `cargo clippy --workspace -- -D warnings` — clean
- **Committed in:** folded into earlier task commits

---

**Total deviations:** 3 auto-fixed (Rule 1: clap naming conflict, invalid DSL syntax, clippy cleanup)
**Impact on plan:** All fixes were correctness/compatibility requirements. No scope creep.

## Issues Encountered

- Test contract syntax required real Tenor DSL (not simplified pseudo-syntax from plan description)
- conformance suite now at 96/96 (plan expected 82+)

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- `tenor pack` fully functional: reads manifest, elaborates contract, creates distributable archive
- `unpack_template()` ready for `tenor install` (Plan 03)
- Foundation complete for `tenor publish` (Plan 02): pack → upload to registry API

---
*Phase: 11-marketplace*
*Completed: 2026-02-27*

## Self-Check: PASSED

- FOUND: `.planning/phases/11-marketplace/11-01-SUMMARY.md`
- FOUND: `crates/cli/src/template/manifest.rs`
- FOUND: `crates/cli/src/template/pack.rs`
- FOUND: `crates/cli/src/template/mod.rs`
- FOUND: `crates/cli/tests/template_pack_e2e.rs`
- FOUND: commits e2fe8b0, cf3c629, 8d91fba, 2c2eb5d (4 task commits)
