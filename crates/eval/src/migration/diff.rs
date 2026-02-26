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

impl std::error::Error for DiffError {}

/// Summary of a construct (kind + id) for added/removed entries.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
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
}
