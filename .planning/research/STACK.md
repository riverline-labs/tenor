# Stack Research

**Domain:** DSL toolchain (compiler, evaluator, code generator, editor extension, documentation)
**Researched:** 2026-02-21
**Confidence:** HIGH (verified via docs.rs, crates.io, official docs)

## Existing Stack (Constraints)

The elaborator already ships with these. All additions must be compatible.

| Technology | Version | Purpose | Constraint |
|------------|---------|---------|------------|
| Rust | edition 2021 | Implementation language | All new crates must support edition 2021 |
| serde | 1.x (derive) | Serialization framework | Already in dependency tree; reuse everywhere |
| serde_json | 1.x | JSON interchange | Already in dependency tree; interchange format is JSON |

The elaborator is a single crate (`tenor-elaborator`) with a hand-rolled `main()` doing manual arg matching. No async runtime. No external I/O beyond file reads. This is a strength -- the project has zero unnecessary dependencies.

---

## Recommended Stack

### 1. Project Structure: Cargo Workspace

**Recommendation:** Convert from single crate to Cargo workspace.

```
tenor/
  Cargo.toml          # [workspace] members = ["crates/*"]
  crates/
    tenor-core/       # AST types, elaboration, type-checking (library)
    tenor-cli/        # `tenor` binary (clap, subcommands)
    tenor-eval/       # Evaluator engine (library)
    tenor-analyze/    # Static analyzer S1-S7 (library)
    tenor-codegen/    # Code generation (library)
    tenor-lsp/        # Language server (binary, optional)
```

**Why:** The elaborator's `elaborate()` function is already a clean public API (`pub fn elaborate(root_path: &Path) -> Result<Value, ElabError>`). Extracting it into a library crate lets the CLI, evaluator, code generator, and LSP all share the same elaboration pipeline without duplication. Cargo workspaces share `Cargo.lock` and a single `target/` directory, so build times stay fast.

**Confidence:** HIGH -- this is standard Rust project architecture for multi-binary compiler toolchains.

---

### 2. CLI Framework

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| clap | 4.5.60 | CLI argument parsing, subcommands | De facto standard for Rust CLIs. Derive API eliminates boilerplate. Built-in help generation, shell completions, colored output. |

