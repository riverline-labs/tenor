# Phase 4: Static Analysis - Context

**Gathered:** 2026-02-22
**Status:** Ready for planning

<domain>
## Phase Boundary

Implement the S1-S8 static analysis suite from Section 15 of the spec in the `tenor-analyze` crate, wire it into `tenor check`, add structured output suitable for CLI and LSP consumption, build comprehensive test coverage, and implement `tenor diff --breaking` using the analysis outputs and CFFP-derived breaking change taxonomy from Phase 3.1.

Does NOT include: domain validation with real contracts (Phase 5), LSP integration (Phase 8), S3b domain enumeration (qualified analysis, deferred per spec).

</domain>

<decisions>
## Implementation Decisions

### Analysis output format
- Each S1-S7 analysis produces a Rust struct with serde Serialize (structured data, not strings)
- `tenor check` in text mode: human-readable summary per analysis (table or list format depending on analysis)
- `tenor check` in JSON mode: full structured output with all analysis data, suitable for LSP consumption
- Follow existing CLI pattern: `--output text` (default) vs `--output json`
- Analysis results are composable -- each analysis returns its own result struct, `tenor check` aggregates

### Analysis granularity
- `tenor check` runs all S1-S7 analyses by default (full suite)
- Individual analysis selection via `--analysis s1,s3a,s6` flag for focused checks
- S8 (verdict uniqueness) is already enforced in Pass 5 -- `tenor check` reports it as pre-verified
- S3b is out of scope per spec qualification ("not always computationally feasible") -- note in output if S3a was used instead

### Breaking change classification
- `tenor diff --breaking` extends the existing `tenor diff` command with a `--breaking` flag
- Uses S1-S8 analysis on both bundles to classify changes per the CFFP taxonomy from Section 17 / Phase 3.1
- Output: each change annotated with severity (BREAKING, COMPATIBLE, REQUIRES_ANALYSIS) and required migration action
- JSON output includes the full taxonomy classification for programmatic consumption
- Text output: summary table of changes by severity, then detailed per-change descriptions

### Analyzer crate architecture
- `tenor-analyze` consumes interchange JSON (like tenor-eval), not raw AST
- Each analysis is a separate module: `s1_state_space.rs`, `s2_reachability.rs`, etc.
- Public API: `analyze(bundle: &Value) -> AnalysisReport` for full suite, plus per-analysis entry functions
- AnalysisReport aggregates all individual analysis results
- Analysis functions are pure (no side effects, no I/O) -- CLI handles presentation

### CLI integration
- `tenor check` first elaborates the .tenor file, then runs analysis on the resulting bundle
- If elaboration fails, report elaboration error and exit (no analysis on invalid contracts)
- Exit code: 0 = all analyses clean, 1 = findings reported (warnings or issues), 2 = elaboration error
- `--analysis` flag accepts comma-separated list of analysis IDs (s1, s2, s3a, s4, s5, s6, s7)

### Claude's Discretion
- Internal data structures for graph traversal in S6 flow path enumeration
- Specific complexity metric formulas for S7
- Test fixture design (how many and which contracts to use as known-good/known-bad)
- Whether to use the escrow contract from conformance suite as the primary test fixture or create dedicated analysis fixtures
- Exact table formatting for text output

</decisions>

<specifics>
## Specific Ideas

- The analyzer consumes interchange JSON (same pattern as tenor-eval) -- this is an established project decision
- S8 is already enforced in Pass 5 of the elaborator (verdict uniqueness check) -- the analyzer can report this as "pre-verified" rather than re-implementing
- The breaking change taxonomy from Phase 3.1 (Section 17 of TENOR.md) defines the severity classifications -- the analyzer applies these, not invents new ones
- Flow migration compatibility conditions FMC1-FMC7 from Phase 3.3 inform the `tenor diff --breaking` flow-level analysis
- Existing `tenor diff` produces DiffEntry structs -- `tenor diff --breaking` should augment these with severity annotations

</specifics>

<deferred>
## Deferred Ideas

- S3b domain satisfiability analysis -- spec explicitly marks as "not always computationally feasible," Phase 5+ if ever
- `tenor diff --explain` (MIGR-05) -- human-readable migration document, can be added after --breaking works
- `tenor diff --migration` -- migration contract generation, already noted as supplementary to DiffEntry JSON
- LSP integration of analysis results -- Phase 8
- Real domain contract validation against S1-S7 -- Phase 5

</deferred>

---

*Phase: 04-static-analysis*
*Context gathered: 2026-02-22*
