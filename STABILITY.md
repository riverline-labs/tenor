# Stability

Tenor is a **v1.0 stable** project.

## Stable

- **Specification:** v1.0 with three amendments (Source Declarations, Multi-Instance Entities, Trust & Security)
- **Interchange format:** v1.1.0 (minor version bumps from additive amendments)
- **Elaborator:** Stable — the DSL parser and 6-pass elaboration pipeline produce conformant interchange output
- **Evaluator:** Stable — the contract evaluator (tenor-eval) honors all v1.0 semantics
- **Contract language syntax:** Stable — existing `.tenor` files will continue to elaborate correctly
- **Public API surface:** Stable — `tenor-eval` and `tenor-storage` traits maintain backward compatibility

## Not Yet Stable

- **Hosted platform** — in development
- **Marketplace** — in development
- **SDK packaging** — published but pre-1.0 semver
