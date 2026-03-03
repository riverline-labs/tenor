//! NumericModel implementation using `rust_decimal`.
//!
//! Implements spec Section 12: all arithmetic uses `rust_decimal::Decimal`
//! with `RoundingStrategy::MidpointNearestEven`. No `f64` anywhere in
//! the evaluation path.

use rust_decimal::Decimal;
use rust_decimal::RoundingStrategy;

use crate::types::{EvalError, TypeSpec, Value};

/// Promote an integer to Decimal with the given target type's precision/scale.
///
/// Per spec Section 12.2, Int-to-Decimal promotion converts the integer
/// to a Decimal representation at the target precision and scale.
pub fn promote_int_to_decimal(val: i64, target: &TypeSpec) -> Result<Decimal, EvalError> {
    let d = Decimal::from(val);
    let scale = target.scale.unwrap_or(0);
    Ok(d.round_dp_with_strategy(scale, RoundingStrategy::MidpointNearestEven))
}

/// Evaluate multiplication with overflow checking and rounding.
///
/// Both operands are Decimal values. The result is rounded to the specified
/// precision/scale and checked for overflow.
pub fn eval_mul(
    left: Decimal,
    right: Decimal,
    result_precision: u32,
    result_scale: u32,
) -> Result<Decimal, EvalError> {
    let product = left.checked_mul(right).ok_or_else(|| EvalError::Overflow {
        message: "multiplication overflow".to_string(),
    })?;
    let rounded =
        product.round_dp_with_strategy(result_scale, RoundingStrategy::MidpointNearestEven);
    // Check that result fits in declared precision
    check_precision(rounded, result_precision, result_scale)?;
    Ok(rounded)
}

/// Evaluate integer multiplication (fact_ref * int_literal).
///
/// When the left operand is Int, we multiply as i64 first, then check bounds.
pub fn eval_int_mul(left: i64, literal: i64, result_type: &TypeSpec) -> Result<Value, EvalError> {
    let product = left
        .checked_mul(literal)
        .ok_or_else(|| EvalError::Overflow {
            message: "integer multiplication overflow".to_string(),
        })?;
    // Check bounds if result_type has min/max
    if let (Some(min), Some(max)) = (result_type.min, result_type.max) {
        if product < min || product > max {
            return Err(EvalError::Overflow {
                message: format!(
                    "result {} outside declared range [{}, {}]",
                    product, min, max
                ),
            });
        }
    }
    Ok(Value::Int(product))
}

/// Check that a decimal value fits within the declared precision.
///
/// Uses checked Decimal arithmetic to avoid panics when precision > 18.
/// For max_int_digits > 28 (rust_decimal max significant digits), any value
/// that fits in a Decimal is valid, so we return Ok(()).
fn check_precision(val: Decimal, precision: u32, scale: u32) -> Result<(), EvalError> {
    if precision <= scale {
        // No integer digits allowed -- value must be fractional only
        let int_part = val.trunc().abs();
        if int_part > Decimal::ZERO {
            return Err(EvalError::Overflow {
                message: format!(
                    "result {} exceeds declared precision({}, {})",
                    val, precision, scale
                ),
            });
        }
        return Ok(());
    }
    let max_int_digits = precision - scale;
    let int_part = val.trunc().abs();

    // rust_decimal supports up to 28 significant digits.
    // If max_int_digits > 28, any value that fits in Decimal is valid.
    if max_int_digits > 28 {
        return Ok(());
    }

    // Build 10^max_int_digits using checked Decimal arithmetic (no i64 overflow)
    let mut bound = Decimal::ONE;
    for _ in 0..max_int_digits {
        bound = bound
            .checked_mul(Decimal::TEN)
            .ok_or_else(|| EvalError::Overflow {
                message: format!(
                    "precision bound computation overflow for precision({}, {})",
                    precision, scale
                ),
            })?;
    }
    let max_val = bound - Decimal::ONE;

    if int_part > max_val {
        return Err(EvalError::Overflow {
            message: format!(
                "result {} exceeds declared precision({}, {})",
                val, precision, scale
            ),
        });
    }
    Ok(())
}

