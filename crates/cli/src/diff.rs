use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt;

/// Error type for diff operations.
#[derive(Debug)]
pub enum DiffError {
    /// The bundle is missing the `constructs` array or a construct is missing `kind`/`id`.
    InvalidBundle(String),
}

impl fmt::Display for DiffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiffError::InvalidBundle(msg) => write!(f, "invalid bundle: {}", msg),
        }
    }
}

/// Summary of a construct (kind + id) for added/removed entries.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConstructSummary {
    pub kind: String,
    pub id: String,
}

/// A changed construct with field-level differences.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstructChange {
    pub kind: String,
    pub id: String,
    pub fields: Vec<FieldDiff>,
}

/// A single field-level difference.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldDiff {
    pub field: String,
    pub before: Value,
    pub after: Value,
}

/// The result of diffing two interchange bundles.
#[derive(Debug, Clone, PartialEq)]
pub struct BundleDiff {
    pub added: Vec<ConstructSummary>,
    pub removed: Vec<ConstructSummary>,
    pub changed: Vec<ConstructChange>,
}

impl BundleDiff {
    /// Returns true if there are no differences.
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.changed.is_empty()
    }

    /// Serialize the diff to a JSON value.
    pub fn to_json(&self) -> Value {
        let added: Vec<Value> = self
            .added
            .iter()
            .map(|s| {
                serde_json::json!({
                    "id": s.id,
                    "kind": s.kind,
                })
            })
            .collect();

        let removed: Vec<Value> = self
            .removed
            .iter()
            .map(|s| {
                serde_json::json!({
                    "id": s.id,
                    "kind": s.kind,
                })
            })
            .collect();

        let changed: Vec<Value> = self
            .changed
            .iter()
            .map(|c| {
                let fields: Vec<Value> = c
                    .fields
                    .iter()
                    .map(|f| {
                        serde_json::json!({
                            "after": f.after,
                            "before": f.before,
                            "field": f.field,
                        })
                    })
                    .collect();
                serde_json::json!({
                    "fields": fields,
                    "id": c.id,
                    "kind": c.kind,
                })
            })
            .collect();

        serde_json::json!({
            "added": added,
            "changed": changed,
            "removed": removed,
        })
    }

    /// Format the diff as human-readable text.
    pub fn to_text(&self) -> String {
        let mut lines = Vec::new();

        for s in &self.added {
            lines.push(format!("+ {} {}", s.kind, s.id));
        }
        for s in &self.removed {
            lines.push(format!("- {} {}", s.kind, s.id));
        }
        for c in &self.changed {
            lines.push(format!("~ {} {}", c.kind, c.id));
            for f in &c.fields {
                let before = serde_json::to_string(&f.before).unwrap_or_default();
                let after = serde_json::to_string(&f.after).unwrap_or_default();
                lines.push(format!("    {}: {} -> {}", f.field, before, after));
            }
        }

        lines.join("\n")
    }
}

/// Fields to ignore when comparing constructs (noise fields).
const IGNORED_FIELDS: &[&str] = &["line", "provenance"];

/// Extract the constructs array from a bundle, returning a BTreeMap keyed by (kind, id).
fn index_constructs(bundle: &Value) -> Result<BTreeMap<(String, String), &Value>, DiffError> {
    let constructs = bundle
        .get("constructs")
        .and_then(|v| v.as_array())
        .ok_or_else(|| DiffError::InvalidBundle("missing 'constructs' array".to_string()))?;

    let mut index = BTreeMap::new();
    for construct in constructs {
        let kind = construct
            .get("kind")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                DiffError::InvalidBundle("construct missing 'kind' field".to_string())
            })?;
        let id = construct
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| DiffError::InvalidBundle("construct missing 'id' field".to_string()))?;
        index.insert((kind.to_string(), id.to_string()), construct);
    }

    Ok(index)
}

/// Normalize a JSON value for comparison: sort arrays of primitives to ignore ordering.
/// This handles arrays like `states` and `allowed_personas` where element order is a set.
fn normalize_for_comparison(value: &Value) -> Value {
    match value {
        Value::Array(arr) => {
            // Check if all elements are primitive (string, number, bool)
            let all_primitive = arr
                .iter()
                .all(|v| v.is_string() || v.is_number() || v.is_boolean());
            if all_primitive && !arr.is_empty() {
                let mut sorted: Vec<Value> = arr.iter().map(normalize_for_comparison).collect();
                sorted.sort_by(|a, b| {
                    let a_str = serde_json::to_string(a).unwrap_or_default();
                    let b_str = serde_json::to_string(b).unwrap_or_default();
                    a_str.cmp(&b_str)
                });
                Value::Array(sorted)
            } else {
                // For arrays of objects (like transitions), preserve order
                Value::Array(arr.iter().map(normalize_for_comparison).collect())
            }
        }
        Value::Object(map) => {
            let mut new_map = serde_json::Map::new();
            for (k, v) in map {
                new_map.insert(k.clone(), normalize_for_comparison(v));
            }
            Value::Object(new_map)
        }
        other => other.clone(),
    }
}

