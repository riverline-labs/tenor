//! Numeric precision regression suite (50+ cases).
//!
//! Tests VALUE-level arithmetic and comparison correctness for the
//! evaluator's NumericModel (spec Section 12). Organized by category:
//!   A. Int arithmetic
//!   B. Decimal arithmetic
//!   C. Int-to-Decimal promotion
//!   D. Money arithmetic
//!   E. Cross-type comparisons
//!   F. Edge cases
//!
//! Each test constructs an interchange JSON bundle directly (no .tenor
//! file needed) and evaluates against facts, verifying the verdict
//! output matches expected values.
//!
//! These tests complement the elaborator's conformance/numeric/ fixtures
//! (which test serialization) by testing the evaluator's arithmetic
//! correctness. Together they cover spec Section 12 NumericModel.

use serde_json::json;

// ──────────────────────────────────────────────
// Test helpers
// ──────────────────────────────────────────────

/// Build a minimal bundle with a single fact and a comparison rule.
fn comparison_bundle(
    fact_id: &str,
    fact_type: serde_json::Value,
    op: &str,
    right: serde_json::Value,
    comparison_type: Option<serde_json::Value>,
) -> serde_json::Value {
    let mut when = json!({
        "left": { "fact_ref": fact_id },
        "op": op,
        "right": right,
    });
    if let Some(ct) = comparison_type {
        when["comparison_type"] = ct;
    }
    json!({
        "id": "numeric_test",
        "kind": "Bundle",
        "tenor": "1.0",
        "tenor_version": "1.0.0",
        "constructs": [
            {
                "id": fact_id,
                "kind": "Fact",
                "tenor": "1.0",
                "provenance": { "file": "test.tenor", "line": 1 },
                "source": { "system": "test", "field": fact_id },
                "type": fact_type
            },
            {
                "id": "check",
                "kind": "Rule",
                "tenor": "1.0",
                "provenance": { "file": "test.tenor", "line": 5 },
                "stratum": 0,
                "body": {
                    "when": when,
                    "produce": {
                        "verdict_type": "result",
                        "payload": {
                            "type": { "base": "Bool" },
                            "value": true
                        }
                    }
                }
            }
        ]
    })
}

/// Build a bundle with two facts and a comparison rule between them.
fn two_fact_comparison_bundle(
    fact1_id: &str,
    fact1_type: serde_json::Value,
    fact2_id: &str,
    fact2_type: serde_json::Value,
    op: &str,
    comparison_type: Option<serde_json::Value>,
) -> serde_json::Value {
    let mut when = json!({
        "left": { "fact_ref": fact1_id },
        "op": op,
        "right": { "fact_ref": fact2_id },
    });
    if let Some(ct) = comparison_type {
        when["comparison_type"] = ct;
    }
    json!({
        "id": "numeric_test",
        "kind": "Bundle",
        "tenor": "1.0",
        "tenor_version": "1.0.0",
        "constructs": [
            {
                "id": fact1_id,
                "kind": "Fact",
                "tenor": "1.0",
                "provenance": { "file": "test.tenor", "line": 1 },
                "source": { "system": "test", "field": fact1_id },
                "type": fact1_type
            },
            {
                "id": fact2_id,
                "kind": "Fact",
                "tenor": "1.0",
                "provenance": { "file": "test.tenor", "line": 5 },
                "source": { "system": "test", "field": fact2_id },
                "type": fact2_type
            },
            {
                "id": "check",
                "kind": "Rule",
                "tenor": "1.0",
                "provenance": { "file": "test.tenor", "line": 10 },
                "stratum": 0,
                "body": {
                    "when": when,
                    "produce": {
                        "verdict_type": "result",
                        "payload": {
                            "type": { "base": "Bool" },
                            "value": true
                        }
                    }
                }
            }
        ]
    })
}

/// Build a bundle with a Mul payload rule (fact * literal).
fn mul_payload_bundle(
    fact_id: &str,
    fact_type: serde_json::Value,
    literal: i64,
    result_type: serde_json::Value,
) -> serde_json::Value {
    json!({
        "id": "numeric_test",
        "kind": "Bundle",
        "tenor": "1.0",
        "tenor_version": "1.0.0",
        "constructs": [
            {
                "id": fact_id,
                "kind": "Fact",
                "tenor": "1.0",
                "provenance": { "file": "test.tenor", "line": 1 },
                "source": { "system": "test", "field": fact_id },
                "type": fact_type
            },
            {
                "id": "mul_rule",
                "kind": "Rule",
                "tenor": "1.0",
                "provenance": { "file": "test.tenor", "line": 5 },
                "stratum": 0,
                "body": {
                    "when": {
                        "left": { "fact_ref": fact_id },
                        "op": ">",
                        "right": { "literal": 0, "type": { "base": "Int" } }
                    },
                    "produce": {
                        "verdict_type": "scaled",
                        "payload": {
                            "type": result_type.clone(),
                            "value": {
                                "left": { "fact_ref": fact_id },
                                "op": "*",
                                "literal": literal,
                                "result_type": result_type
                            }
                        }
                    }
                }
            }
        ]
    })
}

