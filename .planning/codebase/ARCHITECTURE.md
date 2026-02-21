# Architecture

**Analysis Date:** 2026-02-21

## Pattern Overview

**Overall:** Six-pass deterministic elaborator—a compiler that transforms Tenor DSL source code into canonical JSON interchange format. Each pass has a single, well-defined responsibility with no back-edges.

**Key Characteristics:**
- Stratified evaluation with structural termination guarantees
- Static analyzability as a core constraint—no runtime type errors in valid contracts
- Deterministic transformation: identical inputs always produce identical outputs
- Provenance tracking throughout—every error carries file/line information and every value carries derivation metadata

## Layers

**Lexing (Pass 0):**
- Purpose: Tokenize raw source text into a sequence of tokens with line number tracking
- Location: `elaborator/src/lexer.rs`
- Contains: Token definitions (Word, Str, Int, Float, logical operators, punctuation), Unicode symbol support (∧, ∨, ¬, ∀, ∈)
- Depends on: Raw source text and file paths
- Used by: Parser (immediately consumes all tokens from a file)

**Parsing (Pass 0):**
- Purpose: Build abstract syntax tree (AST) from tokens, record provenance (file, line) for every construct
- Location: `elaborator/src/parser.rs` (1300 lines)
- Contains: `RawConstruct` enum (Import, TypeDecl, Fact, Entity, Rule, Operation, Flow), `RawType`, `RawExpr`, `RawTerm`, `RawLiteral` types—all unresolved
- Depends on: Tokens from lexer
- Used by: Bundle loader (Pass 1) and all subsequent passes

**Bundle Assembly (Pass 1):**
- Purpose: Recursively load imported files, perform cycle detection, cross-file duplicate checking, flatten into single construct sequence
- Location: `elaborator/src/elaborate.rs` (functions: `load_bundle`, `load_file`, `check_cross_file_dups`)
- Contains: Imports-first depth-first traversal; cycle detection via visited set and in-stack tracking
- Depends on: Parsed constructs from all files in import graph
- Used by: Indexing pass; produces unified construct sequence

**Construct Indexing (Pass 2):**
- Purpose: Build `Index`—a map from (kind, id) to provenance; detect same-file duplicates; record rule→verdict mappings
- Location: `elaborator/src/elaborate.rs` (function: `build_index`)
- Contains: `Index` struct with six maps: facts, entities, rules, operations, flows, type_decls; rule_verdicts (rule_id → verdict type); verdict_strata (verdict type → (rule_id, stratum))
- Depends on: Construct bundle from Pass 1
- Used by: Type environment construction (Pass 3), validation (Pass 5)

**Type Environment (Pass 3):**
- Purpose: Resolve TypeDecl references, detect TypeDecl cycles, build mapping from named types to fully concrete BaseTypes
- Location: `elaborator/src/elaborate.rs` (functions: `build_type_env`, `detect_typedecl_cycle`, `resolve_typedecl`)
- Contains: TypeEnv = HashMap<String, RawType>; cycle detection via DFS with in-stack tracking
- Depends on: Construct index from Pass 2
- Used by: Type resolution (Pass 4)

**Type Resolution & Checking (Pass 4):**
- Purpose: Replace all TypeRef nodes with concrete BaseTypes; type-check all expressions; apply numeric promotion rules
- Location: `elaborator/src/elaborate.rs` (functions: `resolve_types`, `type_check_rules`, `type_check_expr`, `type_of_fact_term`)
- Contains: Expression type inference for: Compare nodes, Forall loops, Mul operations, verdict_present checks; comparison_type emission for cross-type operations
- Depends on: Type environment from Pass 3
- Used by: Validation (Pass 5)

**Construct Validation (Pass 5):**
- Purpose: Structural validation—entity DAG acyclicity, rule stratum dependencies, operation transition validity, flow structure, verdict references
- Location: `elaborator/src/elaborate.rs` (functions: `validate`, `validate_entity`, `validate_entity_dag`, `validate_rule`, `validate_operation`, `validate_operation_transitions`, `validate_flow`)
- Contains: Entity state machine validation, rule stratum ordering, operation effect checking, flow step graph validation
- Depends on: Typed constructs from Pass 4
- Used by: Serialization (Pass 6)

**Interchange Serialization (Pass 6):**
- Purpose: Transform validated ASTs into canonical JSON with sorted keys, precise numeric representation
- Location: `elaborator/src/elaborate.rs` (function: `serialize`)
- Contains: JSON builder using `serde_json::Value`; decimal defaults serialize as `{"kind": "decimal_value", "precision": P, "scale": S, "value": "..."}`; multiplication nodes include `comparison_type` where needed
- Depends on: Validated constructs from Pass 5
- Used by: Main program output to stdout

## Data Flow

**Core Pipeline:**

1. File I/O → Lexer → Tokens
2. Tokens → Parser → RawConstruct (unresolved AST with provenance)
3. RawConstruct (all files) → Bundle Loader → Flattened RawConstruct sequence (imports-first order)
4. RawConstruct sequence → Indexer → Index (deduplication, construct lookup)
5. Index + RawConstruct sequence → Type Environment Builder → TypeEnv (named types resolved)
6. RawConstruct sequence + TypeEnv → Type Resolver → RawConstruct (TypeRef → BaseType)
7. Typed RawConstruct sequence → Type Checker → Typed RawConstruct (expressions validated)
8. Typed RawConstruct sequence → Validator → Validation Report (structural checks)
9. Validated RawConstruct sequence → Serializer → JSON Value → Pretty-print to stdout

