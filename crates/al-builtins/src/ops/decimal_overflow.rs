//! A Decimal at the limit of the type: the overflow error of `+`, `-`, `*` and `/` past it, the 18 significant digits
//! a sum or a literal keeps at it, and the range of a record field it is assigned to (issue 9267). Entry points:
//! `sum_difference_product`, `past_decimal_range`, `literal_at_limit`, `held_at_limit`, `field_assignment_range`.
//!
//! Pinned: the literal 79228162514264337593543950335.0 (the largest .NET Decimal) holds [`MAX18`]
//! (json/JsonValueSetters.al:161-165, :193 `decimal_big`), and that literal times the Integer 10 raises
//! `Value was either too large or too small for a Decimal.` (AL:Overflow; errors/PnBErrorsCollections.al:564-566,
//! :585-586 `decimalOverflow`). Corpus pull request 1630 (merged as 3eeda76e, not in the pinned corpus yet;
//! tests/al-language/types/decimal-integer/DecimalOverflowCells.al) measures the rest on every supported major. It runs
//! through three assumption entries:
//! - `decimal-overflow-past-limit`: the overflow error for the operand types and sizes its cells measure
//!   ([`measured_overflow`]). Every other result past the range keeps main's refusal.
//! - `decimal-limit-rounds-down`: a value whose 18-digit round-up passes the largest .NET Decimal holds the largest
//!   18-digit Decimal instead, for a literal and for a sum or difference of two Decimals of 18 digits
//!   ([`literal_at_limit`], [`sum_at_limit`]). A product there keeps main's value.
//! - `decimal-field-assign-range`: the assignment of a Decimal outside +-999,999,999,999,999.99 to a Decimal field
//!   raises the range error ([`field_assignment_range`]).

mod exact;

use super::{Arith, keep_18_digits, over_18_digits};
use crate::assumptions::{Assumption, assume};
use crate::native::Signal;
use crate::value::Value;
use rust_decimal::{Decimal, RoundingStrategy};

/// The largest Decimal of 18 significant digits, 79228162514264337500000000000: the value BC holds for the literal
/// 79228162514264337593543950335.0 (json/JsonValueSetters.al:161-165, :193 `decimal_big`).
const MAX18: Decimal = Decimal::from_parts(0x3858_9800, 0xffff_ffea, 0xffff_ffff, false, 0);

/// Twice [`MAX18`]: the smallest sum (`add_max_max`, `sub_negmax_max`) and quotient (`max_div_half`) whose overflow a
/// cell measures.
const TWICE_MAX18: u128 = 158_456_325_028_528_675_000_000_000_000;

/// The smallest product whose overflow a cell measures: 7922816251426433760000000000 * 10 (`literal_29digits_times_10`),
/// the Decimal of 18 digits next above [`MAX18`].
const FIRST_PRODUCT: u128 = 79_228_162_514_264_337_600_000_000_000;

/// `l op r` for `+`, `-` or `*` where `l` and `r` (values `a` and `b`) are a Decimal and a Decimal or an Integer: the
/// fast path of `ops::arith`. The end of `arith_general` gives the same results.
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
        || Err(past_decimal_range(op, (l, a), (r, b))),
        |n| {
            Ok(Value::Decimal(match op {
                Arith::Mul => keep_18_digits(n),
                _ => kept_sum(l, r, n),
            }))
        },
    )
}

/// A sum or difference kept to 18 significant digits, as `keep_18_digits` keeps it, and at the limit as
/// [`sum_at_limit`] says.
#[inline]
fn kept_sum(l: &Value, r: &Value, n: Decimal) -> Decimal {
    if !over_18_digits(n) {
        return n;
    }
    sum_at_limit(l, r, keep_18_digits(n))
}

/// `k`, a sum that `keep_18_digits` kept. Where its round-up would pass the largest .NET Decimal, `k` still has more
/// than 18 digits. For two Decimal operands of 18 digits it is then [`MAX18`] with its sign, with no error
/// (DecimalOverflowCells.al `plus_50000000000`, `plus_93543950335` up to the exact .NET maximum,
/// `negative_minus_50000000000`). Any other operands keep main's value.
#[cold]
#[inline(never)]
fn sum_at_limit(l: &Value, r: &Value, k: Decimal) -> Decimal {
    let measured = matches!((l, r), (Value::Decimal(a), Value::Decimal(b)) if digits_18(*a) && digits_18(*b));
    if digits_18(k) || !measured {
        return k;
    }
    rounded_down(k)
}

