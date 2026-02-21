/// Six-pass elaborator: Tenor → TenorInterchange JSON bundle.
///
/// Pass 0 — Lex and parse
/// Pass 1 — Import resolution and bundle assembly
/// Pass 2 — Construct indexing (duplicate id check)
/// Pass 3 — Type environment construction (TypeDecl resolution, cycle detection)
/// Pass 4 — Expression type-checking and AST materialization
/// Pass 5 — Construct validation
/// Pass 6 — Interchange serialization
use crate::error::ElabError;
use crate::lexer;
use crate::parser::{self, Provenance, RawBranch, RawCompStep, RawConstruct, RawExpr, RawFailureHandler, RawLiteral, RawStep, RawStepTarget, RawTerm, RawType};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

// ──────────────────────────────────────────────────────────────────────────────
// Public entry point
// ──────────────────────────────────────────────────────────────────────────────

/// Elaborate the given root `.tenor` file and return the interchange bundle,
/// or the first elaboration error encountered.
pub fn elaborate(root_path: &Path) -> Result<Value, ElabError> {
    // Passes 0+1: parse all files in the import graph
    let (constructs, bundle_id) = load_bundle(root_path)?;

    // Pass 2: construct indexing
    let index = build_index(&constructs)?;

    // Pass 3: type environment
    let type_env = build_type_env(&constructs, &index)?;

    // Pass 4: resolve types in all constructs (TypeRef → concrete BaseType)
    let constructs = resolve_types(constructs, &type_env)?;

    // Pass 4 (continued): expression type-checking
    type_check_rules(&constructs)?;

    // Pass 5: validation
    validate(&constructs, &index)?;
    validate_operation_transitions(&constructs, &index)?;

    // Pass 6: serialization
    let bundle = serialize(&constructs, &bundle_id);
    Ok(bundle)
}

// ──────────────────────────────────────────────────────────────────────────────
// Pass 0 + 1: load_bundle — parse root file, follow imports, detect cycles
// ──────────────────────────────────────────────────────────────────────────────

fn load_bundle(root: &Path) -> Result<(Vec<RawConstruct>, String), ElabError> {
    let root = root.canonicalize().map_err(|e| ElabError::new(
        1, None, None, None,
        &root.to_string_lossy(), 0,
        format!("cannot open file: {}", e),
    ))?;
    let root_dir = root.parent().unwrap_or(Path::new(".")).to_owned();
    let bundle_id = root.file_stem().unwrap_or_default().to_string_lossy().to_string();

    let mut visited: HashSet<PathBuf> = HashSet::new();
    let mut stack: Vec<PathBuf> = Vec::new(); // for cycle detection
    let mut all_constructs: Vec<RawConstruct> = Vec::new();

    load_file(&root, &root_dir, &mut visited, &mut stack, &mut all_constructs)?;

    // Cross-file duplicate check (Pass 1).
    // all_constructs is in imports-first order (depth-first); scanning in reverse
    // means root-file constructs are encountered first, so they are "first declared".
    check_cross_file_dups(&all_constructs)?;

    Ok((all_constructs, bundle_id))
}