/// Compute field-level diffs between two construct JSON objects.
fn diff_fields(before: &Value, after: &Value) -> Vec<FieldDiff> {
    let mut diffs = Vec::new();

    let before_obj = match before.as_object() {
        Some(o) => o,
        None => return diffs,
    };
    let after_obj = match after.as_object() {
        Some(o) => o,
        None => return diffs,
    };

    // Collect all keys from both objects
    let mut all_keys: Vec<&String> = before_obj.keys().chain(after_obj.keys()).collect();
    all_keys.sort();
    all_keys.dedup();

    for key in all_keys {
        // Skip ignored fields
        if IGNORED_FIELDS.contains(&key.as_str()) {
            continue;
        }

        let b = before_obj.get(key);
        let a = after_obj.get(key);

        match (b, a) {
            (Some(bv), Some(av)) => {
                let bv_norm = normalize_for_comparison(bv);
                let av_norm = normalize_for_comparison(av);
                if bv_norm != av_norm {
                    diffs.push(FieldDiff {
                        field: key.clone(),
                        before: bv.clone(),
                        after: av.clone(),
                    });
                }
            }
            (Some(bv), None) => {
                diffs.push(FieldDiff {
                    field: key.clone(),
                    before: bv.clone(),
                    after: Value::Null,
                });
            }
            (None, Some(av)) => {
                diffs.push(FieldDiff {
                    field: key.clone(),
                    before: Value::Null,
                    after: av.clone(),
                });
            }
            (None, None) => {}
        }
    }

    // Sort by field name for deterministic output
    diffs.sort_by(|a, b| a.field.cmp(&b.field));
    diffs
}

/// Diff two interchange bundles, producing added, removed, and changed construct sets.
///
/// Constructs are compared by `(kind, id)` key, not array position.
/// Line numbers and provenance are excluded from comparison.
pub fn diff_bundles(t1: &Value, t2: &Value) -> Result<BundleDiff, DiffError> {
    let index1 = index_constructs(t1)?;
    let index2 = index_constructs(t2)?;

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();

    // Find removed (in t1 but not t2)
    for key in index1.keys() {
        if !index2.contains_key(key) {
            removed.push(ConstructSummary {
                kind: key.0.clone(),
                id: key.1.clone(),
            });
        }
    }

    // Find added (in t2 but not t1) and changed (in both but different)
    for key in index2.keys() {
        match index1.get(key) {
            None => {
                added.push(ConstructSummary {
                    kind: key.0.clone(),
                    id: key.1.clone(),
                });
            }
            Some(v1) => {
                let v2 = index2[key];
                let fields = diff_fields(v1, v2);
                if !fields.is_empty() {
                    changed.push(ConstructChange {
                        kind: key.0.clone(),
                        id: key.1.clone(),
                        fields,
                    });
                }
            }
        }
    }

    // Sort all lists by (kind, id) for deterministic output
    added.sort();
    removed.sort();
    changed.sort_by(|a, b| (&a.kind, &a.id).cmp(&(&b.kind, &b.id)));

    Ok(BundleDiff {
        added,
        removed,
        changed,
    })
}

// ──────────────────────────────────────────────
// Breaking change classification (Section 17.2)
// ──────────────────────────────────────────────

/// Severity of a change per the breaking change taxonomy (§18.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ChangeSeverity {
    Breaking,
    NonBreaking,
    RequiresAnalysis,
}

impl fmt::Display for ChangeSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChangeSeverity::Breaking => write!(f, "BREAKING"),
            ChangeSeverity::NonBreaking => write!(f, "NON_BREAKING"),
            ChangeSeverity::RequiresAnalysis => write!(f, "REQUIRES_ANALYSIS"),
        }
    }
}

/// Classification for a single change with reason.
#[derive(Debug, Clone, Serialize)]
pub struct ChangeClassification {
    pub severity: ChangeSeverity,
    pub reason: String,
    pub migration_action: Option<String>,
}

/// A construct-level classification (for added/removed).
#[derive(Debug, Clone, Serialize)]
pub struct ClassifiedConstruct {
    pub kind: String,
    pub id: String,
    pub classification: ChangeClassification,
}

/// A field-level classification within a changed construct.
#[derive(Debug, Clone, Serialize)]
pub struct ClassifiedFieldDiff {
    pub field: String,
    pub before: Value,
    pub after: Value,
    pub classification: ChangeClassification,
}

/// A changed construct with per-field classifications.
#[derive(Debug, Clone, Serialize)]
pub struct ClassifiedChange {
    pub kind: String,
    pub id: String,
    pub fields: Vec<ClassifiedFieldDiff>,
}

/// Summary counts of classifications.
#[derive(Debug, Clone, Serialize)]
pub struct ClassificationSummary {
    pub breaking_count: usize,
    pub non_breaking_count: usize,
    pub requires_analysis_count: usize,
    pub total_changes: usize,
}

/// A fully classified diff.
#[derive(Debug, Clone, Serialize)]
pub struct ClassifiedDiff {
    pub added: Vec<ClassifiedConstruct>,
    pub removed: Vec<ClassifiedConstruct>,
    pub changed: Vec<ClassifiedChange>,
    pub summary: ClassificationSummary,
}

impl ClassifiedDiff {
    /// Serialize to JSON.
    pub fn to_json(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }

    /// Format as human-readable text.
    pub fn to_text(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!(
            "{} change(s): {} breaking, {} non-breaking, {} requires analysis",
            self.summary.total_changes,
            self.summary.breaking_count,
            self.summary.non_breaking_count,
            self.summary.requires_analysis_count
        ));
        lines.push(String::new());

