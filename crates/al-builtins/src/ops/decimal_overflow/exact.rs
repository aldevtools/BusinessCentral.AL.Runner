//! Exact comparisons with a whole bound of a sum, product or quotient of two Decimals that no Decimal may hold. Entry
//! points: `sum_at_least`, `product_at_least`, `quotient_at_least`.
//!
//! A Decimal is `m / 10^s` with `m` below 2^96 and `s` at most 28. Each comparison cross-multiplies into whole numbers
//! of up to 256 bits. The side built from the operands stays below 2^192, so a bound side past 256 bits is larger.

use rust_decimal::Decimal;
use std::cmp::Ordering;

/// A whole number of 256 bits, its 64-bit limbs least significant first.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Wide([u64; 4]);

impl Ord for Wide {
    fn cmp(&self, o: &Self) -> Ordering {
        self.0.iter().rev().cmp(o.0.iter().rev())
    }
}

impl PartialOrd for Wide {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

impl Wide {
    fn of(n: u128) -> Wide {
        Wide([n as u64, (n >> 64) as u64, 0, 0])
    }

    /// `self * n`, or `None` past 256 bits.
    fn times(self, n: u128) -> Option<Wide> {
        let mut out = [0u64; 6];
        for (j, b) in [n as u64, (n >> 64) as u64].into_iter().enumerate() {
            let mut carry = 0u128;
            for (i, a) in self.0.into_iter().enumerate() {
                let t = u128::from(a) * u128::from(b) + u128::from(out[i + j]) + carry;
                out[i + j] = t as u64;
                carry = t >> 64;
            }
            out[j + 4] = carry as u64;
        }
        (out[4] == 0 && out[5] == 0).then_some(Wide([out[0], out[1], out[2], out[3]]))
    }

    /// `self + o`, or `None` past 256 bits.
    fn plus(self, o: Wide) -> Option<Wide> {
        let mut out = [0u64; 4];
        let mut carry = false;
        for (i, slot) in out.iter_mut().enumerate() {
            let (s, c1) = self.0[i].overflowing_add(o.0[i]);
            let (s, c2) = s.overflowing_add(u64::from(carry));
            *slot = s;
            carry = c1 || c2;
        }
        (!carry).then_some(Wide(out))
    }

    /// `self * 10^k`, or `None` past 256 bits.
    fn scaled(self, k: u32) -> Option<Wide> {
        self.times(10u128.pow(k.min(38)))?
            .times(10u128.pow(k.saturating_sub(38)))
    }
}

fn mantissa(d: Decimal) -> u128 {
    d.mantissa().unsigned_abs()
}

/// Whether `|a| + |b|` is at least `bound`.
pub(super) fn sum_at_least(a: Decimal, b: Decimal, bound: u128) -> bool {
    let s = a.scale().max(b.scale());
    let sum = Wide::of(mantissa(a)).scaled(s - a.scale()).and_then(|x| {
        let y = Wide::of(mantissa(b)).scaled(s - b.scale())?;
        x.plus(y)
    });
    at_least(sum, Wide::of(bound).scaled(s))
}

/// Whether `|a| * |b|` is at least `bound`.
pub(super) fn product_at_least(a: Decimal, b: Decimal, bound: u128) -> bool {
    let product = Wide::of(mantissa(a)).times(mantissa(b));
    at_least(product, Wide::of(bound).scaled(a.scale() + b.scale()))
}

/// Whether `|a| / |b|` is at least `bound` (`b` is not zero): `|a| * 10^sb >= bound * mb * 10^sa`.
pub(super) fn quotient_at_least(a: Decimal, b: Decimal, bound: u128) -> bool {
    let lhs = Wide::of(mantissa(a)).scaled(b.scale());
    let rhs = Wide::of(bound).times(mantissa(b)).and_then(|x| x.scaled(a.scale()));
    at_least(lhs, rhs)
}

/// `lhs >= rhs`, where `lhs` (built from the operands) always fits and an `rhs` past 256 bits is larger.
fn at_least(lhs: Option<Wide>, rhs: Option<Wide>) -> bool {
    matches!((lhs, rhs), (Some(l), Some(r)) if l >= r)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Decimal {
        s.parse().unwrap()
    }

    #[test]
    fn wide_products_carry_across_limbs() {
        // (2^128 - 1)^2 = 2^256 - 2^129 + 1 fits in 256 bits; twice it does not.
        let sq = Wide::of(u128::MAX).times(u128::MAX).unwrap();
        assert_eq!(sq.0, [1, 0, u64::MAX - 1, u64::MAX]);
        assert!(sq.times(2).is_none());
        assert!(sq.plus(sq).is_none());
        assert_eq!(Wide::of(7).scaled(3), Some(Wide::of(7000)));
        assert_eq!(Wide::of(1).scaled(40), Wide::of(10u128.pow(38)).times(100));
        assert!(Wide::of(1).scaled(39).unwrap() > Wide::of(u128::MAX));
    }

    #[test]
    fn comparisons_are_exact_at_the_bound() {
        let max18 = d("79228162514264337500000000000");
        let two_max18 = 158_456_325_028_528_675_000_000_000_000_u128;
        assert!(sum_at_least(max18, max18, two_max18));
        assert!(!sum_at_least(max18, d("79228162514264337499999999999"), two_max18));
        // Fractions that carry into the whole part.
        assert!(sum_at_least(d("0.5"), d("0.5"), 1));
        assert!(!sum_at_least(d("0.5"), d("0.4999999999999999999999999999"), 1));
        assert!(product_at_least(max18, Decimal::TWO, two_max18));
        assert!(!product_at_least(
            d("79228162514264337499999999999"),
            Decimal::TWO,
            two_max18
        ));
        assert!(product_at_least(d("-0.5"), d("-4"), 2));
        assert!(!product_at_least(
            d("0.0000000000000000000000000001"),
            d("0.0000000000000000000000000001"),
            1
        ));
        assert!(quotient_at_least(max18, d("0.5"), two_max18));
        assert!(!quotient_at_least(
            max18,
            d("0.5000000000000000000000000001"),
            two_max18
        ));
        assert!(quotient_at_least(d("1E28"), d("0.01"), two_max18));
        assert!(!quotient_at_least(
            d("0.0000000000000000000000000001"),
            d("1"),
            two_max18
        ));
    }
}