/// The Decimal that a literal holds when its 18-digit round-up passes the largest .NET Decimal: [`MAX18`]. Pinned for
/// the literal of the largest .NET Decimal (json/JsonValueSetters.al:161-165, :193 `decimal_big`); every other literal
/// there is assumed (DecimalOverflowCells.al `literal_19_digits` 79228162514264337560000000000). al-vm calls it only
/// there.
pub fn literal_at_limit(d: Decimal) -> Decimal {
    if d == Decimal::MAX {
        return MAX18;
    }
    rounded_down(d)
}

/// Whether `v` is a Decimal of [`MAX18`]'s size, the value a literal at the limit holds. al-vm does not cache such a
/// constant, so an assumption it runs through is marked on every run.
pub fn held_at_limit(v: &Value) -> bool {
    matches!(v, Value::Decimal(d) if d.abs() == MAX18)
}

/// Whether `d` has at most 18 significant digits, the trailing zeros of a whole number not counted (`over_18_digits`
/// counts them: MAX18 has 29 digits for it).
fn digits_18(d: Decimal) -> bool {
    let mut m = d.mantissa().unsigned_abs();
    while m != 0 && m % 10 == 0 {
        m /= 10;
    }
    m <= 999_999_999_999_999_999
}

/// `d` cut to 18 significant digits toward zero, under the assumption `decimal-limit-rounds-down`.
fn rounded_down(d: Decimal) -> Decimal {
    assume(Assumption::DecimalLimitRoundsDown);
    d.round_sf_with_strategy(18, RoundingStrategy::ToZero).unwrap_or(d)
}

/// The error of `Rec.Field := d` for a Decimal field of a record that is not temporary: BC checks the field's range on
/// the assignment and raises the range error of `Format` (`format::decimal_range_error`) for a value outside
/// +-999,999,999,999,999.99, so a later `+=` on the field does not run (DecimalOverflowCells.al codeunit 73592
/// FieldRange_Cells `field_assign_max`, `field_assign_1e15`, and `field_assign_below_1e15` taken; codeunit 73591
/// `field_plusassign_max_max`, `field_minusassign_negmax_max` for -MAX). A value whose range error text no pin shows
/// keeps main's store.
pub fn field_assignment_range(d: Decimal) -> Result<(), Signal> {
    if d.abs() <= FIELD_MAX {
        return Ok(());
    }
    field_out_of_range(d)
}

/// The largest Decimal a record field takes, 999,999,999,999,999.99.
const FIELD_MAX: Decimal = Decimal::from_parts(0x5d89_ffff, 0x0163_4578, 0, false, 2);

#[cold]
#[inline(never)]
fn field_out_of_range(d: Decimal) -> Result<(), Signal> {
    match crate::format::decimal_range_error(&d) {
        Some(crate::format::FormatFail::Error { message, code }) => {
            assume(Assumption::DecimalFieldAssignRange);
            crate::native::raise(message, &code)
        }
        _ => Ok(()),
    }
}

/// The signal of `l op r` (values `a` and `b`, a BigInteger beside a Decimal already kept to 18 digits) when
/// `checked_add`, `checked_sub`, `checked_mul` or `checked_div` found no Decimal for it: BC's overflow error for a
/// shape a cell measures, else main's refusal.
#[cold]
#[inline(never)]
pub(super) fn past_decimal_range(op: Arith, (l, a): (&Value, Decimal), (r, b): (&Value, Decimal)) -> Signal {
    // The pinned cell: MAX18 (what the largest .NET Decimal literal holds) times the Integer 10
    // (errors/PnBErrorsCollections.al:564-566).
    let pinned = matches!((op, l, r), (Arith::Mul, Value::Decimal(d), Value::Integer(10)) if *d == MAX18);
    if pinned || measured_overflow(op, (l, a), (r, b)) {
        if !pinned {
            assume(Assumption::DecimalOverflowPastLimit);
        }
        return match crate::mathfns::overflow() {
            Err(s) => s,
            Ok(_) => unreachable!("the overflow error is an error"),
        };
    }
    Signal::Unsupported("decimal overflow".into())
}

