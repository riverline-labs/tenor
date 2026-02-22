# Phase 5: Domain Validation - Research

**Researched:** 2026-02-22
**Domain:** Domain contract authoring, DSL validation, CLI tooling (Rust)
**Confidence:** HIGH

## Summary

Phase 5 is a validation phase, not a feature-building phase. The core question is: "Can Tenor express real-world business contracts across distinct domains?" Five contracts must elaborate, pass static analysis (`tenor check`), and evaluate (`tenor eval`) with correct provenance. The toolchain is already built (Phases 1-4); this phase stress-tests it against real complexity. The secondary deliverables are the `tenor explain` CLI subcommand and a spec gap report.

The research domain is unique: it is not about library selection or architecture patterns but about understanding five business domains deeply enough to author realistic contracts that exercise the full Tenor construct vocabulary (Fact, Entity, Rule, Operation, Persona, Flow, TypeDecl). The primary risk is authoring contracts that are either too simplistic (fail to stress-test the spec) or too ambitious (hit language limitations that block expression entirely). The user's decisions explicitly address this: start from realistic scenarios, intentionally include constructs that exercise underused features, document gaps rather than force workarounds.

**Primary recommendation:** Author contracts domain-by-domain in increasing complexity order (SaaS SMALL, supply chain MEDIUM, trade finance MEDIUM, energy procurement MEDIUM-LARGE, healthcare LARGE). For each contract: write .tenor source, create fact fixtures, verify elaborate/check/eval pipeline, log any spec gaps to a running gap file. Implement `tenor explain` as a standalone plan after at least two contracts are complete (it needs real data to validate output quality). Executor conformance (E10-E14) tests should use the domain contracts as test subjects.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Contract depth & realism:**
- Balance realistic domain models with spec feature coverage — start from realistic scenarios but intentionally include constructs that exercise underused spec features
- Contract size varies by domain: let complexity emerge from what each domain naturally needs (small/medium/large spread required)
- Multi-file imports used where it makes domain sense, not forced on every contract
- At least one contract must be complex enough to produce a "wow" reaction from people evaluating the language

**Domain contracts (5 total):**

| # | Domain | Size | Key Spec Features |
|---|--------|------|-------------------|
| 1 | **SaaS subscription** | SMALL | Entity states, simple rules, feature-flag enums, basic operations |
| 2 | **Healthcare prior auth** | LARGE | Deep flows, escalation/compensation, multi-stratum rules, personas, appeals |
| 3 | **Supply chain inspection** | MEDIUM | Parallel steps, compensation handlers, entity hierarchies, hold/release |
| 4 | **Energy procurement (RFP workflow)** | MEDIUM-LARGE | Approval tiers, delegation, supplier scoring, governed workflows, Money |
| 5 | **Trade finance (letter of credit)** | MEDIUM | Multi-party personas, deadline rules, Money types, document entity states |

- Energy procurement replaces the original generic "internal procurement" (05-04) — specifically models RFP approval workflow with approval tiers by spend amount, delegation rules, supplier scoring, and award criteria
- Energy procurement is a domain the user knows deeply and wants to showcase to specific people in the energy industry

**Spec gap handling:**
- Document only during validation — do not fix the spec or toolchain while authoring domain contracts
- Aggregate all issues at the end and reflect on them holistically before making fixes
- After the gap report, expect an iterative cycle: fix, reimplement domain contracts, log more issues, repeat until solid
- If a gap completely blocks a scenario from being expressed in Tenor, skip that scenario — do NOT force workarounds or make toolchain changes mid-validation
- Skipped scenarios must be documented with extreme clarity: what was attempted, why the language couldn't express it, and what spec change would enable it
- Single running gap log file appended to as each contract is authored; final report (05-07) is the polished synthesis
- Each gap finding is structured: domain, scenario, what was attempted, what failed/was awkward, severity (blocker/friction/cosmetic), suggested fix direction

**Explain command (`tenor explain`):**
- Audience: business stakeholders (default) and developers (--verbose/--dev)
- Format: styled terminal (default, like `kubectl describe`) and Markdown (`--format markdown`)
- Default output includes 4 sections: contract summary, decision flow narrative, fact inventory, risk/coverage notes
- Accepts both .tenor source files and interchange JSON bundles

