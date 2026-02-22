//! Pass 4: Type resolution and expression type-checking.
//!
//! Resolves TypeRef nodes in all constructs, then type-checks rule
//! predicate expressions and produce clauses.

use crate::ast::*;
use crate::error::ElabError;
use crate::pass3_types::TypeEnv;
use std::collections::{BTreeMap, HashMap, HashSet};

// ──────────────────────────────────────────────────────────────────────────────
// Pass 4a: resolve TypeRef nodes throughout all constructs
// ──────────────────────────────────────────────────────────────────────────────

pub fn resolve_types(
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
        RawConstruct::Fact {
            id,
            type_,
            source,
            default,
            prov,
        } => {
            let t = resolve_raw_type(&type_, env, &prov.file, prov.line)?;
            Ok(RawConstruct::Fact {
                id,
                type_: t,
                source,
                default,
                prov,
            })
        }
        RawConstruct::Rule {
            id,
            stratum,
            stratum_line,
            when,
            verdict_type,
            payload_type,
            payload_value,
            produce_line,
            prov,
        } => {
            let pt = resolve_raw_type(&payload_type, env, &prov.file, prov.line)?;
            Ok(RawConstruct::Rule {
                id,
                stratum,
                stratum_line,
                when,
                verdict_type,
                payload_type: pt,
                payload_value,
                produce_line,
                prov,
            })
        }
        other => Ok(other),
    }
}

