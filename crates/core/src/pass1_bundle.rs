//! Pass 0+1: Lex, parse, import resolution, cycle detection, bundle assembly.

use crate::ast::*;
use crate::error::ElabError;
use crate::lexer;
use crate::parser;
use crate::source::{FileSystemProvider, SourceProvider};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Parse the root `.tenor` file and all transitive imports, returning
/// the flat construct list and the bundle id (root file stem).
///
/// Uses the default [`FileSystemProvider`] for file I/O.
pub fn load_bundle(root: &Path) -> Result<(Vec<RawConstruct>, String), ElabError> {
    load_bundle_with_provider(root, &FileSystemProvider)
}

/// Parse the root `.tenor` file and all transitive imports using the given
/// [`SourceProvider`] for file I/O, returning the flat construct list and
/// the bundle id (root file stem).
pub fn load_bundle_with_provider(
    root: &Path,
    provider: &dyn SourceProvider,
) -> Result<(Vec<RawConstruct>, String), ElabError> {
    let root = provider.canonicalize(root).map_err(|e| {
        ElabError::new(
            1,
            None,
            None,
            None,
            &root.to_string_lossy(),
            0,
            format!("cannot open file: {}", e),
        )
    })?;
    let root_dir = root.parent().unwrap_or(Path::new(".")).to_owned();
    let bundle_id = root
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // Canonicalize the root directory once for sandbox boundary checks.
    // All imported files must resolve to paths within this directory.
    let sandbox_root = provider.canonicalize(&root_dir).map_err(|e| {
        ElabError::new(
            1,
            None,
            None,
            None,
            &root.to_string_lossy(),
            0,
            format!("cannot canonicalize root directory: {}", e),
        )
    })?;

    let mut visited: HashSet<PathBuf> = HashSet::new();
    let mut stack: Vec<PathBuf> = Vec::new();
    // Parallel HashSet for O(1) cycle detection lookups.
    // The Vec `stack` is kept for ordered error message reporting.
    let mut stack_set: HashSet<PathBuf> = HashSet::new();
    let mut all_constructs: Vec<RawConstruct> = Vec::new();

    load_file(
        &root,
        &root_dir,
        &sandbox_root,
        provider,
        &mut visited,
        &mut stack,
        &mut stack_set,
        &mut all_constructs,
    )?;

    check_cross_file_dups(&all_constructs)?;

    Ok((all_constructs, bundle_id))
}