### Claude's Discretion
- Which domain becomes the "wow" showcase contract (likely healthcare or energy procurement)
- Exact construct counts per contract — driven by domain needs
- When to use multi-file imports vs single-file
- Verbose/dev flag naming convention
- Exact terminal styling choices (colors, symbols, indentation)

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| DOMN-01 | Multi-tenant SaaS contract (seat limits, feature flags, subscription state) | Plan 05-01: SaaS contract with Enum feature flags, Int seat limits, entity state machine for subscription lifecycle |
| DOMN-02 | Healthcare prior auth contract (policy rules, peer review, appeals) | Plan 05-02: Healthcare contract with deep flows, Escalate handlers, multi-stratum rules, multiple personas |
| DOMN-03 | Supply chain contract (customs, inspection, release gates) | Plan 05-03: Supply chain with ParallelStep for concurrent inspections, entity hierarchies, Compensate handlers |
| DOMN-04 | Internal procurement contract (approval tiers, delegation, budget) | Plan 05-04: Energy procurement RFP workflow with approval tiers, Money types, delegation rules, supplier scoring |
| DOMN-05 | Financial domain contract (lending, escrow, or compliance) | Plan 05-05: Trade finance letter of credit with multi-party personas, deadline-based rules, document entity states |
| DOMN-06 | Each contract elaborates without error | All contract plans include `tenor elaborate` verification step |
| DOMN-07 | Each contract passes `tenor check` | All contract plans include `tenor check` verification step |
| DOMN-08 | Each contract evaluates against sample facts via `tenor eval` with correct provenance | All contract plans include fact fixtures, verdict fixtures, and eval conformance tests |
| DOMN-09 | Spec gap report produced from domain validation findings | Plan 05-07: gap report synthesized from running gap log accumulated during contract authoring |
| CLI-06 | `tenor explain <bundle.json>` produces human-readable contract summary | Plan 05-06: implement explain subcommand with styled terminal + markdown output |
| TEST-11 | E10-E14 executor conformance tests | Plan 05-08: executor conformance tests using domain contracts as test subjects |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tenor-core | workspace | Elaboration pipeline (.tenor -> interchange JSON) | Already built, 6-pass elaborator |
| tenor-eval | workspace | Contract evaluation (rules + flows) | Already built, stratified eval + flow execution |
| tenor-analyze | workspace | S1-S8 static analysis suite | Already built, full analysis pipeline |
| clap | 4.5 | CLI argument parsing | Already in use, derive API |
| serde_json | 1 | JSON serialization | Already in use throughout |
| sha2 | 0.10 | SHA-256 etag computation | Already in use for manifests |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| crossterm | 0.28 (latest stable) | Terminal styling (colors, bold, reset) | `tenor explain` styled output — cross-platform terminal control |
| textwrap | 0.16 | Text wrapping for terminal output | `tenor explain` narrative text formatting |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| crossterm | colored | crossterm is more maintained, supports Windows natively; colored is simpler but less control |
| crossterm | termcolor | termcolor is buffer-oriented, crossterm is more direct; crossterm integrates better with detect-terminal-color |
| textwrap | hand-rolled wrapping | textwrap handles Unicode width, hyphenation, terminal width detection correctly |

**Installation:** Add to `crates/cli/Cargo.toml`:
```toml
crossterm = "0.28"
textwrap = "0.16"
```

## Architecture Patterns

### Recommended Project Structure
```
domains/                          # NEW: domain validation contracts
├── saas/
│   ├── saas_subscription.tenor
│   ├── saas_subscription.facts.json       # happy path facts
│   ├── saas_subscription.verdicts.json    # expected verdicts
│   └── README.md                          # domain description (only if user requests)
├── healthcare/
│   ├── prior_auth.tenor
│   ├── prior_auth_approve.facts.json
│   ├── prior_auth_approve.verdicts.json
│   ├── prior_auth_appeal.facts.json
│   └── prior_auth_appeal.verdicts.json
├── supply_chain/
│   ├── inspection.tenor
│   ├── types.tenor                         # shared types (multi-file import)
│   ├── inspection_pass.facts.json
│   └── inspection_pass.verdicts.json
├── energy_procurement/
│   ├── rfp_workflow.tenor
│   ├── types.tenor
│   ├── rfp_approve.facts.json
│   └── rfp_approve.verdicts.json
└── trade_finance/
    ├── letter_of_credit.tenor
    ├── lc_present.facts.json
    └── lc_present.verdicts.json

crates/cli/src/
├── explain.rs                    # NEW: tenor explain subcommand
├── explain/
│   ├── mod.rs
│   ├── summary.rs                # contract summary section
│   ├── narrative.rs              # decision flow narrative
│   ├── inventory.rs              # fact inventory section
│   └── risk.rs                   # risk/coverage notes from analysis
├── main.rs                       # (modify: wire up explain command)
└── ...

conformance/eval/domains/         # NEW: domain eval fixtures (or integrate into domains/)
    ├── saas_subscription.tenor -> ../../domains/saas/saas_subscription.tenor  (symlink or copy)
    ...

.planning/phases/05-domain-validation/
├── gap-log.md                    # Running gap log (appended during each contract plan)
└── ...
```

