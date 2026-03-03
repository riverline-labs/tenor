# Contributing

Tenor's spec is frozen at v1.0. The spec (`docs/tenor-language-specification.md`) and conformance suite are the source of truth.

## Before you start

- Read the design constraints in the spec (§2). All changes must be consistent with C1–C7. Changes that violate them will be rejected regardless of ergonomic benefit.
- Open an issue before opening a pull request.

## Development

```bash
cargo fmt --all
cargo build --workspace
cargo test --workspace
cargo run -p tenor-cli -- test conformance
cargo clippy --workspace -- -D warnings
```

All five checks must pass before submitting a PR. CI enforces this.

## What lives here

Elaborator, evaluator, CLI, LSP, SDK, conformance suite, and spec.
