# Phase 5: Domain Validation - Context

**Gathered:** 2026-02-22
**Status:** Ready for planning

<domain>
## Phase Boundary

Five real contracts across distinct business domains that elaborate, pass static analysis, and evaluate correctly — proving the spec handles real-world complexity before code generation begins. Also includes `tenor explain` subcommand and a spec gap report synthesizing all findings. Executor conformance validation (E10-E14) tested against domain contracts.

</domain>

<decisions>
## Implementation Decisions

### Contract depth & realism
- Balance realistic domain models with spec feature coverage — start from realistic scenarios but intentionally include constructs that exercise underused spec features
- Contract size varies by domain: let complexity emerge from what each domain naturally needs (small/medium/large spread required)
- Multi-file imports used where it makes domain sense, not forced on every contract
- At least one contract must be complex enough to produce a "wow" reaction from people evaluating the language

### Domain contracts (5 total)

| # | Domain | Size | Key Spec Features |
|---|--------|------|-------------------|
| 1 | **SaaS subscription** | SMALL | Entity states, simple rules, feature-flag enums, basic operations |
| 2 | **Healthcare prior auth** | LARGE | Deep flows, escalation/compensation, multi-stratum rules, personas, appeals |
| 3 | **Supply chain inspection** | MEDIUM | Parallel steps, compensation handlers, entity hierarchies, hold/release |
| 4 | **Energy procurement (RFP workflow)** | MEDIUM-LARGE | Approval tiers, delegation, supplier scoring, governed workflows, Money |
| 5 | **Trade finance (letter of credit)** | MEDIUM | Multi-party personas, deadline rules, Money types, document entity states |

- Energy procurement replaces the original generic "internal procurement" (05-04) — specifically models RFP approval workflow with approval tiers by spend amount, delegation rules, supplier scoring, and award criteria
- Energy procurement is a domain the user knows deeply and wants to showcase to specific people in the energy industry

### Spec gap handling
- **Document only during validation** — do not fix the spec or toolchain while authoring domain contracts
- Aggregate all issues at the end and reflect on them holistically before making fixes, to avoid wrong-perspective patches
- After the gap report, expect an iterative cycle: fix → reimplement domain contracts → log more issues → repeat until solid implementations and all bugs fixed / design issues exposed
- If a gap completely blocks a scenario from being expressed in Tenor, **skip that scenario** — do NOT force workarounds or make toolchain changes mid-validation
- **Skipped scenarios must be documented with extreme clarity**: what was attempted, why the language couldn't express it, and what spec change would enable it. No silent omissions.
- Single running gap log file appended to as each contract is authored; final report (05-07) is the polished synthesis
- Each gap finding is structured: domain, scenario, what was attempted, what failed/was awkward, severity (blocker/friction/cosmetic), suggested fix direction

### Explain command (`tenor explain`)
- Audience: both business stakeholders and developers
  - Default output is business-readable (plain language, no code)
  - `--verbose` or `--dev` flag adds technical details (types, entity states, rule strata)
- Format: both styled terminal output and Markdown
  - Default: styled terminal (colored, formatted, like `kubectl describe`)
  - `--format markdown` flag for Markdown output (portable, works in docs)
- Default explain output includes all four sections:
  1. **Contract summary** — what the contract governs, key entities, personas involved
  2. **Decision flow narrative** — plain-language walkthrough of how decisions flow (who triggers what, conditions checked, possible outcomes)
  3. **Fact inventory** — what information the contract needs as input, with types and defaults
  4. **Risk/coverage notes** — static analysis summary (dead states, unreachable rules, authority gaps)
- Accepts both `.tenor` source files (elaborates internally) and interchange JSON bundles

### Claude's Discretion
- Which domain becomes the "wow" showcase contract (likely healthcare or energy procurement given their natural complexity)
- Exact construct counts per contract — driven by domain needs
- When to use multi-file imports vs single-file
- Verbose/dev flag naming convention
- Exact terminal styling choices (colors, symbols, indentation)

</decisions>

<specifics>
## Specific Ideas

- The energy procurement domain comes from a real product the user built:
  - v1.0: complete procurement cycle with portfolio management, tariff engines, contract intelligence, RFP workflow, all-in cost analytics, bill validation, savings attribution
  - v2.0: matrix pricing with self-service procurement channel and governed approval workflows
  - v3.0: renewals, financial planning, scenario analysis, variance tracking
- The RFP approval workflow slice was chosen: approval tiers by spend amount, delegation rules, supplier scoring, award criteria
- The user knows people in the energy industry who would be specifically excited to see this domain modeled in Tenor
- Contract spread should show small/medium/large sizing so evaluators can see Tenor handles different scales

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 05-domain-validation*
*Context gathered: 2026-02-22*