### Pattern 1: Domain Contract Authoring Workflow
**What:** Each domain contract follows a consistent authoring-and-validation pipeline
**When to use:** Every domain contract plan (05-01 through 05-05)
**Pipeline:**
1. Author `.tenor` source file(s) based on domain model
2. Run `tenor elaborate <contract.tenor>` — must succeed
3. Run `tenor check <contract.tenor>` — must produce no warnings
4. Create `.facts.json` fixture(s) covering key evaluation paths
5. Elaborate to JSON, then run `tenor eval <bundle.json> --facts <facts.json>` — verify verdicts
6. Create `.verdicts.json` expected output, add eval conformance test to `crates/eval/tests/conformance.rs`
7. Log any spec gaps to `gap-log.md`

### Pattern 2: Eval Conformance Test Registration
**What:** Domain contracts are registered as eval conformance tests following the existing fixture triplet convention
**When to use:** After each contract's eval fixtures are verified
**Example:**
```rust
// In crates/eval/tests/conformance.rs

fn domains_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("domains")
}

#[test]
fn domain_saas_subscription() {
    run_eval_fixture(&domains_dir().join("saas"), "saas_subscription");
}

#[test]
fn domain_healthcare_prior_auth_approve() {
    run_eval_flow_fixture(
        &domains_dir().join("healthcare"),
        "prior_auth_approve",
        "prior_auth_flow",
        "requesting_physician",
    );
}
```

### Pattern 3: Explain Command Architecture
**What:** The explain command reads interchange JSON (or elaborates .tenor), runs analysis, and produces a structured human-readable summary
**When to use:** Plan 05-06
**Architecture:**
```
Input (.tenor or .json)
  → Elaborate (if .tenor)
  → Parse interchange bundle
  → Run S1-S8 analysis (via tenor-analyze)
  → Generate 4 sections:
    1. Contract summary (from constructs)
    2. Decision flow narrative (from flows + operations + rules)
    3. Fact inventory (from facts)
    4. Risk/coverage notes (from analysis findings)
  → Format output (terminal styled or markdown)
```

### Pattern 4: Gap Log Structure
**What:** A running gap log file appended to during each contract authoring plan
**When to use:** Plans 05-01 through 05-05, synthesized in 05-07
**Structure per entry:**
```markdown
### GAP-NNN: [Short Title]
- **Domain:** SaaS / Healthcare / Supply Chain / Energy / Trade Finance
- **Scenario:** What was being modeled
- **Attempted:** The Tenor construct or pattern tried
- **Result:** What failed or was awkward
- **Severity:** BLOCKER / FRICTION / COSMETIC
- **Suggested Fix:** Direction for spec or toolchain change
```

### Pattern 5: Executor Conformance Testing (E10-E14)
**What:** Tests that validate executor obligations from spec Section 18
**When to use:** Plan 05-08
**Approach:** These are integration tests in `crates/cli/tests/` or `crates/eval/tests/` that:
- E10: Generate manifest from domain contract, verify JSON Schema validity, verify etag field present
- E11: Verify manifest bundle is complete (all construct references resolved)
- E12: Verify etag changes when bundle changes, stays same when it doesn't (elaborate two versions, compare etags)
- E13: Verify dry-run semantics: evaluate up to step 3, verify no state change (test in tenor-eval)
- E14: Verify capabilities field behavior (static vs dynamic manifests)

