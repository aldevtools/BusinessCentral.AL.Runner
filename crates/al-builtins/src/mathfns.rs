//! `Round` and `Power`. Only the forms a corpus test pins are computed; the rest is refused.

use crate::describe::Describe;
use crate::native::{Ctx, NativeFn, Signal};
use crate::value::Value;
use rust_decimal::{Decimal, RoundingStrategy};

pub const REGISTRY: &[(&str, NativeFn)] = &[("System.Round#1", round), ("System.Power#1", power)];

type Out = Result<Value, Signal>;

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

fn dec(v: &Value) -> Option<Decimal> {
    match v {
        Value::Decimal(d) => Some(*d),
        Value::Integer(i) => Some(Decimal::from(*i)),
        Value::BigInteger(i) => Some(Decimal::from(*i)),
        Value::Byte(b) => Some(Decimal::from(*b)),
        _ => None,
    }
}

/// `Round(Number, Precision, Direction)`: the nearest multiple of `Precision`, halves away from
/// zero (session/TestSystemComplete.al:88-113, types/TestArithmeticContracts.al:126-130);
/// `'='` is the default direction (TestArithmeticContracts.al:108-115), `'>'` rounds away from zero and
/// `'<'` towards it, on negative numbers too (TestArithmeticContracts.al:82-97,
/// types/TestDateRoundFormatStringContracts.al:Round_DefaultPrecision_Directions_AndPrecisions). The
/// default precision is 0.01 (same test). A precision of 0 or -1 raises (types/TestProbeScalarContracts.al:477-528),
/// a smaller one is refused.
///
/// A Duration rounds its milliseconds the same way and gives a Decimal (types/lt-bc-round-duration/BcRoundDuration.al:51-143: zero,
/// positive and negative durations by 1, 1000 with each direction, 0.5 and 60000; no precision; the result type). Its errors are
/// pinned for a precision of 0 and -1000 (shown as `1,000`) and the direction `x` (:91-109).
fn round(_: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    let duration = matches!(a[0], Value::Duration(_));
    let number = match &a[0] {
        Value::Duration(ms) => Decimal::from(*ms),
        v => match dec(v) {
            Some(n) => n,
            None => return unsupported(format!("Round of {}", a[0].describe())),
        },
    };
    let precision = match a.get(1) {
        None => Decimal::new(1, 2),
        Some(p) => match dec(p) {
            Some(p) => p,
            None => return unsupported(format!("Round with a Precision of {}", p.describe())),
        },
    };
    // types/TestProbeScalarContracts.al:Round_PrecisionAndPower, `round_prec_neg` (-1) and `round_prec_zero` (0):
    // the shown "current value" of -1 is 1; other non-positive precisions are not measured.
    if precision <= Decimal::ZERO {
        let shown = if precision.is_zero() {
            "0"
        } else if precision == Decimal::NEGATIVE_ONE && !duration {
            "1"
        } else if precision == Decimal::from(-1000) && duration {
            "1,000"
        } else {
            return unsupported("Round with a Precision below -1 (not pinned)");
        };
        return crate::native::raise(
            format!(
                "The value of ROUND parameter 2 is outside of the permitted range. The current value is: {shown}. The permitted range is: from 1 to 2,147,483,647."
            ),
            "OutsidePermittedRange",
        );
    }
    let direction = match a.get(2) {
        None => "=".to_string(),
        Some(v) => crate::native::text_of(v).unwrap_or_default().to_string(),
    };
    let strategy = match direction.as_str() {
        "=" => RoundingStrategy::MidpointAwayFromZero,
        ">" => RoundingStrategy::AwayFromZero,
        "<" => RoundingStrategy::ToZero,
        "x" if duration => return crate::native::raise("The supplied value is not valid.", "InvalidValue"),
        other => return unsupported(format!("Round with direction `{other}` (not pinned)")),
    };
    let Some(steps) = number.checked_div(precision) else {
        return unsupported("Round outside the Decimal range");
    };
    let whole = steps.round_dp_with_strategy(0, strategy);
    match whole.checked_mul(precision) {
        Some(r) => Ok(Value::Decimal(r.normalize_to(precision))),
        None => unsupported("Round outside the Decimal range"),
    }
}

trait NormalizeTo {
    fn normalize_to(self, precision: Decimal) -> Decimal;
}

impl NormalizeTo for Decimal {
    /// Drops trailing zeros beyond the precision's own scale (`2.50` stays `2.50` for 0.01, `3` for 1).
    fn normalize_to(mut self, precision: Decimal) -> Decimal {
        let keep = precision.normalize().scale();
        if self.scale() > keep {
            self.rescale(keep);
        }
        self
    }
}

