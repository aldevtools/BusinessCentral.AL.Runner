//! A Decimal `+`, `-` or `*` with a Decimal or an Integer: its result of 18 significant digits, or, past the largest
//! Decimal, BC's overflow error for the one pinned shape (issue 9267). Entry points: `sum_difference_product`.
//!
//! BC raises `Value was either too large or too small for a Decimal.` (AL:Overflow) for the largest Decimal times the
//! Integer 10 (errors/PnBErrorsCollections.al:564-566, :585-586 `decimalOverflow`). Only that shape is pinned: a
//! positive Decimal on the left times a positive Integer on the right. Every other overflow (`+`, `-`, a Decimal on
//! both sides, the Integer on the left, a negative operand) is not pinned and refuses. So does a product whose whole
//! part times the Integer is not past the largest Decimal: `checked_mul` fails on some products that fit
//! (`7922816251426433759354395033.5 * 10` is the largest Decimal), so its failure alone is no overflow.

use super::{Arith, keep_18_digits};
use crate::native::Signal;
use crate::value::Value;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

/// `l op r` for `+`, `-` or `*` where `l` and `r` (values `a` and `b`) are a Decimal and a Decimal or an Integer: the
/// fast path of `ops::arith`. The end of `arith_general` gives the same results and refuses every overflow.
#[inline]
pub(super) fn sum_difference_product(
    op: Arith,
    (l, a): (&Value, Decimal),
    (r, b): (&Value, Decimal),
) -> Result<Value, Signal> {
    match op {
        Arith::Add => a.checked_add(b),
        Arith::Sub => a.checked_sub(b),
        _ => a.checked_mul(b),
    }
    .map_or_else(
        || past_decimal_range(op, l, r),
        |n| Ok(Value::Decimal(keep_18_digits(n))),
    )
}

/// The answer to `l op r` when `checked_mul`, `checked_add` or `checked_sub` found no Decimal for it.
fn past_decimal_range(op: Arith, l: &Value, r: &Value) -> Result<Value, Signal> {
    // The whole part of the Decimal times the Integer is already past the largest Decimal (below 2^96 times below 2^31,
    // so it fits a u128).
    let whole_part_past = |d: &Decimal, n: i32| {
        let (Some(whole), Some(max)) = (d.trunc().to_u128(), Decimal::MAX.to_u128()) else {
            return false;
        };
        u128::try_from(n)
            .ok()
            .and_then(|n| whole.checked_mul(n))
            .is_some_and(|p| p > max)
    };
    let pinned = match (op, l, r) {
        (Arith::Mul, Value::Decimal(d), Value::Integer(n)) => d.is_sign_positive() && *n > 0 && whole_part_past(d, *n),
        _ => false,
    };
    if pinned {
        return crate::mathfns::overflow();
    }
    Err(Signal::Unsupported("decimal overflow".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refused(v: Result<Value, Signal>) -> bool {
        matches!(v, Err(Signal::Unsupported(m)) if m == "decimal overflow")
    }

    #[test]
    fn only_a_positive_decimal_times_a_positive_integer_raises() {
        let (max, min) = (Value::Decimal(Decimal::MAX), Value::Decimal(Decimal::MIN));
        let ten = Value::Integer(10);
        match past_decimal_range(Arith::Mul, &max, &ten) {
            Err(Signal::Error(e)) => assert_eq!(
                (e.message.as_str(), e.code.as_str()),
                ("Value was either too large or too small for a Decimal.", "AL:Overflow")
            ),
            other => panic!("{other:?}"),
        }
        // Each fact flipped on its own refuses: the operator, the operand types and order, the signs.
        assert!(refused(past_decimal_range(Arith::Add, &max, &max)));
        assert!(refused(past_decimal_range(Arith::Sub, &min, &max)));
        assert!(refused(past_decimal_range(
            Arith::Mul,
            &max,
            &Value::Decimal(Decimal::TEN)
        )));
        assert!(refused(past_decimal_range(Arith::Mul, &ten, &max)));
        assert!(refused(past_decimal_range(Arith::Mul, &min, &ten)));
        assert!(refused(past_decimal_range(Arith::Mul, &max, &Value::Integer(-10))));
    }

    #[test]
    fn a_product_that_checked_mul_misses_but_may_fit_refuses() {
        // 7922816251426433759354395033.5 * 10 is the largest Decimal: `checked_mul` fails on it, and it is no overflow.
        let near = Value::Decimal("7922816251426433759354395033.5".parse().unwrap());
        assert!(refused(past_decimal_range(Arith::Mul, &near, &Value::Integer(10))));
    }
}
