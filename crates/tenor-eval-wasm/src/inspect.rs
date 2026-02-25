use tenor_interchange::InterchangeConstruct;

pub fn build_inspect(bundle: &serde_json::Value) -> Result<serde_json::Value, String> {
    let parsed = tenor_interchange::from_interchange(bundle).map_err(|e| e.to_string())?;

    let mut facts = Vec::new();
    let mut entities = Vec::new();
    let mut rules = Vec::new();
    let mut personas = Vec::new();
    let mut operations = Vec::new();
    let mut flows = Vec::new();

    for construct in &parsed.constructs {
        match construct {
            InterchangeConstruct::Fact(f) => {
                let base = f
                    .fact_type
                    .get("base")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                let mut fact_json = serde_json::json!({
                    "id": f.id,
                    "type": base,
                });
                if let Some(ref src) = f.source {
                    fact_json["source"] = src.clone();
                }
                if f.default.is_some() {
                    fact_json["has_default"] = serde_json::json!(true);
                }
                fact_json["type_spec"] = f.fact_type.clone();
                facts.push(fact_json);
            }
            InterchangeConstruct::Entity(e) => {
                let transitions: Vec<serde_json::Value> = e
                    .transitions
                    .iter()
                    .map(|t| serde_json::json!({ "from": t.from, "to": t.to }))
                    .collect();
                entities.push(serde_json::json!({
                    "id": e.id,
                    "states": e.states,
                    "initial": e.initial,
                    "transitions": transitions,
                }));
            }
            InterchangeConstruct::Rule(r) => {
                let condition_summary = r.when().map(summarize_condition).unwrap_or_default();
                let verdict_type = r
                    .produce()
                    .and_then(|p| p.get("verdict_type"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                rules.push(serde_json::json!({
                    "id": r.id,
                    "stratum": r.stratum,
                    "produces": verdict_type,
                    "condition_summary": condition_summary,
                }));
            }
            InterchangeConstruct::Persona(p) => {
                personas.push(serde_json::json!({ "id": p.id }));
            }
            InterchangeConstruct::Operation(op) => {
                let effects: Vec<serde_json::Value> = op
                    .effects
                    .iter()
                    .map(|e| {
                        serde_json::json!({
                            "entity_id": e.entity_id,
                            "from": e.from,
                            "to": e.to,
                        })
                    })
                    .collect();
                let precondition_summary = op
                    .precondition
                    .as_ref()
                    .map(summarize_condition)
                    .unwrap_or_else(|| "none".to_string());
                operations.push(serde_json::json!({
                    "id": op.id,
                    "allowed_personas": op.allowed_personas,
                    "effects": effects,
                    "precondition_summary": precondition_summary,
                    "outcomes": op.outcomes,
                }));
            }
            InterchangeConstruct::Flow(f) => {
                let step_ids: Vec<&str> = f
                    .steps
                    .iter()
                    .filter_map(|s| s.get("id").and_then(|v| v.as_str()))
                    .collect();
                flows.push(serde_json::json!({
                    "id": f.id,
                    "entry": f.entry,
                    "steps": step_ids,
                }));
            }
            InterchangeConstruct::System(_) | InterchangeConstruct::TypeDecl(_) => {}
        }
    }

    Ok(serde_json::json!({
        "facts": facts,
        "entities": entities,
        "rules": rules,
        "personas": personas,
        "operations": operations,
        "flows": flows,
    }))
}

fn summarize_condition(cond: &serde_json::Value) -> String {
    if cond.is_null() {
        return "always".to_string();
    }
    if let Some(vp) = cond.get("verdict_present").and_then(|v| v.as_str()) {
        return format!("verdict '{}' present", vp);
    }
    if let Some(op) = cond.get("op").and_then(|v| v.as_str()) {
        let left = cond
            .get("left")
            .and_then(|l| l.get("fact_ref").and_then(|v| v.as_str()))
            .unwrap_or("...");
        let right = cond
            .get("right")
            .and_then(|r| {
                r.get("literal")
                    .map(|l| format!("{}", l))
                    .or_else(|| r.get("fact_ref").and_then(|v| v.as_str()).map(String::from))
            })
            .unwrap_or_else(|| "...".to_string());
        return format!("{} {} {}", left, op, right);
    }
    "complex".to_string()
}