        // Breaking changes
        let breaking: Vec<String> = self
            .added
            .iter()
            .filter(|c| c.classification.severity == ChangeSeverity::Breaking)
            .map(|c| format!("  + {} {} — {}", c.kind, c.id, c.classification.reason))
            .chain(
                self.removed
                    .iter()
                    .filter(|c| c.classification.severity == ChangeSeverity::Breaking)
                    .map(|c| format!("  - {} {} — {}", c.kind, c.id, c.classification.reason)),
            )
            .chain(self.changed.iter().flat_map(|c| {
                c.fields
                    .iter()
                    .filter(|f| f.classification.severity == ChangeSeverity::Breaking)
                    .map(move |f| {
                        format!(
                            "  ~ {} {} .{} — {}",
                            c.kind, c.id, f.field, f.classification.reason
                        )
                    })
            }))
            .collect();

        if !breaking.is_empty() {
            lines.push("BREAKING:".to_string());
            lines.extend(breaking);
            lines.push(String::new());
        }

        // Requires analysis
        let requires: Vec<String> = self
            .added
            .iter()
            .filter(|c| c.classification.severity == ChangeSeverity::RequiresAnalysis)
            .map(|c| format!("  + {} {} — {}", c.kind, c.id, c.classification.reason))
            .chain(self.changed.iter().flat_map(|c| {
                c.fields
                    .iter()
                    .filter(|f| f.classification.severity == ChangeSeverity::RequiresAnalysis)
                    .map(move |f| {
                        format!(
                            "  ~ {} {} .{} — {}",
                            c.kind, c.id, f.field, f.classification.reason
                        )
                    })
            }))
            .collect();

        if !requires.is_empty() {
            lines.push("REQUIRES_ANALYSIS:".to_string());
            lines.extend(requires);
            lines.push(String::new());
        }

        // Non-breaking
        let non_breaking: Vec<String> = self
            .added
            .iter()
            .filter(|c| c.classification.severity == ChangeSeverity::NonBreaking)
            .map(|c| format!("  + {} {} — {}", c.kind, c.id, c.classification.reason))
            .chain(self.changed.iter().flat_map(|c| {
                c.fields
                    .iter()
                    .filter(|f| f.classification.severity == ChangeSeverity::NonBreaking)
                    .map(move |f| {
                        format!(
                            "  ~ {} {} .{} — {}",
                            c.kind, c.id, f.field, f.classification.reason
                        )
                    })
            }))
            .collect();

        if !non_breaking.is_empty() {
            lines.push("NON_BREAKING:".to_string());
            lines.extend(non_breaking);
        }

        lines.join("\n")
    }

    /// Whether there are any breaking changes.
    pub fn has_breaking(&self) -> bool {
        self.summary.breaking_count > 0
    }
}

/// Classify a structural diff using the breaking change taxonomy (§18.2).
pub fn classify_diff(diff: &BundleDiff) -> ClassifiedDiff {
    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();

    let mut breaking_count = 0;
    let mut non_breaking_count = 0;
    let mut requires_analysis_count = 0;

    // Classify added constructs
    for a in &diff.added {
        let classification = classify_add(&a.kind);
        match classification.severity {
            ChangeSeverity::Breaking => breaking_count += 1,
            ChangeSeverity::NonBreaking => non_breaking_count += 1,
            ChangeSeverity::RequiresAnalysis => requires_analysis_count += 1,
        }
        added.push(ClassifiedConstruct {
            kind: a.kind.clone(),
            id: a.id.clone(),
            classification,
        });
    }

    // Classify removed constructs
    for r in &diff.removed {
        let classification = classify_remove(&r.kind);
        match classification.severity {
            ChangeSeverity::Breaking => breaking_count += 1,
            ChangeSeverity::NonBreaking => non_breaking_count += 1,
            ChangeSeverity::RequiresAnalysis => requires_analysis_count += 1,
        }
        removed.push(ClassifiedConstruct {
            kind: r.kind.clone(),
            id: r.id.clone(),
            classification,
        });
    }

    // Classify changed constructs (per field)
    for c in &diff.changed {
        let mut classified_fields = Vec::new();
        for f in &c.fields {
            let classification = classify_field_change(&c.kind, &f.field, &f.before, &f.after);
            match classification.severity {
                ChangeSeverity::Breaking => breaking_count += 1,
                ChangeSeverity::NonBreaking => non_breaking_count += 1,
                ChangeSeverity::RequiresAnalysis => requires_analysis_count += 1,
            }
            classified_fields.push(ClassifiedFieldDiff {
                field: f.field.clone(),
                before: f.before.clone(),
                after: f.after.clone(),
                classification,
            });
        }
        changed.push(ClassifiedChange {
            kind: c.kind.clone(),
            id: c.id.clone(),
            fields: classified_fields,
        });
    }

    let total_changes = breaking_count + non_breaking_count + requires_analysis_count;

    ClassifiedDiff {
        added,
        removed,
        changed,
        summary: ClassificationSummary {
            breaking_count,
            non_breaking_count,
            requires_analysis_count,
            total_changes,
        },
    }
}

/// Classify adding a construct by kind.
fn classify_add(kind: &str) -> ChangeClassification {
    match kind {
        "Rule" => ChangeClassification {
            severity: ChangeSeverity::RequiresAnalysis,
            reason: "New rule may produce verdicts that affect existing operations".to_string(),
            migration_action: Some(
                "Verify new verdicts don't conflict with existing flow logic".to_string(),
            ),
        },
        _ => ChangeClassification {
            severity: ChangeSeverity::NonBreaking,
            reason: format!("Adding a {} does not affect existing behavior", kind),
            migration_action: None,
        },
    }
}