/// Compare two values using the given operator, with optional comparison_type
/// for type-directed promotion.
///
/// Supports all comparison operators: =, !=, <, <=, >, >=
pub fn compare_values(
    left: &Value,
    right: &Value,
    op: &str,
    comparison_type: Option<&TypeSpec>,
) -> Result<bool, EvalError> {
    // If comparison_type is provided, use it for promotion context
    if let Some(ct) = comparison_type {
        return compare_with_promotion(left, right, op, ct);
    }

    // Direct comparison without promotion
    match (left, right) {
        (Value::Bool(l), Value::Bool(r)) => compare_bools(*l, *r, op),
        (Value::Int(l), Value::Int(r)) => compare_ints(*l, *r, op),
        (Value::Decimal(l), Value::Decimal(r)) => compare_decimals(*l, *r, op),
        (Value::Text(l), Value::Text(r)) => {
            if op != "=" && op != "!=" {
                return Err(EvalError::TypeError {
                    message: format!(
                        "operator '{}' not defined for Text; Text supports only = and !=",
                        op
                    ),
                });
            }
            compare_strings(l, r, op)
        }
        (Value::Enum(l), Value::Enum(r)) => {
            if op != "=" && op != "!=" {
                return Err(EvalError::TypeError {
                    message: format!(
                        "operator '{}' not defined for Enum; Enum supports only = and !=",
                        op
                    ),
                });
            }
            compare_strings(l, r, op)
        }
        (
            Value::Money {
                amount: la,
                currency: lc,
            },
            Value::Money {
                amount: ra,
                currency: rc,
            },
        ) => {
            if lc != rc {
                return Err(EvalError::TypeError {
                    message: format!(
                        "cannot compare Money with different currencies: {} vs {}",
                        lc, rc
                    ),
                });
            }
            compare_decimals(*la, *ra, op)
        }
        (Value::Date(l), Value::Date(r)) => compare_strings(l, r, op),
        (Value::DateTime(l), Value::DateTime(r)) => compare_strings(l, r, op),
        (
            Value::Duration {
                value: lv,
                unit: lu,
            },
            Value::Duration {
                value: rv,
                unit: ru,
            },
        ) => {
            if lu != ru {
                return Err(EvalError::TypeError {
                    message: format!(
                        "cannot compare Duration with different units: {} vs {} \
                         (cross-unit Duration comparison not supported)",
                        lu, ru
                    ),
                });
            }
            compare_ints(*lv, *rv, op)
        }
        _ => Err(EvalError::TypeError {
            message: format!(
                "cannot compare {} with {}",
                left.type_name(),
                right.type_name()
            ),
        }),
    }
}

/// Compare values with type-directed promotion per comparison_type.
fn compare_with_promotion(
    left: &Value,
    right: &Value,
    op: &str,
    ct: &TypeSpec,
) -> Result<bool, EvalError> {
    match ct.base.as_str() {
        "Decimal" => {
            let l = coerce_to_decimal(left, ct)?;
            let r = coerce_to_decimal(right, ct)?;
            compare_decimals(l, r, op)
        }
        "Money" => {
            let (l_amount, l_currency) = coerce_to_money(left)?;
            let (r_amount, r_currency) = coerce_to_money(right)?;
            if l_currency != r_currency {
                return Err(EvalError::TypeError {
                    message: format!(
                        "cannot compare Money with different currencies: {} vs {}",
                        l_currency, r_currency
                    ),
                });
            }
            compare_decimals(l_amount, r_amount, op)
        }
        "Int" => {
            let l = coerce_to_int(left)?;
            let r = coerce_to_int(right)?;
            compare_ints(l, r, op)
        }
        _ => {
            // Fall back to direct comparison
            compare_values(left, right, op, None)
        }
    }
}

/// Coerce a value to Decimal, promoting Int if necessary.
fn coerce_to_decimal(val: &Value, target: &TypeSpec) -> Result<Decimal, EvalError> {
    match val {
        Value::Decimal(d) => {
            let scale = target.scale.unwrap_or(0);
            Ok(d.round_dp_with_strategy(scale, RoundingStrategy::MidpointNearestEven))
        }
        Value::Int(i) => promote_int_to_decimal(*i, target),
        _ => Err(EvalError::TypeError {
            message: format!("cannot coerce {} to Decimal", val.type_name()),
        }),
    }
}

/// Coerce a Money value to its (amount, currency) pair.
///
/// Unlike the previous `coerce_to_money_amount` which discarded currency,
/// this returns both so callers can validate currency match.
fn coerce_to_money(val: &Value) -> Result<(Decimal, &str), EvalError> {
    match val {
        Value::Money { amount, currency } => Ok((*amount, currency.as_str())),
        _ => Err(EvalError::TypeError {
            message: format!("cannot coerce {} to Money", val.type_name()),
        }),
    }
}