/// Detect constructs with the same (kind, id) coming from different files.
/// Scans in reverse so that root-file constructs (appended last) are treated as
/// "first declared", and imports with clashing ids get the Pass 1 error.
fn check_cross_file_dups(constructs: &[RawConstruct]) -> Result<(), ElabError> {
    let mut seen: HashMap<(&str, &str), &Provenance> = HashMap::new();
    for c in constructs.iter().rev() {
        let (kind, id, prov): (&str, &str, &Provenance) = match c {
            RawConstruct::Fact     { id, prov, .. } => ("Fact",      id, prov),
            RawConstruct::Entity   { id, prov, .. } => ("Entity",    id, prov),
            RawConstruct::Rule     { id, prov, .. } => ("Rule",      id, prov),
            RawConstruct::Operation{ id, prov, .. } => ("Operation", id, prov),
            RawConstruct::Flow     { id, prov, .. } => ("Flow",      id, prov),
            RawConstruct::TypeDecl { id, prov, .. } => ("TypeDecl",  id, prov),
            RawConstruct::Import   { .. }           => continue,
        };
        if let Some(first) = seen.get(&(kind, id)) {
            if first.file != prov.file {
                return Err(ElabError::new(
                    1, Some(kind), Some(id), Some("id"),
                    &prov.file, prov.line,
                    format!("duplicate {} id '{}': first declared in {}", kind, id, first.file),
                ));
            }
            // Same-file duplicate: leave it to Pass 2.
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
    let canon = path.canonicalize().map_err(|e| ElabError::new(
        1, None, None, None,
        &path.to_string_lossy(), 0,
        format!("cannot resolve import '{}': {}", path.display(), e),
    ))?;

    if stack.contains(&canon) {
        let cycle: Vec<String> = stack.iter()
            .map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string())
            .collect();
        let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        return Err(ElabError::new(
            1, None, None, None,
            &path.to_string_lossy(), 0,
            format!("import cycle detected: {} → {}", cycle.join(" → "), filename),
        ));
    }

    if visited.contains(&canon) {
        return Ok(()); // already loaded
    }

    let src = std::fs::read_to_string(path).map_err(|e| ElabError::new(
        1, None, None, None,
        &path.to_string_lossy(), 0,
        format!("cannot read file '{}': {}", path.display(), e),
    ))?;

    let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
    let tokens = lexer::lex(&src, &filename)?;
    let constructs = parser::parse(&tokens, &filename)?;

    stack.push(canon.clone());

    // Process imports first (depth-first, preserving declaration order for non-imports)
    let mut local: Vec<RawConstruct> = Vec::new();
    for c in constructs {
        match &c {
            RawConstruct::Import { path: import_path, prov } => {
                let resolved = base_dir.join(import_path);
                let import_base = resolved.parent().unwrap_or(Path::new(".")).to_owned();
                if !resolved.exists() {
                    return Err(ElabError::new(
                        1, None, None, Some("import"),
                        &prov.file, prov.line,
                        format!("import resolution failed: file not found: {}", import_path),
                    ));
                }
                // Check for import cycle before recursing
                if let Ok(canon_import) = resolved.canonicalize() {
                    if stack.contains(&canon_import) {
                        let cycle: Vec<String> = stack.iter()
                            .map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string())
                            .collect();
                        let target = resolved.file_name().unwrap_or_default().to_string_lossy().to_string();
                        return Err(ElabError::new(
                            1, None, None, Some("import"),
                            &prov.file, prov.line,
                            format!("import cycle detected: {} → {}", cycle.join(" → "), target),
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

// ──────────────────────────────────────────────────────────────────────────────
// Pass 2: construct indexing
// ──────────────────────────────────────────────────────────────────────────────

struct Index {
    facts: HashMap<String, Provenance>,
    entities: HashMap<String, Provenance>,
    rules: HashMap<String, Provenance>,
    operations: HashMap<String, Provenance>,
    flows: HashMap<String, Provenance>,
    type_decls: HashMap<String, Provenance>,
    /// Map from rule_id → verdict_type name produced by that rule
    rule_verdicts: HashMap<String, String>,
    /// Map from verdict_type → (rule_id, stratum) of the producing rule
    verdict_strata: HashMap<String, (String, i64)>,
}

fn build_index(constructs: &[RawConstruct]) -> Result<Index, ElabError> {
    let mut idx = Index {
        facts: HashMap::new(),
        entities: HashMap::new(),
        rules: HashMap::new(),
        operations: HashMap::new(),
        flows: HashMap::new(),
        type_decls: HashMap::new(),
        rule_verdicts: HashMap::new(),
        verdict_strata: HashMap::new(),
    };

    for c in constructs {
        match c {
            RawConstruct::Fact { id, prov, .. } => {
                if let Some(first) = idx.facts.get(id) {
                    return Err(ElabError::new(
                        2, Some("Fact"), Some(id), Some("id"),
                        &prov.file, prov.line,
                        format!("duplicate Fact id '{}': first declared at line {}", id, first.line),
                    ));
                }
                idx.facts.insert(id.clone(), prov.clone());
            }
            RawConstruct::Entity { id, prov, .. } => {
                if let Some(first) = idx.entities.get(id) {
                    return Err(ElabError::new(
                        2, Some("Entity"), Some(id), Some("id"),
                        &prov.file, prov.line,
                        format!("duplicate Entity id '{}': first declared at line {}", id, first.line),
                    ));
                }
                idx.entities.insert(id.clone(), prov.clone());
            }
            RawConstruct::Rule { id, verdict_type, stratum, prov, .. } => {
                if let Some(first) = idx.rules.get(id) {
                    return Err(ElabError::new(
                        2, Some("Rule"), Some(id), Some("id"),
                        &prov.file, prov.line,
                        format!("duplicate Rule id '{}': first declared at line {}", id, first.line),
                    ));
                }
                idx.rules.insert(id.clone(), prov.clone());
                idx.rule_verdicts.insert(id.clone(), verdict_type.clone());
                idx.verdict_strata.insert(verdict_type.clone(), (id.clone(), *stratum));
            }
            RawConstruct::Operation { id, prov, .. } => {
                if let Some(first) = idx.operations.get(id) {
                    return Err(ElabError::new(
                        2, Some("Operation"), Some(id), Some("id"),
                        &prov.file, prov.line,
                        format!("duplicate Operation id '{}': first declared at line {}", id, first.line),
                    ));
                }
                idx.operations.insert(id.clone(), prov.clone());
            }
            RawConstruct::Flow { id, prov, .. } => {
                if let Some(first) = idx.flows.get(id) {
                    return Err(ElabError::new(
                        2, Some("Flow"), Some(id), Some("id"),
                        &prov.file, prov.line,
                        format!("duplicate Flow id '{}': first declared at line {}", id, first.line),
                    ));
                }
                idx.flows.insert(id.clone(), prov.clone());
            }
            RawConstruct::TypeDecl { id, prov, .. } => {
                if let Some(first) = idx.type_decls.get(id) {
                    return Err(ElabError::new(
                        2, Some("TypeDecl"), Some(id), Some("id"),
                        &prov.file, prov.line,
                        format!("duplicate TypeDecl id '{}': first declared at line {}", id, first.line),
                    ));
                }
                idx.type_decls.insert(id.clone(), prov.clone());
            }
            RawConstruct::Import { .. } => {} // already consumed in Pass 1
        }
    }

    Ok(idx)
}

// ──────────────────────────────────────────────────────────────────────────────
// Pass 3: type environment
// ──────────────────────────────────────────────────────────────────────────────

type TypeEnv = HashMap<String, RawType>;

fn build_type_env(constructs: &[RawConstruct], _index: &Index) -> Result<TypeEnv, ElabError> {
    // Collect all TypeDecl definitions
    let mut decls: BTreeMap<String, (BTreeMap<String, RawType>, Provenance)> = BTreeMap::new();
    for c in constructs {
        if let RawConstruct::TypeDecl { id, fields, prov } = c {
            decls.insert(id.clone(), (fields.clone(), prov.clone()));
        }
    }

    // Detect cycles via DFS over the TypeDecl reference graph
    let mut visited: HashSet<String> = HashSet::new();
    let mut in_stack: Vec<String> = Vec::new();

    for name in decls.keys() {
        detect_typedecl_cycle(name, &decls, &mut visited, &mut in_stack)?;
    }

    // Build type environment: name → fully concrete RawType
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
        // Found a cycle
        let pos = in_stack.iter().position(|x| x == name).unwrap();
        let mut cycle: Vec<String> = in_stack[pos..].to_vec();
        cycle.push(name.to_owned());
        let cycle_str = cycle.join(" → ");
        // Report error on the TypeDecl that contains the back edge (last in stack before the repeated name)
        let back_edge_name = in_stack.last().unwrap();
        let (_, prov) = decls.get(back_edge_name.as_str()).unwrap();
        // Find the field that causes the cycle
        let (fields, _) = decls.get(back_edge_name.as_str()).unwrap();
        let field_name = fields.iter()
            .find_map(|(f, t)| if references_type(t, name) { Some(f.clone()) } else { None })
            .unwrap_or_else(|| "type".to_owned());
        return Err(ElabError::new(
            3, Some("TypeDecl"), Some(back_edge_name),
            Some(&format!("type.fields.{}", field_name)),
            &prov.file, prov.line,
            format!("TypeDecl cycle detected: {}", cycle_str),
        ));
    }

    if !decls.contains_key(name) {
        return Ok(()); // not a TypeDecl reference, skip
    }

    in_stack.push(name.to_owned());
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
            // Need to resolve it now
            if decls.contains_key(name.as_str()) {
                resolve_typedecl(name, decls, env)
            } else {
                Err(ElabError::new(4, None, None, Some("type"),
                    file, line,
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
            Ok(RawType::List { element_type: Box::new(et), max: *max })
        }
        other => Ok(other.clone()),
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Pass 4: resolve TypeRef nodes throughout all constructs
// ──────────────────────────────────────────────────────────────────────────────

fn resolve_types(
    constructs: Vec<RawConstruct>,
    type_env: &TypeEnv,
) -> Result<Vec<RawConstruct>, ElabError> {
    let mut out = Vec::new();
    for c in constructs {
        out.push(resolve_construct(c, type_env)?);
    }
    Ok(out)
}

fn resolve_construct(c: RawConstruct, env: &TypeEnv) -> Result<RawConstruct, ElabError> {
    match c {
        RawConstruct::Fact { id, type_, source, default, prov } => {
            let t = resolve_raw_type(&type_, env, &prov.file, prov.line)?;
            // Validate fact_refs in implicit contexts (no extra check needed here)
            Ok(RawConstruct::Fact { id, type_: t, source, default, prov })
        }
        RawConstruct::Rule { id, stratum, stratum_line, when, verdict_type, payload_type, payload_value, produce_line, prov } => {
            let pt = resolve_raw_type(&payload_type, env, &prov.file, prov.line)?;
            Ok(RawConstruct::Rule { id, stratum, stratum_line, when, verdict_type, payload_type: pt, payload_value, produce_line, prov })
        }
        other => Ok(other), // TypeDecl, Entity, Operation, Flow don't have inline types to resolve
    }
}

fn resolve_raw_type(t: &RawType, env: &TypeEnv, file: &str, line: u32) -> Result<RawType, ElabError> {
    match t {
        RawType::TypeRef(name) => {
            env.get(name.as_str()).cloned().ok_or_else(|| ElabError::new(
                4, None, None, Some("type"),
                file, line,
                format!("unknown type reference '{}'", name),
            ))
        }
        RawType::Record { fields } => {
            let mut resolved = BTreeMap::new();
            for (k, v) in fields {
                resolved.insert(k.clone(), resolve_raw_type(v, env, file, line)?);
            }
            Ok(RawType::Record { fields: resolved })
        }
        RawType::List { element_type, max } => {
            let et = resolve_raw_type(element_type, env, file, line)?;
            Ok(RawType::List { element_type: Box::new(et), max: *max })
        }
        other => Ok(other.clone()),
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Pass 4: expression type-checking (unresolved refs + type errors)
// ──────────────────────────────────────────────────────────────────────────────

fn type_check_rules(constructs: &[RawConstruct]) -> Result<(), ElabError> {
    let mut fact_types: HashMap<&str, &RawType> = HashMap::new();
    for c in constructs {
        if let RawConstruct::Fact { id, type_, .. } = c {
            fact_types.insert(id.as_str(), type_);
        }
    }
    for c in constructs {
        if let RawConstruct::Rule { id, when, payload_type, payload_value, produce_line, prov, .. } = c {
            type_check_expr(id, when, &fact_types, &HashSet::new(), prov)?;
            type_check_produce(id, payload_type, payload_value, *produce_line, &fact_types, prov)?;
        }
    }
    Ok(())
}

/// Returns true if `term` is an unbound fact reference (a "variable" in predicate context).
fn is_var_fact_ref(term: &RawTerm, fact_types: &HashMap<&str, &RawType>, bound_vars: &HashSet<&str>) -> bool {
    matches!(term, RawTerm::FactRef(n) if !bound_vars.contains(n.as_str()) && fact_types.contains_key(n.as_str()))
}

/// Compute the Int range of a term used in multiplication, if determinable.
/// Returns Some((min, max)) for FactRef with Int type or Int literal; None otherwise.
fn mul_range_from_term(term: &RawTerm, fact_types: &HashMap<&str, &RawType>) -> Option<(i64, i64)> {
    match term {
        RawTerm::FactRef(n) => match fact_types.get(n.as_str()) {
            Some(RawType::Int { min, max }) => Some((*min, *max)),
            _ => None,
        },
        RawTerm::Literal(RawLiteral::Int(n)) => Some((*n, *n)),
        _ => None,
    }
}

/// Pass 4: check produce clause for multiplication type errors.
fn type_check_produce(
    rule_id: &str,
    payload_type: &RawType,
    payload_value: &RawTerm,
    produce_line: u32,
    fact_types: &HashMap<&str, &RawType>,
    prov: &Provenance,
) -> Result<(), ElabError> {
    if let RawTerm::Mul { left, right } = payload_value {
        let left_range = mul_range_from_term(left, fact_types);
        let right_range = mul_range_from_term(right, fact_types);
        if let (Some((l_min, l_max)), Some((r_min, r_max))) = (left_range, right_range) {
            let products = [l_min * r_min, l_min * r_max, l_max * r_min, l_max * r_max];
            let prod_min = *products.iter().min().unwrap();
            let prod_max = *products.iter().max().unwrap();
            if let RawType::Int { min: pt_min, max: pt_max } = payload_type {
                if prod_min < *pt_min || prod_max > *pt_max {
                    return Err(ElabError::new(
                        4, Some("Rule"), Some(rule_id), Some("body.produce.payload"),
                        &prov.file, produce_line,
                        format!(
                            "type error: product range {} is not contained in declared verdict payload type {}",
                            type_name(&RawType::Int { min: prod_min, max: prod_max }),
                            type_name(payload_type),
                        ),
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Return the resolved type of a term that is a direct Fact reference,
/// `None` for literals, field-refs, or unrecognised bound variables.
fn type_of_fact_term<'a>(
    term: &RawTerm,
    fact_types: &'a HashMap<&str, &RawType>,
    bound_vars: &HashSet<&str>,
) -> Option<&'a RawType> {
    match term {
        RawTerm::FactRef(name) if !bound_vars.contains(name.as_str()) => {
            fact_types.get(name.as_str()).copied()
        }
        _ => None,
    }
}

fn type_name(t: &RawType) -> String {
    match t {
        RawType::Bool               => "Bool".to_owned(),
        RawType::Int { min, max }   => format!("Int(min: {}, max: {})", min, max),
        RawType::Decimal { .. }     => "Decimal".to_owned(),
        RawType::Text { .. }        => "Text".to_owned(),
        RawType::Enum { .. }        => "Enum".to_owned(),
        RawType::Money { currency } => format!("Money(currency: {})", currency),
        RawType::Date               => "Date".to_owned(),
        RawType::DateTime           => "DateTime".to_owned(),
        RawType::Duration { .. }    => "Duration".to_owned(),
        RawType::List { .. }        => "List".to_owned(),
        RawType::Record { .. }      => "Record".to_owned(),
        RawType::TypeRef(n)         => n.clone(),
    }
}

fn type_check_expr(
    rule_id: &str,
    expr: &RawExpr,
    fact_types: &HashMap<&str, &RawType>,
    bound_vars: &HashSet<&str>,
    prov: &Provenance,
) -> Result<(), ElabError> {
    match expr {
        RawExpr::Compare { op, left, right, line } => {
            // Check for var×var multiplication in PredicateExpression (not permitted)
            for term in &[left, right] {
                if let RawTerm::Mul { left: ml, right: mr } = term {
                    if is_var_fact_ref(ml, fact_types, bound_vars) && is_var_fact_ref(mr, fact_types, bound_vars) {
                        return Err(ElabError::new(
                            4, Some("Rule"), Some(rule_id), Some("body.when"),
                            &prov.file, *line,
                            "type error: variable × variable multiplication is not permitted in PredicateExpression; only variable × literal_numeric is allowed".to_string(),
                        ));
                    }
                }
            }
            // Unresolved-ref check for both sides
            for term in &[left, right] {
                if let RawTerm::FactRef(name) = term {
                    if !bound_vars.contains(name.as_str()) && !fact_types.contains_key(name.as_str()) {
                        return Err(ElabError::new(
                            4, Some("Rule"), Some(rule_id), Some("body.when"),
                            &prov.file, *line,
                            format!("unresolved fact reference: '{}' is not declared in this contract", name),
                        ));
                    }
                }
            }
            // Type checks on left operand
            if let Some(lt) = type_of_fact_term(left, fact_types, bound_vars) {
                match lt {
                    RawType::Bool if op != "=" && op != "!=" => {
                        return Err(ElabError::new(
                            4, Some("Rule"), Some(rule_id), Some("body.when"),
                            &prov.file, *line,
                            format!("type error: operator '{}' not defined for Bool; Bool supports only = and ≠", op),
                        ));
                    }
                    RawType::Money { currency: lc } => {
                        if let Some(RawType::Money { currency: rc }) = type_of_fact_term(right, fact_types, bound_vars) {
                            if lc != rc {
                                return Err(ElabError::new(
                                    4, Some("Rule"), Some(rule_id), Some("body.when"),
                                    &prov.file, *line,
                                    format!("type error: cannot compare Money(currency: {}) with Money(currency: {}); Money comparisons require identical currency codes", lc, rc),
                                ));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        RawExpr::Forall { var, domain, body, line } => {
            if !fact_types.contains_key(domain.as_str()) {
                return Err(ElabError::new(
                    4, Some("Rule"), Some(rule_id), Some("body.when"),
                    &prov.file, *line,
                    format!("unresolved fact reference: '{}' is not declared in this contract", domain),
                ));
            }
            let domain_type = fact_types[domain.as_str()];
            if !matches!(domain_type, RawType::List { .. }) {
                return Err(ElabError::new(
                    4, Some("Rule"), Some(rule_id), Some("body.when"),
                    &prov.file, *line,
                    format!("type error: quantifier domain '{}' has type {}; domain must be List-typed",
                        domain, type_name(domain_type)),
                ));
            }
            let mut inner_bound = bound_vars.clone();
            inner_bound.insert(var.as_str());
            type_check_expr(rule_id, body, fact_types, &inner_bound, prov)?;
        }
        RawExpr::And(a, b) | RawExpr::Or(a, b) => {
            type_check_expr(rule_id, a, fact_types, bound_vars, prov)?;
            type_check_expr(rule_id, b, fact_types, bound_vars, prov)?;
        }
        RawExpr::Not(e) => {
            type_check_expr(rule_id, e, fact_types, bound_vars, prov)?;
        }
        RawExpr::VerdictPresent { .. } => {}
    }
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Pass 5: construct validation
// ──────────────────────────────────────────────────────────────────────────────

fn validate(constructs: &[RawConstruct], index: &Index) -> Result<(), ElabError> {
    // Collect all produced verdict types (rule_id → verdict_type)
    let produced_verdicts: HashSet<String> = index.rule_verdicts.values().cloned().collect();

    for c in constructs {
        match c {
            RawConstruct::Entity { id, states, initial, initial_line, transitions, parent, parent_line, prov } => {
                validate_entity(id, states, initial, *initial_line, transitions, parent.as_deref(), *parent_line, prov, index)?;
            }
            RawConstruct::Rule { id, stratum, stratum_line, when, prov, .. } => {
                validate_rule(id, *stratum, *stratum_line, when, prov, index, &produced_verdicts)?;
            }
            RawConstruct::Operation { id, allowed_personas, allowed_personas_line, effects, prov, .. } => {
                validate_operation(id, allowed_personas, *allowed_personas_line, effects, prov, index)?;
            }
            RawConstruct::Flow { id, entry, entry_line, steps, prov, .. } => {
                validate_flow(id, entry, *entry_line, steps, prov, index)?;
            }
            _ => {}
        }
    }

    // Entity hierarchy cycle check
    validate_entity_dag(constructs, index)?;

    // Flow reference graph cycle check (SubFlowStep cross-flow cycles)
    validate_flow_reference_graph(constructs)?;

    // Parallel branch entity conflict check
    validate_parallel_conflicts(constructs)?;

    Ok(())
}

fn validate_entity(
    id: &str,
    states: &[String],
    initial: &str,
    initial_line: u32,
    transitions: &[(String, String, u32)],
    _parent: Option<&str>,
    _parent_line: Option<u32>,
    prov: &Provenance,
    _index: &Index,
) -> Result<(), ElabError> {
    let state_set: HashSet<&str> = states.iter().map(String::as_str).collect();

    let states_list: Vec<&str> = states.iter().map(String::as_str).collect();

    if !state_set.contains(initial) {
        return Err(ElabError::new(
            5, Some("Entity"), Some(id), Some("initial"),
            &prov.file, initial_line,
            format!("initial state '{}' is not declared in states: [{}]", initial, states_list.join(", ")),
        ));
    }

    for (from, to, t_line) in transitions {
        if !state_set.contains(from.as_str()) {
            return Err(ElabError::new(
                5, Some("Entity"), Some(id), Some("transitions"),
                &prov.file, *t_line,
                format!("transition endpoint '{}' is not declared in states: [{}]", from, states_list.join(", ")),
            ));
        }
        if !state_set.contains(to.as_str()) {
            return Err(ElabError::new(
                5, Some("Entity"), Some(id), Some("transitions"),
                &prov.file, *t_line,
                format!("transition endpoint '{}' is not declared in states: [{}]", to, states_list.join(", ")),
            ));
        }
    }

    Ok(())
}

fn validate_entity_dag(constructs: &[RawConstruct], _index: &Index) -> Result<(), ElabError> {
    // (entity_id → (parent_id, parent_field_line, entity_prov))
    let mut parents: HashMap<&str, (&str, u32, &Provenance)> = HashMap::new();
    for c in constructs {
        if let RawConstruct::Entity { id, parent: Some(p), parent_line, prov, .. } = c {
            parents.insert(id.as_str(), (p.as_str(), parent_line.unwrap_or(prov.line), prov));
        }
    }

    // Detect cycles in parent chain — iterate in sorted order for determinism
    let mut sorted_ids: Vec<&str> = parents.keys().copied().collect();
    sorted_ids.sort_unstable();

    for start in sorted_ids {
        let mut visited: HashSet<&str> = HashSet::new();
        let mut cur = start;
        visited.insert(cur);
        while let Some((p, p_line, prov)) = parents.get(cur) {
            if visited.contains(p) {
                // cur is the entity whose parent closes the cycle
                // Build the cycle path starting from cur
                let mut path = vec![cur.to_string()];
                let mut node = cur;
                loop {
                    if let Some((next, _, _)) = parents.get(node) {
                        path.push(next.to_string());
                        if *next == cur { break; }
                        node = next;
                    } else {
                        break;
                    }
                }
                return Err(ElabError::new(
                    5, Some("Entity"), Some(cur),
                    Some("parent"),
                    prov.file.as_str(), *p_line,
                    format!("entity hierarchy cycle detected: {}", path.join(" → ")),
                ));
            }
            visited.insert(p);
            let _ = p_line;
            cur = p;
        }
    }
    Ok(())
}

fn validate_rule(
    id: &str,
    stratum: i64,
    stratum_line: u32,
    when: &RawExpr,
    prov: &Provenance,
    index: &Index,
    produced_verdicts: &HashSet<String>,
) -> Result<(), ElabError> {
    if stratum < 0 {
        return Err(ElabError::new(
            5, Some("Rule"), Some(id), Some("stratum"),
            &prov.file, stratum_line,
            format!("stratum must be a non-negative integer; got {}", stratum),
        ));
    }

    // Check verdict_present references: must be produced by some rule with stratum < this stratum
    validate_verdict_refs_in_expr(when, id, stratum, prov, index, produced_verdicts)?;

    Ok(())
}

fn validate_verdict_refs_in_expr(
    expr: &RawExpr,
    rule_id: &str,
    rule_stratum: i64,
    prov: &Provenance,
    index: &Index,
    produced_verdicts: &HashSet<String>,
) -> Result<(), ElabError> {
    match expr {
        RawExpr::VerdictPresent { id: vid, line } => {
            if !produced_verdicts.contains(vid.as_str()) {
                return Err(ElabError::new(
                    5, Some("Rule"), Some(rule_id),
                    Some("body.when"),
                    &prov.file, *line,
                    format!("unresolved VerdictType reference: '{}' is not produced by any rule in this contract", vid),
                ));
            }
            // Check stratum ordering: the rule that produces this verdict must be at stratum < rule_stratum
            if let Some((producing_rule_id, producing_stratum)) = find_producing_rule(vid, index) {
                if producing_stratum >= rule_stratum {
                    return Err(ElabError::new(
                        5, Some("Rule"), Some(rule_id),
                        Some("body.when"),
                        &prov.file, *line,
                        format!(
                            "stratum violation: rule '{}' at stratum {} references verdict '{}' produced by rule '{}' at stratum {}; verdict_refs must reference strata strictly less than the referencing rule's stratum",
                            rule_id, rule_stratum, vid, producing_rule_id, producing_stratum
                        ),
                    ));
                }
            }
        }
        RawExpr::And(a, b) | RawExpr::Or(a, b) => {
            validate_verdict_refs_in_expr(a, rule_id, rule_stratum, prov, index, produced_verdicts)?;
            validate_verdict_refs_in_expr(b, rule_id, rule_stratum, prov, index, produced_verdicts)?;
        }
        RawExpr::Not(e) => {
            validate_verdict_refs_in_expr(e, rule_id, rule_stratum, prov, index, produced_verdicts)?;
        }
        _ => {}
    }
    Ok(())
}

fn find_producing_rule(verdict_type: &str, index: &Index) -> Option<(String, i64)> {
    index.verdict_strata.get(verdict_type).cloned()
}

fn validate_operation(
    id: &str,
    allowed_personas: &[String],
    allowed_personas_line: u32,
    effects: &[(String, String, String, u32)],
    prov: &Provenance,
    index: &Index,
) -> Result<(), ElabError> {
    if allowed_personas.is_empty() {
        return Err(ElabError::new(
            5, Some("Operation"), Some(id),
            Some("allowed_personas"),
            &prov.file, allowed_personas_line,
            "allowed_personas must be non-empty; an Operation with no allowed personas can never be invoked".to_string(),
        ));
    }

    for (entity_id, _from, _to, e_line) in effects {
        if !index.entities.contains_key(entity_id.as_str()) {
            return Err(ElabError::new(
                5, Some("Operation"), Some(id),
                Some("effects"),
                &prov.file, *e_line,
                format!("effect references undeclared entity '{}'", entity_id),
            ));
        }
    }

    Ok(())
}

fn validate_operation_transitions(constructs: &[RawConstruct], _index: &Index) -> Result<(), ElabError> {
    // Build entity transition sets (with sorted display list for error messages)
    let mut entity_transitions: HashMap<&str, Vec<(&str, &str)>> = HashMap::new();
    for c in constructs {
        if let RawConstruct::Entity { id, transitions, .. } = c {
            let list = entity_transitions.entry(id.as_str()).or_default();
            for (f, t, _) in transitions {
                list.push((f.as_str(), t.as_str()));
            }
        }
    }

    for c in constructs {
        if let RawConstruct::Operation { id, effects, prov, .. } = c {
            for (entity_id, from, to, e_line) in effects {
                if let Some(transitions) = entity_transitions.get(entity_id.as_str()) {
                    if !transitions.iter().any(|(f, t)| f == from && t == to) {
                        let declared: Vec<String> = transitions.iter()
                            .map(|(f, t)| format!("({}, {})", f, t))
                            .collect();
                        return Err(ElabError::new(
                            5, Some("Operation"), Some(id),
                            Some("effects"),
                            &prov.file, *e_line,
                            format!(
                                "effect ({}, {}, {}) is not a declared transition in entity {}; declared transitions are: [{}]",
                                entity_id, from, to, entity_id, declared.join(", ")
                            ),
                        ));
                    }
                }
            }
        }
    }

    Ok(())
}

fn validate_flow(
    id: &str,
    entry: &str,
    entry_line: u32,
    steps: &BTreeMap<String, RawStep>,
    prov: &Provenance,
    _index: &Index,
) -> Result<(), ElabError> {
    // entry must exist
    if !steps.contains_key(entry) {
        return Err(ElabError::new(
            5, Some("Flow"), Some(id), Some("entry"),
            &prov.file, entry_line,
            format!("entry step '{}' is not declared in steps", entry),
        ));
    }

    // All OperationSteps must declare a failure handler
    for (step_id, step) in steps {
        if let RawStep::OperationStep { on_failure: None, line, .. } = step {
            return Err(ElabError::new(
                5, Some("Flow"), Some(id),
                Some(&format!("steps.{}.on_failure", step_id)),
                &prov.file, *line,
                format!("OperationStep '{}' must declare a FailureHandler", step_id),
            ));
        }
    }

    // All step refs must resolve
    for (step_id, step) in steps {
        match step {
            RawStep::OperationStep { outcomes, .. } => {
                for (label, target) in outcomes {
                    if let RawStepTarget::StepRef(r, ref_line) = target {
                        if !steps.contains_key(r.as_str()) {
                            return Err(ElabError::new(
                                5, Some("Flow"), Some(id),
                                Some(&format!("steps.{}.outcomes.{}", step_id, label)),
                                &prov.file, *ref_line,
                                format!("step reference '{}' is not declared in steps", r),
                            ));
                        }
                    }
                }
            }
            RawStep::BranchStep { if_true, if_false, .. } => {
                if let RawStepTarget::StepRef(r, ref_line) = if_true {
                    if !steps.contains_key(r.as_str()) {
                        return Err(ElabError::new(
                            5, Some("Flow"), Some(id),
                            Some(&format!("steps.{}.if_true", step_id)),
                            &prov.file, *ref_line,
                            format!("step reference '{}' is not declared in steps", r),
                        ));
                    }
                }
                if let RawStepTarget::StepRef(r, ref_line) = if_false {
                    if !steps.contains_key(r.as_str()) {
                        return Err(ElabError::new(
                            5, Some("Flow"), Some(id),
                            Some(&format!("steps.{}.if_false", step_id)),
                            &prov.file, *ref_line,
                            format!("step reference '{}' is not declared in steps", r),
                        ));
                    }
                }
            }
            RawStep::HandoffStep { next, line, .. } => {
                if !steps.contains_key(next.as_str()) {
                    return Err(ElabError::new(
                        5, Some("Flow"), Some(id),
                        Some(&format!("steps.{}.next", step_id)),
                        &prov.file, *line,
                        format!("step reference '{}' is not declared in steps", next),
                    ));
                }
            }
            RawStep::SubFlowStep { on_success, .. } => {
                if let RawStepTarget::StepRef(r, ref_line) = on_success {
                    if !steps.contains_key(r.as_str()) {
                        return Err(ElabError::new(
                            5, Some("Flow"), Some(id),
                            Some(&format!("steps.{}.on_success", step_id)),
                            &prov.file, *ref_line,
                            format!("step reference '{}' is not declared in steps", r),
                        ));
                    }
                }
            }
            RawStep::ParallelStep { .. } => {
                // branches are self-contained sub-graphs with no refs into the outer flow's steps
            }
        }
    }

    // Step graph must be acyclic — detect via topological sort
    detect_step_cycle(id, entry, steps, prov)?;

    Ok(())
}

fn detect_step_cycle(
    flow_id: &str,
    _entry: &str,
    steps: &BTreeMap<String, RawStep>,
    prov: &Provenance,
) -> Result<(), ElabError> {
    // Build adjacency list
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for (sid, step) in steps {
        let mut neighbors: Vec<&str> = Vec::new();
        match step {
            RawStep::OperationStep { outcomes, .. } => {
                for (_, t) in outcomes {
                    if let RawStepTarget::StepRef(r, _) = t {
                        neighbors.push(r.as_str());
                    }
                }
            }
            RawStep::BranchStep { if_true, if_false, .. } => {
                if let RawStepTarget::StepRef(r, _) = if_true { neighbors.push(r.as_str()); }
                if let RawStepTarget::StepRef(r, _) = if_false { neighbors.push(r.as_str()); }
            }
            RawStep::HandoffStep { next, .. } => { neighbors.push(next.as_str()); }
            RawStep::SubFlowStep { on_success, .. } => {
                if let RawStepTarget::StepRef(r, _) = on_success { neighbors.push(r.as_str()); }
            }
            RawStep::ParallelStep { .. } => {
                // terminal node in the outer flow's step graph
            }
        }
        adj.insert(sid.as_str(), neighbors);
    }

    // Kahn's algorithm (topological sort) — if it doesn't consume all nodes, there's a cycle
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    for sid in steps.keys() {
        in_degree.entry(sid.as_str()).or_insert(0);
    }
    for neighbors in adj.values() {
        for &n in neighbors {
            *in_degree.entry(n).or_insert(0) += 1;
        }
    }

    let mut queue: VecDeque<&str> = in_degree.iter()
        .filter(|(_, &d)| d == 0)
        .map(|(&k, _)| k)
        .collect();
    let mut processed: HashSet<&str> = HashSet::new();
    while let Some(node) = queue.pop_front() {
        processed.insert(node);
        for &neighbor in adj.get(node).unwrap_or(&vec![]) {
            let deg = in_degree.get_mut(neighbor).unwrap();
            *deg -= 1;
            if *deg == 0 {
                queue.push_back(neighbor);
            }
        }
    }

    if processed.len() < steps.len() {
        // Collect cyclic step ids (those not processed), sorted for determinism
        let mut cyclic: Vec<&str> = steps.keys()
            .map(String::as_str)
            .filter(|s| !processed.contains(*s))
            .collect();
        cyclic.sort_unstable();
        // Report on the line of the first cyclic step
        let report_line = cyclic.first()
            .and_then(|s| steps.get(*s))
            .map(|step| match step {
                RawStep::OperationStep { line, .. } => *line,
                RawStep::BranchStep    { line, .. } => *line,
                RawStep::HandoffStep   { line, .. } => *line,
                RawStep::SubFlowStep   { line, .. } => *line,
                RawStep::ParallelStep  { line, .. } => *line,
            })
            .unwrap_or(prov.line);
        let cycle_list = cyclic.join(", ");
        return Err(ElabError::new(
            5, Some("Flow"), Some(flow_id), Some("steps"),
            &prov.file, report_line,
            format!("flow step graph is not acyclic: cycle detected involving steps [{}]", cycle_list),
        ));
    }

    Ok(())
}

/// Pass 5: detect cycles in the cross-Flow SubFlowStep reference graph.
/// Uses DFS with an explicit path stack so we can emit the cycle sequence.
fn validate_flow_reference_graph(constructs: &[RawConstruct]) -> Result<(), ElabError> {
    // Build flow index: id → (prov, steps)
    let mut flows: HashMap<&str, (&Provenance, &BTreeMap<String, RawStep>)> = HashMap::new();
    for c in constructs {
        if let RawConstruct::Flow { id, steps, prov, .. } = c {
            flows.insert(id.as_str(), (prov, steps));
        }
    }

    let mut flow_ids: Vec<&str> = flows.keys().copied().collect();
    flow_ids.sort_unstable(); // deterministic order

    let mut visited: HashSet<&str> = HashSet::new();
    let mut in_path: HashSet<&str> = HashSet::new();
    let mut path: Vec<&str> = Vec::new();

    for &fid in &flow_ids {
        if !visited.contains(fid) {
            dfs_flow_refs(fid, &flows, &mut visited, &mut in_path, &mut path)?;
        }
    }
    Ok(())
}

/// Collect all (step_id, flow_line, ref_flow_id) tuples from SubFlowSteps,
/// including those nested inside ParallelStep branches.
fn collect_subflow_refs<'a>(
    step: &'a RawStep,
    step_id: &'a str,
    out: &mut Vec<(&'a str, u32, &'a str)>,
) {
    match step {
        RawStep::SubFlowStep { flow, flow_line, .. } => {
            out.push((step_id, *flow_line, flow.as_str()));
        }
        RawStep::ParallelStep { branches, .. } => {
            for branch in branches {
                for (branch_step_id, branch_step) in &branch.steps {
                    collect_subflow_refs(branch_step, branch_step_id.as_str(), out);
                }
            }
        }
        _ => {}
    }
}

fn dfs_flow_refs<'a>(
    flow_id: &'a str,
    flows: &HashMap<&'a str, (&'a Provenance, &'a BTreeMap<String, RawStep>)>,
    visited: &mut HashSet<&'a str>,
    in_path: &mut HashSet<&'a str>,
    path: &mut Vec<&'a str>,
) -> Result<(), ElabError> {
    path.push(flow_id);
    in_path.insert(flow_id);

    if let Some((prov, steps)) = flows.get(flow_id) {
        // Collect all SubFlowStep references, including those nested inside ParallelStep branches
        let mut sub_refs: Vec<(&str, u32, &str)> = Vec::new(); // (step_id, flow_line, ref_flow)
        for (step_id, step) in steps.iter() {
            collect_subflow_refs(step, step_id.as_str(), &mut sub_refs);
        }
        for (step_id, flow_line, ref_flow) in sub_refs {
            if in_path.contains(ref_flow) {
                let cycle_start = path.iter().position(|&s| s == ref_flow).unwrap();
                let mut cycle_nodes: Vec<&str> = path[cycle_start..].to_vec();
                cycle_nodes.push(ref_flow);
                let cycle_str = cycle_nodes.join(" → ");
                return Err(ElabError::new(
                    5, Some("Flow"), Some(flow_id),
                    Some(&format!("steps.{}.flow", step_id)),
                    &prov.file, flow_line,
                    format!("flow reference cycle detected: {}", cycle_str),
                ));
            }
            if !visited.contains(ref_flow) && flows.contains_key(ref_flow) {
                dfs_flow_refs(ref_flow, flows, visited, in_path, path)?;
            }
        }
    }

    in_path.remove(flow_id);
    visited.insert(flow_id);
    path.pop();
    Ok(())
}

/// Collect entity ids affected by a branch, directly and through one SubFlowStep level.
/// Returns HashMap<entity_id, Option<trace>>:
///   None     = direct effect (OperationStep in this branch)
///   Some(t)  = transitive effect (SubFlowStep → <flow> → <op>)
fn collect_branch_entity_effects<'a>(
    branch: &'a RawBranch,
    op_entities: &HashMap<&'a str, Vec<&'a str>>,
    flow_map: &HashMap<&'a str, &'a BTreeMap<String, RawStep>>,
) -> HashMap<String, Option<String>> {
    let mut effects: HashMap<String, Option<String>> = HashMap::new();

    for (_step_id, step) in &branch.steps {
        match step {
            RawStep::OperationStep { op, .. } => {
                if let Some(entities) = op_entities.get(op.as_str()) {
                    for &entity in entities {
                        effects.entry(entity.to_owned()).or_insert(None);
                    }
                }
            }
            RawStep::SubFlowStep { flow: flow_id, .. } => {
                if let Some(flow_steps) = flow_map.get(flow_id.as_str()) {
                    for (_sub_step_id, sub_step) in flow_steps.iter() {
                        if let RawStep::OperationStep { op, .. } = sub_step {
                            if let Some(entities) = op_entities.get(op.as_str()) {
                                for &entity in entities {
                                    let trace = format!("SubFlowStep → {} → {}", flow_id, op);
                                    effects.entry(entity.to_owned()).or_insert(Some(trace));
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    effects
}

/// Pass 5: check that no two parallel branches affect the same entity (transitively).
fn validate_parallel_conflicts(constructs: &[RawConstruct]) -> Result<(), ElabError> {
    // op_id → entity ids it affects
    let mut op_entities: HashMap<&str, Vec<&str>> = HashMap::new();
    for c in constructs {
        if let RawConstruct::Operation { id, effects, .. } = c {
            let entities: Vec<&str> = effects.iter().map(|(e, _, _, _)| e.as_str()).collect();
            op_entities.insert(id.as_str(), entities);
        }
    }

    // flow_id → steps (for transitive SubFlowStep resolution)
    let mut flow_map: HashMap<&str, &BTreeMap<String, RawStep>> = HashMap::new();
    for c in constructs {
        if let RawConstruct::Flow { id, steps, .. } = c {
            flow_map.insert(id.as_str(), steps);
        }
    }

    for c in constructs {
        if let RawConstruct::Flow { id: flow_id, steps, prov, .. } = c {
            for (step_id, step) in steps {
                if let RawStep::ParallelStep { branches, branches_line, .. } = step {
                    // Collect entity effects per branch
                    let branch_effects: Vec<(&str, HashMap<String, Option<String>>)> = branches.iter()
                        .map(|b| (b.id.as_str(), collect_branch_entity_effects(b, &op_entities, &flow_map)))
                        .collect();

                    // Check all pairs for entity overlap
                    for i in 0..branch_effects.len() {
                        for j in (i + 1)..branch_effects.len() {
                            let (b1_id, b1_effects) = &branch_effects[i];
                            let (b2_id, b2_effects) = &branch_effects[j];

                            // Sort entity keys for deterministic error reporting
                            let mut b1_sorted: Vec<&String> = b1_effects.keys().collect();
                            b1_sorted.sort_unstable();

                            for entity in b1_sorted {
                                if let Some(b2_trace) = b2_effects.get(entity) {
                                    let b1_trace = b1_effects.get(entity).unwrap();
                                    let msg = if b1_trace.is_none() && b2_trace.is_none() {
                                        format!(
                                            "parallel branches '{}' and '{}' both declare effects on entity '{}'; parallel branch entity effect sets must be disjoint",
                                            b1_id, b2_id, entity
                                        )
                                    } else {
                                        let (transitive_id, trace) = if let Some(t) = b1_trace {
                                            (*b1_id, t.as_str())
                                        } else {
                                            (*b2_id, b2_trace.as_ref().unwrap().as_str())
                                        };
                                        format!(
                                            "parallel branches '{}' and '{}' both affect entity '{}' ({} transitively through {}); parallel branch entity effect sets must be disjoint",
                                            b1_id, b2_id, entity, transitive_id, trace
                                        )
                                    };
                                    return Err(ElabError::new(
                                        5, Some("Flow"), Some(flow_id),
                                        Some(&format!("steps.{}.branches", step_id)),
                                        &prov.file, *branches_line,
                                        msg,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Pass 6: serialization
// ──────────────────────────────────────────────────────────────────────────────

fn serialize(constructs: &[RawConstruct], bundle_id: &str) -> Value {
    // Build fact type lookup for expression serialization
    let mut fact_types: HashMap<String, RawType> = HashMap::new();
    for c in constructs {
        if let RawConstruct::Fact { id, type_, .. } = c {
            fact_types.insert(id.clone(), type_.clone());
        }
    }

    // Collect constructs by kind for ordering
    let mut facts: Vec<&RawConstruct> = Vec::new();
    let mut entities: Vec<&RawConstruct> = Vec::new();
    let mut rules_by_stratum: BTreeMap<i64, Vec<&RawConstruct>> = BTreeMap::new();
    let mut operations: Vec<&RawConstruct> = Vec::new();
    let mut flows: Vec<&RawConstruct> = Vec::new();

    for c in constructs {
        match c {
            RawConstruct::Fact { .. } => facts.push(c),
            RawConstruct::Entity { .. } => entities.push(c),
            RawConstruct::Rule { stratum, .. } => {
                rules_by_stratum.entry(*stratum).or_default().push(c);
            }
            RawConstruct::Operation { .. } => operations.push(c),
            RawConstruct::Flow { .. } => flows.push(c),
            _ => {} // TypeDecl and Import are not emitted
        }
    }

    // Sort within categories
    facts.sort_by(|a, b| construct_id(a).cmp(construct_id(b)));
    entities.sort_by(|a, b| construct_id(a).cmp(construct_id(b)));
    for rules in rules_by_stratum.values_mut() {
        rules.sort_by(|a, b| construct_id(a).cmp(construct_id(b)));
    }
    operations.sort_by(|a, b| construct_id(a).cmp(construct_id(b)));
    flows.sort_by(|a, b| construct_id(a).cmp(construct_id(b)));

    // Build ordered constructs array
    let mut result: Vec<Value> = Vec::new();
    for c in &facts { result.push(serialize_construct(c, &fact_types)); }
    for c in &entities { result.push(serialize_construct(c, &fact_types)); }
    for rules in rules_by_stratum.values() {
        for c in rules { result.push(serialize_construct(c, &fact_types)); }
    }
    for c in &operations { result.push(serialize_construct(c, &fact_types)); }
    for c in &flows { result.push(serialize_construct(c, &fact_types)); }

    // Bundle envelope — keys sorted: constructs, id, kind, tenor
    let mut bundle = Map::new();
    bundle.insert("constructs".to_owned(), Value::Array(result));
    bundle.insert("id".to_owned(), Value::String(bundle_id.to_owned()));
    bundle.insert("kind".to_owned(), Value::String("Bundle".to_owned()));
    bundle.insert("tenor".to_owned(), Value::String("0.3".to_owned()));
    Value::Object(bundle)
}

fn construct_id(c: &RawConstruct) -> &str {
    match c {
        RawConstruct::Fact { id, .. } => id,
        RawConstruct::Entity { id, .. } => id,
        RawConstruct::Rule { id, .. } => id,
        RawConstruct::Operation { id, .. } => id,
        RawConstruct::Flow { id, .. } => id,
        RawConstruct::TypeDecl { id, .. } => id,
        RawConstruct::Import { .. } => "",
    }
}

fn serialize_construct(c: &RawConstruct, fact_types: &HashMap<String, RawType>) -> Value {
    match c {
        RawConstruct::Fact { id, type_, source, default, prov } => {
            let mut m = Map::new();
            if let Some(d) = default {
                // Decimal defaults are written as quoted strings in DSL; use declared type's
                // precision/scale so trailing zeros and exact representation are preserved.
                let default_val = match (type_, d) {
                    (RawType::Decimal { precision, scale }, RawLiteral::Str(s)) => {
                        let mut dm = Map::new();
                        dm.insert("kind".to_owned(), json!("decimal_value"));
                        dm.insert("precision".to_owned(), json!(precision));
                        dm.insert("scale".to_owned(), json!(scale));
                        dm.insert("value".to_owned(), json!(s));
                        Value::Object(dm)
                    }
                    _ => serialize_literal(d),
                };
                m.insert("default".to_owned(), default_val);
            }
            m.insert("id".to_owned(), json!(id));
            m.insert("kind".to_owned(), json!("Fact"));
            m.insert("provenance".to_owned(), serialize_prov(prov));
            m.insert("source".to_owned(), serialize_source(source));
            m.insert("tenor".to_owned(), json!("0.3"));
            m.insert("type".to_owned(), serialize_type(type_));
            Value::Object(m)
        }
        RawConstruct::Entity { id, states, initial, transitions, parent, prov, .. } => {
            let mut m = Map::new();
            m.insert("id".to_owned(), json!(id));
            m.insert("initial".to_owned(), json!(initial));
            m.insert("kind".to_owned(), json!("Entity"));
            if let Some(p) = parent {
                m.insert("parent".to_owned(), json!(p));
            }
            m.insert("provenance".to_owned(), serialize_prov(prov));
            m.insert("states".to_owned(), json!(states));
            m.insert("tenor".to_owned(), json!("0.3"));
            let t_arr: Vec<Value> = transitions.iter()
                .map(|(f, to, _)| {
                    let mut tm = Map::new();
                    tm.insert("from".to_owned(), json!(f));
                    tm.insert("to".to_owned(), json!(to));
                    Value::Object(tm)
                })
                .collect();
            m.insert("transitions".to_owned(), Value::Array(t_arr));
            Value::Object(m)
        }
        RawConstruct::Rule { id, stratum, when, verdict_type, payload_type, payload_value, prov, .. } => {
            let mut m = Map::new();
            let mut body = Map::new();
            let mut produce = Map::new();
            produce.insert("payload".to_owned(), serialize_payload(payload_type, payload_value, fact_types));
            produce.insert("verdict_type".to_owned(), json!(verdict_type));
            body.insert("produce".to_owned(), Value::Object(produce));
            body.insert("when".to_owned(), serialize_expr(when, fact_types));
            m.insert("body".to_owned(), Value::Object(body));
            m.insert("id".to_owned(), json!(id));
            m.insert("kind".to_owned(), json!("Rule"));
            m.insert("provenance".to_owned(), serialize_prov(prov));
            m.insert("stratum".to_owned(), json!(stratum));
            m.insert("tenor".to_owned(), json!("0.3"));
            Value::Object(m)
        }
        RawConstruct::Operation { id, allowed_personas, precondition, effects, error_contract, prov, .. } => {
            let mut m = Map::new();
            m.insert("allowed_personas".to_owned(), json!(allowed_personas));
            let effects_arr: Vec<Value> = effects.iter()
                .map(|(eid, f, t, _)| {
                    let mut em = Map::new();
                    em.insert("entity_id".to_owned(), json!(eid));
                    em.insert("from".to_owned(), json!(f));
                    em.insert("to".to_owned(), json!(t));
                    Value::Object(em)
                })
                .collect();
            m.insert("effects".to_owned(), Value::Array(effects_arr));
            m.insert("error_contract".to_owned(), json!(error_contract));
            m.insert("id".to_owned(), json!(id));
            m.insert("kind".to_owned(), json!("Operation"));
            m.insert("precondition".to_owned(), serialize_expr(precondition, fact_types));
            m.insert("provenance".to_owned(), serialize_prov(prov));
            m.insert("tenor".to_owned(), json!("0.3"));
            Value::Object(m)
        }
        RawConstruct::Flow { id, snapshot, entry, steps, prov, .. } => {
            let mut m = Map::new();
            m.insert("entry".to_owned(), json!(entry));
            m.insert("id".to_owned(), json!(id));
            m.insert("kind".to_owned(), json!("Flow"));
            m.insert("provenance".to_owned(), serialize_prov(prov));
            m.insert("snapshot".to_owned(), json!(snapshot));
            m.insert("steps".to_owned(), serialize_steps(steps, entry, fact_types));
            m.insert("tenor".to_owned(), json!("0.3"));
            Value::Object(m)
        }
        _ => json!(null),
    }
}

fn serialize_prov(prov: &Provenance) -> Value {
    let mut m = Map::new();
    m.insert("file".to_owned(), json!(prov.file));
    m.insert("line".to_owned(), json!(prov.line));
    Value::Object(m)
}

fn serialize_source(source: &str) -> Value {
    // "system.field" → {"field": "...", "system": "..."}
    if let Some(dot) = source.find('.') {
        let system = &source[..dot];
        let field = &source[dot + 1..];
        let mut m = Map::new();
        m.insert("field".to_owned(), json!(field));
        m.insert("system".to_owned(), json!(system));
        Value::Object(m)
    } else {
        json!(source)
    }
}

fn serialize_type(t: &RawType) -> Value {
    match t {
        RawType::Bool => json!({"base": "Bool"}),
        RawType::Date => json!({"base": "Date"}),
        RawType::DateTime => json!({"base": "DateTime"}),
        RawType::Int { min, max } => {
            let mut m = Map::new();
            m.insert("base".to_owned(), json!("Int"));
            m.insert("max".to_owned(), json!(max));
            m.insert("min".to_owned(), json!(min));
            Value::Object(m)
        }
        RawType::Decimal { precision, scale } => {
            let mut m = Map::new();
            m.insert("base".to_owned(), json!("Decimal"));
            m.insert("precision".to_owned(), json!(precision));
            m.insert("scale".to_owned(), json!(scale));
            Value::Object(m)
        }
        RawType::Text { max_length } => {
            let mut m = Map::new();
            m.insert("base".to_owned(), json!("Text"));
            m.insert("max_length".to_owned(), json!(max_length));
            Value::Object(m)
        }
        RawType::Enum { values } => json!({"base": "Enum", "values": values}),
        RawType::Money { currency } => {
            let mut m = Map::new();
            m.insert("base".to_owned(), json!("Money"));
            m.insert("currency".to_owned(), json!(currency));
            Value::Object(m)
        }
        RawType::Duration { unit, min, max } => {
            let mut m = Map::new();
            m.insert("base".to_owned(), json!("Duration"));
            m.insert("max".to_owned(), json!(max));
            m.insert("min".to_owned(), json!(min));
            m.insert("unit".to_owned(), json!(unit));
            Value::Object(m)
        }
        RawType::Record { fields } => {
            let mut fm = Map::new();
            for (k, v) in fields {
                fm.insert(k.clone(), serialize_type(v));
            }
            let mut m = Map::new();
            m.insert("base".to_owned(), json!("Record"));
            m.insert("fields".to_owned(), Value::Object(fm));
            Value::Object(m)
        }
        RawType::List { element_type, max } => {
            let mut m = Map::new();
            m.insert("base".to_owned(), json!("List"));
            m.insert("element_type".to_owned(), serialize_type(element_type));
            m.insert("max".to_owned(), json!(max));
            Value::Object(m)
        }
        RawType::TypeRef(name) => json!({"base": "TypeRef", "id": name}), // should have been resolved
    }
}

fn serialize_literal(lit: &RawLiteral) -> Value {
    match lit {
        RawLiteral::Bool(b) => {
            let mut m = Map::new();
            m.insert("kind".to_owned(), json!("bool_literal"));
            m.insert("value".to_owned(), json!(b));
            Value::Object(m)
        }
        RawLiteral::Int(n) => {
            let mut m = Map::new();
            m.insert("kind".to_owned(), json!("int_literal"));
            m.insert("value".to_owned(), json!(n));
            Value::Object(m)
        }
        RawLiteral::Float(f) => {
            // Determine precision and scale from the string
            let (precision, scale) = decimal_precision_scale(f);
            let mut m = Map::new();
            m.insert("kind".to_owned(), json!("decimal_value"));
            m.insert("precision".to_owned(), json!(precision));
            m.insert("scale".to_owned(), json!(scale));
            m.insert("value".to_owned(), json!(f));
            Value::Object(m)
        }
        RawLiteral::Str(s) => json!(s),
        RawLiteral::Money { amount, currency } => {
            let (precision, scale) = money_decimal_precision_scale(amount);
            let mut amount_m = Map::new();
            amount_m.insert("kind".to_owned(), json!("decimal_value"));
            amount_m.insert("precision".to_owned(), json!(precision));
            amount_m.insert("scale".to_owned(), json!(scale));
            amount_m.insert("value".to_owned(), json!(amount));
            let mut m = Map::new();
            m.insert("amount".to_owned(), Value::Object(amount_m));
            m.insert("currency".to_owned(), json!(currency));
            m.insert("kind".to_owned(), json!("money_value"));
            Value::Object(m)
        }
    }
}

/// For Money decimal amounts: always use precision=10, scale=2 (per fixture convention)
fn money_decimal_precision_scale(_amount: &str) -> (u32, u32) {
    (10, 2)
}

fn decimal_precision_scale(s: &str) -> (u32, u32) {
    if let Some(dot) = s.find('.') {
        let integer_part = &s[..dot];
        let frac_part = &s[dot + 1..];
        let scale = frac_part.len() as u32;
        let int_digits = integer_part.trim_start_matches('-').len() as u32;
        let precision = int_digits + scale;
        (precision.max(1), scale)
    } else {
        let digits = s.trim_start_matches('-').len() as u32;
        (digits.max(1), 0)
    }
}

fn serialize_payload(type_: &RawType, value: &RawTerm, fact_types: &HashMap<String, RawType>) -> Value {
    let mut m = Map::new();
    // For Text with max_length=0 (no explicit param), infer from the literal string
    let effective_type = match (type_, value) {
        (RawType::Text { max_length: 0 }, RawTerm::Literal(RawLiteral::Str(s))) => {
            RawType::Text { max_length: s.len() as u32 }
        }
        _ => type_.clone(),
    };
    m.insert("type".to_owned(), serialize_type(&effective_type));
    match value {
        RawTerm::Literal(RawLiteral::Bool(b)) => { m.insert("value".to_owned(), json!(b)); }
        RawTerm::Literal(RawLiteral::Int(n))  => { m.insert("value".to_owned(), json!(n)); }
        RawTerm::Literal(RawLiteral::Str(s))  => { m.insert("value".to_owned(), json!(s)); }
        RawTerm::Literal(lit)                  => { m.insert("value".to_owned(), serialize_literal(lit)); }
        RawTerm::Mul { left, right } => {
            m.insert("value".to_owned(), serialize_mul_term(left, right, fact_types));
        }
        _ => { m.insert("value".to_owned(), json!(null)); }
    }
    Value::Object(m)
}

/// Serialize a multiplication expression: `left * right` or `right * left`.
/// Emits `{"left": fact_ref, "literal": n, "op": "*", "result_type": ...}`.
fn serialize_mul_term(left: &RawTerm, right: &RawTerm, fact_types: &HashMap<String, RawType>) -> Value {
    // Determine which operand is the fact_ref and which is the literal
    let (fact_term, lit_n) = match (left, right) {
        (RawTerm::FactRef(_), RawTerm::Literal(RawLiteral::Int(n))) => (left, *n),
        (RawTerm::Literal(RawLiteral::Int(n)), RawTerm::FactRef(_)) => (right, *n),
        _ => {
            // Fallback: serialize raw
            let mut m = Map::new();
            m.insert("left".to_owned(), serialize_term(left));
            m.insert("op".to_owned(), json!("*"));
            m.insert("right".to_owned(), serialize_term(right));
            return Value::Object(m);
        }
    };
    // Compute result_type from fact's Int range × literal
    let result_type = if let RawTerm::FactRef(name) = fact_term {
        match fact_types.get(name.as_str()) {
            Some(RawType::Int { min, max }) => {
                let (rmin, rmax) = if lit_n >= 0 {
                    (min * lit_n, max * lit_n)
                } else {
                    (max * lit_n, min * lit_n)
                };
                Some(RawType::Int { min: rmin, max: rmax })
            }
            _ => None,
        }
    } else {
        None
    };
    let mut m = Map::new();
    m.insert("left".to_owned(), serialize_term(fact_term));
    m.insert("literal".to_owned(), json!(lit_n));
    m.insert("op".to_owned(), json!("*"));
    if let Some(rt) = result_type {
        m.insert("result_type".to_owned(), serialize_type(&rt));
    }
    Value::Object(m)
}

/// Number of decimal digits needed to represent an Int range when promoted to Decimal.
/// Per §11.2: Decimal(ceil(log10(max(|min|,|max|)))+1, 0)
fn int_to_decimal_precision(min: i64, max: i64) -> u32 {
    let abs_min = if min < 0 { (-(min as i128)) as u64 } else { min as u64 };
    let abs_max_val = if max < 0 { (-(max as i128)) as u64 } else { max as u64 };
    let abs_max = abs_min.max(abs_max_val) as f64;
    if abs_max == 0.0 { return 1; }
    (abs_max.log10().ceil() as u32) + 1
}

/// Return the numeric RawType of a term (FactRef → fact type, Literal(Int) → Int(n,n), Mul → product range).
fn term_numeric_type(term: &RawTerm, fact_types: &HashMap<String, RawType>) -> Option<RawType> {
    match term {
        RawTerm::FactRef(name) => fact_types.get(name.as_str()).cloned(),
        RawTerm::Literal(RawLiteral::Int(n)) => Some(RawType::Int { min: *n, max: *n }),
        RawTerm::Mul { left, right } => {
            let (fact_name, lit_n) = match (left.as_ref(), right.as_ref()) {
                (RawTerm::FactRef(n), RawTerm::Literal(RawLiteral::Int(v))) => (Some(n.as_str()), Some(*v)),
                (RawTerm::Literal(RawLiteral::Int(v)), RawTerm::FactRef(n)) => (Some(n.as_str()), Some(*v)),
                _ => (None, None),
            };
            if let (Some(name), Some(n)) = (fact_name, lit_n) {
                if let Some(RawType::Int { min, max }) = fact_types.get(name) {
                    let (rmin, rmax) = if n >= 0 { (min * n, max * n) } else { (max * n, min * n) };
                    return Some(RawType::Int { min: rmin, max: rmax });
                }
            }
            None
        }
        _ => None,
    }
}

/// Return the comparison_type to emit for a Compare node, if any.
/// Emitted for: Money (always); Int×Decimal cross-type (promoted); Mul×Int (combined range).
fn comparison_type_for_compare(left: &RawTerm, right: &RawTerm, fact_types: &HashMap<String, RawType>) -> Option<RawType> {
    let lt = term_numeric_type(left, fact_types);
    let rt = term_numeric_type(right, fact_types);
    match (&lt, &rt) {
        (Some(t @ RawType::Money { .. }), _) | (_, Some(t @ RawType::Money { .. })) => Some(t.clone()),
        (Some(RawType::Int { min, max }), Some(RawType::Decimal { precision, scale })) => {
            let int_prec = int_to_decimal_precision(*min, *max);
            Some(RawType::Decimal { precision: (*precision).max(int_prec) + 1, scale: *scale })
        }
        (Some(RawType::Decimal { precision, scale }), Some(RawType::Int { min, max })) => {
            let int_prec = int_to_decimal_precision(*min, *max);
            Some(RawType::Decimal { precision: (*precision).max(int_prec) + 1, scale: *scale })
        }
        // Mul on left with Int right: combined range
        (Some(RawType::Int { min: lmin, max: lmax }), Some(RawType::Int { min: rmin, max: rmax }))
            if matches!(left, RawTerm::Mul { .. }) =>
        {
            Some(RawType::Int { min: (*lmin).min(*rmin), max: (*lmax).max(*rmax) })
        }
        _ => None,
    }
}

/// Serialize a term with fact_types context so Mul uses the canonical interchange form.
fn serialize_term_ctx(term: &RawTerm, fact_types: &HashMap<String, RawType>) -> Value {
    match term {
        RawTerm::Mul { left, right } => serialize_mul_term(left, right, fact_types),
        _ => serialize_term(term),
    }
}

fn serialize_expr(expr: &RawExpr, fact_types: &HashMap<String, RawType>) -> Value {
    match expr {
        RawExpr::Compare { op, left, right, .. } => {
            // Keep left fact type for Enum literal annotation on right side
            let left_fact_type: Option<RawType> = match left {
                RawTerm::FactRef(name) => fact_types.get(name.as_str()).cloned(),
                _ => None,
            };
            let mut m = Map::new();
            if let Some(ct) = comparison_type_for_compare(left, right, fact_types) {
                m.insert("comparison_type".to_owned(), serialize_type(&ct));
            }
            m.insert("left".to_owned(), serialize_term_ctx(left, fact_types));
            m.insert("op".to_owned(), json!(op));
            // For Str literals compared against an Enum fact, annotate with the enum type
            let right_val = match (right, &left_fact_type) {
                (RawTerm::Literal(RawLiteral::Str(s)), Some(t @ RawType::Enum { .. })) => {
                    json!({"literal": s, "type": serialize_type(t)})
                }
                _ => serialize_term_ctx(right, fact_types),
            };
            m.insert("right".to_owned(), right_val);
            Value::Object(m)
        }
        RawExpr::VerdictPresent { id, .. } => json!({"verdict_present": id}),
        RawExpr::And(a, b) => {
            json!({
                "left": serialize_expr(a, fact_types),
                "op": "and",
                "right": serialize_expr(b, fact_types)
            })
        }
        RawExpr::Or(a, b) => {
            json!({
                "left": serialize_expr(a, fact_types),
                "op": "or",
                "right": serialize_expr(b, fact_types)
            })
        }
        RawExpr::Not(e) => {
            json!({"op": "not", "operand": serialize_expr(e, fact_types)})
        }
        RawExpr::Forall { var, domain, body, .. } => {
            // Look up the domain fact's list element type for variable_type
            let variable_type = match fact_types.get(domain.as_str()) {
                Some(RawType::List { element_type, .. }) => Some(element_type.as_ref().clone()),
                _ => None,
            };
            let mut m = Map::new();
            m.insert("body".to_owned(), serialize_expr(body, fact_types));
            m.insert("domain".to_owned(), json!({"fact_ref": domain}));
            m.insert("quantifier".to_owned(), json!("forall"));
            m.insert("variable".to_owned(), json!(var));
            if let Some(vt) = variable_type {
                m.insert("variable_type".to_owned(), serialize_type(&vt));
            }
            Value::Object(m)
        }
    }
}

fn serialize_term(term: &RawTerm) -> Value {
    match term {
        RawTerm::FactRef(name) => json!({"fact_ref": name}),
        RawTerm::FieldRef { var, field } => {
            json!({"field_ref": {"field": field, "var": var}})
        }
        RawTerm::Literal(lit) => {
            match lit {
                RawLiteral::Bool(b) => json!({"literal": b, "type": {"base": "Bool"}}),
                RawLiteral::Int(n) => json!({"literal": n, "type": {"base": "Int", "min": n, "max": n}}),
                RawLiteral::Str(s) => json!({"literal": s}),
                RawLiteral::Float(f) => {
                    let (p, sc) = decimal_precision_scale(f);
                    json!({"literal": f, "type": {"base": "Decimal", "precision": p, "scale": sc}})
                }
                RawLiteral::Money { amount, currency } => {
                    let (p, sc) = money_decimal_precision_scale(amount);
                    json!({
                        "literal": {
                            "amount": {"kind": "decimal_value", "precision": p, "scale": sc, "value": amount},
                            "currency": currency
                        },
                        "type": {"base": "Money", "currency": currency}
                    })
                }
            }
        }
        RawTerm::Mul { left, right } => {
            // Bare serialization (no fact_types available here); used only when Mul
            // appears in a where-clause term position. Full form via serialize_mul_term.
            let mut m = Map::new();
            m.insert("left".to_owned(), serialize_term(left));
            m.insert("op".to_owned(), json!("*"));
            m.insert("right".to_owned(), serialize_term(right));
            Value::Object(m)
        }
    }
}

/// Serialize Flow steps as an array: entry first, then topological order
fn serialize_steps(steps: &BTreeMap<String, RawStep>, entry: &str, fact_types: &HashMap<String, RawType>) -> Value {
    let order = topological_order(steps, entry);
    let arr: Vec<Value> = order.iter()
        .filter_map(|sid| steps.get(sid.as_str()).map(|s| (sid, s)))
        .map(|(sid, step)| serialize_step(sid, step, fact_types))
        .collect();
    Value::Array(arr)
}

fn topological_order(steps: &BTreeMap<String, RawStep>, entry: &str) -> Vec<String> {
    // Build adjacency list
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for (sid, step) in steps {
        let mut neighbors: Vec<&str> = Vec::new();
        match step {
            RawStep::OperationStep { outcomes, .. } => {
                for (_, t) in outcomes {
                    if let RawStepTarget::StepRef(r, _) = t { neighbors.push(r.as_str()); }
                }
            }
            RawStep::BranchStep { if_true, if_false, .. } => {
                if let RawStepTarget::StepRef(r, _) = if_true { neighbors.push(r.as_str()); }
                if let RawStepTarget::StepRef(r, _) = if_false { neighbors.push(r.as_str()); }
            }
            RawStep::HandoffStep { next, .. } => { neighbors.push(next.as_str()); }
            RawStep::SubFlowStep { on_success, .. } => {
                if let RawStepTarget::StepRef(r, _) = on_success { neighbors.push(r.as_str()); }
            }
            RawStep::ParallelStep { .. } => {
                // terminal node in the outer flow's step graph
            }
        }
        adj.insert(sid.as_str(), neighbors);
    }

    // BFS from entry to get reachable steps in order
    let mut result: Vec<String> = Vec::new();
    let mut seen: HashSet<&str> = HashSet::new();
    let mut queue: VecDeque<&str> = VecDeque::new();
    queue.push_back(entry);
    seen.insert(entry);
    while let Some(node) = queue.pop_front() {
        result.push(node.to_owned());
        for &neighbor in adj.get(node).unwrap_or(&vec![]) {
            if !seen.contains(neighbor) && steps.contains_key(neighbor) {
                seen.insert(neighbor);
                queue.push_back(neighbor);
            }
        }
    }
    // Append any steps not reachable from entry (shouldn't happen in valid flows)
    for sid in steps.keys() {
        if !seen.contains(sid.as_str()) {
            result.push(sid.clone());
        }
    }
    result
}

fn serialize_step(id: &str, step: &RawStep, fact_types: &HashMap<String, RawType>) -> Value {
    match step {
        RawStep::OperationStep { op, persona, outcomes, on_failure, .. } => {
            let mut m = Map::new();
            m.insert("id".to_owned(), json!(id));
            m.insert("kind".to_owned(), json!("OperationStep"));
            if let Some(h) = on_failure {
                m.insert("on_failure".to_owned(), serialize_failure_handler(h));
            }
            m.insert("op".to_owned(), json!(op));
            let mut out_m = Map::new();
            for (label, target) in outcomes {
                out_m.insert(label.clone(), serialize_step_target(target));
            }
            m.insert("outcomes".to_owned(), Value::Object(out_m));
            m.insert("persona".to_owned(), json!(persona));
            Value::Object(m)
        }
        RawStep::BranchStep { condition, persona, if_true, if_false, .. } => {
            let mut m = Map::new();
            m.insert("condition".to_owned(), serialize_expr(condition, fact_types));
            m.insert("id".to_owned(), json!(id));
            m.insert("if_false".to_owned(), serialize_step_target(if_false));
            m.insert("if_true".to_owned(), serialize_step_target(if_true));
            m.insert("kind".to_owned(), json!("BranchStep"));
            m.insert("persona".to_owned(), json!(persona));
            Value::Object(m)
        }
        RawStep::HandoffStep { from_persona, to_persona, next, .. } => {
            let mut m = Map::new();
            m.insert("from_persona".to_owned(), json!(from_persona));
            m.insert("id".to_owned(), json!(id));
            m.insert("kind".to_owned(), json!("HandoffStep"));
            m.insert("next".to_owned(), json!(next));
            m.insert("to_persona".to_owned(), json!(to_persona));
            Value::Object(m)
        }
        RawStep::SubFlowStep { flow, persona, on_success, on_failure, .. } => {
            let mut m = Map::new();
            m.insert("flow".to_owned(), json!(flow));
            m.insert("id".to_owned(), json!(id));
            m.insert("kind".to_owned(), json!("SubFlowStep"));
            m.insert("on_failure".to_owned(), serialize_failure_handler(on_failure));
            m.insert("on_success".to_owned(), serialize_step_target(on_success));
            m.insert("persona".to_owned(), json!(persona));
            Value::Object(m)
        }
        RawStep::ParallelStep { branches, join, .. } => {
            let branches_arr: Vec<Value> = branches.iter().map(|b| {
                let mut bm = Map::new();
                bm.insert("entry".to_owned(), json!(b.entry));
                bm.insert("id".to_owned(), json!(b.id));
                bm.insert("steps".to_owned(), serialize_steps(&b.steps, &b.entry, fact_types));
                Value::Object(bm)
            }).collect();
            let mut join_m = Map::new();
            if let Some(t) = &join.on_all_success {
                join_m.insert("on_all_success".to_owned(), serialize_step_target(t));
            }
            if let Some(h) = &join.on_any_failure {
                join_m.insert("on_any_failure".to_owned(), serialize_failure_handler(h));
            }
            if let Some(t) = &join.on_all_complete {
                join_m.insert("on_all_complete".to_owned(), serialize_step_target(t));
            }
            let mut m = Map::new();
            m.insert("branches".to_owned(), Value::Array(branches_arr));
            m.insert("id".to_owned(), json!(id));
            m.insert("join".to_owned(), Value::Object(join_m));
            m.insert("kind".to_owned(), json!("ParallelStep"));
            Value::Object(m)
        }
    }
}

fn serialize_step_target(target: &RawStepTarget) -> Value {
    match target {
        RawStepTarget::StepRef(r, _) => json!(r),
        RawStepTarget::Terminal { outcome } => {
            json!({"kind": "Terminal", "outcome": outcome})
        }
    }
}

fn serialize_failure_handler(handler: &RawFailureHandler) -> Value {
    match handler {
        RawFailureHandler::Terminate { outcome } => {
            json!({"kind": "Terminate", "outcome": outcome})
        }
        RawFailureHandler::Compensate { steps, then } => {
            let steps_arr: Vec<Value> = steps.iter().map(serialize_comp_step).collect();
            json!({
                "kind": "Compensate",
                "steps": steps_arr,
                "then": {"kind": "Terminal", "outcome": then}
            })
        }
    }
}

fn serialize_comp_step(step: &RawCompStep) -> Value {
    json!({
        "on_failure": {"kind": "Terminal", "outcome": step.on_failure},
        "op": step.op,
        "persona": step.persona
    })
}

