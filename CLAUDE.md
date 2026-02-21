# Tenor — Agent Context

## DSL keyword casing

**Tenor keywords are lowercase.** The parser expects lowercase:

```
fact, entity, rule, operation, flow, type
```

Uppercase construct names (`Rule`, `Fact`, `Entity`, `Operation`, `Flow`) appear in:
- `TENOR.md` when referring to formal construct definitions as concepts
- Comments and prose that discuss the language rather than writing it
- The interchange JSON `"kind"` field values

They do **not** appear in `.tenor` source files. Generated DSL must use lowercase.

## Build and test

```bash
# Build
cd elaborator && cargo build

# Run conformance suite (from repo root)
cd elaborator && cargo run -- run ../conformance
# Expected: 47/47 passing

# Elaborate a single file
cd elaborator && cargo run -- elaborate path/to/file.tenor
```

## Repository layout

```
TENOR.md          — full formal specification (v0.3)
conformance/      — elaborator conformance suite
  positive/       — valid DSL → expected interchange JSON
  negative/       — invalid DSL → expected error JSON
  numeric/        — decimal/money precision fixtures
  promotion/      — numeric type promotion fixtures
  shorthand/      — DSL shorthand expansion fixtures
  cross_file/     — multi-file import fixtures
  parallel/       — parallel entity conflict fixtures
elaborator/src/   — reference elaborator (Rust)
  lexer.rs        — tokenizer
  parser.rs       — DSL → raw AST
  elaborate.rs    — 6-pass elaboration + serialization
  runner.rs       — conformance suite runner
  tap.rs          — TAP output
  error.rs        — ElabError type
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