**Error Handling Strategy:** Every pass returns `Result<T, ElabError>`. First error encountered stops elaboration. `ElabError` structure includes: pass number (0–6), construct kind/id, field name (if applicable), file path, line number, human-readable message. Errors serialize to JSON matching `expected-error.json` format.

**State Management:**
- Pass 0: Stateless per file (lexer state is just position pointer; parser state is token stream)
- Pass 1: Import resolution state carried through recursive descent (visited set, stack for cycle detection)
- Pass 2: Mutable accumulation into Index HashMap (no back-refs)
- Pass 3: Mutable TypeEnv HashMap built bottom-up from TypeDecl graph (no circular dependencies by design)
- Pass 4: Immutable transformation of RawConstruct via TypeEnv lookup
- Pass 5: Read-only validation using Index and already-typed constructs
- Pass 6: Immutable traversal to JSON

## Key Abstractions

**RawConstruct Enum:**
- Purpose: Represents the six semantic constructs (Fact, Entity, Rule, Operation, Flow) plus tooling constructs (Import, TypeDecl)
- Examples: `elaborator/src/parser.rs` lines 78–180 (enum definition with all variants)
- Pattern: Each variant carries its id, provenance (file/line), and variant-specific fields; Import and TypeDecl are elaborator-internal

**RawType Enum:**
- Purpose: Represents BaseType before TypeRef resolution; supports all twelve base types plus TypeRef node
- Examples: `elaborator/src/parser.rs` lines 20–34
- Pattern: Recursive (List wraps element type; Record wraps field types); TypeRef(String) is resolved in Pass 4

**RawExpr & RawTerm:**
- Purpose: PredicateExpression terms (Compare, VerdictPresent, And, Or, Not, Forall); Fact/field references, literals, arithmetic
- Examples: `elaborator/src/parser.rs` lines 48–75
- Pattern: Tree structure; Mul node for multiplication; Forall carries bound variable and domain list_ref

**Index:**
- Purpose: O(1) lookup of any construct by (kind, id); rule verdict mappings for validation
- Location: `elaborator/src/elaborate.rs` lines 191–202
- Pattern: Six separate HashMaps per construct kind; separate rule_verdicts and verdict_strata maps

**Provenance:**
- Purpose: Tracks source location (file path, line number) for error reporting; attached to every construct and every error
- Pattern: Simple struct with file: String, line: u32; cloned through passes for error context

## Entry Points

**Command: `tenor-elaborator run <suite-dir>`:**
- Location: `elaborator/src/main.rs` (lines 21–37)
- Triggers: User runs conformance suite
- Responsibilities: Invoke `runner::run_suite()`, orchestrate all test categories (positive, negative by pass, cross-file, parallel, numeric, promotion, shorthand), output TAP format

**Command: `tenor-elaborator elaborate <file.tenor>`:**
- Location: `elaborator/src/main.rs` (lines 39–57)
- Triggers: User elaborates single Tenor file
- Responsibilities: Call `elaborate::elaborate()`, catch errors, pretty-print JSON or error JSON to stdout

**Function: `elaborate(root_path: &Path)`:**
- Location: `elaborator/src/elaborate.rs` (lines 23–46)
- Entry point for all elaboration: coordinates all six passes in order
- Returns: Either `Value` (JSON interchange bundle) or `ElabError`

## Error Handling

**Strategy:** Fail-fast with structured error output.

**Patterns:**
- All pass functions return `Result<T, ElabError>` or `Result<(), ElabError>`
- First error stops elaboration immediately
- Error JSON includes: pass (0–6), construct_kind, construct_id, field, file, line, message
- JSON keys always sorted alphabetically (enforced in `ElabError::to_json_value()`)
- Lex errors: file, line, message only (pass 0)
- Parse errors: file, line, message only (pass 0)
- All other errors: all fields populated as applicable

**Examples:**
- Pass 1: "duplicate Fact id 'x': first declared in other_file.tenor"
- Pass 2: "duplicate Rule id 'y': first declared at line 42"
- Pass 3: "TypeDecl cycle: TypeA → TypeB → TypeC → TypeA"
- Pass 4: "type mismatch in Compare: fact 'x' is Decimal, literal '42' is Int"
- Pass 5: "Operation 'op_x' references undefined Entity 'BadEntity'"

## Cross-Cutting Concerns

**Logging:** None—elaborator is silent on success, prints JSON error on failure

**Validation:** Distributed across passes; Validation pass (5) is final structural check; Pass 4 type-checking is simultaneous with type resolution

**Numeric Handling:** Pass 4 applies promotion rules; Decimal and Money defaults serialize with declared (not inferred) precision/scale; Multiplication nodes include `comparison_type` field when comparing Money or cross-type Int×Decimal

---

*Architecture analysis: 2026-02-21*
