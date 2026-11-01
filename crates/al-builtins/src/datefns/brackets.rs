//! Angle brackets inside a term of a date formula, where the reader of [`super::normalize_formula`] does not accept
//! them at the start of the formula, after a sign or at the end. Pinned by
//! types/lt-date-formula-edges/DateFormulaEdges.al:533-635 (Y43_G413_*): a bracket after the number and before the
//! unit is dropped (`-62<D>` is `-62D`, `2<D>` is stored as `2D`, for D, WD, W, M, Q and Y), as are `<1>D`, `1<>D`
//! and `1D<+1D>`. A bracket after the number and before `C` or a digit raises the missing time unit error, and one
//! between `C` and the unit (`C<M>`) the missing number error; the message shows the formula as written.

use crate::native::{Signal, raise};

/// What to do with an angle bracket inside a term.
pub(super) enum Inside {
    /// Drop it, as BC does.
    Drop,
    /// Not pinned: refuse.
    Refuse,
}

const UNITS: [char; 5] = ['D', 'W', 'M', 'Q', 'Y'];

/// `<` at `chars[i]` that is neither at the start nor after a sign or another `<`. `text` is the formula as written.
pub(super) fn open(text: &str, chars: &[char], i: usize) -> Result<Inside, Signal> {
    let prev = chars[i - 1];
    let next = chars[i + 1..].iter().find(|c| !matches!(c, '<' | '>')).copied();
    // The pinned shapes have no bracket around the whole formula and are upper case.
    let shown: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let plain = !shown.starts_with('<') && !shown.chars().any(|c| c.is_ascii_lowercase());
    if !plain {
        return Ok(Inside::Refuse);
    }
    if prev.is_ascii_digit() {
        return match next {
            // DateFormulaEdges.al:539-548 (-62<D>, 2<D>, 62<D>, +2<D>), :559-568 (-1<W>, 1<M>, -1<Q>, 1<Y>), :579 (1<WD>).
            Some(c) if UNITS.contains(&c) => Ok(Inside::Drop),
            // DateFormulaEdges.al:583-584 (-1<CM>) and :603-604 (1<0D>).
            Some(c) if c == 'C' || c.is_ascii_digit() => raise(
                format!(
                    "The date formula '{shown}' should include a time unit.\\\\Time units can be: D,WD,W,M,Q, or Y.\\C, meaning current time unit based on date, can be specified as a prefix to the time unit."
                ),
                "Evaluate",
            ),
            _ => Ok(Inside::Refuse),
        };
    }
    // DateFormulaEdges.al:581-582 (C<M>): only the month is pinned.
    if prev == 'C' && next == Some('M') {
        return raise(
            format!("The date formula '{shown}' should include a number."),
            "Evaluate",
        );
    }
    // DateFormulaEdges.al:629-630 (1D<+1D>): a bracket between a unit and the sign of the next term.
    if UNITS.contains(&prev) && matches!(next, Some('+' | '-')) {
        return Ok(Inside::Drop);
    }
    Ok(Inside::Refuse)
}

/// `>` at `chars[i]` that is neither at the end nor before a sign: dropped between the number and the unit
/// (DateFormulaEdges.al:623-626, `<1>D` and `1<>D`).
pub(super) fn close(chars: &[char], i: usize) -> Inside {
    let prev_ok = i > 0 && (chars[i - 1].is_ascii_digit() || chars[i - 1] == '<');
    match chars.get(i + 1) {
        Some(c) if prev_ok && UNITS.contains(c) => Inside::Drop,
        _ => Inside::Refuse,
    }
}

#[cfg(test)]
mod tests {
    use crate::datefns::{Evaluated, evaluate_formula, formula_text};
    use crate::format::day_from_civil;
    use crate::native::Signal;
    use crate::native::tests::{call, t};
    use crate::value::Value;

