//! Pass 6: Interchange JSON serialization -- canonical output with sorted
//! keys and structured numeric values.

use crate::ast::*;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

pub fn serialize(constructs: &[RawConstruct], bundle_id: &str) -> Value {
    let mut fact_types: HashMap<String, RawType> = HashMap::new();
    for c in constructs {
        if let RawConstruct::Fact { id, type_, .. } = c {
            fact_types.insert(id.clone(), type_.clone());
        }
    }

    let mut facts: Vec<&RawConstruct> = Vec::new();
    let mut entities: Vec<&RawConstruct> = Vec::new();
    let mut rules_by_stratum: BTreeMap<i64, Vec<&RawConstruct>> = BTreeMap::new();
    let mut operations: Vec<&RawConstruct> = Vec::new();
    let mut flows: Vec<&RawConstruct> = Vec::new();
    let mut personas: Vec<&RawConstruct> = Vec::new();

    for c in constructs {
        match c {
            RawConstruct::Fact { .. } => facts.push(c),
            RawConstruct::Entity { .. } => entities.push(c),
            RawConstruct::Rule { stratum, .. } => {
                rules_by_stratum.entry(*stratum).or_default().push(c);
            }
            RawConstruct::Operation { .. } => operations.push(c),
            RawConstruct::Flow { .. } => flows.push(c),
            RawConstruct::Persona { .. } => personas.push(c),
            _ => {}
        }
    }

    facts.sort_by(|a, b| construct_id(a).cmp(construct_id(b)));
    entities.sort_by(|a, b| construct_id(a).cmp(construct_id(b)));
    for rules in rules_by_stratum.values_mut() {
        rules.sort_by(|a, b| construct_id(a).cmp(construct_id(b)));
    }
    operations.sort_by(|a, b| construct_id(a).cmp(construct_id(b)));
    flows.sort_by(|a, b| construct_id(a).cmp(construct_id(b)));
    personas.sort_by(|a, b| construct_id(a).cmp(construct_id(b)));

    let mut result: Vec<Value> = Vec::new();
    for c in &facts {
        result.push(serialize_construct(c, &fact_types));
    }
    for c in &entities {
        result.push(serialize_construct(c, &fact_types));
    }
    for c in &personas {
        result.push(serialize_construct(c, &fact_types));
    }
    for rules in rules_by_stratum.values() {
        for c in rules {
            result.push(serialize_construct(c, &fact_types));
        }
    }
    for c in &operations {
        result.push(serialize_construct(c, &fact_types));
    }
    for c in &flows {
        result.push(serialize_construct(c, &fact_types));
    }

    let mut bundle = Map::new();
    bundle.insert("constructs".to_owned(), Value::Array(result));
    bundle.insert("id".to_owned(), Value::String(bundle_id.to_owned()));
    bundle.insert("kind".to_owned(), Value::String("Bundle".to_owned()));
    bundle.insert("tenor".to_owned(), Value::String("1.0".to_owned()));
    bundle.insert(
        "tenor_version".to_owned(),
        Value::String("1.1.0".to_owned()),
    );
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
        RawConstruct::Persona { id, .. } => id,
        RawConstruct::Import { .. } => "",
    }
}