/// Coerce a value to i64.
fn coerce_to_int(val: &Value) -> Result<i64, EvalError> {
    match val {
        Value::Int(i) => Ok(*i),
        _ => Err(EvalError::TypeError {
            message: format!("cannot coerce {} to Int", val.type_name()),
        }),
    }
}

fn compare_bools(l: bool, r: bool, op: &str) -> Result<bool, EvalError> {
    match op {
        "=" => Ok(l == r),
        "!=" => Ok(l != r),
        _ => Err(EvalError::InvalidOperator { op: op.to_string() }),
    }
}

fn compare_ints(l: i64, r: i64, op: &str) -> Result<bool, EvalError> {
    match op {
        "=" => Ok(l == r),
        "!=" => Ok(l != r),
        "<" => Ok(l < r),
        "<=" => Ok(l <= r),
        ">" => Ok(l > r),
        ">=" => Ok(l >= r),
        _ => Err(EvalError::InvalidOperator { op: op.to_string() }),
    }
}

fn compare_decimals(l: Decimal, r: Decimal, op: &str) -> Result<bool, EvalError> {
    match op {
        "=" => Ok(l == r),
        "!=" => Ok(l != r),
        "<" => Ok(l < r),
        "<=" => Ok(l <= r),
        ">" => Ok(l > r),
        ">=" => Ok(l >= r),
        _ => Err(EvalError::InvalidOperator { op: op.to_string() }),
    }
}