### Anti-Patterns to Avoid
- **Authoring contracts that avoid Tenor's limitations:** The goal is to find gaps, not hide them. If a domain scenario cannot be expressed, document it rather than simplifying the domain model.
- **Fixing the toolchain mid-validation:** Document gaps but do not modify the elaborator, evaluator, or analyzer during contract authoring. Fixes happen after 05-07.
- **Treating domain contracts as conformance tests only:** These contracts serve a dual purpose: validation AND showcase. They should be impressive enough that someone in each industry would recognize the domain.
- **Making all contracts single-file:** Multi-file imports should be used where the domain naturally has shared types (e.g., supply chain shared types, energy procurement shared types).
- **Skipping eval flow tests for complex flows:** Every flow path needs at least one eval fixture to verify end-to-end provenance.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Terminal coloring | ANSI escape code strings | crossterm | Cross-platform, handles terminal detection, reset sequences |
| Text wrapping | Manual line breaking | textwrap | Unicode width, terminal width detection, proper hyphenation |
| JSON interchange parsing | Custom JSON walker | Existing `AnalysisBundle::from_interchange()` and `Contract::from_interchange()` | Already proven correct in Phases 3-4 |
| Gap report format | Ad hoc text file | Structured markdown with consistent headings | Planner defines structure, each plan appends to it |
| Manifest generation | New manifest builder | Existing `build_manifest()` in main.rs | Already implemented and tested in Phase 3.4 |

**Key insight:** This phase builds very little new Rust code. The only significant new code is `tenor explain`. Everything else is contract authoring (.tenor files), fixture creation (.facts.json, .verdicts.json), test registration, and documentation (gap log/report).

## Common Pitfalls

### Pitfall 1: Overly Simplistic Contracts
**What goes wrong:** Contracts that only use 2-3 constructs don't stress-test the spec
**Why it happens:** Temptation to keep things simple for faster completion
**How to avoid:** Each contract has an assigned set of key spec features to exercise. The user explicitly required a "wow" contract.
**Warning signs:** A contract with fewer than 3 entities, no multi-stratum rules, or no flows

### Pitfall 2: Spec Gaps Blocking Progress
**What goes wrong:** A domain scenario cannot be expressed in Tenor, and the author spends time trying workarounds
**Why it happens:** Tenor is a new language; some real-world patterns may not fit its current constructs
**How to avoid:** The user explicitly decided: skip blocked scenarios, document with extreme clarity, move on
**Warning signs:** More than 30 minutes spent trying to express a single domain concept

### Pitfall 3: Facts vs Computed Values Confusion
**What goes wrong:** Attempting to compute aggregates (sum, count, average) within rules
**Why it happens:** Domain models naturally require aggregations that Tenor prohibits (spec Section 5.5, C1 decidability)
**How to avoid:** Pre-compute aggregates as Facts from external systems. For each domain model, identify what must be a Fact vs what can be a Rule verdict.
**Warning signs:** Rule conditions that feel like they need `sum()` or `count()` — those values must arrive as Facts

### Pitfall 4: Entity State as Predicate Term
**What goes wrong:** Trying to check entity state in preconditions (spec Section 10.6 explicitly prohibits this)
**Why it happens:** Natural domain modeling often thinks in terms of "if order is in state X"
**How to avoid:** Entity state constraints are enforced through effect declarations (from_state -> to_state), not preconditions. Use verdicts to gate operations.
**Warning signs:** Preconditions that reference entity current state

### Pitfall 5: Explain Command Feature Creep
**What goes wrong:** The explain command becomes a full contract IDE instead of a summary tool
**Why it happens:** There are many useful things to show about a contract
**How to avoid:** Stick to the 4 decided sections: summary, narrative, fact inventory, risk notes. Default is business-readable. --verbose adds technical detail.
**Warning signs:** Adding interactive features, contract editing, or visualization beyond text

### Pitfall 6: Eval Fixture Complexity
**What goes wrong:** Creating dozens of fact fixtures per contract, each requiring manual expected-verdict computation
**Why it happens:** Complex contracts have many evaluation paths
**How to avoid:** Focus on 2-4 key paths per contract: happy path, key failure path, boundary conditions. Use `tenor eval --output json` to generate initial expected output, then hand-verify.
**Warning signs:** More than 5 fact fixture files per contract

### Pitfall 7: Missing Persona Declarations
**What goes wrong:** Writing operations with persona references that lack `persona` declarations
**Why it happens:** Persona is a v1.0 addition; easy to forget the declaration when focused on domain modeling
**How to avoid:** Always declare personas at the top of the contract, before any operation or flow references them
**Warning signs:** Pass 5 elaboration error on persona validation