fn serialize_construct(c: &RawConstruct, fact_types: &HashMap<String, RawType>) -> Value {
    match c {
        RawConstruct::Fact {
            id,
            type_,
            source,
            default,
            prov,
        } => {
            let mut m = Map::new();
            if let Some(d) = default {
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
            m.insert("tenor".to_owned(), json!("1.0"));
            m.insert("type".to_owned(), serialize_type(type_));
            Value::Object(m)
        }
        RawConstruct::Entity {
            id,
            states,
            initial,
            transitions,
            parent,
            prov,
            ..
        } => {
            let mut m = Map::new();
            m.insert("id".to_owned(), json!(id));
            m.insert("initial".to_owned(), json!(initial));
            m.insert("kind".to_owned(), json!("Entity"));
            if let Some(p) = parent {
                m.insert("parent".to_owned(), json!(p));
            }
            m.insert("provenance".to_owned(), serialize_prov(prov));
            m.insert("states".to_owned(), json!(states));
            m.insert("tenor".to_owned(), json!("1.0"));
            let t_arr: Vec<Value> = transitions
                .iter()
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
        RawConstruct::Rule {
            id,
            stratum,
            when,
            verdict_type,
            payload_type,
            payload_value,
            prov,
            ..
        } => {
            let mut m = Map::new();
            let mut body = Map::new();
            let mut produce = Map::new();
            produce.insert(
                "payload".to_owned(),
                serialize_payload(payload_type, payload_value, fact_types),
            );
            produce.insert("verdict_type".to_owned(), json!(verdict_type));
            body.insert("produce".to_owned(), Value::Object(produce));
            body.insert("when".to_owned(), serialize_expr(when, fact_types));
            m.insert("body".to_owned(), Value::Object(body));
            m.insert("id".to_owned(), json!(id));
            m.insert("kind".to_owned(), json!("Rule"));
            m.insert("provenance".to_owned(), serialize_prov(prov));
            m.insert("stratum".to_owned(), json!(stratum));
            m.insert("tenor".to_owned(), json!("1.0"));
            Value::Object(m)
        }
        RawConstruct::Operation {
            id,
            allowed_personas,
            precondition,
            effects,
            error_contract,
            outcomes,
            prov,
            ..
        } => {
            let mut m = Map::new();
            m.insert("allowed_personas".to_owned(), json!(allowed_personas));
            let effects_arr: Vec<Value> = effects
                .iter()
                .map(|(eid, f, t, outcome, _)| {
                    let mut em = Map::new();
                    em.insert("entity_id".to_owned(), json!(eid));
                    em.insert("from".to_owned(), json!(f));
                    if let Some(o) = outcome {
                        em.insert("outcome".to_owned(), json!(o));
                    }
                    em.insert("to".to_owned(), json!(t));
                    Value::Object(em)
                })
                .collect();
            m.insert("effects".to_owned(), Value::Array(effects_arr));
            m.insert("error_contract".to_owned(), json!(error_contract));
            m.insert("id".to_owned(), json!(id));
            m.insert("kind".to_owned(), json!("Operation"));
            if !outcomes.is_empty() {
                m.insert("outcomes".to_owned(), json!(outcomes));
            }
            m.insert(
                "precondition".to_owned(),
                serialize_expr(precondition, fact_types),
            );
            m.insert("provenance".to_owned(), serialize_prov(prov));
            m.insert("tenor".to_owned(), json!("1.0"));
            Value::Object(m)
        }
        RawConstruct::Flow {
            id,
            snapshot,
            entry,
            steps,
            prov,
            ..
        } => {
            let mut m = Map::new();
            m.insert("entry".to_owned(), json!(entry));
            m.insert("id".to_owned(), json!(id));
            m.insert("kind".to_owned(), json!("Flow"));
            m.insert("provenance".to_owned(), serialize_prov(prov));
            m.insert("snapshot".to_owned(), json!(snapshot));
            m.insert(
                "steps".to_owned(),
                serialize_steps(steps, entry, fact_types),
            );
            m.insert("tenor".to_owned(), json!("1.0"));
            Value::Object(m)
        }
        RawConstruct::Persona { id, prov } => {
            let mut m = Map::new();
            m.insert("id".to_owned(), json!(id));
            m.insert("kind".to_owned(), json!("Persona"));
            m.insert("provenance".to_owned(), serialize_prov(prov));
            m.insert("tenor".to_owned(), json!("1.0"));
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
        RawType::TypeRef(name) => json!({"base": "TypeRef", "id": name}),
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

fn serialize_payload(
    type_: &RawType,
    value: &RawTerm,
    fact_types: &HashMap<String, RawType>,
) -> Value {
    let mut m = Map::new();
    let effective_type = match (type_, value) {
        (RawType::Text { max_length: 0 }, RawTerm::Literal(RawLiteral::Str(s))) => RawType::Text {
            max_length: s.len() as u32,
        },
        _ => type_.clone(),
    };
    m.insert("type".to_owned(), serialize_type(&effective_type));
    match value {
        RawTerm::Literal(RawLiteral::Bool(b)) => {
            m.insert("value".to_owned(), json!(b));
        }
        RawTerm::Literal(RawLiteral::Int(n)) => {
            m.insert("value".to_owned(), json!(n));
        }
        RawTerm::Literal(RawLiteral::Str(s)) => {
            m.insert("value".to_owned(), json!(s));
        }
        RawTerm::Literal(lit) => {
            m.insert("value".to_owned(), serialize_literal(lit));
        }
        RawTerm::Mul { left, right } => {
            m.insert(
                "value".to_owned(),
                serialize_mul_term(left, right, fact_types),
            );
        }
        _ => {
            m.insert("value".to_owned(), json!(null));
        }
    }
    Value::Object(m)
}

fn serialize_mul_term(
    left: &RawTerm,
    right: &RawTerm,
    fact_types: &HashMap<String, RawType>,
) -> Value {
    let (fact_term, lit_n) = match (left, right) {
        (RawTerm::FactRef(_), RawTerm::Literal(RawLiteral::Int(n))) => (left, *n),
        (RawTerm::Literal(RawLiteral::Int(n)), RawTerm::FactRef(_)) => (right, *n),
        _ => {
            let mut m = Map::new();
            m.insert("left".to_owned(), serialize_term(left));
            m.insert("op".to_owned(), json!("*"));
            m.insert("right".to_owned(), serialize_term(right));
            return Value::Object(m);
        }
    };
    let result_type = if let RawTerm::FactRef(name) = fact_term {
        match fact_types.get(name.as_str()) {
            Some(RawType::Int { min, max }) => {
                let (rmin, rmax) = if lit_n >= 0 {
                    (min * lit_n, max * lit_n)
                } else {
                    (max * lit_n, min * lit_n)
                };
                Some(RawType::Int {
                    min: rmin,
                    max: rmax,
                })
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

fn int_to_decimal_precision(min: i64, max: i64) -> u32 {
    let abs_min = if min < 0 {
        (-(min as i128)) as u64
    } else {
        min as u64
    };
    let abs_max_val = if max < 0 {
        (-(max as i128)) as u64
    } else {
        max as u64
    };
    let abs_max = abs_min.max(abs_max_val) as f64;
    if abs_max == 0.0 {
        return 1;
    }
    (abs_max.log10().ceil() as u32) + 1
}

fn term_numeric_type(term: &RawTerm, fact_types: &HashMap<String, RawType>) -> Option<RawType> {
    match term {
        RawTerm::FactRef(name) => fact_types.get(name.as_str()).cloned(),
        RawTerm::Literal(RawLiteral::Int(n)) => Some(RawType::Int { min: *n, max: *n }),
        RawTerm::Mul { left, right } => {
            let (fact_name, lit_n) = match (left.as_ref(), right.as_ref()) {
                (RawTerm::FactRef(n), RawTerm::Literal(RawLiteral::Int(v))) => {
                    (Some(n.as_str()), Some(*v))
                }
                (RawTerm::Literal(RawLiteral::Int(v)), RawTerm::FactRef(n)) => {
                    (Some(n.as_str()), Some(*v))
                }
                _ => (None, None),
            };
            if let (Some(name), Some(n)) = (fact_name, lit_n) {
                if let Some(RawType::Int { min, max }) = fact_types.get(name) {
                    let (rmin, rmax) = if n >= 0 {
                        (min * n, max * n)
                    } else {
                        (max * n, min * n)
                    };
                    return Some(RawType::Int {
                        min: rmin,
                        max: rmax,
                    });
                }
            }
            None
        }
        _ => None,
    }
}

fn comparison_type_for_compare(
    left: &RawTerm,
    right: &RawTerm,
    fact_types: &HashMap<String, RawType>,
) -> Option<RawType> {
    let lt = term_numeric_type(left, fact_types);
    let rt = term_numeric_type(right, fact_types);
    match (&lt, &rt) {
        (Some(t @ RawType::Money { .. }), _) | (_, Some(t @ RawType::Money { .. })) => {
            Some(t.clone())
        }
        (Some(RawType::Int { min, max }), Some(RawType::Decimal { precision, scale })) => {
            let int_prec = int_to_decimal_precision(*min, *max);
            Some(RawType::Decimal {
                precision: (*precision).max(int_prec) + 1,
                scale: *scale,
            })
        }
        (Some(RawType::Decimal { precision, scale }), Some(RawType::Int { min, max })) => {
            let int_prec = int_to_decimal_precision(*min, *max);
            Some(RawType::Decimal {
                precision: (*precision).max(int_prec) + 1,
                scale: *scale,
            })
        }
        (
            Some(RawType::Int {
                min: lmin,
                max: lmax,
            }),
            Some(RawType::Int {
                min: rmin,
                max: rmax,
            }),
        ) if matches!(left, RawTerm::Mul { .. }) => Some(RawType::Int {
            min: (*lmin).min(*rmin),
            max: (*lmax).max(*rmax),
        }),
        _ => None,
    }
}

fn serialize_term_ctx(term: &RawTerm, fact_types: &HashMap<String, RawType>) -> Value {
    match term {
        RawTerm::Mul { left, right } => serialize_mul_term(left, right, fact_types),
        _ => serialize_term(term),
    }
}

fn serialize_expr(expr: &RawExpr, fact_types: &HashMap<String, RawType>) -> Value {
    match expr {
        RawExpr::Compare {
            op, left, right, ..
        } => {
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
        RawExpr::Forall {
            var, domain, body, ..
        } => {
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
        RawExpr::Exists {
            var, domain, body, ..
        } => {
            let variable_type = match fact_types.get(domain.as_str()) {
                Some(RawType::List { element_type, .. }) => Some(element_type.as_ref().clone()),
                _ => None,
            };
            let mut m = Map::new();
            m.insert("body".to_owned(), serialize_expr(body, fact_types));
            m.insert("domain".to_owned(), json!({"fact_ref": domain}));
            m.insert("quantifier".to_owned(), json!("exists"));
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
        RawTerm::Literal(lit) => match lit {
            RawLiteral::Bool(b) => json!({"literal": b, "type": {"base": "Bool"}}),
            RawLiteral::Int(n) => {
                json!({"literal": n, "type": {"base": "Int", "min": n, "max": n}})
            }
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
        },
        RawTerm::Mul { left, right } => {
            let mut m = Map::new();
            m.insert("left".to_owned(), serialize_term(left));
            m.insert("op".to_owned(), json!("*"));
            m.insert("right".to_owned(), serialize_term(right));
            Value::Object(m)
        }
    }
}

fn serialize_steps(
    steps: &BTreeMap<String, RawStep>,
    entry: &str,
    fact_types: &HashMap<String, RawType>,
) -> Value {
    let order = topological_order(steps, entry);
    let arr: Vec<Value> = order
        .iter()
        .filter_map(|sid| steps.get(sid.as_str()).map(|s| (sid, s)))
        .map(|(sid, step)| serialize_step(sid, step, fact_types))
        .collect();
    Value::Array(arr)
}

fn topological_order(steps: &BTreeMap<String, RawStep>, entry: &str) -> Vec<String> {
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for (sid, step) in steps {
        let mut neighbors: Vec<&str> = Vec::new();
        match step {
            RawStep::OperationStep { outcomes, .. } => {
                for t in outcomes.values() {
                    if let RawStepTarget::StepRef(r, _) = t {
                        neighbors.push(r.as_str());
                    }
                }
            }
            RawStep::BranchStep {
                if_true, if_false, ..
            } => {
                if let RawStepTarget::StepRef(r, _) = if_true {
                    neighbors.push(r.as_str());
                }
                if let RawStepTarget::StepRef(r, _) = if_false {
                    neighbors.push(r.as_str());
                }
            }
            RawStep::HandoffStep { next, .. } => {
                neighbors.push(next.as_str());
            }
            RawStep::SubFlowStep { on_success, .. } => {
                if let RawStepTarget::StepRef(r, _) = on_success {
                    neighbors.push(r.as_str());
                }
            }
            RawStep::ParallelStep { .. } => {}
        }
        adj.insert(sid.as_str(), neighbors);
    }

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
    for sid in steps.keys() {
        if !seen.contains(sid.as_str()) {
            result.push(sid.clone());
        }
    }
    result
}

fn serialize_step(id: &str, step: &RawStep, fact_types: &HashMap<String, RawType>) -> Value {
    match step {
        RawStep::OperationStep {
            op,
            persona,
            outcomes,
            on_failure,
            ..
        } => {
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
        RawStep::BranchStep {
            condition,
            persona,
            if_true,
            if_false,
            ..
        } => {
            let mut m = Map::new();
            m.insert(
                "condition".to_owned(),
                serialize_expr(condition, fact_types),
            );
            m.insert("id".to_owned(), json!(id));
            m.insert("if_false".to_owned(), serialize_step_target(if_false));
            m.insert("if_true".to_owned(), serialize_step_target(if_true));
            m.insert("kind".to_owned(), json!("BranchStep"));
            m.insert("persona".to_owned(), json!(persona));
            Value::Object(m)
        }
        RawStep::HandoffStep {
            from_persona,
            to_persona,
            next,
            ..
        } => {
            let mut m = Map::new();
            m.insert("from_persona".to_owned(), json!(from_persona));
            m.insert("id".to_owned(), json!(id));
            m.insert("kind".to_owned(), json!("HandoffStep"));
            m.insert("next".to_owned(), json!(next));
            m.insert("to_persona".to_owned(), json!(to_persona));
            Value::Object(m)
        }
        RawStep::SubFlowStep {
            flow,
            persona,
            on_success,
            on_failure,
            ..
        } => {
            let mut m = Map::new();
            m.insert("flow".to_owned(), json!(flow));
            m.insert("id".to_owned(), json!(id));
            m.insert("kind".to_owned(), json!("SubFlowStep"));
            m.insert(
                "on_failure".to_owned(),
                serialize_failure_handler(on_failure),
            );
            m.insert("on_success".to_owned(), serialize_step_target(on_success));
            m.insert("persona".to_owned(), json!(persona));
            Value::Object(m)
        }
        RawStep::ParallelStep { branches, join, .. } => {
            let branches_arr: Vec<Value> = branches
                .iter()
                .map(|b| {
                    let mut bm = Map::new();
                    bm.insert("entry".to_owned(), json!(b.entry));
                    bm.insert("id".to_owned(), json!(b.id));
                    bm.insert(
                        "steps".to_owned(),
                        serialize_steps(&b.steps, &b.entry, fact_types),
                    );
                    Value::Object(bm)
                })
                .collect();
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
        RawFailureHandler::Escalate { to_persona, next } => {
            json!({
                "kind": "Escalate",
                "next": next,
                "to_persona": to_persona
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
