# Tenor — Agent Context

## DSL keyword casing

**Tenor keywords are lowercase.** The parser expects lowercase:

```
fact, entity, rule, operation, flow, type
```

Uppercase construct names (`Rule`, `Fact`, `Entity`, `Operation`, `Flow`) appear in:
- `docs/tenor-language-specification.md` when referring to formal construct definitions as concepts
- Comments and prose that discuss the language rather than writing it
- The interchange JSON `"kind"` field values

They do **not** appear in `.tenor` source files. Generated DSL must use lowercase.

## Pre-commit quality gates (mandatory)

**Before EVERY commit that touches Rust code, run ALL of the following in order:**

```bash
cargo fmt --all
cargo build --workspace
cargo test --workspace
cargo run -p tenor-cli -- test conformance
cargo clippy --workspace -- -D warnings
```

**If the commit touches `crates/eval/`, `crates/interchange/`, or `crates/tenor-eval-wasm/`**, also run:

```bash
cd crates/tenor-eval-wasm && wasm-pack build --target nodejs && wasm-pack test --node && cd ../..
```

The WASM crate is excluded from the workspace — workspace commands do not build or test it. CI runs both.

**Every single check must pass before committing.** No exceptions. No "I'll fix it later." If any check fails, fix it before the commit. You own all test failures, lint failures, and formatting issues. CI runs all checks and treats clippy warnings as errors (`-D warnings`). This is non-negotiable.

## Build and test

```bash
# Build all crates
cargo build --workspace

# Run conformance suite
cargo run -p tenor-cli -- test conformance

# Run all tests (schema validation, unit tests)
cargo test --workspace

# Elaborate a single file
cargo run -p tenor-cli -- elaborate path/to/file.tenor

# Validate interchange JSON against schema
cargo run -p tenor-cli -- validate path/to/bundle.json

# Show all CLI subcommands
cargo run -p tenor-cli -- --help
```

CI pipeline: `.github/workflows/ci.yml` runs on push/PR to `main`.
Stages: workspace build, conformance suite, schema validation + unit tests, formatting, clippy.

## Repository layout

```
docs/tenor-language-specification.md           — full formal specification (v1.0)
conformance/            — elaborator conformance suite
  positive/             — valid DSL -> expected interchange JSON
  negative/             — invalid DSL -> expected error JSON
  numeric/              — decimal/money precision fixtures
  promotion/            — numeric type promotion fixtures
  shorthand/            — DSL shorthand expansion fixtures
  cross_file/           — multi-file import fixtures
  parallel/             — parallel entity conflict fixtures
crates/
  core/src/             — tenor-core library (elaboration pipeline)
    ast.rs              — shared AST types
    elaborate.rs        — 6-pass orchestrator
    error.rs            — ElabError type
    lexer.rs            — tokenizer
    parser.rs           — DSL -> raw AST
    pass1_bundle.rs     — pass 0+1: import resolution, bundle assembly
    pass2_index.rs      — pass 2: construct indexing
    pass3_types.rs      — pass 3: type environment
    pass4_typecheck.rs  — pass 4: type resolution, expression checking
    pass5_validate.rs   — pass 5: structural validation
    pass6_serialize.rs  — pass 6: JSON interchange serialization
  cli/src/              — tenor-cli binary (command-line tool)
    main.rs             — CLI entry point (clap-based subcommand dispatch)
    runner.rs           — conformance suite runner
    tap.rs              — TAP v14 output formatter
    ambiguity/          — AI ambiguity testing module
  eval/src/             — tenor-eval library (contract evaluator, Phase 3)
  analyze/src/          — tenor-analyze library (static analysis, Phase 4)
  codegen/src/          — tenor-codegen library (code generation, Phase 6)
  lsp/src/              — tenor-lsp library (Language Server Protocol, Phase 8)
```

## Conformance fixture conventions

- **Positive tests**: `<name>.tenor` + `<name>.expected.json` — must elaborate without error and match exactly
- **Negative tests**: `<name>.tenor` + `<name>.expected-error.json` — must fail with the specified error
- Error JSON fields: `pass`, `construct_kind`, `construct_id`, `field`, `file`, `line`, `message`
- All JSON keys in interchange output are sorted lexicographically within each object

## Elaborator pass overview

| Pass | Input → Output | Key obligations |
| ---- | -------------- | --------------- |
| 0 | Source text → tokens + parse tree | Lex, parse, record line numbers |
| 1 | Parse trees → unified bundle | Import resolution, cycle detection, duplicate id check |
| 2 | Bundle → construct index | Index by (kind, id) |
| 3 | TypeDecl + Fact decls → type environment | TypeDecl cycle detection, named type resolution |
| 4 | ASTs → typed ASTs | Type-check expressions, apply promotion rules, resolve refs |
| 5 | Typed ASTs → validation report | Entity, Operation, Rule, Flow structural checks |
| 6 | Validated ASTs → JSON interchange | Canonical serialization, sorted keys, structured numeric values |

## Key serialization rules

- Decimal and Money defaults serialize as `{"kind": "decimal_value", "precision": P, "scale": S, "value": "..."}` using the **declared** type's P/S — not inferred from the literal string
- Multiplication in interchange: `{"left": {"fact_ref": "x"}, "literal": N, "op": "*", "result_type": {...}}`
- `comparison_type` is emitted on Compare nodes for: Money (always), Int × Decimal cross-type, Mul × Int
- `→` and `->` are the same token in transitions; comma is also accepted as separator
