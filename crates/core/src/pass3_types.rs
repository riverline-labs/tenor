//! Pass 3: Type environment construction -- resolve TypeDecl references,
//! detect cycles, build the name-to-concrete-type map.

use crate::ast::*;
use crate::error::ElabError;
use crate::pass2_index::Index;
use std::collections::{BTreeMap, HashMap, HashSet};

pub type TypeEnv = HashMap<String, RawType>;

pub fn build_type_env(constructs: &[RawConstruct], _index: &Index) -> Result<TypeEnv, ElabError> {
    let mut decls: BTreeMap<String, (BTreeMap<String, RawType>, Provenance)> = BTreeMap::new();
    for c in constructs {
        if let RawConstruct::TypeDecl { id, fields, prov } = c {
            decls.insert(id.clone(), (fields.clone(), prov.clone()));
        }
    }

    let mut visited: HashSet<String> = HashSet::new();
    let mut in_stack: Vec<String> = Vec::new();

    for name in decls.keys() {
        detect_typedecl_cycle(name, &decls, &mut visited, &mut in_stack)?;
    }

    let mut env: TypeEnv = HashMap::new();
    let names: Vec<String> = decls.keys().cloned().collect();
    for name in &names {
        let t = resolve_typedecl(name, &decls, &env)?;
        env.insert(name.clone(), t);
    }

    Ok(env)
}

fn detect_typedecl_cycle(
    name: &str,
    decls: &BTreeMap<String, (BTreeMap<String, RawType>, Provenance)>,
    visited: &mut HashSet<String>,
    in_stack: &mut Vec<String>,
) -> Result<(), ElabError> {
    if visited.contains(name) {
        return Ok(());
    }
    if in_stack.contains(&name.to_owned()) {
        // SAFETY: name was just detected in in_stack by the contains() check above
        let pos = in_stack.iter().position(|x| x == name).unwrap();
        let mut cycle: Vec<String> = in_stack[pos..].to_vec();
        cycle.push(name.to_owned());
        let cycle_str = cycle.join(" \u{2192} ");
        // SAFETY: in_stack is non-empty because it contains name (verified above)
        let back_edge_name = in_stack.last().unwrap();
        // SAFETY: back_edge_name came from in_stack, which is only populated from decls keys
        let (_, prov) = decls.get(back_edge_name.as_str()).unwrap();
        // SAFETY: same key existence guaranteed as the line above
        let (fields, _) = decls.get(back_edge_name.as_str()).unwrap();
        let field_name = fields
            .iter()
            .find_map(|(f, t)| {
                if references_type(t, name) {
                    Some(f.clone())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "type".to_owned());
        return Err(ElabError::new(
            3,
            Some("TypeDecl"),
            Some(back_edge_name),
            Some(&format!("type.fields.{}", field_name)),
            &prov.file,
            prov.line,
            format!("TypeDecl cycle detected: {}", cycle_str),
        ));
    }

    if !decls.contains_key(name) {
        return Ok(());
    }

    in_stack.push(name.to_owned());
    // SAFETY: name existence in decls verified by the contains_key() guard above (line 74)
    let (fields, _) = decls.get(name).unwrap().clone();
    for t in fields.values() {
        for ref_name in type_refs(t) {
            detect_typedecl_cycle(&ref_name, decls, visited, in_stack)?;
        }
    }
    in_stack.pop();
    visited.insert(name.to_owned());
    Ok(())
}

fn references_type(t: &RawType, target: &str) -> bool {
    match t {
        RawType::TypeRef(n) => n == target,
        RawType::Record { fields } => fields.values().any(|f| references_type(f, target)),
        RawType::List { element_type, .. } => references_type(element_type, target),
        _ => false,
    }
}

fn type_refs(t: &RawType) -> Vec<String> {
    match t {
        RawType::TypeRef(n) => vec![n.clone()],
        RawType::Record { fields } => fields.values().flat_map(type_refs).collect(),
        RawType::List { element_type, .. } => type_refs(element_type),
        _ => vec![],
    }
}

fn resolve_typedecl(
    name: &str,
    decls: &BTreeMap<String, (BTreeMap<String, RawType>, Provenance)>,
    env: &TypeEnv,
) -> Result<RawType, ElabError> {
    // SAFETY: resolve_typedecl is only called with names from decls.keys() or after
    // a decls.contains_key() check, so the key is guaranteed to exist
    let (fields, prov) = decls.get(name).unwrap();
    let mut resolved = BTreeMap::new();
    for (fname, ft) in fields {
        let rt = resolve_type_in_env(ft, decls, env, &prov.file, prov.line)?;
        resolved.insert(fname.clone(), rt);
    }
    Ok(RawType::Record { fields: resolved })
}

fn resolve_type_in_env(
    t: &RawType,
    decls: &BTreeMap<String, (BTreeMap<String, RawType>, Provenance)>,
    env: &TypeEnv,
    file: &str,
    line: u32,
) -> Result<RawType, ElabError> {
    match t {
        RawType::TypeRef(name) => {
            if let Some(resolved) = env.get(name) {
                return Ok(resolved.clone());
            }
            if decls.contains_key(name.as_str()) {
                resolve_typedecl(name, decls, env)
            } else {
                Err(ElabError::new(
                    4,
                    None,
                    None,
                    Some("type"),
                    file,
                    line,
                    format!("unknown type reference '{}'", name),
                ))
            }
        }
        RawType::Record { fields } => {
            let mut resolved = BTreeMap::new();
            for (k, v) in fields {
                resolved.insert(k.clone(), resolve_type_in_env(v, decls, env, file, line)?);
            }
            Ok(RawType::Record { fields: resolved })
        }
        RawType::List { element_type, max } => {
            let et = resolve_type_in_env(element_type, decls, env, file, line)?;
            Ok(RawType::List {
                element_type: Box::new(et),
                max: *max,
            })
        }
        other => Ok(other.clone()),
    }
}