/// A result that no Decimal holds: a too large result, zero to a negative power, a negative base to a fractional power
/// (types/TestProbeScalarContracts.al:Round_PrecisionAndPower, `power_2_100`, `power_0_neg1`, `power_neg2_0_5`, `power_2_1000`).
fn overflow() -> Out {
    crate::native::raise("Value was either too large or too small for a Decimal.", "AL:Overflow")
}

/// `Power(Number, Power)` with a whole exponent: `Power(2, 3)`, `(5, 0)`, `(1, 100)`, `(0, 0) = 1`,
/// `(2, -1) = 0.5`, `(2, 10)` (session/TestSystemComplete.al:124-144,
/// types/TestArithmeticContracts.al:46-73, types/TestDateRoundFormatStringContracts.al:Power_NonWholeAndNegativeExponents:
/// `(-8, 3) = -512`). Any other exponent is computed in floating point and read back as a Decimal
/// of 15 significant digits, as .NET converts a double (`Power(2, 0.5)` is `1.4142135623731`, same
/// test); a result that is not a finite number is refused, as is zero to a negative power.
fn power(_: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    let (Some(base), Some(exp)) = (dec(&a[0]), dec(&a[1])) else {
        return unsupported(format!("Power of {} and {}", a[0].describe(), a[1].describe()));
    };
    if exp.fract() != Decimal::ZERO || exp.abs() > Decimal::from(10_000) {
        use rust_decimal::prelude::ToPrimitive;
        let (Some(b), Some(e)) = (base.to_f64(), exp.to_f64()) else {
            return overflow();
        };
        let r = b.powf(e);
        if !r.is_finite() {
            return overflow();
        }
        let Some(d) = Decimal::from_f64_retain(r)
            .and_then(|d| d.round_sf_with_strategy(15, RoundingStrategy::MidpointNearestEven))
        else {
            return overflow();
        };
        return Ok(Value::Decimal(d.normalize()));
    }
    use rust_decimal::prelude::ToPrimitive;
    let n = exp.trunc().to_i64().unwrap_or(i64::MAX);
    let mut result = Decimal::ONE;
    for _ in 0..n.unsigned_abs() {
        result = match result.checked_mul(base) {
            Some(r) => r,
            None => return overflow(),
        };
    }
    if n < 0 {
        result = match Decimal::ONE.checked_div(result) {
            Some(r) => r,
            None => return overflow(),
        };
    }
    // A result keeps 18 significant digits: `Power(2, 62)` into a BigInteger is 4611686018427387900
    // (misc/lt-system-functions/Round.al:LT_BU_Power_LargeResultPrecision, misc/lt-cq-odd-leads/CqOddLeads.al:419-446).
    Ok(Value::Decimal(crate::ops::keep_18_digits(result.normalize())))
}