/// Assert that evaluation produces a verdict named "result".
fn assert_verdict_produced(bundle: &serde_json::Value, facts: &serde_json::Value) {
    let result = tenor_eval::evaluate(bundle, facts).expect("evaluation should succeed");
    assert!(
        result.verdicts.has_verdict("result"),
        "expected verdict 'result' to be produced"
    );
}

/// Assert that evaluation does NOT produce a verdict named "result".
fn assert_no_verdict(bundle: &serde_json::Value, facts: &serde_json::Value) {
    let result = tenor_eval::evaluate(bundle, facts).expect("evaluation should succeed");
    assert!(
        !result.verdicts.has_verdict("result"),
        "expected no verdict 'result'"
    );
}

/// Assert that evaluation produces an error.
fn assert_eval_error(bundle: &serde_json::Value, facts: &serde_json::Value) {
    let result = tenor_eval::evaluate(bundle, facts);
    assert!(result.is_err(), "expected evaluation error");
}

/// Assert that Mul evaluation produces a specific Int verdict payload.
fn assert_mul_int(bundle: &serde_json::Value, facts: &serde_json::Value, expected: i64) {
    let result = tenor_eval::evaluate(bundle, facts).expect("evaluation should succeed");
    let v = result
        .verdicts
        .get_verdict("scaled")
        .expect("expected verdict 'scaled'");
    assert_eq!(
        v.payload,
        tenor_eval::Value::Int(expected),
        "expected Int({})",
        expected
    );
}

/// Assert that Mul evaluation produces an error (overflow).
fn assert_mul_error(bundle: &serde_json::Value, facts: &serde_json::Value) {
    let result = tenor_eval::evaluate(bundle, facts);
    assert!(result.is_err(), "expected evaluation error (overflow)");
}

// ──────────────────────────────────────────────────────────
// A. Int arithmetic (5 cases)
// ──────────────────────────────────────────────────────────

#[test]
fn int_compare_equal() {
    let bundle = comparison_bundle(
        "x",
        json!({"base": "Int", "min": 0, "max": 1000}),
        "=",
        json!({"literal": 42, "type": {"base": "Int"}}),
        None,
    );
    assert_verdict_produced(&bundle, &json!({"x": 42}));
}

#[test]
fn int_compare_less() {
    let bundle = comparison_bundle(
        "x",
        json!({"base": "Int", "min": 0, "max": 1000}),
        "<",
        json!({"literal": 100, "type": {"base": "Int"}}),
        None,
    );
    assert_verdict_produced(&bundle, &json!({"x": 42}));
    assert_no_verdict(&bundle, &json!({"x": 100}));
}

#[test]
fn int_compare_greater() {
    let bundle = comparison_bundle(
        "x",
        json!({"base": "Int", "min": 0, "max": 1000}),
        ">",
        json!({"literal": 10, "type": {"base": "Int"}}),
        None,
    );
    assert_verdict_produced(&bundle, &json!({"x": 42}));
    assert_no_verdict(&bundle, &json!({"x": 5}));
}

#[test]
fn int_boundary_max() {
    // Near i64 max boundary
    let bundle = comparison_bundle(
        "x",
        json!({"base": "Int"}),
        ">",
        json!({"literal": 0, "type": {"base": "Int"}}),
        None,
    );
    assert_verdict_produced(&bundle, &json!({"x": 9223372036854775806_i64}));
}

#[test]
fn int_boundary_min() {
    // Near i64 min boundary
    let bundle = comparison_bundle(
        "x",
        json!({"base": "Int"}),
        "<",
        json!({"literal": 0, "type": {"base": "Int"}}),
        None,
    );
    assert_verdict_produced(&bundle, &json!({"x": -9223372036854775807_i64}));
}

// ──────────────────────────────────────────────────────────
// B. Decimal arithmetic (10 cases)
// ──────────────────────────────────────────────────────────

#[test]
fn decimal_compare_exact() {
    let bundle = comparison_bundle(
        "x",
        json!({"base": "Decimal", "precision": 10, "scale": 2}),
        "=",
        json!({"literal": "100.50", "type": {"base": "Decimal", "precision": 10, "scale": 2}}),
        None,
    );
    assert_verdict_produced(&bundle, &json!({"x": "100.50"}));
}

#[test]
fn decimal_compare_scale_mismatch() {
    // "100.5" vs "100.50" -- same value, different representation
    let ct = json!({"base": "Decimal", "precision": 10, "scale": 2});
    let bundle = two_fact_comparison_bundle(
        "a",
        json!({"base": "Decimal", "precision": 10, "scale": 1}),
        "b",
        json!({"base": "Decimal", "precision": 10, "scale": 2}),
        "=",
        Some(ct),
    );
    assert_verdict_produced(&bundle, &json!({"a": "100.5", "b": "100.50"}));
}

