# Codebase Structure

**Analysis Date:** 2026-02-21

## Directory Layout

```
tenor/
├── docs/TENOR.md                  # Formal specification v0.3 (81 KB)
├── elaborator/                    # Reference elaborator (Rust)
│   ├── Cargo.toml                 # Package manifest (serde, serde_json)
│   ├── src/
│   │   ├── main.rs                # CLI entry point
│   │   ├── lexer.rs               # Tokenizer (239 lines)
│   │   ├── parser.rs              # DSL → RawAST (1300 lines)
│   │   ├── elaborate.rs           # 6-pass elaboration (2066 lines)
│   │   ├── error.rs               # ElabError type (60 lines)
│   │   ├── runner.rs              # Conformance suite runner (259 lines)
│   │   └── tap.rs                 # TAP test output format (64 lines)
├── conformance/                   # 47 elaborator conformance tests
│   ├── positive/                  # Valid DSL → expected JSON
│   │   ├── fact_basic.tenor
│   │   ├── fact_basic.expected.json
│   │   ├── entity_basic.tenor
│   │   ├── entity_basic.expected.json
│   │   ├── rule_basic.tenor
│   │   ├── rule_basic.expected.json
│   │   ├── operation_basic.tenor
│   │   ├── operation_basic.expected.json
│   │   ├── flow_basic.tenor
│   │   ├── flow_basic.expected.json
│   │   ├── typedecl_basic.tenor
│   │   ├── typedecl_basic.expected.json
│   │   └── integration_escrow.tenor (7.9 KB comprehensive example)
│   │   └── integration_escrow.expected.json (19 KB)
│   ├── negative/
│   │   ├── pass0/                 # Lex/parse errors (2 tests)
│   │   │   ├── bad_token.tenor
│   │   │   └── unterminated_string.tenor
│   │   ├── pass1/                 # Import resolution errors
│   │   ├── pass2/                 # Duplicate ID errors
│   │   ├── pass3/                 # Type environment errors (TypeDecl cycles)
│   │   ├── pass4/                 # Type-checking errors
│   │   ├── pass5/                 # Validation errors
│   │   └── pass6/                 # Serialization errors (rare)
│   ├── cross_file/                # Multi-file import tests
│   │   ├── rules.tenor            # Root file (imports facts.tenor)
│   │   ├── facts.tenor            # Leaf file
│   │   └── bundle.expected.json
│   ├── parallel/                  # Parallel entity conflict tests
│   │   ├── conflict_direct.tenor
│   │   └── conflict_transitive.tenor
│   ├── numeric/                   # Decimal/Money precision fixtures
│   ├── promotion/                 # Numeric type promotion fixtures
│   └── shorthand/                 # DSL shorthand expansion fixtures
├── CLAUDE.md                      # Project instructions (agent context)
├── README.md                      # Overview, constraints, examples
├── STABILITY.md                   # Pre-release notice
└── CONTRIBUTING.md                # Contribution guidelines
```

## Directory Purposes

**`elaborator/src/`:**
- Purpose: Reference elaborator implementation in Rust
- Contains: Lexer, parser, type system, elaboration passes, conformance runner
- Key files: `elaborate.rs` (core logic), `parser.rs` (AST definition)

**`elaborator/src/main.rs`:**
- Purpose: CLI command dispatcher
- Responsibilities: Route `run` vs `elaborate` commands, handle file I/O, exit codes

**`conformance/positive/`:**
- Purpose: Valid Tenor DSL files that must elaborate without error
- Contains: Paired `.tenor` and `.expected.json` files
- Pattern: One construct per test (e.g., `fact_basic.tenor`), plus one integration test
- Key files: `integration_escrow.tenor` (comprehensive example using all constructs)

**`conformance/negative/{pass0..pass6}/`:**
- Purpose: Invalid Tenor DSL files that must fail at the given pass with exact error
- Contains: `.tenor` file + `.expected-error.json` file
- Pattern: One error case per file; error JSON includes pass, construct_kind, construct_id, field, file, line, message

**`conformance/cross_file/`:**
- Purpose: Test multi-file import, cycle detection, bundle assembly
- Pattern: Root file (`rules.tenor`) imports leaf file (`facts.tenor`); elaborator must flatten to single `bundle.expected.json`

**`conformance/parallel/`:**
- Purpose: Test parallel entity conflict detection (Pass 5 validation)
- Contents: Entities declared in parallel that violate DAG constraint

**`conformance/numeric/`, `conformance/promotion/`, `conformance/shorthand/`:**
- Purpose: Specialized fixture sets for decimal handling, type promotion, DSL shorthand expansion
- Pattern: Positive tests only (no error fixtures)

**`docs/TENOR.md`:**
- Purpose: Formal language specification (v0.3, 81 KB)
- Contains: Design constraints (C1–C7), BaseType definitions, Construct specifications, Evaluation model, ElaboratorSpec

## Key File Locations

**Entry Points:**
- `elaborator/src/main.rs`: CLI commands (lines 11–64)
  - `run` command: `runner::run_suite(&suite_dir)`
  - `elaborate` command: `elaborate::elaborate(&path)`

**Configuration:**
- `elaborator/Cargo.toml`: Package name = "tenor-elaborator", dependencies: serde, serde_json
- `CLAUDE.md`: Project instructions for Claude agents (includes build/test commands)