/// Classify removing a construct by kind.
fn classify_remove(kind: &str) -> ChangeClassification {
    let reason = match kind {
        "Fact" => "Removing a Fact breaks rules and operations that reference it",
        "Entity" => "Removing an Entity breaks operations with effects on this entity",
        "Rule" => "Removing a Rule breaks verdict dependencies",
        "Persona" => "Removing a Persona breaks operations with this persona in allowed_personas",
        "Operation" => "Removing an Operation breaks flow steps that reference it",
        "Flow" => "Removing a Flow breaks SubFlowSteps and in-flight instances",
        _ => "Removing a construct may break references to it",
    };

    ChangeClassification {
        severity: ChangeSeverity::Breaking,
        reason: reason.to_string(),
        migration_action: Some(format!(
            "Remove all references to this {} before removing it",
            kind
        )),
    }
}

/// Classify a field-level change using the Section 17.2 taxonomy.
fn classify_field_change(
    kind: &str,
    field: &str,
    before: &Value,
    after: &Value,
) -> ChangeClassification {
    // Skip metadata fields
    if matches!(field, "tenor" | "kind" | "id" | "provenance") {
        return ChangeClassification {
            severity: ChangeSeverity::NonBreaking,
            reason: "Metadata field change".to_string(),
            migration_action: None,
        };
    }

    match (kind, field) {
        // Entity fields
        ("Entity", "states") => classify_set_change(before, after, "state"),
        ("Entity", "initial") => ChangeClassification {
            severity: ChangeSeverity::Breaking,
            reason: "Changing initial state affects all new entity instances".to_string(),
            migration_action: Some(
                "Update all in-flight entities or version the entity".to_string(),
            ),
        },
        ("Entity", "transitions") => classify_transitions_change(before, after),
        ("Entity", "parent") => ChangeClassification {
            severity: ChangeSeverity::Breaking,
            reason: "Changing parent entity hierarchy is breaking".to_string(),
            migration_action: Some("Update all entity hierarchy references".to_string()),
        },

        // Fact fields
        ("Fact", "type") => classify_type_change(before, after),
        ("Fact", "default") => {
            if before.is_null() {
                // Adding a default
                ChangeClassification {
                    severity: ChangeSeverity::NonBreaking,
                    reason: "Adding a default value does not affect existing behavior".to_string(),
                    migration_action: None,
                }
            } else if after.is_null() {
                // Removing a default
                ChangeClassification {
                    severity: ChangeSeverity::Breaking,
                    reason: "Removing a default value breaks callers that don't supply this fact"
                        .to_string(),
                    migration_action: Some(
                        "Ensure all callers now provide this fact explicitly".to_string(),
                    ),
                }
            } else {
                // Changing a default
                ChangeClassification {
                    severity: ChangeSeverity::RequiresAnalysis,
                    reason: "Changing default value may affect evaluation results".to_string(),
                    migration_action: Some(
                        "Verify new default doesn't change verdict outcomes".to_string(),
                    ),
                }
            }
        }
        ("Fact", "source") => ChangeClassification {
            severity: ChangeSeverity::NonBreaking,
            reason: "Source metadata change does not affect contract logic".to_string(),
            migration_action: None,
        },

        // Rule fields
        ("Rule", "stratum") => ChangeClassification {
            severity: ChangeSeverity::Breaking,
            reason: "Changing stratum affects rule evaluation order and verdict dependencies"
                .to_string(),
            migration_action: Some(
                "Verify cross-stratum verdict dependencies still hold".to_string(),
            ),
        },
        ("Rule", "body") => ChangeClassification {
            severity: ChangeSeverity::RequiresAnalysis,
            reason: "Rule body change may affect which verdicts are produced".to_string(),
            migration_action: Some("Run static analysis to verify verdict impact".to_string()),
        },

        // Operation fields
        ("Operation", "allowed_personas") => classify_set_change(before, after, "persona"),
        ("Operation", "precondition") => ChangeClassification {
            severity: ChangeSeverity::RequiresAnalysis,
            reason: "Precondition change may affect operation admissibility".to_string(),
            migration_action: Some(
                "Run S3a analysis to verify structural admissibility".to_string(),
            ),
        },
        ("Operation", "effects") => classify_effects_change(before, after),
        ("Operation", "outcomes") => classify_set_change(before, after, "outcome"),
        ("Operation", "error_contract") => ChangeClassification {
            severity: ChangeSeverity::NonBreaking,
            reason: "Error contract change affects error handling but not core logic".to_string(),
            migration_action: None,
        },

        // Flow fields
        ("Flow", "entry") => ChangeClassification {
            severity: ChangeSeverity::Breaking,
            reason: "Changing flow entry point breaks all in-flight flow instances".to_string(),
            migration_action: Some(
                "Version the flow or migrate all in-flight instances".to_string(),
            ),
        },
        ("Flow", "steps") => ChangeClassification {
            severity: ChangeSeverity::RequiresAnalysis,
            reason: "Flow step changes may affect execution paths".to_string(),
            migration_action: Some("Run S6 analysis to verify path impact".to_string()),
        },
        ("Flow", "snapshot") => ChangeClassification {
            severity: ChangeSeverity::NonBreaking,
            reason: "Snapshot policy change does not affect flow logic".to_string(),
            migration_action: None,
        },

        // Default: unknown field changes require analysis
        _ => ChangeClassification {
            severity: ChangeSeverity::RequiresAnalysis,
            reason: format!("Unknown field '{}' on '{}' changed", field, kind),
            migration_action: None,
        },
    }
}