**Specific configuration:**

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "tenor", version, about = "Tenor contract language toolchain")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Elaborate { file: PathBuf },
    Validate { bundle: PathBuf },
    Check { file: PathBuf },
    Eval { bundle: PathBuf, #[arg(long)] facts: PathBuf },
    Explain { bundle: PathBuf },
    Test { #[arg(default_value = "conformance")] suite_dir: PathBuf },
    Generate { bundle: PathBuf, #[arg(long)] target: Target },
}
```

**Cargo.toml:**
```toml
clap = { version = "4.5", features = ["derive", "color", "suggestions"] }
```

**Confidence:** HIGH -- verified on docs.rs (4.5.60, released 2026-02-19). clap v4 has been stable since late 2022 and the derive API is mature.

---

### 3. Error Reporting / Diagnostics

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| ariadne | 0.6.0 | Source-level diagnostic rendering | Purpose-built for compilers and language tools. Renders rustc-style error messages with source snippets, underlines, labels, and colors. Lower-level than miette -- gives you control over exactly how diagnostics look. |

**Why ariadne over miette:**
- ariadne is designed for compiler diagnostics specifically (source spans, multi-file errors, labeled arrows pointing at code)
- miette is a general-purpose error reporting library that wraps `std::error::Error` -- better for CLI tools where errors come from the application, not from user-authored source code
- Tenor's errors originate from `.tenor` source files and need to point at specific lines/columns. ariadne does this natively.
- ariadne has zero required dependencies beyond `std`

**Why ariadne over codespan-reporting:**
- codespan-reporting (0.13.1) is also viable and battle-tested (used by many Rust compilers)
- ariadne produces visually superior output with less boilerplate
- ariadne's `Report` builder API is more ergonomic than codespan's `Diagnostic` type

**Cargo.toml:**
```toml
ariadne = "0.6"
```

**Confidence:** HIGH -- verified on docs.rs (0.6.0, released 2025-10-28). Widely used in Rust language tooling.

---

### 4. Evaluator Engine

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| (hand-rolled) | -- | Contract evaluation against facts | Tenor's evaluation model is bespoke (verdict-producing, stratified, provenance-tracked). No off-the-shelf engine fits. |

**Why no external library:** Tenor is a non-Turing-complete, deterministic, closed-world evaluation system with provenance semantics (C1-C7 in the spec). The evaluator must:
- Accept interchange JSON + facts JSON
- Walk the stratified rule/entity/operation/flow structure
- Produce verdict sets with full provenance chains
- Guarantee termination and determinism

This is a tree-walking interpreter over a fixed, finite structure. No general-purpose rule engine, datalog, or expression evaluator matches these constraints without introducing unwanted complexity. The evaluation model is ~500 lines of straightforward Rust pattern matching over the interchange AST.

**Supporting dependency for the evaluator:**

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| rust_decimal | 1.x | Fixed-point decimal arithmetic | Tenor's NumericModel requires exact decimal/money arithmetic with specified precision and scale. `f64` is not acceptable. |

**Cargo.toml:**
```toml
rust_decimal = { version = "1", features = ["serde"] }
```

**Confidence:** HIGH -- the evaluator design follows directly from the spec's evaluation model (Section 13). Hand-rolling is the only correct approach.

---

### 5. Code Generation

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| tera | 1.20.1 | Template-based code generation | Runtime template loading for TypeScript/Rust output. Jinja2-compatible syntax. Supports template inheritance, macros, filters. |

**Why tera over askama:**
- Code generation templates change frequently during development. Tera loads templates at runtime, so you iterate without recompiling the Rust binary.
- Code generation produces files in _other_ languages (TypeScript, Rust). Askama's compile-time checking validates the _template syntax_, not the _output language_ -- so its main advantage (compile-time safety) provides less value here.
- Tera's runtime flexibility lets users potentially customize templates later (shipping default templates that advanced users override).
- Askama (0.15.4) is better for HTML/web where the output format is well-defined and the struct-to-template mapping is tight.

**Why not string concatenation / `write!` macros:**
- Code generation templates are 50-200 lines each (entity store, rule engine, operation handlers, flow orchestrator, port interfaces). String formatting becomes unreadable at this scale.
- Templates make it obvious what the output looks like. A `write!` chain obscures the output structure behind Rust formatting syntax.
- Adding a second target language (Rust after TypeScript) means writing a parallel set of templates, not a parallel set of format strings.

**Template organization:**
```
templates/
  typescript/
    entity_store.ts.tera
    rule_engine.ts.tera
    operation_handler.ts.tera
    flow_orchestrator.ts.tera
    ports.ts.tera
    types.ts.tera
  rust/
    entity_store.rs.tera
    rule_engine.rs.tera
    ...
```

**Cargo.toml:**
```toml
tera = "1.20"
```

**Note on Tera 2.0:** Tera 2.0.0-alpha.1 was released 2026-02-03. Do NOT use it -- it is an alpha with breaking API changes. Stick with 1.20.x which is stable and feature-complete for this use case.

**Confidence:** HIGH -- verified on docs.rs. Tera 1.20.1 released 2025-10-30.

---

### 6. Language Server (VS Code Extension Backend)

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| lsp-server | 0.7.9 | Synchronous LSP scaffolding | Same crate used by rust-analyzer. Synchronous design matches Tenor's synchronous elaborator. No async runtime needed. |
| lsp-types | 0.97.0 | LSP protocol type definitions | Standard type definitions for LSP messages. Used with both lsp-server and tower-lsp. |

**Why lsp-server over tower-lsp / tower-lsp-server / async-lsp:**
- Tenor's elaborator is synchronous. It reads files, parses, type-checks, and returns. No I/O, no network, no async.
- `lsp-server` is synchronous (crossbeam-channel based). It matches the elaborator's execution model. You call `elaborate()` in the request handler and return the result.
- `tower-lsp-server` (0.23.0) and `async-lsp` (0.2.2) both require tokio and an async runtime. This would force the entire elaborator into an async context for zero benefit -- Tenor has no I/O to overlap.
- `lsp-server` is battle-tested: it's the exact crate that rust-analyzer uses for its LSP implementation.
- The tradeoff: `lsp-server` gives you a raw message loop. You dispatch requests/notifications yourself. This is ~100 lines of boilerplate, but it means you understand and control every message.

**Cargo.toml (for tenor-lsp crate):**
```toml
lsp-server = "0.7"
lsp-types = "0.97"
```

**Confidence:** HIGH -- verified on docs.rs. lsp-server 0.7.9 released 2025-08-06. Used by rust-analyzer.

---

### 7. VS Code Extension (Client Side)

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| TextMate grammar | -- | Syntax highlighting | VS Code's native highlighting engine. JSON-based grammar definition. No build step, no WASM, no tree-sitter dependency. |
| vscode-languageclient | latest | LSP client for VS Code | Official Microsoft library for connecting VS Code to a language server over stdio. |
| @vscode/vsce | latest | Extension packaging | Official tool for packaging and publishing VS Code extensions. |

**Why TextMate grammar over tree-sitter for syntax highlighting:**
- Tenor's grammar is small and keyword-driven (11 constructs, no complex nesting). TextMate regex-based highlighting handles it trivially.
- VS Code does not natively support tree-sitter for syntax highlighting (as of 2026). Using tree-sitter would require a third-party extension or WASM embedding, adding complexity for zero benefit in a simple grammar.
- Tree-sitter is warranted for languages with deep nesting, complex disambiguation, or where incremental re-parsing matters (e.g., C++, Rust). Tenor's grammar is LL(1)-parseable with keyword-initiated blocks.
- TextMate grammars ship as a single `.tmLanguage.json` file with zero build dependencies.

**Extension structure:**
```
vscode-tenor/
  package.json              # Extension manifest
  syntaxes/
    tenor.tmLanguage.json   # TextMate grammar
  src/
    extension.ts            # LSP client activation
```

**Confidence:** HIGH -- TextMate grammars are the documented VS Code mechanism for syntax highlighting.

---

### 8. Documentation

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| mdBook | 0.5.2 | Book-style documentation | Rust ecosystem standard (used by The Rust Book, rustc-dev-guide, many Rust projects). Markdown input, static HTML output. |

**Why mdBook:**
- Tenor's documentation needs are book-shaped: language reference, authoring guide, executor implementation guide, code generation guide. These are chapters, not API docs.
- mdBook is the standard tool for this in the Rust ecosystem. Users of Rust tools expect and recognize the mdBook format.
- Outputs static HTML that can be served from GitHub Pages with zero infrastructure.
- Supports search, custom themes, preprocessors for code testing.

**Do NOT use:** `rustdoc` for user-facing documentation. `rustdoc` is for API reference of Rust libraries. Tenor needs prose documentation for contract authors who may never touch Rust.

**Confidence:** HIGH -- verified on docs.rs (0.5.2, released 2025-12-11). De facto standard for Rust project documentation.

---

### 9. Testing and Quality

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| (built-in) | -- | Conformance suite runner | Already exists (runner.rs + TAP output). Extend, don't replace. |
| insta | latest | Snapshot testing | For evaluator output, code generation output, diagnostic formatting. Avoids writing expected-output files by hand for new test categories. |

**Cargo.toml (dev dependency):**
```toml
[dev-dependencies]
insta = { version = "1", features = ["json"] }
```

**Confidence:** MEDIUM -- insta is well-established but its use for this project is a recommendation, not a requirement. The existing TAP-based conformance runner works and should remain the primary test mechanism.

---

## Full Dependency Summary

### tenor-core (library)

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
ariadne = "0.6"
```

### tenor-cli (binary)

```toml
[dependencies]
tenor-core = { path = "../tenor-core" }
tenor-eval = { path = "../tenor-eval" }
tenor-analyze = { path = "../tenor-analyze" }
tenor-codegen = { path = "../tenor-codegen" }
clap = { version = "4.5", features = ["derive", "color", "suggestions"] }
```

### tenor-eval (library)

```toml
[dependencies]
tenor-core = { path = "../tenor-core" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rust_decimal = { version = "1", features = ["serde"] }
```

### tenor-codegen (library)

```toml
[dependencies]
tenor-core = { path = "../tenor-core" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tera = "1.20"
```

### tenor-lsp (binary)

```toml
[dependencies]
tenor-core = { path = "../tenor-core" }
tenor-analyze = { path = "../tenor-analyze" }
lsp-server = "0.7"
lsp-types = "0.97"
serde_json = "1"
```

---

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| clap 4.5 | argh | If binary size is the primary concern (~10KB vs ~200KB). argh is simpler but lacks colored help, shell completions, and subcommand suggestions. Not worth the tradeoff for a developer tool. |
| ariadne 0.6 | codespan-reporting 0.13 | If you need LSP diagnostic integration (codespan's `Diagnostic` maps more directly to LSP `Diagnostic`). But since tenor-lsp can construct LSP diagnostics from the elaborator's error type directly, this isn't needed. |
| ariadne 0.6 | miette 7.6 | If you're building a general CLI tool where errors come from the application. For a language tool where errors point at source code, ariadne is purpose-built. |
| tera 1.20 | askama 0.15 | If generating HTML or Rust from a fixed set of templates where compile-time checking of the template itself is valuable. Not the case for code generation that targets multiple output languages. |
| tera 1.20 | hand-rolled `write!` | If you have fewer than 5 small templates. Tenor's code generator will have 10+ templates across 2 targets. Templates win. |
| lsp-server 0.7 | tower-lsp-server 0.23 | If the language server needs to do async I/O (e.g., fetching remote schemas, network-based type resolution). Tenor's elaborator is purely file-based and synchronous. |
| lsp-server 0.7 | async-lsp 0.2 | Same rationale as tower-lsp-server. Async adds complexity for no benefit here. |
| mdBook 0.5 | Docusaurus | If you want a JS/React-based doc site with blog, versioning, i18n. Overkill for a Rust-ecosystem language tool. |
| TextMate grammar | tree-sitter | If Tenor's grammar becomes complex enough to need incremental parsing or deep semantic highlighting. Currently unnecessary. |

---

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| tokio (in core/eval/codegen) | Tenor has no async I/O. Adding tokio inflates compile times by 15-30 seconds and binary size by ~2MB for zero benefit. | Synchronous Rust. Only the LSP server might eventually benefit from async, and lsp-server handles that with crossbeam channels. |
| diesel / sqlx / any database | Tenor is a pure transformation pipeline (files in, JSON/code out). There is no persistent state, no database, no server. | File I/O with `std::fs`. |
| wasm-bindgen / wasm-pack | Premature. WASM compilation can come later for browser-based playground. Not needed for the core toolchain. | Native binaries for CLI and LSP. |
| Tera 2.0-alpha | Alpha release (2026-02-03). Breaking API changes. Not production-ready. | Tera 1.20.1 (stable). |
| reqwest / hyper / any HTTP | Tenor is offline-first. No network I/O in the toolchain. | Nothing. If package registry comes later (post-1.0), revisit then. |
| log + env_logger | For a compiler/language tool, structured diagnostics (ariadne) replace unstructured logging. | ariadne for user-facing diagnostics. `eprintln!` for internal debug output during development. |

---

## Stack Patterns by Variant

**If building the CLI first (recommended):**
- Start with `clap` + `ariadne` + workspace restructuring
- These are the minimum additions to transform the current manual arg parsing into a real CLI
- Evaluator and codegen crates can start empty and fill in over time

**If building the VS Code extension first:**
- Start with TextMate grammar (zero Rust dependencies, just JSON)
- Then add `lsp-server` + `lsp-types` when ready for inline diagnostics
- The LSP server depends on `tenor-core`, so workspace restructuring is still needed first

**If adding code generation first:**
- Add `tera` + template files
- Code generation depends on the interchange JSON format being stable
- Per the ROADMAP, this comes after domain validation -- correct sequencing

---

## Version Compatibility

| Package | Compatible With | Notes |
|---------|-----------------|-------|
| clap 4.5.x | Rust 1.74+ | MSRV is well below edition 2021's minimum |
| ariadne 0.6.x | Rust 1.65+ | Minimal MSRV |
| tera 1.20.x | Rust 1.63+ | Minimal MSRV |
| lsp-server 0.7.x | lsp-types 0.97.x | These are versioned together by the rust-analyzer team |
| rust_decimal 1.x | serde 1.x | Enable the `serde` feature for JSON serialization |
| mdBook 0.5.x | Rust 1.88+ | Only needed as a CLI tool, not a library dependency |
| All crates | serde 1.x | All use serde 1.x; no version conflicts |

---

## Sources

- [clap 4.5.60](https://docs.rs/crate/clap/latest) -- docs.rs, verified 2026-02-21, released 2026-02-19 (HIGH confidence)
- [ariadne 0.6.0](https://docs.rs/crate/ariadne/latest) -- docs.rs, verified 2026-02-21, released 2025-10-28 (HIGH confidence)
- [tera 1.20.1](https://docs.rs/crate/tera/latest) -- docs.rs, verified 2026-02-21, released 2025-10-30 (HIGH confidence)
- [lsp-server 0.7.9](https://docs.rs/crate/lsp-server/latest) -- docs.rs, verified 2026-02-21, released 2025-08-06 (HIGH confidence)
- [lsp-types 0.97.0](https://docs.rs/crate/lsp-types/latest) -- docs.rs, verified 2026-02-21, released 2024-06-04 (HIGH confidence)
- [miette 7.6.0](https://docs.rs/crate/miette/latest) -- docs.rs, verified 2026-02-21, released 2025-04-27 (HIGH confidence)
- [codespan-reporting 0.13.1](https://docs.rs/crate/codespan-reporting/latest) -- docs.rs, verified 2026-02-21, released 2025-10-22 (HIGH confidence)
- [askama 0.15.4](https://docs.rs/crate/askama/latest) -- docs.rs, verified 2026-02-21, released 2026-01-28 (HIGH confidence)
- [tower-lsp-server 0.23.0](https://docs.rs/crate/tower-lsp-server/latest) -- docs.rs, verified 2026-02-21, released 2025-12-07 (HIGH confidence)
- [async-lsp 0.2.2](https://docs.rs/crate/async-lsp/latest) -- docs.rs, verified 2026-02-21, released 2025-03-07 (HIGH confidence)
- [mdBook 0.5.2](https://docs.rs/crate/mdbook/latest) -- docs.rs, verified 2026-02-21, released 2025-12-11 (HIGH confidence)
- [tokio 1.49.0](https://docs.rs/crate/tokio/latest) -- docs.rs, verified 2026-02-21, released 2026-01-03 (HIGH confidence)
- [VS Code Syntax Highlight Guide](https://code.visualstudio.com/api/language-extensions/syntax-highlight-guide) -- official VS Code docs (HIGH confidence)
- [VS Code Language Server Extension Guide](https://code.visualstudio.com/api/language-extensions/language-server-extension-guide) -- official VS Code docs (HIGH confidence)
- [rust-analyzer/lsp-server](https://github.com/rust-analyzer/lsp-server) -- GitHub, used by rust-analyzer (HIGH confidence)

---
*Stack research for: Tenor DSL toolchain (compiler, evaluator, code generator, editor extension, documentation)*
*Researched: 2026-02-21*
