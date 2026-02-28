//! Pass 6: Interchange JSON serialization -- canonical output with sorted
//! keys and structured numeric values.

use crate::ast::*;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

// Static key constants to avoid repeated heap allocations for the most
// frequently used JSON keys across construct serialization.
const K_BASE: &str = "base";
const K_ID: &str = "id";
const K_KIND: &str = "kind";
const K_OP: &str = "op";
const K_PROVENANCE: &str = "provenance";
const K_TENOR: &str = "tenor";
const K_VALUE: &str = "value";

/// Insert a key-value pair into a JSON map, allocating the key from a `&str`.
#[inline]
fn ins(m: &mut Map<String, Value>, key: &str, val: Value) {
    m.insert(key.to_owned(), val);
}

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
    let mut systems: Vec<&RawConstruct> = Vec::new();
    let mut sources: Vec<&RawConstruct> = Vec::new();

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
            RawConstruct::System { .. } => systems.push(c),
            RawConstruct::Source { .. } => sources.push(c),
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
    systems.sort_by(|a, b| construct_id(a).cmp(construct_id(b)));
    sources.sort_by(|a, b| construct_id(a).cmp(construct_id(b)));

    let mut result: Vec<Value> = Vec::new();
    for c in &personas {
        result.push(serialize_construct(c, &fact_types));
    }
    for c in &sources {
        result.push(serialize_construct(c, &fact_types));
    }
    for c in &facts {
        result.push(serialize_construct(c, &fact_types));
    }
    for c in &entities {
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
    for c in &systems {
        result.push(serialize_construct(c, &fact_types));
    }

    let mut bundle = Map::new();
    ins(&mut bundle, "constructs", Value::Array(result));
    ins(&mut bundle, K_ID, Value::String(bundle_id.to_owned()));
    ins(&mut bundle, K_KIND, Value::String("Bundle".to_owned()));
    ins(
        &mut bundle,
        K_TENOR,
        Value::String(crate::TENOR_VERSION.to_owned()),
    );
    ins(
        &mut bundle,
        "tenor_version",
        Value::String(crate::TENOR_BUNDLE_VERSION.to_owned()),
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
        RawConstruct::System { id, .. } => id,
        RawConstruct::Source { id, .. } => id,
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
                        let rounded = round_decimal_to_scale(s, *scale);
                        let mut dm = Map::new();
                        ins(&mut dm, K_KIND, json!("decimal_value"));
                        ins(&mut dm, "precision", json!(precision));
                        ins(&mut dm, "scale", json!(scale));
                        ins(&mut dm, K_VALUE, json!(rounded));
                        Value::Object(dm)
                    }
                    (RawType::Decimal { precision, scale }, RawLiteral::Float(s)) => {
                        let rounded = round_decimal_to_scale(s, *scale);
                        let mut dm = Map::new();
                        ins(&mut dm, K_KIND, json!("decimal_value"));
                        ins(&mut dm, "precision", json!(precision));
                        ins(&mut dm, "scale", json!(scale));
                        ins(&mut dm, K_VALUE, json!(rounded));
                        Value::Object(dm)
                    }
                    (RawType::Money { .. }, RawLiteral::Money { amount, currency }) => {
                        let (p, sc) = money_decimal_precision_scale(amount);
                        let rounded = round_decimal_to_scale(amount, sc);
                        let mut amount_m = Map::new();
                        ins(&mut amount_m, K_KIND, json!("decimal_value"));
                        ins(&mut amount_m, "precision", json!(p));
                        ins(&mut amount_m, "scale", json!(sc));
                        ins(&mut amount_m, K_VALUE, json!(rounded));
                        let mut m = Map::new();
                        ins(&mut m, "amount", Value::Object(amount_m));
                        ins(&mut m, "currency", json!(currency));
                        ins(&mut m, K_KIND, json!("money_value"));
                        Value::Object(m)
                    }
                    _ => serialize_literal(d),
                };
                ins(&mut m, "default", default_val);
            }
            ins(&mut m, K_ID, json!(id));
            ins(&mut m, K_KIND, json!("Fact"));
            ins(&mut m, K_PROVENANCE, serialize_prov(prov));
            ins(&mut m, "source", serialize_source(source));
            ins(&mut m, K_TENOR, json!(crate::TENOR_VERSION));
            ins(&mut m, "type", serialize_type(type_));
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
            ins(&mut m, K_ID, json!(id));
            ins(&mut m, "initial", json!(initial));
            ins(&mut m, K_KIND, json!("Entity"));
            if let Some(p) = parent {
                ins(&mut m, "parent", json!(p));
            }
            ins(&mut m, K_PROVENANCE, serialize_prov(prov));
            ins(&mut m, "states", json!(states));
            ins(&mut m, K_TENOR, json!(crate::TENOR_VERSION));
            let t_arr: Vec<Value> = transitions
                .iter()
                .map(|(f, to, _)| {
                    let mut tm = Map::new();
                    ins(&mut tm, "from", json!(f));
                    ins(&mut tm, "to", json!(to));
                    Value::Object(tm)
                })
                .collect();
            ins(&mut m, "transitions", Value::Array(t_arr));
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
            ins(
                &mut produce,
                "payload",
                serialize_payload(payload_type, payload_value, fact_types),
            );
            ins(&mut produce, "verdict_type", json!(verdict_type));
            ins(&mut body, "produce", Value::Object(produce));
            ins(&mut body, "when", serialize_expr(when, fact_types));
            ins(&mut m, "body", Value::Object(body));
            ins(&mut m, K_ID, json!(id));
            ins(&mut m, K_KIND, json!("Rule"));
            ins(&mut m, K_PROVENANCE, serialize_prov(prov));
            ins(&mut m, "stratum", json!(stratum));
            ins(&mut m, K_TENOR, json!(crate::TENOR_VERSION));
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
            ins(&mut m, "allowed_personas", json!(allowed_personas));
            let effects_arr: Vec<Value> = effects
                .iter()
                .map(|(eid, f, t, outcome, _)| {
                    let mut em = Map::new();
                    ins(&mut em, "entity_id", json!(eid));
                    ins(&mut em, "from", json!(f));
                    if let Some(o) = outcome {
                        ins(&mut em, "outcome", json!(o));
                    }
                    ins(&mut em, "to", json!(t));
                    Value::Object(em)
                })
                .collect();
            ins(&mut m, "effects", Value::Array(effects_arr));
            ins(&mut m, "error_contract", json!(error_contract));
            ins(&mut m, K_ID, json!(id));
            ins(&mut m, K_KIND, json!("Operation"));
            if !outcomes.is_empty() {
                ins(&mut m, "outcomes", json!(outcomes));
            }
            ins(
                &mut m,
                "precondition",
                serialize_expr(precondition, fact_types),
            );
            ins(&mut m, K_PROVENANCE, serialize_prov(prov));
            ins(&mut m, K_TENOR, json!(crate::TENOR_VERSION));
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
            ins(&mut m, "entry", json!(entry));
            ins(&mut m, K_ID, json!(id));
            ins(&mut m, K_KIND, json!("Flow"));
            ins(&mut m, K_PROVENANCE, serialize_prov(prov));
            ins(&mut m, "snapshot", json!(snapshot));
            ins(&mut m, "steps", serialize_steps(steps, entry, fact_types));
            ins(&mut m, K_TENOR, json!(crate::TENOR_VERSION));
            Value::Object(m)
        }
        RawConstruct::Persona { id, prov } => {
            let mut m = Map::new();
            ins(&mut m, K_ID, json!(id));
            ins(&mut m, K_KIND, json!("Persona"));
            ins(&mut m, K_PROVENANCE, serialize_prov(prov));
            ins(&mut m, K_TENOR, json!(crate::TENOR_VERSION));
            Value::Object(m)
        }
        RawConstruct::Source {
            id,
            protocol,
            fields,
            description,
            prov,
        } => {
            let mut m = Map::new();
            if let Some(desc) = description {
                ins(&mut m, "description", json!(desc));
            }
            let mut fm = Map::new();
            for (k, v) in fields {
                fm.insert(k.clone(), json!(v));
            }
            ins(&mut m, "fields", Value::Object(fm));
            ins(&mut m, K_ID, json!(id));
            ins(&mut m, K_KIND, json!("Source"));
            ins(&mut m, "protocol", json!(protocol));
            ins(&mut m, K_PROVENANCE, serialize_prov(prov));
            ins(&mut m, K_TENOR, json!(crate::TENOR_VERSION));
            Value::Object(m)
        }
        RawConstruct::System {
            id,
            members,
            shared_personas,
            triggers,
            shared_entities,
            prov,
        } => serialize_system(
            id,
            members,
            shared_personas,
            triggers,
            shared_entities,
            prov,
        ),
        _ => json!(null),
    }
}