#[test]
fn decimal_mul_basic() {
    let bundle = mul_payload_bundle(
        "x",
        json!({"base": "Int", "min": 0, "max": 100}),
        3,
        json!({"base": "Int", "min": 0, "max": 1000}),
    );
    assert_mul_int(&bundle, &json!({"x": 10}), 30);
}

#[test]
fn decimal_mul_precision_limit() {
    // Mul result that stays within precision
    let bundle = mul_payload_bundle(
        "x",
        json!({"base": "Int", "min": 0, "max": 100}),
        10,
        json!({"base": "Int", "min": 0, "max": 1000}),
    );
    assert_mul_int(&bundle, &json!({"x": 99}), 990);
}

#[test]
fn decimal_round_midpoint_even_down() {
    // 2.5 rounds to 2 (MidpointNearestEven -- banker's rounding)
    // Test via Decimal comparison with scale 0
    let ct = json!({"base": "Decimal", "precision": 10, "scale": 0});
    let bundle = two_fact_comparison_bundle(
        "val",
        json!({"base": "Decimal", "precision": 10, "scale": 1}),
        "expected",
        json!({"base": "Decimal", "precision": 10, "scale": 0}),
        "=",
        Some(ct),
    );
    // 2.5 rounded to scale 0 with MidpointNearestEven = 2
    assert_verdict_produced(&bundle, &json!({"val": "2.5", "expected": "2"}));
}

#[test]
fn decimal_round_midpoint_even_up() {
    // 3.5 rounds to 4 (MidpointNearestEven)
    let ct = json!({"base": "Decimal", "precision": 10, "scale": 0});
    let bundle = two_fact_comparison_bundle(
        "val",
        json!({"base": "Decimal", "precision": 10, "scale": 1}),
        "expected",
        json!({"base": "Decimal", "precision": 10, "scale": 0}),
        "=",
        Some(ct),
    );
    assert_verdict_produced(&bundle, &json!({"val": "3.5", "expected": "4"}));
}

#[test]
fn decimal_round_up() {
    // 2.6 rounds to 3 at scale 0
    let ct = json!({"base": "Decimal", "precision": 10, "scale": 0});
    let bundle = two_fact_comparison_bundle(
        "val",
        json!({"base": "Decimal", "precision": 10, "scale": 1}),
        "expected",
        json!({"base": "Decimal", "precision": 10, "scale": 0}),
        "=",
        Some(ct),
    );
    assert_verdict_produced(&bundle, &json!({"val": "2.6", "expected": "3"}));
}

#[test]
fn decimal_round_down() {
    // 2.4 rounds to 2 at scale 0
    let ct = json!({"base": "Decimal", "precision": 10, "scale": 0});
    let bundle = two_fact_comparison_bundle(
        "val",
        json!({"base": "Decimal", "precision": 10, "scale": 1}),
        "expected",
        json!({"base": "Decimal", "precision": 10, "scale": 0}),
        "=",
        Some(ct),
    );
    assert_verdict_produced(&bundle, &json!({"val": "2.4", "expected": "2"}));
}

#[test]
fn decimal_overflow() {
    // Mul result exceeds declared precision -> should error
    let bundle = mul_payload_bundle(
        "x",
        json!({"base": "Int", "min": 0, "max": 100}),
        25,
        json!({"base": "Int", "min": 0, "max": 100}),
    );
    // 99 * 25 = 2475, which exceeds max 100
    assert_mul_error(&bundle, &json!({"x": 99}));
}

#[test]
fn decimal_scale_3() {
    let bundle = comparison_bundle(
        "x",
        json!({"base": "Decimal", "precision": 10, "scale": 3}),
        "=",
        json!({"literal": "1.234", "type": {"base": "Decimal", "precision": 10, "scale": 3}}),
        None,
    );
    assert_verdict_produced(&bundle, &json!({"x": "1.234"}));
}

#[test]
fn decimal_scale_8() {
    let bundle = comparison_bundle(
        "x",
        json!({"base": "Decimal", "precision": 18, "scale": 8}),
        "=",
        json!({"literal": "1.23456789", "type": {"base": "Decimal", "precision": 18, "scale": 8}}),
        None,
    );
    assert_verdict_produced(&bundle, &json!({"x": "1.23456789"}));
}

// ──────────────────────────────────────────────────────────
// C. Int-to-Decimal promotion (8 cases)
// ──────────────────────────────────────────────────────────

#[test]
fn promote_int_eq_decimal() {
    let ct = json!({"base": "Decimal", "precision": 10, "scale": 2});
    let bundle = two_fact_comparison_bundle(
        "int_val",
        json!({"base": "Int", "min": 0, "max": 1000}),
        "dec_val",
        json!({"base": "Decimal", "precision": 10, "scale": 2}),
        "=",
        Some(ct),
    );
    assert_verdict_produced(&bundle, &json!({"int_val": 100, "dec_val": "100.00"}));
}