#[cfg(test)]
mod tests {
    use crate::native::Signal;
    use crate::native::tests::{call, t};
    use crate::value::Value;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn d(s: &str) -> Value {
        Value::Decimal(Decimal::from_str(s).unwrap())
    }
    fn is_dec(r: Result<Value, Signal>, want: &str) {
        match r {
            Ok(Value::Decimal(got)) => assert_eq!(got, Decimal::from_str(want).unwrap(), "got {got}"),
            other => panic!("{other:?}"),
        }
    }
    #[track_caller]
    fn refused(r: Result<Value, Signal>) {
        assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?} should be a refusal");
    }

    #[test]
    fn round_half_away_from_zero() {
        // session/TestSystemComplete.al:88,100,112; types/TestArithmeticContracts.al:126-130.
        is_dec(call("System.Round#1", vec![d("2.5"), Value::Integer(1)]), "3");
        is_dec(call("System.Round#1", vec![d("-2.5"), Value::Integer(1)]), "-3");
        is_dec(call("System.Round#1", vec![d("3.14159"), d("0.01")]), "3.14");
        // record/TestSalesQuoteVatRounding.al:45-47.
        is_dec(call("System.Round#1", vec![d("8.437"), d("0.01")]), "8.44");
        is_dec(call("System.Round#1", vec![d("7.337"), d("0.01")]), "7.34");
        is_dec(call("System.Round#1", vec![d("14.3454"), d("0.01")]), "14.35");
    }

    #[test]
    fn round_directions() {
        // types/TestArithmeticContracts.al:82-115.
        is_dec(call("System.Round#1", vec![d("2.5"), Value::Integer(1), t(">")]), "3");
        is_dec(call("System.Round#1", vec![d("2.5"), Value::Integer(1), t("<")]), "2");
        is_dec(call("System.Round#1", vec![d("2.6"), Value::Integer(1), t("=")]), "3");
        is_dec(call("System.Round#1", vec![d("2.4"), Value::Integer(1), t("=")]), "2");
        is_dec(call("System.Round#1", vec![d("2.5"), Value::Integer(1), t("=")]), "3");
    }

    #[test]
    fn round_refuses_what_the_corpus_does_not_pin() {
        refused(call("System.Round#1", vec![d("2.5"), Value::Integer(1), t("x")]));
    }

    fn raised(r: Result<Value, Signal>, message: &str, code: &str) {
        match r {
            Err(Signal::Error(e)) => assert_eq!((e.message.as_str(), e.code.as_str()), (message, code)),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn round_and_power_raise_what_the_probe_pins() {
        // types/TestProbeScalarContracts.al:Round_PrecisionAndPower.
        raised(
            call("System.Round#1", vec![d("2.5"), Value::Integer(0)]),
            "The value of ROUND parameter 2 is outside of the permitted range. The current value is: 0. The permitted range is: from 1 to 2,147,483,647.",
            "OutsidePermittedRange",
        );
        let overflow = "Value was either too large or too small for a Decimal.";
        raised(
            call("System.Power#1", vec![Value::Integer(0), Value::Integer(-1)]),
            overflow,
            "AL:Overflow",
        );
        raised(
            call("System.Power#1", vec![Value::Integer(10), Value::Integer(100)]),
            overflow,
            "AL:Overflow",
        );
        raised(
            call("System.Power#1", vec![Value::Integer(2), Value::Integer(10_001)]),
            overflow,
            "AL:Overflow",
        );
    }

    #[test]
    fn power_with_a_whole_exponent() {
        let p = |a: i32, b: i32| call("System.Power#1", vec![Value::Integer(a), Value::Integer(b)]);
        is_dec(p(2, 3), "8");
        is_dec(p(5, 0), "1");
        is_dec(p(1, 100), "1");
        is_dec(p(0, 0), "1");
        is_dec(p(2, -1), "0.5");
        is_dec(p(2, 10), "1024");
        is_dec(call("System.Power#1", vec![d("1.5"), Value::Integer(2)]), "2.25");
    }

    // Mutation-testing additions (docs/status.md debt).

    #[test]
    fn round_takes_every_whole_number_type() {
        is_dec(
            call("System.Round#1", vec![Value::BigInteger(25), Value::Integer(10)]),
            "30",
        );
        is_dec(call("System.Round#1", vec![Value::Byte(7), Value::Integer(5)]), "5");
        is_dec(
            call("System.Round#1", vec![Value::Integer(7), Value::BigInteger(5)]),
            "5",
        );
        is_dec(call("System.Round#1", vec![Value::Integer(7), Value::Byte(5)]), "5");
    }

    #[test]
    fn a_direction_on_zero_is_not_a_negative_number() {
        // The refusal is for numbers below zero only; zero rounds like any other number.
        is_dec(call("System.Round#1", vec![d("0"), Value::Integer(1), t(">")]), "0");
        is_dec(call("System.Round#1", vec![d("0"), Value::Integer(1), t("<")]), "0");
    }

    #[test]
    fn the_result_keeps_the_scale_of_the_precision_and_no_more() {
        let scale = |r: Result<Value, Signal>| match r {
            Ok(Value::Decimal(d)) => d.scale(),
            other => panic!("{other:?}"),
        };
        // 2.50 for a precision of 0.01, 3 for 1, and a precision written 0.50 counts as 0.5.
        assert_eq!(scale(call("System.Round#1", vec![d("2.5"), d("0.01")])), 2);
        assert_eq!(scale(call("System.Round#1", vec![d("2.5"), Value::Integer(1)])), 0);
        assert_eq!(scale(call("System.Round#1", vec![d("1.4"), d("0.50")])), 1);
        is_dec(call("System.Round#1", vec![d("1.4"), d("0.50")]), "1.5");
    }

    #[test]
    fn power_exponent_limit_and_the_scale_of_a_written_exponent() {
        let p = |a: &str, b: Value| call("System.Power#1", vec![d(a), b]);
        // Up to 10000 the product is exact; past it the result comes from a double, and one that
        // is not a finite number is refused.
        is_dec(p("1", Value::Integer(10_000)), "1");
        is_dec(p("1", Value::Integer(10_001)), "1");
        raised(
            p("2", Value::Integer(10_001)),
            "Value was either too large or too small for a Decimal.",
            "AL:Overflow",
        );
    }

    // Found by mutation testing: an exponent written with a scale (3.0, or 6 / 2 computed as 3.0) was
    // divided by ten once too often, so Power(2, 3.0) answered 1.
    #[test]
    fn power_exponent_written_with_a_scale_is_still_a_whole_number() {
        let p = |a: &str, b: Value| call("System.Power#1", vec![d(a), b]);
        is_dec(p("2", d("3.0")), "8");
        is_dec(p("2", d("-2.00")), "0.25");
    }
}
