# Tenor Audit Agent Example

Compliance report generator that traces contract evaluation provenance chains. Given a contract and facts, the agent evaluates, then walks every verdict back through its dependency chain to produce a full audit trail.

## Why This Matters

Regulatory auditors need to know **why** a decision was made, not just **what** the decision was. When an insurance claim is denied, a trade is blocked, or access is restricted, the audit trail must show:

- Which rules fired and in what order (stratum)
- What facts drove each rule's decision
- Which verdicts depended on other verdicts (multi-level reasoning)
- Whether any declared facts were ignored (potential gaps)

Tenor's provenance system captures this automatically. This agent turns that provenance into a structured compliance report.

## Architecture

```
CLI (cli.ts)
  |
  +-- Auditor (auditor.ts)
  |     |
  |     +-- TenorClient SDK
  |     |     |
  |     |     +-- tenor serve (HTTP API)
  |     |           |
  |     |           +-- Contract evaluation with provenance
  |     |
  |     +-- Provenance walking + gap detection
  |
  +-- Report formatter (report.ts)
        |
        +-- Terminal (ANSI colors)
        +-- Markdown
```

## Prerequisites

- Node.js 22+
- A running `tenor serve` instance with contracts loaded

## Quick Start

1. **Start the Tenor evaluator:**

```bash
# From the repo root
cargo run -p tenor-cli -- serve --port 8080 domains/saas/saas_subscription.tenor
```

2. **Install dependencies:**

```bash
cd examples/audit-agent
npm install
```

3. **Run the audit with sample facts:**

```bash
# Terminal output
npm run audit -- --contract saas_subscription --facts sample-facts/saas.json

# Markdown report
npm run audit -- --contract saas_subscription --facts sample-facts/saas.json \
  --format markdown --output audit-report.md
```

## CLI Options

| Option | Required | Default | Description |
|--------|----------|---------|-------------|
| `--contract <id>` | Yes | - | Contract ID to audit |
| `--facts <path>` | Yes | - | Path to JSON file with fact values |
| `--format <fmt>` | No | `terminal` | Output format: `terminal` or `markdown` |
| `--output <path>` | No | stdout | Write report to file |
| `--url <url>` | No | `http://localhost:8080` | Tenor evaluator URL |
| `--help` | No | - | Show usage |

## Report Sections

### Verdict Audit Trail

Each verdict in the evaluation is traced through its full provenance:

- **Type and payload**: What the verdict decided (e.g., `approve_seat = true`)
- **Rule**: Which rule produced this verdict
- **Stratum**: The evaluation order (lower strata evaluate first)
- **Facts used**: Direct fact inputs to this rule
- **Dependency chain**: The full recursive chain showing how facts flow through rules to produce verdicts

```
verdict:approve_seat <- rule:seat_check <- fact:current_seat_count, fact:plan_features
```

### Fact Coverage Matrix

Shows which facts influence which verdicts and through which rules. This tells auditors:

- **Full coverage**: Every provided fact influences at least one verdict
- **Gaps**: Facts that were provided but had no influence on the outcome

### Compliance Gaps

Three types of gaps are detected:

| Gap Type | Severity | Meaning |
|----------|----------|---------|
| `orphan_fact` | Warning | A fact was provided but no verdict references it. The fact had no influence on the evaluation. |
| `shallow_provenance` | Info | A verdict depends only on facts (no verdict chain). Simple rules -- not necessarily a problem. |
| `single_rule_dependency` | Info | A verdict type is produced by exactly one rule. No redundancy if that rule has a defect. |

### Summary

Aggregate statistics and an overall assessment:
- **No critical gaps**: Evaluation provenance is well-covered.
- **Warnings only**: Review recommended but no blocking issues.
- **Critical gaps**: Review required before compliance sign-off.

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Audit complete, no critical gaps |
| 1 | Audit complete with critical gaps, or audit failed |

## Use Cases

- **Insurance claim adjudication**: Trace why a claim was approved or denied through all underwriting rules
- **Healthcare decision tracing**: Document which clinical facts drove a treatment authorization
- **Trade finance compliance**: Show regulators the full decision chain for trade approval
- **Access control auditing**: Prove why a user was granted or denied access to a resource

## Note

This is a reference implementation showing the audit/compliance pattern. For production use, add structured logging, report archival, and integration with your compliance management system.