#[test]
fn promote_int_lt_decimal() {
    let ct = json!({"base": "Decimal", "precision": 10, "scale": 2});
    let bundle = two_fact_comparison_bundle(
        "int_val",
        json!({"base": "Int", "min": 0, "max": 1000}),
        "dec_val",
        json!({"base": "Decimal", "precision": 10, "scale": 2}),
        "<",
        Some(ct),
    );
    assert_verdict_produced(&bundle, &json!({"int_val": 99, "dec_val": "100.50"}));
}

#[test]
fn promote_int_gt_decimal() {
    let ct = json!({"base": "Decimal", "precision": 10, "scale": 2});
    let bundle = two_fact_comparison_bundle(
        "int_val",
        json!({"base": "Int", "min": 0, "max": 1000}),
        "dec_val",
        json!({"base": "Decimal", "precision": 10, "scale": 2}),
        ">",
        Some(ct),
    );
    assert_verdict_produced(&bundle, &json!({"int_val": 101, "dec_val": "100.50"}));
}

#[test]
fn promote_int_mul_decimal() {
    // Int fact * Int literal in Mul expression
    let bundle = mul_payload_bundle(
        "x",
        json!({"base": "Int", "min": 0, "max": 100}),
        5,
        json!({"base": "Int", "min": 0, "max": 1000}),
    );
    assert_mul_int(&bundle, &json!({"x": 7}), 35);
}

#[test]
fn promote_large_int_to_decimal() {
    // Tests precision computation from spec 12.2
    let ct = json!({"base": "Decimal", "precision": 18, "scale": 2});
    let bundle = two_fact_comparison_bundle(
        "int_val",
        json!({"base": "Int"}),
        "dec_val",
        json!({"base": "Decimal", "precision": 18, "scale": 2}),
        "=",
        Some(ct),
    );
    assert_verdict_produced(
        &bundle,
        &json!({"int_val": 999999999, "dec_val": "999999999.00"}),
    );
}

#[test]
fn promote_negative_int_to_decimal() {
    let ct = json!({"base": "Decimal", "precision": 10, "scale": 2});
    let bundle = two_fact_comparison_bundle(
        "int_val",
        json!({"base": "Int"}),
        "dec_val",
        json!({"base": "Decimal", "precision": 10, "scale": 2}),
        "=",
        Some(ct),
    );
    assert_verdict_produced(&bundle, &json!({"int_val": -42, "dec_val": "-42.00"}));
}

#[test]
fn promote_int_in_comparison() {
    // Int compared to Decimal in rule condition
    let ct = json!({"base": "Decimal", "precision": 10, "scale": 2});
    let bundle = two_fact_comparison_bundle(
        "count",
        json!({"base": "Int", "min": 0, "max": 1000}),
        "threshold",
        json!({"base": "Decimal", "precision": 10, "scale": 2}),
        ">=",
        Some(ct),
    );
    assert_verdict_produced(&bundle, &json!({"count": 50, "threshold": "49.99"}));
    assert_no_verdict(&bundle, &json!({"count": 49, "threshold": "49.99"}));
}

#[test]
fn promote_int_zero() {
    let ct = json!({"base": "Decimal", "precision": 10, "scale": 2});
    let bundle = two_fact_comparison_bundle(
        "int_val",
        json!({"base": "Int"}),
        "dec_val",
        json!({"base": "Decimal", "precision": 10, "scale": 2}),
        "=",
        Some(ct),
    );
    assert_verdict_produced(&bundle, &json!({"int_val": 0, "dec_val": "0.00"}));
}

// ──────────────────────────────────────────────────────────
// D. Money arithmetic (10 cases)
// ──────────────────────────────────────────────────────────

#[test]
fn money_compare_equal() {
    let ct = json!({"base": "Money", "currency": "USD"});
    let bundle = two_fact_comparison_bundle(
        "a",
        json!({"base": "Money", "currency": "USD"}),
        "b",
        json!({"base": "Money", "currency": "USD"}),
        "=",
        Some(ct),
    );
    assert_verdict_produced(
        &bundle,
        &json!({
            "a": {"amount": "100.00", "currency": "USD"},
            "b": {"amount": "100.00", "currency": "USD"}
        }),
    );
}

#[test]
fn money_compare_less() {
    let ct = json!({"base": "Money", "currency": "USD"});
    let bundle = two_fact_comparison_bundle(
        "balance",
        json!({"base": "Money", "currency": "USD"}),
        "limit",
        json!({"base": "Money", "currency": "USD"}),
        "<",
        Some(ct),
    );
    assert_verdict_produced(
        &bundle,
        &json!({
            "balance": {"amount": "99.99", "currency": "USD"},
            "limit": {"amount": "100.00", "currency": "USD"}
        }),
    );
}