/// The shapes of DecimalOverflowCells.al that raise AL:Overflow, by equality on each fact:
/// - the (operator, operand types, operand signs) triples its cells measure, one cell each in [`measured_triple`];
/// - every operand of 18 significant digits or fewer, as a Decimal variable holds it;
/// - the exact result's size at least the smallest one a cell measures for that operator: [`TWICE_MAX18`] for `+`, `-`
///   and `/`, [`FIRST_PRODUCT`] for `*` (`mul_4e19_2e9`, `bigmax_times_1e10`, `mul_max_2` and the larger products lie
///   above it).
///
/// Not measured, so main's refusal: a result past the .NET maximum below that size (`D + 93543950336` with D the
/// largest 18-digit Decimal, a quotient just past the limit), an operand of more than 18 digits (whose 18-digit value
/// may give another result), and every other triple (a negative quotient, a negative sum, a positive difference, a
/// negative operand beside a Decimal, a BigInteger or an Integer on the left of a Decimal).
fn measured_overflow(op: Arith, (l, a): (&Value, Decimal), (r, b): (&Value, Decimal)) -> bool {
    if !measured_triple(op, l, r) || !digits_18(a) || !digits_18(b) {
        return false;
    }
    match op {
        Arith::Add | Arith::Sub => exact::sum_at_least(a, b, TWICE_MAX18),
        Arith::Mul => exact::product_at_least(a, b, FIRST_PRODUCT),
        _ => exact::quotient_at_least(a, b, TWICE_MAX18),
    }
}