## Code Examples

### Example 1: Domain Contract Structure (SaaS)
```tenor
// ── Named Types ──────────────────────────────────────────────
type PlanFeatures {
  max_seats:          Int(min: 1, max: 10000)
  api_access:         Bool
  sso_enabled:        Bool
  custom_branding:    Bool
}

// ── Personas ─────────────────────────────────────────────────
persona account_admin
persona billing_system
persona support_agent

// ── Facts ────────────────────────────────────────────────────
fact current_seat_count {
  type:   Int(min: 0, max: 10000)
  source: "identity_service.active_users"
}

fact subscription_plan {
  type:   Enum(values: ["free", "starter", "professional", "enterprise"])
  source: "billing_service.current_plan"
}

fact plan_features {
  type:   PlanFeatures
  source: "plan_service.features"
}

// ── Entities ─────────────────────────────────────────────────
entity Subscription {
  states:  [trial, active, suspended, cancelled]
  initial: trial
  transitions: [
    (trial, active),
    (trial, cancelled),
    (active, suspended),
    (active, cancelled),
    (suspended, active),
    (suspended, cancelled)
  ]
}

// ── Rules ────────────────────────────────────────────────────
rule seats_within_limit {
  stratum: 0
  when:    current_seat_count <= plan_features.max_seats
  produce: verdict seats_ok { payload: Bool = true }
}

// ── Operations ───────────────────────────────────────────────
operation activate_subscription {
  allowed_personas: [billing_system]
  precondition:     verdict_present(seats_ok)
  effects:          [(Subscription, trial, active)]
  error_contract:   [precondition_failed, persona_rejected]
}
```

### Example 2: Explain Command Output (Styled Terminal)
```
CONTRACT SUMMARY
════════════════
  Name:       saas_subscription
  Entities:   Subscription (4 states)
  Personas:   account_admin, billing_system, support_agent
  Rules:      3 rules across 2 strata
  Flows:      1 flow (subscription_lifecycle)

DECISION FLOW
═════════════
  Flow: subscription_lifecycle
    1. billing_system activates the subscription
       → requires: seats within plan limit
    2. If payment fails → subscription is suspended
       → support_agent can reactivate after resolution
    3. account_admin can cancel at any time

FACT INVENTORY
══════════════
  current_seat_count    Int(0..10000)     identity_service.active_users
  subscription_plan     Enum(4 values)    billing_service.current_plan
  plan_features         PlanFeatures      plan_service.features
  payment_status        Bool              billing_service.payment_ok        default: true

RISK / COVERAGE
═══════════════
  ✓ All entity states reachable
  ✓ All verdict types unique
  ✓ 3 flow paths enumerated
  ⚠ No parallel steps (single-threaded flow)
```

### Example 3: Eval Flow Test Registration
```rust
// Source: crates/eval/tests/conformance.rs (existing pattern)

fn domains_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("domains")
}

#[test]
fn domain_saas_activate() {
    run_eval_flow_fixture(
        &domains_dir().join("saas"),
        "saas_activate",
        "subscription_lifecycle",
        "billing_system",
    );
}
```

### Example 4: Gap Log Entry
```markdown
### GAP-001: No aggregate functions for seat counting
- **Domain:** SaaS
- **Scenario:** Need to count active seats across a list of team members
- **Attempted:** Rule with `count(member ∈ members . member.active = true)`
- **Result:** Tenor prohibits aggregate computation in rules (§5.5, C1)
- **Severity:** FRICTION (workaround exists)
- **Suggested Fix:** Not needed — pre-computed count as Fact is the intended pattern.
  Document this pattern prominently in authoring guide.
```

