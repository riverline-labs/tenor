//! Pass 0+1: Lex, parse, import resolution, cycle detection, bundle assembly.

use crate::ast::*;
use crate::error::ElabError;
use crate::lexer;
use crate::parser;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Parse the root `.tenor` file and all transitive imports, returning
/// the flat construct list and the bundle id (root file stem).
pub fn load_bundle(root: &Path) -> Result<(Vec<RawConstruct>, String), ElabError> {
    let root = root.canonicalize().map_err(|e| {
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

    let mut visited: HashSet<PathBuf> = HashSet::new();
    let mut stack: Vec<PathBuf> = Vec::new();
    let mut all_constructs: Vec<RawConstruct> = Vec::new();

    load_file(
        &root,
        &root_dir,
        &mut visited,
        &mut stack,
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

fn load_file(
    path: &Path,
    base_dir: &Path,
    visited: &mut HashSet<PathBuf>,
    stack: &mut Vec<PathBuf>,
    out: &mut Vec<RawConstruct>,
) -> Result<(), ElabError> {
    let canon = path.canonicalize().map_err(|e| {
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

    if stack.contains(&canon) {
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

    let src = std::fs::read_to_string(path).map_err(|e| {
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

    stack.push(canon.clone());

    let mut local: Vec<RawConstruct> = Vec::new();
    for c in constructs {
        match &c {
            RawConstruct::Import {
                path: import_path,
                prov,
            } => {
                let resolved = base_dir.join(import_path);
                let import_base = resolved.parent().unwrap_or(Path::new(".")).to_owned();
                if !resolved.exists() {
                    return Err(ElabError::new(
                        1,
                        None,
                        None,
                        Some("import"),
                        &prov.file,
                        prov.line,
                        format!("import resolution failed: file not found: {}", import_path),
                    ));
                }
                if let Ok(canon_import) = resolved.canonicalize() {
                    if stack.contains(&canon_import) {
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
                }
                load_file(&resolved, &import_base, visited, stack, out)?;
            }
            _ => local.push(c),
        }
    }
    out.extend(local);

    stack.pop();
    visited.insert(canon);
    Ok(())
}