/// Whether a cell of DecimalOverflowCells.al measures the overflow of `l op r` for this operator, these operand types
/// and these operand signs (`+` for a value at or above zero).
fn measured_triple(op: Arith, l: &Value, r: &Value) -> bool {
    use Value as V;
    let sign = |v: &Value| match v {
        V::Decimal(d) => !d.is_sign_negative(),
        V::Integer(n) => *n >= 0,
        V::BigInteger(n) => *n >= 0,
        _ => false,
    };
    let (pl, pr) = (sign(l), sign(r));
    match (op, l, r) {
        // `add_max_max`, `plusassign_max_max`, `field_add_max_max`, `argument_add_max_max`: (+, +).
        (Arith::Add, V::Decimal(_), V::Decimal(_)) => pl && pr,
        // `sub_negmax_max`, `minusassign_negmax_max`, `field_sub_negmax_max`: (-, +).
        (Arith::Sub, V::Decimal(_), V::Decimal(_)) => !pl && pr,
        // Every sign pair: `mul_max_10` (+, +), `mul_negmax_10` (-, +), `mul_max_neg10` (+, -), `mul_negmax_neg10` (-, -).
        (Arith::Mul, V::Decimal(_), V::Integer(_)) => true,
        // `mul_max_dec10`, `bigmax_times_1e10`: (+, +).
        (Arith::Mul, V::Decimal(_), V::Decimal(_)) => pl && pr,
        // `mul_10_max`: (+, +).
        (Arith::Mul, V::Integer(_), V::Decimal(_)) => pl && pr,
        // `mul_max_big10`, `mul_1e19_bigmax`: (+, +).
        (Arith::Mul, V::Decimal(_), V::BigInteger(_)) => pl && pr,
        // codeunit 73592 `max_div_half`, `1e28_div_001`: (+, +).
        (Arith::Div, V::Decimal(_), V::Decimal(_)) => pl && pr,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assumptions::take_hits;

    fn d(s: &str) -> Decimal {
        s.parse().unwrap()
    }

    fn dec(s: &str) -> Value {
        Value::Decimal(d(s))
    }

    /// `l op r` through `ops::arith`, the way the VM runs it.
    fn run(op: Arith, l: &Value, r: &Value) -> Result<Value, Signal> {
        super::super::arith(op, l, r)
    }

    fn overflow(v: Result<Value, Signal>) -> bool {
        matches!(&v, Err(Signal::Error(e))
            if e.message == "Value was either too large or too small for a Decimal." && e.code == "AL:Overflow")
    }

    fn refused(v: Result<Value, Signal>) -> bool {
        matches!(v, Err(Signal::Unsupported(m)) if m == "decimal overflow")
    }

    #[test]
    fn the_largest_18_digit_decimal_is_the_literal_value() {
        assert_eq!(MAX18, d("79228162514264337500000000000"));
        assert_eq!(Decimal::from_i128_with_scale(TWICE_MAX18 as i128 / 2, 0), MAX18);
        assert_eq!(literal_at_limit(Decimal::MAX), MAX18);
        assert!(!take_hits().contains(&Assumption::DecimalLimitRoundsDown));
        assert_eq!(literal_at_limit(d("79228162514264337560000000000")), MAX18);
        assert!(take_hits().contains(&Assumption::DecimalLimitRoundsDown));
    }

    #[test]
    fn a_field_takes_up_to_its_range() {
        assert_eq!(FIELD_MAX, d("999999999999999.99"));
        assert!(field_assignment_range(d("999999999999999.99")).is_ok());
        assert!(field_assignment_range(d("-999999999999999.99")).is_ok());
        assert!(!take_hits().contains(&Assumption::DecimalFieldAssignRange));
        for (v, shown) in [
            (MAX18, "79,228,162,514,264,337,500,000,000,000"),
            (-MAX18, "-79,228,162,514,264,337,500,000,000,000"),
            (d("1000000000000000"), "1,000,000,000,000,000"),
        ] {
            match field_assignment_range(v) {
                Err(Signal::Error(e)) => {
                    assert_eq!(e.code, "OutsidePermittedRange");
                    assert!(
                        e.message.contains(&format!("The current value is: {shown}.")),
                        "{}",
                        e.message
                    );
                }
                other => panic!("{other:?}"),
            }
        }
        assert!(take_hits().contains(&Assumption::DecimalFieldAssignRange));
        // A value whose range error text no pin shows (below 1,000,000,000,000,000 when kept to 18 digits): main's store.
        assert!(field_assignment_range(d("999999999999999.995")).is_ok());
    }

    #[test]
    fn the_pinned_cell_raises_without_the_assumption() {
        assert!(overflow(run(Arith::Mul, &Value::Decimal(MAX18), &Value::Integer(10))));
        assert!(!take_hits().contains(&Assumption::DecimalOverflowPastLimit));
        // Each fact flipped runs through the assumption: another Integer, a Decimal 10, the Integer on the left.
        assert!(overflow(run(Arith::Mul, &Value::Decimal(MAX18), &Value::Integer(11))));
        assert!(take_hits().contains(&Assumption::DecimalOverflowPastLimit));
        assert!(overflow(run(Arith::Mul, &Value::Decimal(MAX18), &dec("10"))));
        assert!(take_hits().contains(&Assumption::DecimalOverflowPastLimit));
        assert!(overflow(run(Arith::Mul, &Value::Integer(10), &Value::Decimal(MAX18))));
        assert!(take_hits().contains(&Assumption::DecimalOverflowPastLimit));
    }

    #[test]
    fn each_measured_overflow_raises() {
        let max = Value::Decimal(MAX18);
        let neg = Value::Decimal(-MAX18);
        let ten = Value::Integer(10);
        assert!(overflow(run(Arith::Add, &max, &max)));
        assert!(overflow(run(Arith::Sub, &neg, &max)));
        assert!(overflow(run(Arith::Mul, &max, &ten)));
        assert!(overflow(run(Arith::Mul, &max, &Value::Integer(2))));
        assert!(overflow(run(Arith::Mul, &ten, &max)));
        assert!(overflow(run(Arith::Mul, &max, &dec("10"))));
        assert!(overflow(run(Arith::Mul, &neg, &Value::Integer(-10))));
        assert!(overflow(run(Arith::Mul, &max, &Value::BigInteger(10))));
        assert!(overflow(run(Arith::Mul, &dec("1E19"), &Value::BigInteger(i64::MAX))));
        assert!(overflow(run(
            Arith::Mul,
            &dec("40000000000000000000"),
            &Value::Integer(2_000_000_000)
        )));
        assert!(overflow(run(Arith::Mul, &dec("7922816251426433760000000000"), &ten)));
        assert!(overflow(run(
            Arith::Mul,
            &dec("9223372036854775810"),
            &dec("10000000000")
        )));
        assert!(overflow(run(Arith::Div, &max, &dec("0.5"))));
        assert!(overflow(run(Arith::Div, &dec("1E28"), &dec("0.01"))));
        assert!(take_hits().contains(&Assumption::DecimalOverflowPastLimit));
    }

    #[test]
    fn unmeasured_results_past_the_range_keep_the_refusal() {
        let max = Value::Decimal(MAX18);
        // Past the .NET maximum, below twice the largest 18-digit Decimal.
        assert!(refused(run(Arith::Add, &max, &dec("93543950336"))));
        assert!(refused(run(Arith::Sub, &Value::Decimal(-MAX18), &dec("93543950336"))));
        // A product past the .NET maximum below the first measured product.
        assert!(refused(run(
            Arith::Mul,
            &dec("138753349412021607000000000"),
            &Value::Integer(571)
        )));
        // A quotient just past the limit.
        assert!(refused(run(Arith::Div, &max, &dec("0.999999999999999998"))));
        // An operand of more than 18 digits: 7922816251426433759354395033.5 * 10 is the .NET maximum itself, main's value.
        let near = run(Arith::Mul, &dec("7922816251426433759354395033.5"), &Value::Integer(10));
        assert!(matches!(near, Ok(Value::Decimal(n)) if n == Decimal::MAX));
        assert!(refused(run(
            Arith::Add,
            &Value::Decimal(Decimal::MAX),
            &Value::Decimal(Decimal::MAX)
        )));
        // Unmeasured operand types: a BigInteger on the left of a product.
        assert!(refused(run(Arith::Mul, &Value::BigInteger(10), &max)));
        assert!(!take_hits().contains(&Assumption::DecimalOverflowPastLimit));
    }

    /// Only the sign pairs a cell measures overflow; every other pair keeps main's refusal.
    #[test]
    fn unmeasured_signs_keep_the_refusal() {
        let (max, neg) = (Value::Decimal(MAX18), Value::Decimal(-MAX18));
        assert!(refused(run(Arith::Div, &neg, &dec("0.5"))));
        assert!(refused(run(Arith::Div, &max, &dec("-0.5"))));
        assert!(refused(run(Arith::Add, &neg, &neg)));
        assert!(refused(run(Arith::Sub, &max, &neg)));
        assert!(refused(run(Arith::Mul, &neg, &dec("10"))));
        assert!(refused(run(Arith::Mul, &Value::Integer(-10), &max)));
        assert!(refused(run(Arith::Mul, &neg, &Value::BigInteger(10))));
        assert!(!take_hits().contains(&Assumption::DecimalOverflowPastLimit));
        // Decimal times Integer: every sign pair is measured.
        assert!(overflow(run(Arith::Mul, &neg, &Value::Integer(-10))));
        assert!(overflow(run(Arith::Mul, &max, &Value::Integer(-10))));
    }

    #[test]
    fn a_sum_at_the_limit_rounds_down_only_for_two_decimals_of_18_digits() {
        let max = Value::Decimal(MAX18);
        for (add, op, start) in [
            ("50000000000", Arith::Add, MAX18),
            ("93543950335", Arith::Add, MAX18),
            ("50000000000", Arith::Sub, -MAX18),
        ] {
            let l = Value::Decimal(start);
            assert!(matches!(run(op, &l, &dec(add)), Ok(Value::Decimal(n)) if n == start));
        }
        assert!(take_hits().contains(&Assumption::DecimalLimitRoundsDown));
        // Below the limit: the ordinary rounding, no assumption.
        assert!(matches!(run(Arith::Add, &max, &dec("49999999999")), Ok(Value::Decimal(n)) if n == MAX18));
        assert!(!take_hits().contains(&Assumption::DecimalLimitRoundsDown));
        // A product at the limit keeps main's value.
        let p = run(Arith::Mul, &dec("7922816251426433755"), &dec("10000000000"));
        assert!(matches!(p, Ok(Value::Decimal(n)) if n == d("79228162514264337550000000000")));
        // An operand of more than 18 digits keeps main's value.
        let near = dec("79228162514264337549999999999");
        let sum = run(Arith::Add, &near, &dec("1"));
        assert!(matches!(sum, Ok(Value::Decimal(n)) if n == d("79228162514264337550000000000")));
        assert!(!take_hits().contains(&Assumption::DecimalLimitRoundsDown));
    }
}
