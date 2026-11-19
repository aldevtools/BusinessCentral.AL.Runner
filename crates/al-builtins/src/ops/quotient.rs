//! The quotient `/` of two numbers: one rule for a Decimal, an Integer, a BigInteger and a Duration's milliseconds.

use super::keep_18_digits;
use crate::describe::Describe;
use crate::native::{Signal, unsupported};
use crate::value::Value;
use rust_decimal::Decimal;

/// A quotient keeps 18 significant digits, rounded at the last one
/// (types/TestDateRoundFormatStringContracts.al:Decimal_Division_KeepsEighteenSignificantDigits), and carries no
/// trailing zeros (types/decimal-integer/ScT70056.al:Integer_DivisionResult `value 3.5`).
fn quotient(a: Decimal, b: Decimal) -> Result<Decimal, Signal> {
    a.checked_div(b)
        .and_then(|q| q.round_sf_with_strategy(18, rust_decimal::RoundingStrategy::MidpointAwayFromZero))
        .map(|q| q.normalize())
        .ok_or_else(|| Signal::Unsupported("decimal overflow".into()))
}

/// The quotient of `l` and `r` (their values `a` and `b`) where a side whose `keep_*` is set is a whole number of up to
/// 19 digits (a BigInteger, a Duration's milliseconds). Whether BC keeps such a side to 18 digits first is pinned only
/// where the quotient is the same either way (types/decimal-integer/DecimalDigits.al:309-310 `b62p1 / 3 back`, a
/// BigInteger); a quotient that differs refuses (issues 8349, 8423).
pub(super) fn whole_quotient(
    (l, a, keep_l): (&Value, Decimal, bool),
    (r, b, keep_r): (&Value, Decimal, bool),
) -> Result<Decimal, Signal> {
    let q = quotient(a, b)?;
    let kept = |keep: bool, d: Decimal| if keep { keep_18_digits(d) } else { d };
    let (ka, kb) = (kept(keep_l, a), kept(keep_r, b));
    if (ka, kb) != (a, b) && quotient(ka, kb)? != q {
        return unsupported(format!(
            "the quotient of {l} and {r}, a number of more than 18 digits (not pinned)",
            l = l.describe(),
            r = r.describe()
        ));
    }
    Ok(q)
}

#[cfg(test)]
mod tests {
    use super::super::{Arith, arith};
    use crate::native::Signal;
    use crate::value::Value;
    use rust_decimal::Decimal;

    const MAX: i64 = 9_223_372_036_854_775_807;
    const B62P1: i64 = 4_611_686_018_427_387_905;

    fn refused(r: Result<Value, Signal>) -> bool {
        matches!(r, Err(Signal::Unsupported(_)))
    }

    /// Issue 8423: a Duration of more than 18 digits divided by a number, where keeping the milliseconds to 18 digits first
    /// gives another quotient, refuses like the BigInteger (issue 8349); the divisor on either side of the 18 digits.
    #[test]
    fn a_duration_quotient_that_depends_on_the_kept_digits_refuses() {
        let d = |ms: i64, r: Value| arith(Arith::Div, &Value::Duration(ms), &r);
        assert!(refused(d(MAX, Value::Integer(2))));
        assert!(refused(d(-MAX, Value::Integer(2))));
        assert!(refused(d(MAX, Value::BigInteger(2))));
        assert!(refused(d(MAX, Value::Decimal(Decimal::from(2)))));
        assert!(refused(d(999_999_999_999_999_999, Value::Duration(B62P1))));
        assert!(refused(d(MAX, Value::Duration(2))));
    }

    /// The same quotient either way, and every Duration of 18 digits or fewer, keep main's answer.
    #[test]
    fn a_duration_quotient_the_kept_digits_do_not_change_keeps_its_value() {
        let d = |ms: i64, r: Value| arith(Arith::Div, &Value::Duration(ms), &r);
        // types/duration/D_Duration.al:354-370: 90000 ms / 7 and / 4.
        assert!(
            matches!(d(90_000, Value::Integer(7)), Ok(Value::Decimal(q)) if q.to_string() == "12857.1428571428571")
        );
        assert!(matches!(d(90_000, Value::Integer(4)), Ok(Value::Decimal(q)) if q == Decimal::from(22_500)));
        assert!(matches!(d(90_000, Value::Duration(30_000)), Ok(Value::Decimal(q)) if q == Decimal::from(3)));
        let third = Decimal::from(1_537_228_672_809_129_300_i64);
        assert!(matches!(d(B62P1, Value::Integer(3)), Ok(Value::Decimal(q)) if q == third));
        assert!(matches!(d(B62P1, Value::Decimal(Decimal::from(3))), Ok(Value::Decimal(q)) if q == third));
        // `div` and `mod` are not quotients of this rule: main's whole numbers.
        assert!(arith(Arith::IntDiv, &Value::Duration(MAX), &Value::Integer(2)).is_ok());
        assert!(arith(Arith::Mod, &Value::Duration(MAX), &Value::Integer(2)).is_ok());
    }

    /// A BigInteger of more than 18 digits divided by a number: the quotient is answered only where it does not depend
    /// on whether the BigInteger is first kept to 18 digits (types/decimal-integer/DecimalDigits.al:309-310 `b62p1 / 3 back`);
    /// `9223372036854775807L / 2` is not pinned (issue 8349) and refuses.
    #[test]
    fn a_biginteger_quotient_that_depends_on_the_kept_digits_refuses() {
        let q = |a: i64, b: i32| arith(Arith::Div, &Value::BigInteger(a), &Value::Integer(b));
        assert!(matches!(q(9_223_372_036_854_775_807, 2), Err(Signal::Unsupported(_))));
        assert!(matches!(q(-9_223_372_036_854_775_807, 2), Err(Signal::Unsupported(_))));
        let third = rust_decimal::Decimal::from(1_537_228_672_809_129_300_i64);
        assert!(matches!(q(4_611_686_018_427_387_905, 3), Ok(Value::Decimal(d)) if d == third));
        assert!(matches!(q(7, 2), Ok(Value::Decimal(d)) if d == rust_decimal::Decimal::new(35, 1)));
        // The BigInteger of 19 digits on the right side.
        let r = |a: i64| {
            arith(
                Arith::Div,
                &Value::BigInteger(a),
                &Value::BigInteger(4_611_686_018_427_387_905),
            )
        };
        assert!(matches!(r(999_999_999_999_999_999), Err(Signal::Unsupported(_))));
        assert!(matches!(r(500_000_000_000_000_000), Ok(Value::Decimal(_))));
    }
}