/// Classify a set change (add/remove elements in an array).
fn classify_set_change(before: &Value, after: &Value, element_type: &str) -> ChangeClassification {
    let before_set: std::collections::BTreeSet<String> = before
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let after_set: std::collections::BTreeSet<String> = after
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let added: Vec<_> = after_set.difference(&before_set).collect();
    let removed: Vec<_> = before_set.difference(&after_set).collect();

    if !removed.is_empty() {
        ChangeClassification {
            severity: ChangeSeverity::Breaking,
            reason: format!(
                "Removed {}(s): {}",
                element_type,
                removed
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            migration_action: Some(format!(
                "Remove all references to removed {}(s) first",
                element_type
            )),
        }
    } else if !added.is_empty() {
        ChangeClassification {
            severity: ChangeSeverity::NonBreaking,
            reason: format!(
                "Added {}(s): {}",
                element_type,
                added
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            migration_action: None,
        }
    } else {
        ChangeClassification {
            severity: ChangeSeverity::NonBreaking,
            reason: "Set contents unchanged (order only)".to_string(),
            migration_action: None,
        }
    }
}

/// Classify a transitions change.
fn classify_transitions_change(before: &Value, after: &Value) -> ChangeClassification {
    let before_set: std::collections::BTreeSet<(String, String)> = before
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    let from = t.get("from")?.as_str()?.to_string();
                    let to = t.get("to")?.as_str()?.to_string();
                    Some((from, to))
                })
                .collect()
        })
        .unwrap_or_default();

    let after_set: std::collections::BTreeSet<(String, String)> = after
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    let from = t.get("from")?.as_str()?.to_string();
                    let to = t.get("to")?.as_str()?.to_string();
                    Some((from, to))
                })
                .collect()
        })
        .unwrap_or_default();

    let removed: Vec<_> = before_set.difference(&after_set).collect();

    if !removed.is_empty() {
        ChangeClassification {
            severity: ChangeSeverity::Breaking,
            reason: "Removed transition(s) may break operations with effects on those paths"
                .to_string(),
            migration_action: Some(
                "Update operations that depend on removed transitions".to_string(),
            ),
        }
    } else {
        ChangeClassification {
            severity: ChangeSeverity::NonBreaking,
            reason: "Added transition(s) expand the state graph".to_string(),
            migration_action: None,
        }
    }
}

/// Classify a type change (widen vs narrow).
fn classify_type_change(before: &Value, after: &Value) -> ChangeClassification {
    let before_base = before.get("base").and_then(|b| b.as_str()).unwrap_or("");
    let after_base = after.get("base").and_then(|b| b.as_str()).unwrap_or("");

    if before_base != after_base {
        return ChangeClassification {
            severity: ChangeSeverity::Breaking,
            reason: format!("Base type change: {} -> {}", before_base, after_base),
            migration_action: Some("Update all references to use the new type".to_string()),
        };
    }

    // Same base type -- check for widen/narrow
    match before_base {
        "Enum" => {
            let before_values: std::collections::BTreeSet<String> = before
                .get("values")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let after_values: std::collections::BTreeSet<String> = after
                .get("values")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            let removed: Vec<_> = before_values.difference(&after_values).collect();
            if !removed.is_empty() {
                ChangeClassification {
                    severity: ChangeSeverity::Breaking,
                    reason: format!("Enum narrowing: removed values {:?}", removed),
                    migration_action: Some(
                        "Update all references to removed enum values".to_string(),
                    ),
                }
            } else {
                ChangeClassification {
                    severity: ChangeSeverity::NonBreaking,
                    reason: "Enum widening: new values added".to_string(),
                    migration_action: None,
                }
            }
        }
        "Int" => {
            // Check range changes
            let before_min = before.get("min").and_then(|v| v.as_i64());
            let before_max = before.get("max").and_then(|v| v.as_i64());
            let after_min = after.get("min").and_then(|v| v.as_i64());
            let after_max = after.get("max").and_then(|v| v.as_i64());

            let min_narrowed = match (before_min, after_min) {
                (Some(b), Some(a)) => a > b,
                (None, Some(_)) => true, // Added min constraint
                _ => false,
            };
            let max_narrowed = match (before_max, after_max) {
                (Some(b), Some(a)) => a < b,
                (None, Some(_)) => true, // Added max constraint
                _ => false,
            };

            if min_narrowed || max_narrowed {
                ChangeClassification {
                    severity: ChangeSeverity::Breaking,
                    reason: "Int range narrowing".to_string(),
                    migration_action: Some("Verify all values fit in new range".to_string()),
                }
            } else {
                ChangeClassification {
                    severity: ChangeSeverity::NonBreaking,
                    reason: "Int range widening or unchanged".to_string(),
                    migration_action: None,
                }
            }
        }
        _ => ChangeClassification {
            severity: ChangeSeverity::RequiresAnalysis,
            reason: format!("Type parameter change for {}", before_base),
            migration_action: Some("Verify type compatibility".to_string()),
        },
    }
}

