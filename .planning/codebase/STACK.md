# Technology Stack

**Analysis Date:** 2026-02-21

## Languages

**Primary:**
- Rust 2021 edition - All core elaborator implementation

## Runtime

**Environment:**
- Rust 1.56+ (via Cargo.toml edition = "2021")

**Build System:**
- Cargo (Rust package manager)
- Lockfile: Present (`elaborator/Cargo.lock`)

## Frameworks

**Core:**
- None - This is a standalone CLI tool, not a framework-dependent application

**Testing:**
- TAP (Test Anything Protocol) - Custom TAP output implementation in `elaborator/src/tap.rs` for conformance suite reporting
- Manual conformance fixture testing via test runner in `elaborator/src/runner.rs`

**Build/Dev:**
- Cargo - Build configuration and dependency management

## Key Dependencies

**Critical:**
- `serde` 1.0.228 - Serialization/deserialization framework with derive macros
  - `serde_derive` 1.0.228 - Procedural macros for `#[derive(Serialize, Deserialize)]`
  - `serde_core` 1.0.228 - Core serialization traits

- `serde_json` 1.0.149 - JSON serialization and interchange format
  - Used for: Elaborate output (interchange JSON), test fixture comparison, error JSON serialization
  - Depends on: `itoa` 1.0.17 (integer to ASCII), `memchr` 2.8.0 (string parsing), `zmij` 1.0.21 (JSON number handling)

**Macro Infrastructure:**
- `proc-macro2` 1.0.106 - Procedural macro utilities
- `quote` 1.0.44 - Code generation for macros
- `syn` 2.0.117 - Rust syntax parsing and AST manipulation
- `unicode-ident` 1.0.24 - Unicode identifier validation

## Configuration

**Build Configuration:**
- `Cargo.toml`: `elaborator/Cargo.toml` defines package, dependencies, and edition
- No external config files (YAML, TOML, environment-based) for the elaborator itself
- Test fixtures are plain `.tenor` source files (DSL) paired with `.expected.json` or `.expected-error.json` fixture files

**Environment:**
- No `.env` files or environment variable configuration
- CLI arguments parsed directly in `main()` in `elaborator/src/main.rs`
- Paths passed as command-line arguments

## Platform Requirements

**Development:**
- Rust toolchain (1.56+ for edition 2021)
- Cargo for building and running

**Production:**
- Compiled binary runs on any platform with native x86_64 or ARM64 support
- No runtime dependencies beyond Rust std library
- Single stateless binary

## Build Commands

```bash
cd elaborator && cargo build                # Debug build
cd elaborator && cargo build --release      # Optimized binary
cd elaborator && cargo run -- elaborate <file.tenor>  # Single file elaboration
cd elaborator && cargo run -- run ../conformance     # Run test suite
```

## No External Integrations

This is a language elaborator and reference implementation. It has:
- No network I/O
- No database dependencies
- No external API calls
- No authentication/authorization framework dependencies
- No cloud platform SDKs
- No logging frameworks (uses `eprintln!` for errors)

Input/output is:
- **File I/O only**: Read `.tenor` DSL files, write JSON to stdout, errors to stderr
- **Self-contained**: All validation, parsing, type checking, and elaboration is local computation

---

*Stack analysis: 2026-02-21*