fn serialize_prov(prov: &Provenance) -> Value {
    let mut m = Map::new();
    ins(&mut m, "file", json!(prov.file));
    ins(&mut m, "line", json!(prov.line));
    Value::Object(m)
}

fn serialize_source(source: &RawSourceDecl) -> Value {
    match source {
        RawSourceDecl::Freetext(s) => {
            // Legacy freetext: split on first dot for backward compat
            if let Some(dot) = s.find('.') {
                let system = &s[..dot];
                let field = &s[dot + 1..];
                let mut m = Map::new();
                ins(&mut m, "field", json!(field));
                ins(&mut m, "system", json!(system));
                Value::Object(m)
            } else {
                json!(s)
            }
        }
        RawSourceDecl::Structured { source_id, path } => {
            let mut m = Map::new();
            ins(&mut m, "path", json!(path));
            ins(&mut m, "source_id", json!(source_id));
            Value::Object(m)
        }
    }
}

fn serialize_type(t: &RawType) -> Value {
    match t {
        RawType::Bool => json!({"base": "Bool"}),
        RawType::Date => json!({"base": "Date"}),
        RawType::DateTime => json!({"base": "DateTime"}),
        RawType::Int { min, max } => {
            let mut m = Map::new();
            ins(&mut m, K_BASE, json!("Int"));
            ins(&mut m, "max", json!(max));
            ins(&mut m, "min", json!(min));
            Value::Object(m)
        }
        RawType::Decimal { precision, scale } => {
            let mut m = Map::new();
            ins(&mut m, K_BASE, json!("Decimal"));
            ins(&mut m, "precision", json!(precision));
            ins(&mut m, "scale", json!(scale));
            Value::Object(m)
        }
        RawType::Text { max_length } => {
            let mut m = Map::new();
            ins(&mut m, K_BASE, json!("Text"));
            ins(&mut m, "max_length", json!(max_length));
            Value::Object(m)
        }
        RawType::Enum { values } => json!({"base": "Enum", "values": values}),
        RawType::Money { currency } => {
            let mut m = Map::new();
            ins(&mut m, K_BASE, json!("Money"));
            ins(&mut m, "currency", json!(currency));
            Value::Object(m)
        }
        RawType::Duration { unit, min, max } => {
            let mut m = Map::new();
            ins(&mut m, K_BASE, json!("Duration"));
            ins(&mut m, "max", json!(max));
            ins(&mut m, "min", json!(min));
            ins(&mut m, "unit", json!(unit));
            Value::Object(m)
        }
        RawType::Record { fields } => {
            let mut fm = Map::new();
            for (k, v) in fields {
                fm.insert(k.clone(), serialize_type(v));
            }
            let mut m = Map::new();
            ins(&mut m, K_BASE, json!("Record"));
            ins(&mut m, "fields", Value::Object(fm));
            Value::Object(m)
        }
        RawType::List { element_type, max } => {
            let mut m = Map::new();
            ins(&mut m, K_BASE, json!("List"));
            ins(&mut m, "element_type", serialize_type(element_type));
            ins(&mut m, "max", json!(max));
            Value::Object(m)
        }
        RawType::TaggedUnion { variants } => {
            let mut vm = Map::new();
            for (tag, payload_type) in variants {
                vm.insert(tag.clone(), serialize_type(payload_type));
            }
            let mut m = Map::new();
            ins(&mut m, K_BASE, json!("TaggedUnion"));
            ins(&mut m, "variants", Value::Object(vm));
            Value::Object(m)
        }
        RawType::TypeRef(name) => json!({"base": "TypeRef", "id": name}),
    }
}