    fn calc(f: &str) -> Result<Value, Signal> {
        call(
            "System.CalcDate#2",
            vec![t(f), Value::Date(day_from_civil(2024, 1, 15), false)],
        )
    }

    #[track_caller]
    fn both_ok(f: &str, stored: &str, (y, m, d): (i32, u32, u32)) {
        match calc(f) {
            Ok(Value::Date(n, false)) => assert_eq!(n, day_from_civil(y, m, d), "calc {f}"),
            other => panic!("calc {f}: {other:?}"),
        }
        match evaluate_formula(f) {
            Ok(Evaluated::Stored(s)) => assert_eq!(formula_text(&s), stored, "eval {f}"),
            Ok(_) => panic!("eval {f}: not stored"),
            Err(e) => panic!("eval {f}: {e:?}"),
        }
    }

    #[track_caller]
    fn both_err(f: &str, msg: &str) {
        match calc(f) {
            Err(Signal::Error(e)) => assert_eq!((e.message.as_str(), e.code.as_str()), (msg, "Evaluate"), "calc {f}"),
            other => panic!("calc {f}: {other:?}"),
        }
        match evaluate_formula(f) {
            Ok(Evaluated::Failed(m) | Evaluated::Rejected(Some(m))) => assert_eq!(m, msg, "eval {f}"),
            Ok(_) => panic!("eval {f}: no message"),
            Err(e) => panic!("eval {f}: {e:?}"),
        }
    }

    /// DateFormulaEdges.al:533-615.
    #[test]
    fn a_bracket_between_the_number_and_the_unit_is_dropped() {
        both_ok("-62<D>", "-62D", (2023, 11, 14));
        both_ok("2<D>", "2D", (2024, 1, 17));
        both_ok("62<D>", "62D", (2024, 3, 17));
        both_ok("+2<D>", "+2D", (2024, 1, 17));
        both_ok("-1<W>", "-1W", (2024, 1, 8));
        both_ok("1<M>", "1M", (2024, 2, 15));
        both_ok("-1<Q>", "-1Q", (2023, 10, 15));
        both_ok("1<Y>", "1Y", (2025, 1, 15));
        both_ok("1<WD>", "1WD", (2024, 1, 16));
        both_ok("1<D>+1<D>", "1D+1D", (2024, 1, 17));
        both_ok("1D+2<D>", "1D+2D", (2024, 1, 18));
        both_ok("-62<D>>", "-62D", (2023, 11, 14));
        both_ok("-<62D>", "-62D", (2023, 11, 14));
    }

    /// DateFormulaEdges.al:617-634.
    #[test]
    fn brackets_in_other_places_of_a_term() {
        both_ok("<1>D", "1D", (2024, 1, 16));
        both_ok("1<>D", "1D", (2024, 1, 16));
        both_ok("1D<+1D>", "1D+1D", (2024, 1, 17));
        both_err(
            "<1D><1D>",
            "The date formula '1D><1D' must include plus (+) or minus (-).",
        );
    }

    /// DateFormulaEdges.al:581-584, 603-604.
    #[test]
    fn a_bracket_before_c_or_a_digit_raises() {
        let unit = |f: &str| {
            format!(
                "The date formula '{f}' should include a time unit.\\\\Time units can be: D,WD,W,M,Q, or Y.\\C, meaning current time unit based on date, can be specified as a prefix to the time unit."
            )
        };
        both_err("C<M>", "The date formula 'C<M>' should include a number.");
        both_err("-1<CM>", &unit("-1<CM>"));
        both_err("1<0D>", &unit("1<0D>"));
    }

    /// Shapes no test pins stay refused.
    #[test]
    fn other_brackets_inside_a_term_stay_refused() {
        for f in ["C<D>", "<1<0D>", "<1<D>", "1D<1D", "<1D><+1D>", "1<X>", "1>D<"] {
            assert!(matches!(calc(f), Err(Signal::Unsupported(_))), "{f}");
        }
    }
}