### Example 5: Executor Conformance Test (E10/E12)
```rust
// In crates/cli/tests/ or crates/eval/tests/

#[test]
fn e10_manifest_valid_schema() {
    // Elaborate a domain contract with --manifest
    let tenor_path = domains_dir().join("saas/saas_subscription.tenor");
    let bundle = tenor_core::elaborate::elaborate(&tenor_path).unwrap();
    let manifest = build_manifest(bundle);

    // Validate against manifest JSON Schema
    let schema: serde_json::Value = serde_json::from_str(MANIFEST_SCHEMA_STR).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let errors: Vec<_> = validator.iter_errors(&manifest).collect();
    assert!(errors.is_empty(), "Manifest validation errors: {:?}", errors);
}

#[test]
fn e12_etag_determinism() {
    // Elaborate same contract twice, verify identical etag
    let path = domains_dir().join("saas/saas_subscription.tenor");
    let b1 = tenor_core::elaborate::elaborate(&path).unwrap();
    let b2 = tenor_core::elaborate::elaborate(&path).unwrap();
    let etag1 = compute_etag(&b1);
    let etag2 = compute_etag(&b2);
    assert_eq!(etag1, etag2, "Etag must be deterministic");
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Escrow contract was the only integration test | Phase 5 adds 5 domain contracts | Phase 5 (current) | Proves spec handles diverse domains |
| `tenor explain` was a stub (exit 2) | Full implementation needed | Phase 5 (current) | CLI-06 requirement |
| No executor conformance tests | E10-E14 tests needed | Phase 5 (current) | TEST-11 requirement |
| Gap issues found ad hoc | Structured gap log with severity | Phase 5 (current) | Enables systematic spec improvement |

**Deprecated/outdated:**
- The existing `integration_escrow.tenor` conformance test and its eval variants (`escrow_release`, `escrow_compliance`, `escrow_compensate`) are already in the conformance suite from Appendix D. The Trade Finance (letter of credit) contract for DOMN-05 should be a different contract, not a continuation of escrow. The phase description notes "escrow happy path complete" — this refers to the existing conformance tests, not a domain contract.

## Domain-Specific Research: Contract Feature Coverage

### Feature Coverage Matrix

The following matrix maps Tenor spec features to the domain contracts that should exercise them. This ensures comprehensive coverage without forcing features where they don't fit naturally.

| Spec Feature | SaaS | Healthcare | Supply Chain | Energy | Trade Finance |
|---|---|---|---|---|---|
| **TypeDecl (Record)** | PlanFeatures | PolicyCriteria, ReviewRecord | InspectionReport | SupplierScore, CostBreakdown | DocumentSet, Presentation |
| **TypeDecl (TaggedUnion)** | — | DenialReason | DefectType | — | DiscrepancyType |
| **Fact (Bool)** | payment_ok | — | — | — | docs_compliant |
| **Fact (Int)** | seat_count | — | defect_count | — | — |
| **Fact (Decimal)** | — | — | — | — | — |
| **Fact (Money)** | — | — | — | rfp_amount, bid_total | lc_amount, draft_amount |
| **Fact (Enum)** | plan_tier | auth_status | inspection_type | procurement_stage | lc_status |
| **Fact (Text)** | — | — | — | — | — |
| **Fact (Date)** | — | — | — | rfp_deadline | expiry_date |
| **Fact (Record)** | plan_features | policy_rules | — | winning_bid | — |
| **Fact (List)** | — | medical_records | inspection_items | supplier_bids | required_documents |
| **Fact (default)** | payment_ok=true | — | defect_threshold=3 | — | docs_compliant=false |
| **Entity (multi-state)** | Subscription | PriorAuth, AppealCase | Shipment, InspectionLot | RFP, PurchaseOrder | LetterOfCredit, Document |
| **Entity (hierarchy)** | — | — | Shipment > InspectionLot | — | — |
| **Rule (stratum 0)** | fact checks | policy checks | inspection results | bid evaluation | document checks |
| **Rule (stratum 1+)** | composite | eligibility | release decision | award decision | compliance |
| **Rule (verdict_present)** | all | all | all | all | all |
| **Rule (bounded quant ∀)** | — | ∀ record | ∀ item | ∀ bid | ∀ document |
| **Rule (bounded quant ∃)** | — | — | ∃ defect | ∃ qualified_bid | — |
| **Rule (arithmetic)** | seat comparison | — | — | money comparison | money comparison |
| **Persona (multi)** | 3 | 5+ | 3-4 | 4-5 | 4-5 |
| **Operation (single outcome)** | activate, suspend | — | release | — | — |
| **Operation (multi-outcome)** | — | decide_auth | — | award_contract | accept_or_reject |
| **Operation (multi-entity effect)** | — | — | Shipment + Lot | RFP + PO | LC + Document |
| **Flow (linear)** | simple | — | — | — | — |
| **Flow (BranchStep)** | plan check | tier routing | defect check | approval tier | compliance check |
| **Flow (HandoffStep)** | — | reviewer handoff | — | approval chain | beneficiary handoff |
| **Flow (ParallelStep)** | — | — | concurrent inspections | — | — |
| **Flow (SubFlowStep)** | — | appeal sub-flow | — | — | — |
| **Flow (Compensate)** | — | — | rollback inspection | — | — |
| **Flow (Escalate)** | — | escalate to peer review | — | escalate to VP | — |
| **Flow (Terminate)** | cancel | denial | hold | rejection | expiry |
| **Multi-file import** | no | no | types.tenor | types.tenor | no |
| **Money types** | — | — | — | rfp_amount, bid comparisons | lc_amount, draft_amount |
| **Date types** | — | — | — | rfp_deadline | lc_expiry |

### Domain Contract Sizing Estimates

Based on the feature matrix and existing escrow reference:

| Domain | Est. Lines | Personas | Entities | Rules | Operations | Flows | Facts |
|--------|-----------|----------|----------|-------|------------|-------|-------|
| SaaS (SMALL) | ~120 | 3 | 1-2 | 4-5 | 3-4 | 1 | 5-6 |
| Healthcare (LARGE) | ~350+ | 5-6 | 2-3 | 8-12 | 6-8 | 2-3 | 8-10 |
| Supply Chain (MEDIUM) | ~200 | 3-4 | 2-3 | 5-7 | 4-5 | 1-2 | 6-8 |
| Energy (MEDIUM-LARGE) | ~280 | 4-5 | 2-3 | 7-10 | 5-7 | 1-2 | 8-10 |
| Trade Finance (MEDIUM) | ~200 | 4-5 | 2-3 | 5-8 | 4-6 | 1-2 | 6-8 |

For reference, the existing escrow contract (Appendix D) is ~255 lines and is considered non-trivial.

## Domain-Specific Research: Explain Command

### Bundle Structure for Explain
The explain command consumes interchange JSON. The bundle structure is:
```json
{
  "id": "contract_name",
  "kind": "Bundle",
  "tenor": "1.0",
  "tenor_version": "1.1.0",
  "constructs": [
    { "kind": "Fact", "id": "...", ... },
    { "kind": "Entity", "id": "...", ... },
    { "kind": "Rule", "id": "...", ... },
    { "kind": "Persona", "id": "...", ... },
    { "kind": "Operation", "id": "...", ... },
    { "kind": "Flow", "id": "...", ... }
  ]
}
```

The explain command needs to:
1. Group constructs by kind
2. Trace flows to build narrative (entry -> steps -> terminals)
3. Extract fact types and sources for inventory
4. Run analysis and summarize findings for risk section

### Input Handling
The explain command accepts both `.tenor` files and `.json` bundles:
- `.tenor` input: elaborate internally (same as `tenor check`)
- `.json` input: parse directly as interchange bundle (same as `tenor eval`)

Detection by file extension is the simplest approach. The existing CLI already does this pattern implicitly.

### Terminal Styling Approach
Using crossterm for styled output:
```rust
use crossterm::style::{Stylize, Color};