/// Classify effects change.
fn classify_effects_change(before: &Value, after: &Value) -> ChangeClassification {
    let before_effects: Vec<String> = before
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|e| serde_json::to_string(e).unwrap_or_default())
                .collect()
        })
        .unwrap_or_default();
    let after_effects: Vec<String> = after
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|e| serde_json::to_string(e).unwrap_or_default())
                .collect()
        })
        .unwrap_or_default();

    let before_set: std::collections::BTreeSet<_> = before_effects.iter().collect();
    let after_set: std::collections::BTreeSet<_> = after_effects.iter().collect();

    let removed: Vec<_> = before_set.difference(&after_set).collect();

    if !removed.is_empty() {
        ChangeClassification {
            severity: ChangeSeverity::Breaking,
            reason: "Removed or changed effects".to_string(),
            migration_action: Some("Update affected entities and flow steps".to_string()),
        }
    } else {
        ChangeClassification {
            severity: ChangeSeverity::NonBreaking,
            reason: "Added effects expand operation behavior".to_string(),
            migration_action: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_bundle(constructs: Vec<Value>) -> Value {
        json!({
            "constructs": constructs,
            "id": "test_bundle",
            "kind": "Bundle",
            "tenor": "1.0",
            "tenor_version": "1.0.0"
        })
    }

    fn make_fact(id: &str, base: &str, line: u64) -> Value {
        json!({
            "id": id,
            "kind": "Fact",
            "provenance": { "file": "test.tenor", "line": line },
            "source": { "field": id, "system": "test_service" },
            "tenor": "1.0",
            "type": { "base": base }
        })
    }

    fn make_entity(id: &str, states: Vec<&str>, line: u64) -> Value {
        json!({
            "id": id,
            "kind": "Entity",
            "initial": states[0],
            "provenance": { "file": "test.tenor", "line": line },
            "states": states,
            "tenor": "1.0",
            "transitions": []
        })
    }

    #[test]
    fn identical_bundles_produce_empty_diff() {
        let b = make_bundle(vec![
            make_fact("is_active", "Bool", 4),
            make_fact("amount", "Decimal", 10),
        ]);
        let diff = diff_bundles(&b, &b).unwrap();
        assert!(diff.is_empty());
        assert_eq!(diff.added.len(), 0);
        assert_eq!(diff.removed.len(), 0);
        assert_eq!(diff.changed.len(), 0);
    }

    #[test]
    fn added_construct() {
        let t1 = make_bundle(vec![make_fact("is_active", "Bool", 4)]);
        let t2 = make_bundle(vec![
            make_fact("is_active", "Bool", 4),
            make_fact("amount", "Decimal", 10),
        ]);
        let diff = diff_bundles(&t1, &t2).unwrap();
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.added[0].kind, "Fact");
        assert_eq!(diff.added[0].id, "amount");
        assert_eq!(diff.removed.len(), 0);
        assert_eq!(diff.changed.len(), 0);
    }

    #[test]
    fn removed_construct() {
        let t1 = make_bundle(vec![
            make_fact("is_active", "Bool", 4),
            make_fact("amount", "Decimal", 10),
        ]);
        let t2 = make_bundle(vec![make_fact("is_active", "Bool", 4)]);
        let diff = diff_bundles(&t1, &t2).unwrap();
        assert_eq!(diff.added.len(), 0);
        assert_eq!(diff.removed.len(), 1);
        assert_eq!(diff.removed[0].kind, "Fact");
        assert_eq!(diff.removed[0].id, "amount");
        assert_eq!(diff.changed.len(), 0);
    }

    #[test]
    fn changed_construct_with_field_diff() {
        let t1 = make_bundle(vec![make_entity("Order", vec!["draft", "submitted"], 5)]);
        let t2 = make_bundle(vec![make_entity(
            "Order",
            vec!["draft", "submitted", "approved"],
            5,
        )]);
        let diff = diff_bundles(&t1, &t2).unwrap();
        assert_eq!(diff.added.len(), 0);
        assert_eq!(diff.removed.len(), 0);
        assert_eq!(diff.changed.len(), 1);
        assert_eq!(diff.changed[0].kind, "Entity");
        assert_eq!(diff.changed[0].id, "Order");
        // Should have a field diff for "states"
        let states_diff = diff.changed[0]
            .fields
            .iter()
            .find(|f| f.field == "states")
            .expect("should have states field diff");
        assert_eq!(states_diff.before, json!(["draft", "submitted"]));
        assert_eq!(states_diff.after, json!(["draft", "submitted", "approved"]));
    }

    #[test]
    fn line_number_changes_ignored() {
        let t1 = make_bundle(vec![make_fact("is_active", "Bool", 4)]);
        let t2 = make_bundle(vec![make_fact("is_active", "Bool", 20)]);
        let diff = diff_bundles(&t1, &t2).unwrap();
        assert!(
            diff.is_empty(),
            "line number changes should be ignored; got {:?}",
            diff
        );
    }

    #[test]
    fn multiple_changes_in_single_diff() {
        let t1 = make_bundle(vec![
            make_fact("is_active", "Bool", 4),
            make_fact("amount", "Decimal", 10),
            make_entity("Order", vec!["draft", "submitted"], 20),
        ]);
        let t2 = make_bundle(vec![
            // is_active removed
            make_fact("amount", "Int", 10), // amount type changed
            make_entity("Order", vec!["draft", "submitted", "approved"], 20), // states changed
            make_fact("status", "Enum", 30), // status added
        ]);
        let diff = diff_bundles(&t1, &t2).unwrap();
        assert_eq!(diff.added.len(), 1, "should have 1 added");
        assert_eq!(diff.added[0].id, "status");
        assert_eq!(diff.removed.len(), 1, "should have 1 removed");
        assert_eq!(diff.removed[0].id, "is_active");
        assert_eq!(diff.changed.len(), 2, "should have 2 changed");
        // Changed should be sorted by (kind, id): Entity/Order, Fact/amount
        assert_eq!(diff.changed[0].kind, "Entity");
        assert_eq!(diff.changed[0].id, "Order");
        assert_eq!(diff.changed[1].kind, "Fact");
        assert_eq!(diff.changed[1].id, "amount");
    }

    #[test]
    fn set_order_ignored_for_primitive_arrays() {
        // States in different order but same set should not be a diff.
        // Use json! directly to control "initial" independently of array order.
        let t1 = make_bundle(vec![json!({
            "id": "Order",
            "kind": "Entity",
            "initial": "draft",
            "provenance": { "file": "test.tenor", "line": 5 },
            "states": ["submitted", "draft", "approved"],
            "tenor": "1.0",
            "transitions": []
        })]);
        let t2 = make_bundle(vec![json!({
            "id": "Order",
            "kind": "Entity",
            "initial": "draft",
            "provenance": { "file": "test.tenor", "line": 5 },
            "states": ["draft", "approved", "submitted"],
            "tenor": "1.0",
            "transitions": []
        })]);
        let diff = diff_bundles(&t1, &t2).unwrap();
        assert!(
            diff.is_empty(),
            "reordered primitive arrays should be treated as same set; got {:?}",
            diff
        );
    }

    #[test]
    fn invalid_bundle_missing_constructs() {
        let b1 = json!({"id": "test"});
        let b2 = make_bundle(vec![]);
        let result = diff_bundles(&b1, &b2);
        assert!(result.is_err());
        match result.unwrap_err() {
            DiffError::InvalidBundle(msg) => {
                assert!(
                    msg.contains("constructs"),
                    "error should mention constructs"
                );
            }
        }
    }

    #[test]
    fn invalid_bundle_missing_kind() {
        let b1 = json!({
            "constructs": [{"id": "foo"}]
        });
        let b2 = make_bundle(vec![]);
        let result = diff_bundles(&b1, &b2);
        assert!(result.is_err());
        match result.unwrap_err() {
            DiffError::InvalidBundle(msg) => {
                assert!(msg.contains("kind"), "error should mention kind");
            }
        }
    }

    #[test]
    fn invalid_bundle_missing_id() {
        let b1 = json!({
            "constructs": [{"kind": "Fact"}]
        });
        let b2 = make_bundle(vec![]);
        let result = diff_bundles(&b1, &b2);
        assert!(result.is_err());
        match result.unwrap_err() {
            DiffError::InvalidBundle(msg) => {
                assert!(msg.contains("id"), "error should mention id");
            }
        }
    }

    #[test]
    fn json_output_format() {
        let t1 = make_bundle(vec![make_fact("old_fact", "Bool", 4)]);
        let t2 = make_bundle(vec![make_fact("new_fact", "Bool", 4)]);
        let diff = diff_bundles(&t1, &t2).unwrap();
        let json = diff.to_json();

        // Added should contain new_fact
        let added = json["added"].as_array().unwrap();
        assert_eq!(added.len(), 1);
        assert_eq!(added[0]["id"], "new_fact");
        assert_eq!(added[0]["kind"], "Fact");

        // Removed should contain old_fact
        let removed = json["removed"].as_array().unwrap();
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0]["id"], "old_fact");
        assert_eq!(removed[0]["kind"], "Fact");
    }

    #[test]
    fn text_output_format() {
        let t1 = make_bundle(vec![
            make_fact("old_fact", "Bool", 4),
            make_entity("Order", vec!["draft", "submitted"], 10),
        ]);
        let t2 = make_bundle(vec![
            make_fact("new_fact", "Bool", 4),
            make_entity("Order", vec!["draft", "submitted", "approved"], 10),
        ]);
        let diff = diff_bundles(&t1, &t2).unwrap();
        let text = diff.to_text();

        assert!(text.contains("+ Fact new_fact"), "should show added");
        assert!(text.contains("- Fact old_fact"), "should show removed");
        assert!(text.contains("~ Entity Order"), "should show changed");
        assert!(text.contains("states:"), "should show field name");
    }

    #[test]
    fn empty_bundles_produce_empty_diff() {
        let b = make_bundle(vec![]);
        let diff = diff_bundles(&b, &b).unwrap();
        assert!(diff.is_empty());
    }

    #[test]
    fn field_added_to_construct() {
        let t1 = make_bundle(vec![json!({
            "id": "order_amount",
            "kind": "Fact",
            "provenance": { "file": "test.tenor", "line": 4 },
            "source": { "field": "amount", "system": "test" },
            "tenor": "1.0",
            "type": { "base": "Decimal", "precision": 10, "scale": 2 }
        })]);
        let t2 = make_bundle(vec![json!({
            "id": "order_amount",
            "kind": "Fact",
            "default": { "kind": "decimal_value", "precision": 10, "scale": 2, "value": "0.00" },
            "provenance": { "file": "test.tenor", "line": 4 },
            "source": { "field": "amount", "system": "test" },
            "tenor": "1.0",
            "type": { "base": "Decimal", "precision": 10, "scale": 2 }
        })]);
        let diff = diff_bundles(&t1, &t2).unwrap();
        assert_eq!(diff.changed.len(), 1);
        let default_diff = diff.changed[0]
            .fields
            .iter()
            .find(|f| f.field == "default")
            .expect("should have default field diff");
        assert_eq!(default_diff.before, Value::Null);
        assert!(default_diff.after.is_object());
    }

    // ──────────────────────────────────────────────
    // Breaking change classification tests
    // ──────────────────────────────────────────────

    #[test]
    fn classify_added_fact_is_non_breaking() {
        let t1 = make_bundle(vec![]);
        let t2 = make_bundle(vec![make_fact("amount", "Int", 4)]);
        let diff = diff_bundles(&t1, &t2).unwrap();
        let classified = classify_diff(&diff);

        assert_eq!(classified.added.len(), 1);
        assert_eq!(
            classified.added[0].classification.severity,
            ChangeSeverity::NonBreaking
        );
        assert_eq!(classified.summary.non_breaking_count, 1);
        assert_eq!(classified.summary.breaking_count, 0);
    }

    #[test]
    fn classify_added_rule_requires_analysis() {
        let t1 = make_bundle(vec![]);
        let t2 = make_bundle(vec![json!({
            "id": "check_amount",
            "kind": "Rule",
            "stratum": 0,
            "body": { "when": {}, "produce": { "verdict_type": "v", "payload": {} } },
            "provenance": { "file": "t.tenor", "line": 1 },
            "tenor": "1.0"
        })]);
        let diff = diff_bundles(&t1, &t2).unwrap();
        let classified = classify_diff(&diff);

        assert_eq!(
            classified.added[0].classification.severity,
            ChangeSeverity::RequiresAnalysis
        );
    }

    #[test]
    fn classify_removed_fact_is_breaking() {
        let t1 = make_bundle(vec![make_fact("amount", "Int", 4)]);
        let t2 = make_bundle(vec![]);
        let diff = diff_bundles(&t1, &t2).unwrap();
        let classified = classify_diff(&diff);

        assert_eq!(classified.removed.len(), 1);
        assert_eq!(
            classified.removed[0].classification.severity,
            ChangeSeverity::Breaking
        );
        assert!(classified.has_breaking());
    }

    #[test]
    fn classify_state_addition_non_breaking() {
        let t1 = make_bundle(vec![make_entity("Order", vec!["draft", "submitted"], 5)]);
        let t2 = make_bundle(vec![make_entity(
            "Order",
            vec!["draft", "submitted", "approved"],
            5,
        )]);
        let diff = diff_bundles(&t1, &t2).unwrap();
        let classified = classify_diff(&diff);

        assert_eq!(classified.changed.len(), 1);
        let states_field = classified.changed[0]
            .fields
            .iter()
            .find(|f| f.field == "states")
            .expect("should have states field");
        assert_eq!(
            states_field.classification.severity,
            ChangeSeverity::NonBreaking,
            "Adding states should be non-breaking"
        );
    }

    #[test]
    fn classify_state_removal_breaking() {
        let t1 = make_bundle(vec![make_entity(
            "Order",
            vec!["draft", "submitted", "approved"],
            5,
        )]);
        let t2 = make_bundle(vec![make_entity("Order", vec!["draft", "submitted"], 5)]);
        let diff = diff_bundles(&t1, &t2).unwrap();
        let classified = classify_diff(&diff);

        let states_field = classified.changed[0]
            .fields
            .iter()
            .find(|f| f.field == "states")
            .expect("should have states field");
        assert_eq!(
            states_field.classification.severity,
            ChangeSeverity::Breaking,
            "Removing states should be breaking"
        );
    }

    #[test]
    fn classify_type_base_change_breaking() {
        let t1 = make_bundle(vec![make_fact("amount", "Int", 4)]);
        let t2 = make_bundle(vec![make_fact("amount", "Decimal", 4)]);
        let diff = diff_bundles(&t1, &t2).unwrap();
        let classified = classify_diff(&diff);

        let type_field = classified.changed[0]
            .fields
            .iter()
            .find(|f| f.field == "type")
            .expect("should have type field");
        assert_eq!(
            type_field.classification.severity,
            ChangeSeverity::Breaking,
            "Base type change should be breaking"
        );
    }

    #[test]
    fn classify_mixed_diff_summary() {
        let t1 = make_bundle(vec![
            make_fact("old_fact", "Bool", 4),
            make_entity("Order", vec!["draft", "submitted"], 10),
        ]);
        let t2 = make_bundle(vec![
            make_fact("new_fact", "Bool", 4),
            make_entity("Order", vec!["draft", "submitted", "approved"], 10),
        ]);
        let diff = diff_bundles(&t1, &t2).unwrap();
        let classified = classify_diff(&diff);

        // Removed fact = breaking, added fact = non_breaking, added state = non_breaking
        assert!(classified.summary.breaking_count >= 1);
        assert!(classified.summary.non_breaking_count >= 1);
        assert!(classified.summary.total_changes >= 2);
    }

    #[test]
    fn classify_text_output_format() {
        let t1 = make_bundle(vec![make_fact("old_fact", "Bool", 4)]);
        let t2 = make_bundle(vec![]);
        let diff = diff_bundles(&t1, &t2).unwrap();
        let classified = classify_diff(&diff);
        let text = classified.to_text();

        assert!(text.contains("BREAKING:"), "should have BREAKING section");
        assert!(
            text.contains("old_fact"),
            "should mention removed construct"
        );
    }

    #[test]
    fn classify_json_output_format() {
        let t1 = make_bundle(vec![make_fact("old_fact", "Bool", 4)]);
        let t2 = make_bundle(vec![]);
        let diff = diff_bundles(&t1, &t2).unwrap();
        let classified = classify_diff(&diff);
        let json = classified.to_json();

        assert!(json.get("summary").is_some());
        assert!(json.get("removed").unwrap().is_array());
        assert_eq!(json["summary"]["breaking_count"], serde_json::json!(1));
    }
}