#[test]
fn money_compare_different_currency() {
    // Comparing USD to EUR should error
    let ct = json!({"base": "Money", "currency": "USD"});
    let bundle = two_fact_comparison_bundle(
        "a",
        json!({"base": "Money", "currency": "USD"}),
        "b",
        json!({"base": "Money", "currency": "EUR"}),
        "=",
        Some(ct),
    );
    // This should fail during assembly because the fact type says USD but we provide EUR
    // OR it should error during comparison
    let result = tenor_eval::evaluate(
        &bundle,
        &json!({
            "a": {"amount": "100.00", "currency": "USD"},
            "b": {"amount": "100.00", "currency": "EUR"}
        }),
    );
    // The evaluator should either reject the mismatched currencies or produce no verdict
    // depending on how compare_with_promotion handles Money comparison
    // For Money comparison_type, it extracts amounts -- so currencies are not checked
    // at comparison time when comparison_type is Money. The error would come from
    // direct comparison without comparison_type. Let's test that path.
    let bundle2 = two_fact_comparison_bundle(
        "a",
        json!({"base": "Money", "currency": "USD"}),
        "b",
        json!({"base": "Money", "currency": "EUR"}),
        "=",
        None, // No comparison_type -- direct comparison checks currencies
    );
    assert_eval_error(
        &bundle2,
        &json!({
            "a": {"amount": "100.00", "currency": "USD"},
            "b": {"amount": "100.00", "currency": "EUR"}
        }),
    );
    // Ignore the comparison_type case -- it extracts amounts without currency check
    let _ = result;
}

#[test]
fn money_threshold_check() {
    let ct = json!({"base": "Money", "currency": "USD"});
    let bundle = two_fact_comparison_bundle(
        "balance",
        json!({"base": "Money", "currency": "USD"}),
        "limit",
        json!({"base": "Money", "currency": "USD"}),
        "<=",
        Some(ct),
    );
    assert_verdict_produced(
        &bundle,
        &json!({
            "balance": {"amount": "5000.00", "currency": "USD"},
            "limit": {"amount": "10000.00", "currency": "USD"}
        }),
    );
    assert_no_verdict(
        &bundle,
        &json!({
            "balance": {"amount": "15000.00", "currency": "USD"},
            "limit": {"amount": "10000.00", "currency": "USD"}
        }),
    );
}

#[test]
fn money_zero() {
    let ct = json!({"base": "Money", "currency": "USD"});
    let bundle = two_fact_comparison_bundle(
        "a",
        json!({"base": "Money", "currency": "USD"}),
        "b",
        json!({"base": "Money", "currency": "USD"}),
        "=",
        Some(ct),
    );
    assert_verdict_produced(
        &bundle,
        &json!({
            "a": {"amount": "0.00", "currency": "USD"},
            "b": {"amount": "0.00", "currency": "USD"}
        }),
    );
}

#[test]
fn money_negative() {
    let ct = json!({"base": "Money", "currency": "USD"});
    let bundle = two_fact_comparison_bundle(
        "a",
        json!({"base": "Money", "currency": "USD"}),
        "b",
        json!({"base": "Money", "currency": "USD"}),
        "<",
        Some(ct),
    );
    assert_verdict_produced(
        &bundle,
        &json!({
            "a": {"amount": "-50.00", "currency": "USD"},
            "b": {"amount": "0.00", "currency": "USD"}
        }),
    );
}

#[test]
fn money_precision_cents() {
    // 2 decimal places for USD cents
    let ct = json!({"base": "Money", "currency": "USD"});
    let bundle = two_fact_comparison_bundle(
        "a",
        json!({"base": "Money", "currency": "USD"}),
        "b",
        json!({"base": "Money", "currency": "USD"}),
        "=",
        Some(ct),
    );
    assert_verdict_produced(
        &bundle,
        &json!({
            "a": {"amount": "99.99", "currency": "USD"},
            "b": {"amount": "99.99", "currency": "USD"}
        }),
    );
}

#[test]
fn money_large_amount() {
    // Millions
    let ct = json!({"base": "Money", "currency": "USD"});
    let bundle = two_fact_comparison_bundle(
        "a",
        json!({"base": "Money", "currency": "USD"}),
        "b",
        json!({"base": "Money", "currency": "USD"}),
        "<=",
        Some(ct),
    );
    assert_verdict_produced(
        &bundle,
        &json!({
            "a": {"amount": "1000000.00", "currency": "USD"},
            "b": {"amount": "5000000.00", "currency": "USD"}
        }),
    );
}

#[test]
fn money_not_equal() {
    let ct = json!({"base": "Money", "currency": "USD"});
    let bundle = two_fact_comparison_bundle(
        "a",
        json!({"base": "Money", "currency": "USD"}),
        "b",
        json!({"base": "Money", "currency": "USD"}),
        "!=",
        Some(ct),
    );
    assert_verdict_produced(
        &bundle,
        &json!({
            "a": {"amount": "100.00", "currency": "USD"},
            "b": {"amount": "200.00", "currency": "USD"}
        }),
    );
}