fn serialize_literal(lit: &RawLiteral) -> Value {
    match lit {
        RawLiteral::Bool(b) => {
            let mut m = Map::new();
            ins(&mut m, K_KIND, json!("bool_literal"));
            ins(&mut m, K_VALUE, json!(b));
            Value::Object(m)
        }
        RawLiteral::Int(n) => {
            let mut m = Map::new();
            ins(&mut m, K_KIND, json!("int_literal"));
            ins(&mut m, K_VALUE, json!(n));
            Value::Object(m)
        }
        RawLiteral::Float(f) => {
            let (precision, scale) = decimal_precision_scale(f);
            let mut m = Map::new();
            ins(&mut m, K_KIND, json!("decimal_value"));
            ins(&mut m, "precision", json!(precision));
            ins(&mut m, "scale", json!(scale));
            ins(&mut m, K_VALUE, json!(f));
            Value::Object(m)
        }
        RawLiteral::Str(s) => json!(s),
        RawLiteral::Money { amount, currency } => {
            let (precision, scale) = money_decimal_precision_scale(amount);
            let mut amount_m = Map::new();
            ins(&mut amount_m, K_KIND, json!("decimal_value"));
            ins(&mut amount_m, "precision", json!(precision));
            ins(&mut amount_m, "scale", json!(scale));
            ins(&mut amount_m, K_VALUE, json!(amount));
            let mut m = Map::new();
            ins(&mut m, "amount", Value::Object(amount_m));
            ins(&mut m, "currency", json!(currency));
            ins(&mut m, K_KIND, json!("money_value"));
            Value::Object(m)
        }
    }
}