fn resolve_raw_type(
    t: &RawType,
    env: &TypeEnv,
    file: &str,
    line: u32,
) -> Result<RawType, ElabError> {
    match t {
        RawType::TypeRef(name) => env.get(name.as_str()).cloned().ok_or_else(|| {
            ElabError::new(
                4,
                None,
                None,
                Some("type"),
                file,
                line,
                format!("unknown type reference '{}'", name),
            )
        }),
        RawType::Record { fields } => {
            let mut resolved = BTreeMap::new();
            for (k, v) in fields {
                resolved.insert(k.clone(), resolve_raw_type(v, env, file, line)?);
            }
            Ok(RawType::Record { fields: resolved })
        }
        RawType::List { element_type, max } => {
            let et = resolve_raw_type(element_type, env, file, line)?;
            Ok(RawType::List {
                element_type: Box::new(et),
                max: *max,
            })
        }
        other => Ok(other.clone()),
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Pass 4b: expression type-checking (unresolved refs + type errors)
// ──────────────────────────────────────────────────────────────────────────────

pub fn type_check_rules(constructs: &[RawConstruct]) -> Result<(), ElabError> {
    let mut fact_types: HashMap<&str, &RawType> = HashMap::new();
    for c in constructs {
        if let RawConstruct::Fact { id, type_, .. } = c {
            fact_types.insert(id.as_str(), type_);
        }
    }
    for c in constructs {
        if let RawConstruct::Rule {
            id,
            when,
            payload_type,
            payload_value,
            produce_line,
            prov,
            ..
        } = c
        {
            type_check_expr(id, when, &fact_types, &HashSet::new(), prov)?;
            type_check_produce(
                id,
                payload_type,
                payload_value,
                *produce_line,
                &fact_types,
                prov,
            )?;
        }
    }
    Ok(())
}

fn is_var_fact_ref(
    term: &RawTerm,
    fact_types: &HashMap<&str, &RawType>,
    bound_vars: &HashSet<&str>,
) -> bool {
    matches!(term, RawTerm::FactRef(n) if !bound_vars.contains(n.as_str()) && fact_types.contains_key(n.as_str()))
}

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
            // SAFETY: products has exactly 4 elements (all range endpoint products)
            let prod_min = *products.iter().min().unwrap();
            let prod_max = *products.iter().max().unwrap();
            if let RawType::Int {
                min: pt_min,
                max: pt_max,
            } = payload_type
            {
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

pub fn type_name(t: &RawType) -> String {
    match t {
        RawType::Bool => "Bool".to_owned(),
        RawType::Int { min, max } => format!("Int(min: {}, max: {})", min, max),
        RawType::Decimal { .. } => "Decimal".to_owned(),
        RawType::Text { .. } => "Text".to_owned(),
        RawType::Enum { .. } => "Enum".to_owned(),
        RawType::Money { currency } => format!("Money(currency: {})", currency),
        RawType::Date => "Date".to_owned(),
        RawType::DateTime => "DateTime".to_owned(),
        RawType::Duration { .. } => "Duration".to_owned(),
        RawType::List { .. } => "List".to_owned(),
        RawType::Record { .. } => "Record".to_owned(),
        RawType::TypeRef(n) => n.clone(),
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
        RawExpr::Compare {
            op,
            left,
            right,
            line,
        } => {
            for term in &[left, right] {
                if let RawTerm::Mul {
                    left: ml,
                    right: mr,
                } = term
                {
                    if is_var_fact_ref(ml, fact_types, bound_vars)
                        && is_var_fact_ref(mr, fact_types, bound_vars)
                    {
                        return Err(ElabError::new(
                            4, Some("Rule"), Some(rule_id), Some("body.when"),
                            &prov.file, *line,
                            "type error: variable \u{00d7} variable multiplication is not permitted in PredicateExpression; only variable \u{00d7} literal_numeric is allowed".to_string(),
                        ));
                    }
                }
            }
            for term in &[left, right] {
                if let RawTerm::FactRef(name) = term {
                    if !bound_vars.contains(name.as_str())
                        && !fact_types.contains_key(name.as_str())
                    {
                        return Err(ElabError::new(
                            4,
                            Some("Rule"),
                            Some(rule_id),
                            Some("body.when"),
                            &prov.file,
                            *line,
                            format!(
                                "unresolved fact reference: '{}' is not declared in this contract",
                                name
                            ),
                        ));
                    }
                }
            }
            if let Some(lt) = type_of_fact_term(left, fact_types, bound_vars) {
                match lt {
                    RawType::Bool if op != "=" && op != "!=" => {
                        return Err(ElabError::new(
                            4, Some("Rule"), Some(rule_id), Some("body.when"),
                            &prov.file, *line,
                            format!("type error: operator '{}' not defined for Bool; Bool supports only = and \u{2260}", op),
                        ));
                    }
                    RawType::Money { currency: lc } => {
                        if let Some(RawType::Money { currency: rc }) =
                            type_of_fact_term(right, fact_types, bound_vars)
                        {
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
        RawExpr::Forall {
            var,
            domain,
            body,
            line,
        } => {
            if !fact_types.contains_key(domain.as_str()) {
                return Err(ElabError::new(
                    4,
                    Some("Rule"),
                    Some(rule_id),
                    Some("body.when"),
                    &prov.file,
                    *line,
                    format!(
                        "unresolved fact reference: '{}' is not declared in this contract",
                        domain
                    ),
                ));
            }
            let domain_type = fact_types[domain.as_str()];
            if !matches!(domain_type, RawType::List { .. }) {
                return Err(ElabError::new(
                    4,
                    Some("Rule"),
                    Some(rule_id),
                    Some("body.when"),
                    &prov.file,
                    *line,
                    format!(
                        "type error: quantifier domain '{}' has type {}; domain must be List-typed",
                        domain,
                        type_name(domain_type)
                    ),
                ));
            }
            let mut inner_bound = bound_vars.clone();
            inner_bound.insert(var.as_str());
            type_check_expr(rule_id, body, fact_types, &inner_bound, prov)?;
        }
        RawExpr::Exists {
            var,
            domain,
            body,
            line,
        } => {
            if !fact_types.contains_key(domain.as_str()) {
                return Err(ElabError::new(
                    4,
                    Some("Rule"),
                    Some(rule_id),
                    Some("body.when"),
                    &prov.file,
                    *line,
                    format!(
                        "unresolved fact reference: '{}' is not declared in this contract",
                        domain
                    ),
                ));
            }
            let domain_type = fact_types[domain.as_str()];
            if !matches!(domain_type, RawType::List { .. }) {
                return Err(ElabError::new(
                    4,
                    Some("Rule"),
                    Some(rule_id),
                    Some("body.when"),
                    &prov.file,
                    *line,
                    format!(
                        "type error: quantifier domain '{}' has type {}; domain must be List-typed",
                        domain,
                        type_name(domain_type)
                    ),
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
