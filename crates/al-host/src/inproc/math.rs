//! `System.Math` members whose .NET result is exact and the same on every platform, and `TimeSpan.FromTicks`
//! (ADR-038 amendment 2026-11-09). System Application's `Math` codeunit is a thin wrapper over `System.Math`; a formula
//! engine that calls it in a loop made millions of sidecar round trips (a large partner app: 8.2 million requests,
//! 292 s of a 1,359 s test run).
//!
//! Only the overloads that take and give a .NET `decimal` are served: `Abs`, `Floor`, `Ceiling`, `Truncate`, `Max`,
//! `Min` and `Sign`. They are integer arithmetic on the decimal's 96-bit mantissa and scale, with no rounding, so the
//! answer is the same on every platform. Everything that goes through a `double` (`Sqrt`, `Pow`, `Sin`, `Log`, `Exp`,
//! ...) stays in the sidecar: the AL Decimal is converted to a `double` and the result back to a `decimal` by .NET's own
//! conversions, and the transcendental functions come from the platform's C library, so equality on every platform we
//! ship is not shown. A decimal text this module does not read for sure (an exponent, more than 28 decimals, a mantissa
//! past 96 bits) also goes to the sidecar.
//!
//! What .NET answers, measured with .NET 8 and 10 and kept equal by the differential test (`dotnet_diff.rs`):
//! a zero never prints a sign; `Abs` keeps the scale; `Floor`, `Ceiling` and `Truncate` give scale 0; `Max` of two equal
//! values gives the first, `Min` the second (`Max(1.0, 1.00)` is `1.0`, `Min(1.0, 1.00)` is `1.00`).

use crate::dotnet::DotNetValue;
use std::cmp::Ordering;

/// The largest mantissa of a .NET `decimal`: 2^96 - 1.
const MAX_MANTISSA: u128 = (1u128 << 96) - 1;

/// A .NET `decimal` as sign, mantissa and scale (number of decimals, 0 to 28).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Dec {
    neg: bool,
    mantissa: u128,
    scale: u32,
}

impl Dec {
    /// The value of `text` as `decimal.Parse(text, NumberStyles.Float, InvariantCulture)` reads it, when that read is
    /// exact: an optional `-`, digits, and an optional `.` with digits. `None` for anything else.
    pub(super) fn parse(text: &str) -> Option<Dec> {
        let (neg, rest) = match text.strip_prefix('-') {
            Some(r) => (true, r),
            None => (false, text),
        };
        let (int, frac) = match rest.split_once('.') {
            Some((i, f)) => (i, f),
            None => (rest, ""),
        };
        if int.is_empty() || !int.bytes().all(|b| b.is_ascii_digit()) || !frac.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        if rest.contains('.') && frac.is_empty() {
            return None;
        }
        let scale = u32::try_from(frac.len()).ok()?;
        if scale > 28 {
            return None;
        }
        let mut mantissa: u128 = 0;
        for b in int.bytes().chain(frac.bytes()) {
            mantissa = mantissa.checked_mul(10)?.checked_add(u128::from(b - b'0'))?;
            if mantissa > MAX_MANTISSA {
                return None;
            }
        }
        Some(Dec { neg, mantissa, scale })
    }

    /// `decimal.ToString(InvariantCulture)`: every decimal of the scale, and no sign on a zero.
    pub(super) fn text(self) -> String {
        let digits = self.mantissa.to_string();
        let scale = self.scale as usize;
        let body = if scale == 0 {
            digits
        } else {
            let padded = format!("{digits:0>width$}", width = scale + 1);
            let (i, f) = padded.split_at(padded.len() - scale);
            format!("{i}.{f}")
        };
        if self.neg && self.mantissa != 0 {
            format!("-{body}")
        } else {
            body
        }
    }

    fn signum(self) -> i32 {
        match (self.mantissa, self.neg) {
            (0, _) => 0,
            (_, true) => -1,
            (_, false) => 1,
        }
    }

    /// Rounds to a whole number toward zero (`Truncate`), toward minus infinity (`Floor`) or toward plus infinity
    /// (`Ceiling`); the result has scale 0.
    fn whole(self, toward: Toward) -> Dec {
        let p = 10u128.pow(self.scale);
        let (q, r) = (self.mantissa / p, self.mantissa % p);
        let up = r != 0
            && match toward {
                Toward::Zero => false,
                Toward::MinusInfinity => self.neg,
                Toward::PlusInfinity => !self.neg,
            };
        Dec {
            neg: self.neg,
            mantissa: q + u128::from(up),
            scale: 0,
        }
    }