fn money_decimal_precision_scale(_amount: &str) -> (u32, u32) {
    (10, 2)
}

/// Round a decimal string to `target_scale` fractional digits using
/// round-half-to-even (banker's rounding, per spec §13).
fn round_decimal_to_scale(s: &str, target_scale: u32) -> String {
    let is_negative = s.starts_with('-');
    let abs_s = if is_negative { &s[1..] } else { s };
    let (integer_part, frac_part) = if let Some(dot) = abs_s.find('.') {
        (&abs_s[..dot], &abs_s[dot + 1..])
    } else {
        (abs_s, "")
    };
    let ts = target_scale as usize;

    if frac_part.len() <= ts {
        // Pad with zeros to reach target scale
        let padded = if ts == 0 {
            integer_part.to_string()
        } else {
            format!("{}.{:0<width$}", integer_part, frac_part, width = ts)
        };
        return if is_negative {
            format!("-{}", padded)
        } else {
            padded
        };
    }

    // Need to round — frac_part.len() > ts
    let kept = &frac_part[..ts];
    let rest = &frac_part[ts..];
    let first_removed = rest.as_bytes()[0] - b'0';

    let round_up = if first_removed < 5 {
        false
    } else if first_removed > 5 {
        true
    } else {
        // Exactly 5 — check for trailing non-zero digits
        let has_trailing = rest[1..].bytes().any(|b| b != b'0');
        if has_trailing {
            true
        } else {
            // Tie: round to even (the last kept digit)
            let last_kept_digit = if ts > 0 {
                kept.as_bytes()[ts - 1] - b'0'
            } else {
                integer_part
                    .as_bytes()
                    .last()
                    .map(|b| b - b'0')
                    .unwrap_or(0)
            };
            last_kept_digit % 2 != 0
        }
    };

    if !round_up {
        let result = if ts > 0 {
            format!("{}.{}", integer_part, kept)
        } else {
            integer_part.to_string()
        };
        return if is_negative {
            format!("-{}", result)
        } else {
            result
        };
    }

    // Increment: collect integer + kept-fraction digits, add 1 from the end
    let mut digits: Vec<u8> = integer_part
        .bytes()
        .chain(kept.bytes())
        .map(|b| b - b'0')
        .collect();
    let mut carry = 1u8;
    for d in digits.iter_mut().rev() {
        let sum = *d + carry;
        *d = sum % 10;
        carry = sum / 10;
        if carry == 0 {
            break;
        }
    }

    let int_len = integer_part.len();
    let mut r = String::new();
    if is_negative {
        r.push('-');
    }
    if carry > 0 {
        r.push('1');
    }
    for &d in &digits[..int_len] {
        r.push((b'0' + d) as char);
    }
    if ts > 0 {
        r.push('.');
        for &d in &digits[int_len..] {
            r.push((b'0' + d) as char);
        }
    }
    r
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
    ins(&mut m, "type", serialize_type(&effective_type));
    match value {
        RawTerm::Literal(RawLiteral::Bool(b)) => {
            ins(&mut m, K_VALUE, json!(b));
        }
        RawTerm::Literal(RawLiteral::Int(n)) => {
            ins(&mut m, K_VALUE, json!(n));
        }
        RawTerm::Literal(RawLiteral::Str(s)) => {
            ins(&mut m, K_VALUE, json!(s));
        }
        RawTerm::Literal(lit) => {
            ins(&mut m, K_VALUE, serialize_literal(lit));
        }
        RawTerm::Mul { left, right } => {
            ins(&mut m, K_VALUE, serialize_mul_term(left, right, fact_types));
        }
        _ => {
            ins(&mut m, K_VALUE, json!(null));
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
            ins(&mut m, "left", serialize_term(left));
            ins(&mut m, K_OP, json!("*"));
            ins(&mut m, "right", serialize_term(right));
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
    ins(&mut m, "left", serialize_term(fact_term));
    ins(&mut m, "literal", json!(lit_n));
    ins(&mut m, K_OP, json!("*"));
    if let Some(rt) = result_type {
        ins(&mut m, "result_type", serialize_type(&rt));
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
                ins(&mut m, "comparison_type", serialize_type(&ct));
            }
            ins(&mut m, "left", serialize_term_ctx(left, fact_types));
            ins(&mut m, K_OP, json!(op));
            let right_val = match (right, &left_fact_type) {
                (RawTerm::Literal(RawLiteral::Str(s)), Some(t @ RawType::Enum { .. })) => {
                    json!({"literal": s, "type": serialize_type(t)})
                }
                _ => serialize_term_ctx(right, fact_types),
            };
            ins(&mut m, "right", right_val);
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
            ins(&mut m, "body", serialize_expr(body, fact_types));
            ins(&mut m, "domain", json!({"fact_ref": domain}));
            ins(&mut m, "quantifier", json!("forall"));
            ins(&mut m, "variable", json!(var));
            if let Some(vt) = variable_type {
                ins(&mut m, "variable_type", serialize_type(&vt));
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
            ins(&mut m, "body", serialize_expr(body, fact_types));
            ins(&mut m, "domain", json!({"fact_ref": domain}));
            ins(&mut m, "quantifier", json!("exists"));
            ins(&mut m, "variable", json!(var));
            if let Some(vt) = variable_type {
                ins(&mut m, "variable_type", serialize_type(&vt));
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
            ins(&mut m, "left", serialize_term(left));
            ins(&mut m, K_OP, json!("*"));
            ins(&mut m, "right", serialize_term(right));
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
            ins(&mut m, K_ID, json!(id));
            ins(&mut m, K_KIND, json!("OperationStep"));
            if let Some(h) = on_failure {
                ins(&mut m, "on_failure", serialize_failure_handler(h));
            }
            ins(&mut m, K_OP, json!(op));
            let mut out_m = Map::new();
            for (label, target) in outcomes {
                out_m.insert(label.clone(), serialize_step_target(target));
            }
            ins(&mut m, "outcomes", Value::Object(out_m));
            ins(&mut m, "persona", json!(persona));
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
            ins(&mut m, "condition", serialize_expr(condition, fact_types));
            ins(&mut m, K_ID, json!(id));
            ins(&mut m, "if_false", serialize_step_target(if_false));
            ins(&mut m, "if_true", serialize_step_target(if_true));
            ins(&mut m, K_KIND, json!("BranchStep"));
            ins(&mut m, "persona", json!(persona));
            Value::Object(m)
        }
        RawStep::HandoffStep {
            from_persona,
            to_persona,
            next,
            ..
        } => {
            let mut m = Map::new();
            ins(&mut m, "from_persona", json!(from_persona));
            ins(&mut m, K_ID, json!(id));
            ins(&mut m, K_KIND, json!("HandoffStep"));
            ins(&mut m, "next", json!(next));
            ins(&mut m, "to_persona", json!(to_persona));
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
            ins(&mut m, "flow", json!(flow));
            ins(&mut m, K_ID, json!(id));
            ins(&mut m, K_KIND, json!("SubFlowStep"));
            ins(&mut m, "on_failure", serialize_failure_handler(on_failure));
            ins(&mut m, "on_success", serialize_step_target(on_success));
            ins(&mut m, "persona", json!(persona));
            Value::Object(m)
        }
        RawStep::ParallelStep { branches, join, .. } => {
            let branches_arr: Vec<Value> = branches
                .iter()
                .map(|b| {
                    let mut bm = Map::new();
                    ins(&mut bm, "entry", json!(b.entry));
                    ins(&mut bm, K_ID, json!(b.id));
                    ins(
                        &mut bm,
                        "steps",
                        serialize_steps(&b.steps, &b.entry, fact_types),
                    );
                    Value::Object(bm)
                })
                .collect();
            let mut join_m = Map::new();
            if let Some(t) = &join.on_all_success {
                ins(&mut join_m, "on_all_success", serialize_step_target(t));
            }
            if let Some(h) = &join.on_any_failure {
                ins(&mut join_m, "on_any_failure", serialize_failure_handler(h));
            }
            if let Some(t) = &join.on_all_complete {
                ins(&mut join_m, "on_all_complete", serialize_step_target(t));
            }
            let mut m = Map::new();
            ins(&mut m, "branches", Value::Array(branches_arr));
            ins(&mut m, K_ID, json!(id));
            ins(&mut m, "join", Value::Object(join_m));
            ins(&mut m, K_KIND, json!("ParallelStep"));
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

// ── System serialization ─────────────────────────────────────────────────────

/// Serialize a System construct to canonical interchange JSON per Section 12.5
/// of the Tenor specification. All keys are lexicographically sorted within each object.
fn serialize_system(
    id: &str,
    members: &[(String, String)],
    shared_personas: &[(String, Vec<String>)],
    triggers: &[RawTrigger],
    shared_entities: &[(String, Vec<String>)],
    prov: &Provenance,
) -> Value {
    let mut m = Map::new();

    // flow_triggers -- empty array if none (omitted if empty per spec example)
    // Per spec Section 12.5, triggers is always present as a field

    // id
    ins(&mut m, K_ID, json!(id));

    // kind
    ins(&mut m, K_KIND, json!("System"));

    // members: sorted by member id (lex order for canonical output)
    let mut sorted_members: Vec<(&String, &String)> =
        members.iter().map(|(mid, path)| (mid, path)).collect();
    sorted_members.sort_by_key(|(mid, _)| mid.as_str());
    let members_arr: Vec<Value> = sorted_members
        .iter()
        .map(|(mid, path)| {
            let mut mm = Map::new();
            ins(&mut mm, K_ID, json!(mid));
            ins(&mut mm, "path", json!(path));
            Value::Object(mm)
        })
        .collect();
    ins(&mut m, "members", Value::Array(members_arr));

    // provenance
    ins(&mut m, K_PROVENANCE, serialize_prov(prov));

    // shared_entities: sorted by entity id, contracts sorted within each entry
    let mut sorted_entities: Vec<(&String, &Vec<String>)> =
        shared_entities.iter().map(|(eid, cs)| (eid, cs)).collect();
    sorted_entities.sort_by_key(|(eid, _)| eid.as_str());
    let entities_arr: Vec<Value> = sorted_entities
        .iter()
        .map(|(eid, contracts)| {
            let mut sorted_cs: Vec<&str> = contracts.iter().map(String::as_str).collect();
            sorted_cs.sort_unstable();
            let mut em = Map::new();
            ins(&mut em, "contracts", json!(sorted_cs));
            ins(&mut em, "entity", json!(eid));
            Value::Object(em)
        })
        .collect();
    ins(&mut m, "shared_entities", Value::Array(entities_arr));

    // shared_personas: sorted by persona id, contracts sorted within each entry
    let mut sorted_personas: Vec<(&String, &Vec<String>)> =
        shared_personas.iter().map(|(pid, cs)| (pid, cs)).collect();
    sorted_personas.sort_by_key(|(pid, _)| pid.as_str());
    let personas_arr: Vec<Value> = sorted_personas
        .iter()
        .map(|(pid, contracts)| {
            let mut sorted_cs: Vec<&str> = contracts.iter().map(String::as_str).collect();
            sorted_cs.sort_unstable();
            let mut pm = Map::new();
            ins(&mut pm, "contracts", json!(sorted_cs));
            ins(&mut pm, "persona", json!(pid));
            Value::Object(pm)
        })
        .collect();
    ins(&mut m, "shared_personas", Value::Array(personas_arr));

    // tenor
    ins(&mut m, K_TENOR, json!(crate::TENOR_VERSION));

    // triggers: sorted by (source_contract, source_flow, target_contract, target_flow)
    let mut sorted_triggers: Vec<&RawTrigger> = triggers.iter().collect();
    sorted_triggers.sort_by(|a, b| {
        a.source_contract
            .cmp(&b.source_contract)
            .then_with(|| a.source_flow.cmp(&b.source_flow))
            .then_with(|| a.target_contract.cmp(&b.target_contract))
            .then_with(|| a.target_flow.cmp(&b.target_flow))
    });
    let triggers_arr: Vec<Value> = sorted_triggers
        .iter()
        .map(|t| {
            let mut tm = Map::new();
            ins(&mut tm, "on", json!(t.on));
            ins(&mut tm, "persona", json!(t.persona));
            ins(&mut tm, "source_contract", json!(t.source_contract));
            ins(&mut tm, "source_flow", json!(t.source_flow));
            ins(&mut tm, "target_contract", json!(t.target_contract));
            ins(&mut tm, "target_flow", json!(t.target_flow));
            Value::Object(tm)
        })
        .collect();
    ins(&mut m, "triggers", Value::Array(triggers_arr));

    Value::Object(m)
}