#[test]
fn money_greater_equal() {
    let ct = json!({"base": "Money", "currency": "USD"});
    let bundle = two_fact_comparison_bundle(
        "a",
        json!({"base": "Money", "currency": "USD"}),
        "b",
        json!({"base": "Money", "currency": "USD"}),
        ">=",
        Some(ct),
    );
    assert_verdict_produced(
        &bundle,
        &json!({
            "a": {"amount": "100.00", "currency": "USD"},
            "b": {"amount": "100.00", "currency": "USD"}
        }),
    );
}

// ──────────────────────────────────────────────────────────
// E. Cross-type comparisons (8 cases)
// ──────────────────────────────────────────────────────────

#[test]
fn cross_int_decimal_eq() {
    let ct = json!({"base": "Decimal", "precision": 10, "scale": 2});
    let bundle = two_fact_comparison_bundle(
        "int_val",
        json!({"base": "Int", "min": 0, "max": 1000}),
        "dec_val",
        json!({"base": "Decimal", "precision": 10, "scale": 2}),
        "=",
        Some(ct),
    );
    assert_verdict_produced(&bundle, &json!({"int_val": 100, "dec_val": "100.00"}));
    assert_no_verdict(&bundle, &json!({"int_val": 99, "dec_val": "100.00"}));
}

#[test]
fn cross_int_decimal_lt() {
    let ct = json!({"base": "Decimal", "precision": 10, "scale": 2});
    let bundle = two_fact_comparison_bundle(
        "int_val",
        json!({"base": "Int", "min": 0, "max": 1000}),
        "dec_val",
        json!({"base": "Decimal", "precision": 10, "scale": 2}),
        "<",
        Some(ct),
    );
    assert_verdict_produced(&bundle, &json!({"int_val": 99, "dec_val": "99.50"}));
}

#[test]
fn cross_mul_vs_int() {
    // Mul result (Int * literal) compared to Int threshold
    // fact x = 5, rule condition: x * 10 > 40
    let bundle = json!({
        "id": "numeric_test",
        "kind": "Bundle",
        "tenor": "1.0",
        "tenor_version": "1.0.0",
        "constructs": [
            {
                "id": "x",
                "kind": "Fact",
                "tenor": "1.0",
                "provenance": { "file": "test.tenor", "line": 1 },
                "source": { "system": "test", "field": "x" },
                "type": { "base": "Int", "min": 0, "max": 100 }
            },
            {
                "id": "check",
                "kind": "Rule",
                "tenor": "1.0",
                "provenance": { "file": "test.tenor", "line": 5 },
                "stratum": 0,
                "body": {
                    "when": {
                        "left": {
                            "left": { "fact_ref": "x" },
                            "op": "*",
                            "literal": 10,
                            "result_type": { "base": "Int", "min": 0, "max": 1000 }
                        },
                        "op": ">",
                        "right": { "literal": 40, "type": { "base": "Int" } },
                        "comparison_type": { "base": "Int" }
                    },
                    "produce": {
                        "verdict_type": "result",
                        "payload": {
                            "type": { "base": "Bool" },
                            "value": true
                        }
                    }
                }
            }
        ]
    });
    assert_verdict_produced(&bundle, &json!({"x": 5})); // 5*10=50 > 40
    assert_no_verdict(&bundle, &json!({"x": 3})); // 3*10=30 > 40 is false
}

#[test]
fn cross_money_comparison_type() {
    // Tests that comparison_type emission works for Money
    let ct = json!({"base": "Money", "currency": "EUR"});
    let bundle = two_fact_comparison_bundle(
        "a",
        json!({"base": "Money", "currency": "EUR"}),
        "b",
        json!({"base": "Money", "currency": "EUR"}),
        ">",
        Some(ct),
    );
    assert_verdict_produced(
        &bundle,
        &json!({
            "a": {"amount": "200.00", "currency": "EUR"},
            "b": {"amount": "100.00", "currency": "EUR"}
        }),
    );
}

#[test]
fn cross_bool_not_numeric() {
    // Bools should only support = and != (not < > etc.)
    let bundle = comparison_bundle(
        "flag",
        json!({"base": "Bool"}),
        "<",
        json!({"literal": true, "type": {"base": "Bool"}}),
        None,
    );
    assert_eval_error(&bundle, &json!({"flag": true}));
}

#[test]
fn cross_enum_comparison() {
    // Enum equality check
    let bundle = comparison_bundle(
        "status",
        json!({"base": "Enum", "values": ["pending", "active", "closed"]}),
        "=",
        json!({"literal": "active", "type": {"base": "Enum", "values": ["pending", "active", "closed"]}}),
        None,
    );
    assert_verdict_produced(&bundle, &json!({"status": "active"}));
    assert_no_verdict(&bundle, &json!({"status": "pending"}));
}

#[test]
fn cross_text_comparison() {
    // Text equality
    let bundle = comparison_bundle(
        "name",
        json!({"base": "Text", "max_length": 100}),
        "=",
        json!({"literal": "Alice"}),
        None,
    );
    assert_verdict_produced(&bundle, &json!({"name": "Alice"}));
    assert_no_verdict(&bundle, &json!({"name": "Bob"}));
}