**Core Logic:**
- `elaborator/src/elaborate.rs`: All six passes
  - Pass 0+1 (lines 52–73): `load_bundle()`, `load_file()`, `check_cross_file_dups()`
  - Pass 2 (lines 204–285): `build_index()`
  - Pass 3 (lines 293–319): `build_type_env()`, `detect_typedecl_cycle()`
  - Pass 4 (lines 441–688): `resolve_types()`, `type_check_rules()`, `type_check_expr()`
  - Pass 5 (lines 690–1100+): `validate()`, `validate_entity()`, `validate_rule()`, `validate_operation()`, `validate_flow()`
  - Pass 6 (lines ~1700–2066): `serialize()`

**Data Model Definitions:**
- `elaborator/src/parser.rs`: RawConstruct, RawType, RawExpr, RawTerm enums (lines 1–180)
  - RawConstruct (lines 79–180): 7 variants (Import, TypeDecl, Fact, Entity, Rule, Operation, Flow)
  - RawType (lines 20–34): 12 variants + TypeRef
  - RawExpr (lines 48–66): 6 variants (Compare, VerdictPresent, And, Or, Not, Forall)

**Error Handling:**
- `elaborator/src/error.rs`: ElabError struct and JSON serialization (lines 1–61)
  - `ElabError::new()`: Construct full error with all context
  - `ElabError::to_json_value()`: Serialize to expected-error.json format with sorted keys

**Testing:**
- `elaborator/src/runner.rs`: Conformance suite orchestration
  - `run_suite()`: Coordinates all test categories
  - `run_positive_dir()`: Tests that must pass
  - `run_negative_tests()`: Tests that must fail with specific error
  - `run_cross_file_tests()`: Multi-file elaboration tests
  - `run_parallel_tests()`: Entity DAG conflict tests
  - TAP output via `crate::tap::Tap`

**Lexer & Parser:**
- `elaborator/src/lexer.rs`: Token enum, `lex()` function
  - Token types: Word, Str, Int, Float, punctuation, operators, Unicode symbols (∧∨¬∀∈)
  - Line tracking during lexing
- `elaborator/src/parser.rs`: Parser state machine, token consumption, RawConstruct building
  - Entry point: `parse()` function (line ~1200)
  - Precedence handling for expressions
  - Error recovery: reports errors without panicking

## Naming Conventions

**Files:**
- Construct test files: `<construct>_<case>.tenor` (e.g., `fact_basic.tenor`, `rule_mul_valid.tenor`)
- Expected output: `<name>.expected.json` (success case)
- Expected errors: `<name>.expected-error.json` (failure case)
- Integration tests: `integration_<domain>.tenor` (e.g., `integration_escrow.tenor`)

**Directories:**
- Construct types form directory names in lowercase: `positive/`, `negative/`, `cross_file/`, `parallel/`
- Pass-organized error tests: `negative/pass0/`, `negative/pass1/`, etc.
- Feature-focused tests: `numeric/`, `promotion/`, `shorthand/`

**Rust Code:**
- Module names match file names: `lexer`, `parser`, `elaborate`, `error`, `runner`, `tap`
- Public functions use snake_case: `load_bundle()`, `build_index()`, `type_check_rules()`
- Structs use PascalCase: `Index`, `ElabError`, `Provenance`, `Spanned`
- Enums use PascalCase variants: `RawConstruct::Fact`, `Token::Word`, `RawExpr::Compare`
- Type aliases use snake_case: `TypeEnv` (actually a HashMap alias at line 291)

## Where to Add New Code

**New Positive Test:**
- `.tenor` file: `conformance/positive/<name>.tenor`
- Expected output: `conformance/positive/<name>.expected.json`
- Follow structure: Use DSL keywords (lowercase), reference examples in `integration_escrow.tenor`

**New Negative Test (for specific pass):**
- `.tenor` file: `conformance/negative/pass<N>/<name>.tenor` (where N = pass number)
- Expected error: `conformance/negative/pass<N>/<name>.expected-error.json`
- Match format: JSON with keys: pass, construct_kind, construct_id, field, file, line, message

**New Elaborator Feature (unlikely—spec is finalized):**
- Add token type to `Token` enum in `elaborator/src/lexer.rs` if new syntax
- Extend `RawConstruct` / `RawType` / `RawExpr` variants in `elaborator/src/parser.rs`
- Implement parsing in parser (add case to token matching)
- Add pass logic in `elaborator/src/elaborate.rs` (which pass validates/transforms the new construct)
- Add validation rules in relevant `validate_*()` function
- Add serialization in `serialize()` function (Pass 6)
- Add conformance tests in `conformance/positive/` and `conformance/negative/`

**New Utility Function:**
- In `elaborator/src/elaborate.rs`: Type-checking, validation, or serialization helpers
- In `elaborator/src/parser.rs`: AST construction or token matching
- Helper functions are private (`fn`, not `pub fn`) unless used by multiple modules

## Special Directories

**`.planning/codebase/`:**
- Purpose: GSD codebase analysis documents (auto-generated)
- Contents: ARCHITECTURE.md, STRUCTURE.md, CONVENTIONS.md, TESTING.md, CONCERNS.md, STACK.md, INTEGRATIONS.md
- Generated: Yes (by `/gsd:map-codebase` command)
- Committed: Yes (.gitignore does not exclude)

**`.claude/`:**
- Purpose: Claude agent working directory
- Contents: Conversation history, intermediate artifacts
- Generated: Yes (by Claude during `/gsd` commands)
- Committed: No (not in repo, likely ignored)

**`target/`:**
- Purpose: Cargo build artifacts
- Generated: Yes (by `cargo build`)
- Committed: No (.gitignore excludes)

**`.git/`:**
- Purpose: Git repository metadata
- Committed: Yes (implicit)

---

*Structure analysis: 2026-02-21*