    /// Compares the values (the scale does not count, and minus zero equals zero).
    fn cmp_value(self, other: Dec) -> Ordering {
        match (self.signum(), other.signum()) {
            (a, b) if a != b => a.cmp(&b),
            (0, _) => Ordering::Equal,
            (s, _) => {
                let scale = self.scale.max(other.scale);
                // Scaling a mantissa up to the common scale can pass u128 (2^96 * 10^28), so whole parts and
                // fractions are compared apart.
                let ord = cmp_scaled(self.mantissa, self.scale, other.mantissa, other.scale, scale);
                if s < 0 { ord.reverse() } else { ord }
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Toward {
    Zero,
    MinusInfinity,
    PlusInfinity,
}

/// Compares `a / 10^sa` with `b / 10^sb`, both non-negative, without overflow: whole parts first, then the fractions
/// brought to the common scale `s` (a fraction is below 10^28, times 10^(s - its scale) stays below 10^28 < 2^94).
fn cmp_scaled(a: u128, sa: u32, b: u128, sb: u32, s: u32) -> Ordering {
    let (pa, pb) = (10u128.pow(sa), 10u128.pow(sb));
    match (a / pa).cmp(&(b / pb)) {
        Ordering::Equal => {
            let fa = (a % pa) * 10u128.pow(s - sa);
            let fb = (b % pb) * 10u128.pow(s - sb);
            fa.cmp(&fb)
        }
        o => o,
    }
}

fn decimal_args(vals: &[&DotNetValue]) -> Option<Vec<Dec>> {
    vals.iter()
        .map(|v| match v {
            DotNetValue::Decimal(t) => Dec::parse(t),
            _ => None,
        })
        .collect()
}

/// The answer of `System.Math.<member>` to `vals`, when this module serves the call; `None` sends it to the sidecar.
pub(super) fn math(member: &str, vals: &[&DotNetValue]) -> Option<DotNetValue> {
    let d = decimal_args(vals)?;
    let dec = |x: Dec| Some(DotNetValue::Decimal(x.text()));
    match (member, d.as_slice()) {
        ("Abs", [x]) => dec(Dec { neg: false, ..*x }),
        ("Floor", [x]) => dec(x.whole(Toward::MinusInfinity)),
        ("Ceiling", [x]) => dec(x.whole(Toward::PlusInfinity)),
        ("Truncate", [x]) => dec(x.whole(Toward::Zero)),
        ("Sign", [x]) => Some(DotNetValue::Int32(x.signum())),
        // decimal.Max: `d1 >= d2 ? d1 : d2`; decimal.Min: `d1 < d2 ? d1 : d2`.
        ("Max", [x, y]) => dec(if x.cmp_value(*y) != Ordering::Less { *x } else { *y }),
        ("Min", [x, y]) => dec(if x.cmp_value(*y) == Ordering::Less { *x } else { *y }),
        _ => None,
    }
}

/// `System.TimeSpan.FromTicks(Int64)`: a `TimeSpan` of exactly those ticks, for every `Int64`. What System Application's
/// "Regex Impl." builds the match timeout with before every `Regex` it makes.
pub(super) fn time_span(member: &str, vals: &[&DotNetValue]) -> Option<DotNetValue> {
    match (member, vals) {
        ("FromTicks", [DotNetValue::Int64(t)]) => Some(DotNetValue::TimeSpan(*t)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(t: &str) -> DotNetValue {
        DotNetValue::Decimal(t.into())
    }

    fn call(m: &str, args: &[&str]) -> Option<String> {
        let vals: Vec<DotNetValue> = args.iter().map(|t| d(t)).collect();
        let refs: Vec<&DotNetValue> = vals.iter().collect();
        math(m, &refs).map(|v| match v {
            DotNetValue::Decimal(t) => t,
            DotNetValue::Int32(i) => i.to_string(),
            other => panic!("{other:?}"),
        })
    }

    /// Each row: input, then what .NET 8 and .NET 10 print for Abs, Floor, Ceiling, Truncate and Sign (measured).
    #[test]
    fn one_argument_members_match_what_dotnet_printed() {
        let rows: &[[&str; 6]] = &[
            ["-0", "0", "0", "0", "0", "0"],
            ["0.0", "0.0", "0", "0", "0", "0"],
            ["-0.0", "0.0", "0", "0", "0", "0"],
            ["-0.50", "0.50", "-1", "0", "0", "-1"],
            ["-1.50", "1.50", "-2", "-1", "-1", "-1"],
            ["2.5", "2.5", "2", "3", "2", "1"],
            ["-2.5", "2.5", "-3", "-2", "-2", "-1"],
            ["1024", "1024", "1024", "1024", "1024", "1"],
            [
                "79228162514264337593543950335",
                "79228162514264337593543950335",
                "79228162514264337593543950335",
                "79228162514264337593543950335",
                "79228162514264337593543950335",
                "1",
            ],
            [
                "-79228162514264337593543950335",
                "79228162514264337593543950335",
                "-79228162514264337593543950335",
                "-79228162514264337593543950335",
                "-79228162514264337593543950335",
                "-1",
            ],
            [
                "0.0000000000000000000000000001",
                "0.0000000000000000000000000001",
                "0",
                "1",
                "0",
                "1",
            ],
            [
                "-0.0000000000000000000000000001",
                "0.0000000000000000000000000001",
                "-1",
                "0",
                "0",
                "-1",
            ],
            ["1.00", "1.00", "1", "1", "1", "1"],
            ["-1.00", "1.00", "-1", "-1", "-1", "-1"],
        ];
        for r in rows {
            for (k, m) in ["Abs", "Floor", "Ceiling", "Truncate", "Sign"].iter().enumerate() {
                assert_eq!(call(m, &[r[0]]).as_deref(), Some(r[k + 1]), "{m}({})", r[0]);
            }
        }
    }

    #[test]
    fn max_and_min_of_equal_values_pick_the_argument_dotnet_picks() {
        assert_eq!(call("Max", &["1.0", "1.00"]).unwrap(), "1.0");
        assert_eq!(call("Max", &["1.00", "1.0"]).unwrap(), "1.00");
        assert_eq!(call("Min", &["1.0", "1.00"]).unwrap(), "1.00");
        assert_eq!(call("Min", &["1.00", "1.0"]).unwrap(), "1.0");
        assert_eq!(call("Max", &["-0", "0"]).unwrap(), "0");
        assert_eq!(call("Max", &["-3.5", "-3.25"]).unwrap(), "-3.25");
        assert_eq!(call("Min", &["-3.5", "-3.25"]).unwrap(), "-3.5");
        assert_eq!(
            call(
                "Max",
                &["0.0000000000000000000000000001", "-79228162514264337593543950335"]
            )
            .unwrap(),
            "0.0000000000000000000000000001"
        );
        assert_eq!(
            call(
                "Min",
                &["7922816251426433759354395033.5", "7922816251426433759354395033.4"]
            )
            .unwrap(),
            "7922816251426433759354395033.4"
        );
    }

    #[test]
    fn what_is_not_read_for_sure_goes_to_the_sidecar() {
        for t in [
            "1E3",
            "1e3",
            " 1",
            "+1",
            "1.",
            ".5",
            "",
            "-",
            "0.00000000000000000000000000001",
            "79228162514264337593543950336",
            "1,5",
        ] {
            assert_eq!(call("Abs", &[t]), None, "{t:?}");
        }
        for m in [
            "Sqrt",
            "Pow",
            "Sin",
            "Cos",
            "Log",
            "Exp",
            "Round",
            "Log10",
            "Atan2",
            "IEEERemainder",
        ] {
            assert_eq!(call(m, &["4"]), None, "{m}");
            assert_eq!(call(m, &["4", "2"]), None, "{m}");
        }
        // Another overload than the decimal one (Abs(Int32) throws for Int32.MinValue in .NET).
        assert_eq!(math("Abs", &[&DotNetValue::Int32(i32::MIN)]), None);
        assert_eq!(math("Max", &[&d("1"), &DotNetValue::Int32(2)]), None);
    }

    #[test]
    fn from_ticks_is_the_ticks() {
        for t in [0, 10_000_000, -1, i64::MAX, i64::MIN] {
            assert_eq!(
                time_span("FromTicks", &[&DotNetValue::Int64(t)]),
                Some(DotNetValue::TimeSpan(t))
            );
        }
        assert_eq!(time_span("FromTicks", &[&DotNetValue::Int32(5)]), None);
        assert_eq!(time_span("FromMilliseconds", &[&DotNetValue::Int64(5)]), None);
    }
}