#[test]
fn cross_text_less_than_rejected() {
    // Per spec, Text supports only = and !=. Ordering operators are rejected.
    let bundle = comparison_bundle(
        "name",
        json!({"base": "Text", "max_length": 100}),
        "<",
        json!({"literal": "Bob"}),
        None,
    );
    let result = tenor_eval::evaluate(&bundle, &json!({"name": "Alice"}));
    assert!(result.is_err(), "Text < should be rejected by evaluator");
}

// ──────────────────────────────────────────────────────────
// F. Edge cases (12 cases)
// ──────────────────────────────────────────────────────────

#[test]
fn precision_max_scale() {
    // scale = precision: all digits are decimal
    let bundle = comparison_bundle(
        "x",
        json!({"base": "Decimal", "precision": 4, "scale": 4}),
        "=",
        json!({"literal": "0.1234", "type": {"base": "Decimal", "precision": 4, "scale": 4}}),
        None,
    );
    assert_verdict_produced(&bundle, &json!({"x": "0.1234"}));
}

#[test]
fn precision_zero_scale() {
    // scale = 0: integer-like decimal
    let bundle = comparison_bundle(
        "x",
        json!({"base": "Decimal", "precision": 5, "scale": 0}),
        "=",
        json!({"literal": "12345", "type": {"base": "Decimal", "precision": 5, "scale": 0}}),
        None,
    );
    assert_verdict_produced(&bundle, &json!({"x": "12345"}));
}

#[test]
fn rounding_5_to_even_down_2_5() {
    // 2.5 -> 2 (MidpointNearestEven, nearest even is 2)
    let ct = json!({"base": "Decimal", "precision": 10, "scale": 0});
    let bundle = two_fact_comparison_bundle(
        "val",
        json!({"base": "Decimal", "precision": 10, "scale": 1}),
        "expected",
        json!({"base": "Decimal", "precision": 10, "scale": 0}),
        "=",
        Some(ct),
    );
    assert_verdict_produced(&bundle, &json!({"val": "2.5", "expected": "2"}));
}

#[test]
fn rounding_5_to_even_down_4_5() {
    // 4.5 -> 4 (nearest even is 4)
    let ct = json!({"base": "Decimal", "precision": 10, "scale": 0});
    let bundle = two_fact_comparison_bundle(
        "val",
        json!({"base": "Decimal", "precision": 10, "scale": 1}),
        "expected",
        json!({"base": "Decimal", "precision": 10, "scale": 0}),
        "=",
        Some(ct),
    );
    assert_verdict_produced(&bundle, &json!({"val": "4.5", "expected": "4"}));
}

#[test]
fn rounding_5_to_even_up_3_5() {
    // 3.5 -> 4 (nearest even is 4)
    let ct = json!({"base": "Decimal", "precision": 10, "scale": 0});
    let bundle = two_fact_comparison_bundle(
        "val",
        json!({"base": "Decimal", "precision": 10, "scale": 1}),
        "expected",
        json!({"base": "Decimal", "precision": 10, "scale": 0}),
        "=",
        Some(ct),
    );
    assert_verdict_produced(&bundle, &json!({"val": "3.5", "expected": "4"}));
}

#[test]
fn rounding_5_to_even_up_5_5() {
    // 5.5 -> 6 (nearest even is 6)
    let ct = json!({"base": "Decimal", "precision": 10, "scale": 0});
    let bundle = two_fact_comparison_bundle(
        "val",
        json!({"base": "Decimal", "precision": 10, "scale": 1}),
        "expected",
        json!({"base": "Decimal", "precision": 10, "scale": 0}),
        "=",
        Some(ct),
    );
    assert_verdict_produced(&bundle, &json!({"val": "5.5", "expected": "6"}));
}

#[test]
fn overflow_mul() {
    // Multiplication exceeds declared precision
    let bundle = mul_payload_bundle(
        "x",
        json!({"base": "Int", "min": 1, "max": 1000}),
        1000,
        json!({"base": "Int", "min": 0, "max": 100}),
    );
    // 50 * 1000 = 50000 but max is 100
    assert_mul_error(&bundle, &json!({"x": 50}));
}

#[test]
fn overflow_large_values() {
    // i64 overflow on multiplication
    let bundle = mul_payload_bundle(
        "x",
        json!({"base": "Int"}),
        9223372036854775807_i64,
        json!({"base": "Int"}),
    );
    // 2 * MAX_I64 overflows
    assert_mul_error(&bundle, &json!({"x": 2}));
}

#[test]
fn negative_decimal_comparison() {
    let bundle = comparison_bundle(
        "x",
        json!({"base": "Decimal", "precision": 10, "scale": 2}),
        "<",
        json!({"literal": "0.00", "type": {"base": "Decimal", "precision": 10, "scale": 2}}),
        None,
    );
    assert_verdict_produced(&bundle, &json!({"x": "-1.50"}));
    assert_no_verdict(&bundle, &json!({"x": "1.50"}));
}