println!("{}", "CONTRACT SUMMARY".bold());
println!("{}", "════════════════".dark_grey());
println!("  Name:       {}", bundle_id.cyan());
println!("  Entities:   {}", entity_summary.white());
```

For `--format markdown`, same logic but output markdown heading syntax instead of ANSI codes.

### Analysis Integration
The explain command should call `tenor_analyze::analyze()` to get S1-S8 results for the risk/coverage section. This is the same API used by `tenor check`. The difference: explain formats findings as human-readable notes, not as structured warnings.

## Domain-Specific Research: Executor Conformance (E10-E14)

### E10: Manifest Serving
Test that `tenor elaborate --manifest <contract.tenor>` produces valid output:
- Output validates against `docs/manifest-schema.json`
- `etag` field is present and is a hex string
- `bundle` field contains a valid interchange bundle
- `tenor` field is "1.1"

### E11: Cold-Start Completeness
Test that the manifest bundle is self-contained:
- All construct references resolve (fact_refs in rules point to declared facts)
- All persona references resolve
- All entity references in operations resolve
- All operation references in flows resolve

This is already guaranteed by the elaborator (Passes 1-5), but the E11 test validates it from the consumer perspective.

### E12: Change Detection (Etag)
Test etag semantics:
- Same contract produces same etag across multiple elaborations (determinism)
- Modified contract produces different etag
- `capabilities` field (if present) does not affect etag

### E13: Dry-Run Evaluation
Test that evaluation can be run without state changes:
- Evaluate rules (read path) — this is already side-effect-free
- Simulate operation execution: persona check + precondition eval + outcome determination WITHOUT applying effects
- The current `tenor-eval` does not have a dry-run API; this may need a new function or flag

Note: A full dry-run API requires a new function in `tenor-eval`. This is a potential implementation item.

### E14: Capability Advertisement
Test manifest capabilities field:
- Static manifests (no capabilities) are valid
- Dynamic manifests with `capabilities: { migration_analysis_mode: "conservative" }` are valid
- Validate against manifest schema

## Open Questions

1. **Dry-run API in tenor-eval**
   - What we know: E13 requires dry-run evaluation (steps 1-3 without step 4 effect application)
   - What's unclear: Whether to add a `dry_run_operation()` function to tenor-eval or test it conceptually
   - Recommendation: Add a minimal `dry_run` function to tenor-eval that runs persona check + precondition eval + outcome determination. The existing `execute_operation` in tenor-eval already separates these steps; dry-run is a subset.

2. **Domain contract file location**
   - What we know: Existing conformance tests live in `conformance/`. Domain contracts serve a different purpose (showcase, not just testing).
   - What's unclear: Whether to put them in `domains/` (separate from conformance) or `conformance/domains/`
   - Recommendation: Use `domains/` at the repo root — these are showcase contracts, not just test fixtures. Register them as eval conformance tests via path reference.

3. **Healthcare contract as "wow" contract**
   - What we know: Healthcare and energy procurement are candidates. Healthcare naturally exercises the most spec features (deep flows, escalation, appeals, multi-stratum). Energy procurement is deeply personal to the user.
   - What's unclear: Whether both should aim for "wow" or just one
   - Recommendation: Make healthcare the "wow" for spec breadth (demonstrates all construct types including Escalate, SubFlowStep, ParallelStep). Make energy procurement the "wow" for domain authenticity (real RFP workflow recognizable to energy industry professionals).

4. **How many eval paths per contract**
   - What we know: The escrow reference has 3 eval fixture sets (release, compliance, compensate)
   - What's unclear: How many paths to test for the larger contracts
   - Recommendation: 2-4 paths per contract. Happy path always required. At least one failure/alternative path. Larger contracts (healthcare, energy) may have 3-4 paths.

5. **crossterm dependency for CLI**
   - What we know: The user prefers staying in Rust. crossterm is the standard Rust terminal styling crate.
   - What's unclear: Whether adding a dependency is acceptable for explain styling
   - Recommendation: Add crossterm. The alternative (hand-rolling ANSI codes) is fragile and non-portable. The user's standing preference is Rust-native solutions.

## Sources

### Primary (HIGH confidence)
- Tenor spec v1.0 (`docs/TENOR.md`) — all construct definitions, evaluation model, executor obligations (E10-E14)
- Existing codebase: `crates/eval/`, `crates/analyze/`, `crates/cli/` — implementation patterns, API surfaces
- Conformance suite: `conformance/eval/positive/escrow_*.tenor` — reference contract and fixture triplet pattern
- `crates/eval/tests/conformance.rs` — eval test registration pattern (run_eval_fixture, run_eval_flow_fixture)

### Secondary (MEDIUM confidence)
- crossterm crate documentation (crates.io) — terminal styling API, version compatibility
- textwrap crate documentation (crates.io) — text wrapping capabilities

### Tertiary (LOW confidence)
- Domain knowledge for contract authoring (healthcare prior auth, supply chain, energy procurement, trade finance) — based on general knowledge. The energy procurement domain has explicit user guidance (real product background).

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — using existing crates + 2 small additions (crossterm, textwrap)
- Architecture: HIGH — patterns directly extend existing conformance test patterns
- Domain contracts: MEDIUM — contract authoring quality depends on domain understanding, but the toolchain validation pipeline is well-defined
- Explain command: HIGH — straightforward CLI feature with well-defined output sections
- Executor conformance: HIGH — spec defines E10-E14 precisely; implementation is straightforward testing
- Pitfalls: HIGH — based on observed spec constraints (no aggregates, no entity state predicates, no pattern matching)

**Research date:** 2026-02-22
**Valid until:** 2026-04-22 (stable — spec is frozen, toolchain is built)