/// Detect constructs with the same (kind, id) coming from different files.
fn check_cross_file_dups(constructs: &[RawConstruct]) -> Result<(), ElabError> {
    let mut seen: HashMap<(&str, &str), &Provenance> = HashMap::new();
    for c in constructs.iter().rev() {
        let (kind, id, prov): (&str, &str, &Provenance) = match c {
            RawConstruct::Fact { id, prov, .. } => ("Fact", id, prov),
            RawConstruct::Entity { id, prov, .. } => ("Entity", id, prov),
            RawConstruct::Rule { id, prov, .. } => ("Rule", id, prov),
            RawConstruct::Operation { id, prov, .. } => ("Operation", id, prov),
            RawConstruct::Flow { id, prov, .. } => ("Flow", id, prov),
            RawConstruct::TypeDecl { id, prov, .. } => ("TypeDecl", id, prov),
            RawConstruct::Persona { id, prov, .. } => ("Persona", id, prov),
            RawConstruct::System { id, prov, .. } => ("System", id, prov),
            RawConstruct::Import { .. } => continue,
        };
        if let Some(first) = seen.get(&(kind, id)) {
            if first.file != prov.file {
                return Err(ElabError::new(
                    1,
                    Some(kind),
                    Some(id),
                    Some("id"),
                    &prov.file,
                    prov.line,
                    format!(
                        "duplicate {} id '{}': first declared in {}",
                        kind, id, first.file
                    ),
                ));
            }
        } else {
            seen.insert((kind, id), prov);
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn load_file(
    path: &Path,
    base_dir: &Path,
    sandbox_root: &Path,
    provider: &dyn SourceProvider,
    visited: &mut HashSet<PathBuf>,
    stack: &mut Vec<PathBuf>,
    stack_set: &mut HashSet<PathBuf>,
    out: &mut Vec<RawConstruct>,
) -> Result<(), ElabError> {
    let canon = provider.canonicalize(path).map_err(|e| {
        ElabError::new(
            1,
            None,
            None,
            None,
            &path.to_string_lossy(),
            0,
            format!("cannot resolve import '{}': {}", path.display(), e),
        )
    })?;

    // O(1) cycle detection via parallel HashSet
    if stack_set.contains(&canon) {
        let cycle: Vec<String> = stack
            .iter()
            .map(|p| {
                p.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
            })
            .collect();
        let filename = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        return Err(ElabError::new(
            1,
            None,
            None,
            None,
            &path.to_string_lossy(),
            0,
            format!(
                "import cycle detected: {} \u{2192} {}",
                cycle.join(" \u{2192} "),
                filename
            ),
        ));
    }

    if visited.contains(&canon) {
        return Ok(());
    }

    let src = provider.read_source(path).map_err(|e| {
        ElabError::new(
            1,
            None,
            None,
            None,
            &path.to_string_lossy(),
            0,
            format!("cannot read file '{}': {}", path.display(), e),
        )
    })?;

    let filename = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let tokens = lexer::lex(&src, &filename)?;
    let constructs = parser::parse(&tokens, &filename)?;

    stack_set.insert(canon.clone());
    stack.push(canon.clone());

    let mut local: Vec<RawConstruct> = Vec::new();
    for c in constructs {
        match &c {
            RawConstruct::Import {
                path: import_path,
                prov,
            } => {
                let resolved = provider
                    .resolve_import(base_dir, import_path)
                    .map_err(|e| {
                        ElabError::new(
                            1,
                            None,
                            None,
                            Some("import"),
                            &prov.file,
                            prov.line,
                            format!(
                                "import resolution failed: cannot resolve path '{}': {}",
                                import_path, e
                            ),
                        )
                    })?;

                // Sandbox check: canonicalize the resolved path (fail closed)
                // and verify it stays within the contract root directory.
                let canon_import = provider.canonicalize(&resolved).map_err(|_| {
                    ElabError::new(
                        1,
                        None,
                        None,
                        Some("import"),
                        &prov.file,
                        prov.line,
                        format!(
                            "import resolution failed: cannot resolve path '{}'",
                            import_path
                        ),
                    )
                })?;

                if !canon_import.starts_with(sandbox_root) {
                    return Err(ElabError::new(
                        1,
                        None,
                        None,
                        Some("import"),
                        &prov.file,
                        prov.line,
                        format!(
                            "import '{}' escapes the contract root directory",
                            import_path
                        ),
                    ));
                }

                // O(1) cycle detection via parallel HashSet
                if stack_set.contains(&canon_import) {
                    let cycle: Vec<String> = stack
                        .iter()
                        .map(|p| {
                            p.file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string()
                        })
                        .collect();
                    let target = resolved
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    return Err(ElabError::new(
                        1,
                        None,
                        None,
                        Some("import"),
                        &prov.file,
                        prov.line,
                        format!(
                            "import cycle detected: {} \u{2192} {}",
                            cycle.join(" \u{2192} "),
                            target
                        ),
                    ));
                }

                let import_base = canon_import.parent().unwrap_or(Path::new(".")).to_owned();
                load_file(
                    &resolved,
                    &import_base,
                    sandbox_root,
                    provider,
                    visited,
                    stack,
                    stack_set,
                    out,
                )?;
            }
            _ => local.push(c),
        }
    }
    out.extend(local);

    stack.pop();
    stack_set.remove(&canon);
    visited.insert(canon);
    Ok(())
}