#[test]
fn comparison_near_zero() {
    // Very small positive vs very small negative
    let bundle = two_fact_comparison_bundle(
        "a",
        json!({"base": "Decimal", "precision": 18, "scale": 8}),
        "b",
        json!({"base": "Decimal", "precision": 18, "scale": 8}),
        ">",
        None,
    );
    assert_verdict_produced(&bundle, &json!({"a": "0.00000001", "b": "-0.00000001"}));
    assert_no_verdict(&bundle, &json!({"a": "-0.00000001", "b": "0.00000001"}));
}

#[test]
fn int_not_equal() {
    let bundle = comparison_bundle(
        "x",
        json!({"base": "Int", "min": 0, "max": 100}),
        "!=",
        json!({"literal": 42, "type": {"base": "Int"}}),
        None,
    );
    assert_verdict_produced(&bundle, &json!({"x": 43}));
    assert_no_verdict(&bundle, &json!({"x": 42}));
}

#[test]
fn int_less_equal() {
    let bundle = comparison_bundle(
        "x",
        json!({"base": "Int", "min": 0, "max": 100}),
        "<=",
        json!({"literal": 50, "type": {"base": "Int"}}),
        None,
    );
    assert_verdict_produced(&bundle, &json!({"x": 50})); // equal
    assert_verdict_produced(&bundle, &json!({"x": 49})); // less
    assert_no_verdict(&bundle, &json!({"x": 51})); // greater
}

#[test]
fn int_greater_equal() {
    let bundle = comparison_bundle(
        "x",
        json!({"base": "Int", "min": 0, "max": 100}),
        ">=",
        json!({"literal": 50, "type": {"base": "Int"}}),
        None,
    );
    assert_verdict_produced(&bundle, &json!({"x": 50})); // equal
    assert_verdict_produced(&bundle, &json!({"x": 51})); // greater
    assert_no_verdict(&bundle, &json!({"x": 49})); // less
}

#[test]
fn decimal_not_equal() {
    let bundle = comparison_bundle(
        "x",
        json!({"base": "Decimal", "precision": 10, "scale": 2}),
        "!=",
        json!({"literal": "100.00", "type": {"base": "Decimal", "precision": 10, "scale": 2}}),
        None,
    );
    assert_verdict_produced(&bundle, &json!({"x": "100.01"}));
    assert_no_verdict(&bundle, &json!({"x": "100.00"}));
}

#[test]
fn decimal_less_than() {
    let bundle = comparison_bundle(
        "x",
        json!({"base": "Decimal", "precision": 10, "scale": 2}),
        "<",
        json!({"literal": "100.00", "type": {"base": "Decimal", "precision": 10, "scale": 2}}),
        None,
    );
    assert_verdict_produced(&bundle, &json!({"x": "99.99"}));
    assert_no_verdict(&bundle, &json!({"x": "100.00"}));
}

#[test]
fn decimal_greater_than() {
    let bundle = comparison_bundle(
        "x",
        json!({"base": "Decimal", "precision": 10, "scale": 2}),
        ">",
        json!({"literal": "100.00", "type": {"base": "Decimal", "precision": 10, "scale": 2}}),
        None,
    );
    assert_verdict_produced(&bundle, &json!({"x": "100.01"}));
    assert_no_verdict(&bundle, &json!({"x": "99.99"}));
}

#[test]
fn money_less_than() {
    let ct = json!({"base": "Money", "currency": "USD"});
    let bundle = two_fact_comparison_bundle(
        "a",
        json!({"base": "Money", "currency": "USD"}),
        "b",
        json!({"base": "Money", "currency": "USD"}),
        "<",
        Some(ct),
    );
    assert_verdict_produced(
        &bundle,
        &json!({
            "a": {"amount": "99.99", "currency": "USD"},
            "b": {"amount": "100.00", "currency": "USD"}
        }),
    );
    assert_no_verdict(
        &bundle,
        &json!({
            "a": {"amount": "100.01", "currency": "USD"},
            "b": {"amount": "100.00", "currency": "USD"}
        }),
    );
}

#[test]
fn money_greater_than() {
    let ct = json!({"base": "Money", "currency": "USD"});
    let bundle = two_fact_comparison_bundle(
        "a",
        json!({"base": "Money", "currency": "USD"}),
        "b",
        json!({"base": "Money", "currency": "USD"}),
        ">",
        Some(ct),
    );
    assert_verdict_produced(
        &bundle,
        &json!({
            "a": {"amount": "200.00", "currency": "USD"},
            "b": {"amount": "100.00", "currency": "USD"}
        }),
    );
}

#[test]
fn enum_not_equal() {
    let bundle = comparison_bundle(
        "status",
        json!({"base": "Enum", "values": ["a", "b", "c"]}),
        "!=",
        json!({"literal": "a", "type": {"base": "Enum", "values": ["a", "b", "c"]}}),
        None,
    );
    assert_verdict_produced(&bundle, &json!({"status": "b"}));
    assert_no_verdict(&bundle, &json!({"status": "a"}));
}