fn compare_strings(l: &str, r: &str, op: &str) -> Result<bool, EvalError> {
    match op {
        "=" => Ok(l == r),
        "!=" => Ok(l != r),
        "<" => Ok(l < r),
        "<=" => Ok(l <= r),
        ">" => Ok(l > r),
        ">=" => Ok(l >= r),
        _ => Err(EvalError::InvalidOperator { op: op.to_string() }),
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn promote_int_to_decimal_basic() {
        let target = TypeSpec {
            base: "Decimal".to_string(),
            precision: Some(10),
            scale: Some(2),
            currency: None,
            min: None,
            max: None,
            max_length: None,
            values: None,
            fields: None,
            element_type: None,
            unit: None,
            variants: None,
        };
        let result = promote_int_to_decimal(42, &target).unwrap();
        assert_eq!(result, dec("42.00"));
    }

    #[test]
    fn promote_int_to_decimal_zero_scale() {
        let target = TypeSpec {
            base: "Decimal".to_string(),
            precision: Some(5),
            scale: Some(0),
            currency: None,
            min: None,
            max: None,
            max_length: None,
            values: None,
            fields: None,
            element_type: None,
            unit: None,
            variants: None,
        };
        let result = promote_int_to_decimal(123, &target).unwrap();
        assert_eq!(result, dec("123"));
    }

    #[test]
    fn eval_mul_basic() {
        let result = eval_mul(dec("10.50"), dec("3"), 10, 2).unwrap();
        assert_eq!(result, dec("31.50"));
    }

    #[test]
    fn eval_mul_overflow() {
        // precision=4, scale=2 means max integer part is 99
        let result = eval_mul(dec("50.00"), dec("3"), 4, 2);
        assert!(result.is_err());
        if let Err(EvalError::Overflow { .. }) = result {
            // expected
        } else {
            panic!("expected Overflow error");
        }
    }

    #[test]
    fn eval_mul_rounding_midpoint_nearest_even() {
        // Test MidpointNearestEven: 2.5 rounds to 2, 3.5 rounds to 4
        // (banker's rounding)
        let result = eval_mul(dec("2.5"), dec("1"), 10, 0).unwrap();
        assert_eq!(result, dec("2")); // 2.5 -> 2 (nearest even)

        let result = eval_mul(dec("3.5"), dec("1"), 10, 0).unwrap();
        assert_eq!(result, dec("4")); // 3.5 -> 4 (nearest even)
    }

    #[test]
    fn eval_int_mul_basic() {
        let result_type = TypeSpec {
            base: "Int".to_string(),
            precision: None,
            scale: None,
            currency: None,
            min: Some(0),
            max: Some(100),
            max_length: None,
            values: None,
            fields: None,
            element_type: None,
            unit: None,
            variants: None,
        };
        let result = eval_int_mul(5, 10, &result_type).unwrap();
        assert_eq!(result, Value::Int(50));
    }

    #[test]
    fn eval_int_mul_overflow_bounds() {
        let result_type = TypeSpec {
            base: "Int".to_string(),
            precision: None,
            scale: None,
            currency: None,
            min: Some(0),
            max: Some(100),
            max_length: None,
            values: None,
            fields: None,
            element_type: None,
            unit: None,
            variants: None,
        };
        let result = eval_int_mul(5, 25, &result_type);
        assert!(result.is_err());
    }

    #[test]
    fn compare_bool_eq() {
        let result = compare_values(&Value::Bool(true), &Value::Bool(true), "=", None).unwrap();
        assert!(result);
    }

    #[test]
    fn compare_bool_ne() {
        let result = compare_values(&Value::Bool(true), &Value::Bool(false), "!=", None).unwrap();
        assert!(result);
    }

    #[test]
    fn compare_int_lt() {
        let result = compare_values(&Value::Int(5), &Value::Int(10), "<", None).unwrap();
        assert!(result);
    }

    #[test]
    fn compare_int_gte() {
        let result = compare_values(&Value::Int(10), &Value::Int(10), ">=", None).unwrap();
        assert!(result);
    }

    #[test]
    fn compare_money_le() {
        let l = Value::Money {
            amount: dec("100.00"),
            currency: "USD".to_string(),
        };
        let r = Value::Money {
            amount: dec("200.00"),
            currency: "USD".to_string(),
        };
        let ct = TypeSpec {
            base: "Money".to_string(),
            precision: None,
            scale: None,
            currency: Some("USD".to_string()),
            min: None,
            max: None,
            max_length: None,
            values: None,
            fields: None,
            element_type: None,
            unit: None,
            variants: None,
        };
        let result = compare_values(&l, &r, "<=", Some(&ct)).unwrap();
        assert!(result);
    }

    #[test]
    fn compare_money_different_currencies() {
        let l = Value::Money {
            amount: dec("100.00"),
            currency: "USD".to_string(),
        };
        let r = Value::Money {
            amount: dec("100.00"),
            currency: "EUR".to_string(),
        };
        let result = compare_values(&l, &r, "=", None);
        assert!(result.is_err());
    }

    #[test]
    fn compare_enum_eq() {
        let result = compare_values(
            &Value::Enum("active".to_string()),
            &Value::Enum("active".to_string()),
            "=",
            None,
        )
        .unwrap();
        assert!(result);
    }

    #[test]
    fn compare_int_decimal_promotion() {
        // Int(100) compared to Decimal(99.50) with Decimal comparison_type
        let ct = TypeSpec {
            base: "Decimal".to_string(),
            precision: Some(9),
            scale: Some(2),
            currency: None,
            min: None,
            max: None,
            max_length: None,
            values: None,
            fields: None,
            element_type: None,
            unit: None,
            variants: None,
        };
        let result = compare_values(
            &Value::Int(100),
            &Value::Decimal(dec("99.50")),
            ">",
            Some(&ct),
        )
        .unwrap();
        assert!(result); // 100.00 > 99.50
    }

    #[test]
    fn compare_int_decimal_promotion_equal() {
        let ct = TypeSpec {
            base: "Decimal".to_string(),
            precision: Some(9),
            scale: Some(2),
            currency: None,
            min: None,
            max: None,
            max_length: None,
            values: None,
            fields: None,
            element_type: None,
            unit: None,
            variants: None,
        };
        let result = compare_values(
            &Value::Int(100),
            &Value::Decimal(dec("100.00")),
            "=",
            Some(&ct),
        )
        .unwrap();
        assert!(result);
    }

    #[test]
    fn compare_invalid_operator() {
        let result = compare_values(&Value::Int(1), &Value::Int(2), "~", None);
        assert!(result.is_err());
        if let Err(EvalError::InvalidOperator { op }) = result {
            assert_eq!(op, "~");
        }
    }

    // ──────────────────────────────────────
    // C1: Precision overflow tests (B1 fix)
    // ──────────────────────────────────────

    #[test]
    fn check_precision_28_scale_0_within_bounds() {
        // precision=28, scale=0: the exact case that panicked before.
        // Value 123 should fit within 28 integer digits.
        let result = check_precision(dec("123"), 28, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn check_precision_28_scale_0_exceeds_bounds() {
        // A very large value (but within Decimal range) that exceeds 28 digits
        // This is impossible in practice since Decimal max is ~28 digits,
        // but a value at 28 digits should be checked.
        // 10^28 - 1 = 9999999999999999999999999999 (28 nines) -- max valid integer
        let max_valid = Decimal::from_str_exact("9999999999999999999999999999").unwrap();
        let result = check_precision(max_valid, 28, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn check_precision_beyond_decimal_range_ok() {
        // precision=38 (beyond Decimal's 28-digit range): any Decimal value fits.
        let result = check_precision(dec("999999999"), 38, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn check_precision_normal_overflow() {
        // precision=4, scale=2 means max integer part is 99.
        // Value 100.00 exceeds it.
        let result = check_precision(dec("100.00"), 4, 2);
        assert!(result.is_err());
        match result {
            Err(EvalError::Overflow { .. }) => {} // expected
            other => panic!("expected Overflow, got {:?}", other),
        }
    }

    #[test]
    fn check_precision_20_scale_0_no_panic() {
        // precision=20, scale=0: would have panicked with 10i64.pow(20).
        // Should handle safely with Decimal arithmetic.
        let result = check_precision(dec("12345"), 20, 0);
        assert!(result.is_ok());
    }

    // ──────────────────────────────────────
    // C2: Money cross-currency promotion test (B2 fix)
    // ──────────────────────────────────────

    #[test]
    fn compare_money_cross_currency_with_promotion_fails() {
        // Money comparison via comparison_type with different currencies
        // should return TypeError even when amounts are equal.
        let l = Value::Money {
            amount: dec("100.00"),
            currency: "USD".to_string(),
        };
        let r = Value::Money {
            amount: dec("100.00"),
            currency: "EUR".to_string(),
        };
        let ct = TypeSpec {
            base: "Money".to_string(),
            precision: None,
            scale: None,
            currency: Some("USD".to_string()),
            min: None,
            max: None,
            max_length: None,
            values: None,
            fields: None,
            element_type: None,
            unit: None,
            variants: None,
        };
        let result = compare_values(&l, &r, "=", Some(&ct));
        assert!(result.is_err());
        match result {
            Err(EvalError::TypeError { message }) => {
                assert!(
                    message.contains("different currencies"),
                    "error should mention different currencies, got: {}",
                    message
                );
            }
            other => panic!("expected TypeError, got {:?}", other),
        }
    }

    #[test]
    fn compare_money_same_currency_with_promotion_succeeds() {
        let l = Value::Money {
            amount: dec("100.00"),
            currency: "USD".to_string(),
        };
        let r = Value::Money {
            amount: dec("200.00"),
            currency: "USD".to_string(),
        };
        let ct = TypeSpec {
            base: "Money".to_string(),
            precision: None,
            scale: None,
            currency: Some("USD".to_string()),
            min: None,
            max: None,
            max_length: None,
            values: None,
            fields: None,
            element_type: None,
            unit: None,
            variants: None,
        };
        let result = compare_values(&l, &r, "<", Some(&ct)).unwrap();
        assert!(result); // 100 < 200
    }

    // ──────────────────────────────────────
    // C7: Duration comparison tests (T9 fix)
    // ──────────────────────────────────────

    #[test]
    fn compare_duration_same_unit() {
        let l = Value::Duration {
            value: 30,
            unit: "seconds".to_string(),
        };
        let r = Value::Duration {
            value: 60,
            unit: "seconds".to_string(),
        };
        let result = compare_values(&l, &r, "<", None).unwrap();
        assert!(result); // 30 < 60

        let result_eq = compare_values(&l, &l, "=", None).unwrap();
        assert!(result_eq); // 30 == 30
    }

    #[test]
    fn compare_duration_cross_unit_fails() {
        let l = Value::Duration {
            value: 60,
            unit: "seconds".to_string(),
        };
        let r = Value::Duration {
            value: 1,
            unit: "minutes".to_string(),
        };
        let result = compare_values(&l, &r, "=", None);
        assert!(result.is_err());
        match result {
            Err(EvalError::TypeError { message }) => {
                assert!(
                    message.contains("different units"),
                    "error should mention different units, got: {}",
                    message
                );
            }
            other => panic!("expected TypeError, got {:?}", other),
        }
    }

    #[test]
    fn compare_duration_gt() {
        let l = Value::Duration {
            value: 3600,
            unit: "seconds".to_string(),
        };
        let r = Value::Duration {
            value: 1800,
            unit: "seconds".to_string(),
        };
        let result = compare_values(&l, &r, ">", None).unwrap();
        assert!(result); // 3600 > 1800
    }
}
