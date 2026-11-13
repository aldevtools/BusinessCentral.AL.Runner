//! CalcDate at the ends of the date range (0001-01-01 to 9999-12-31): which formulas raise "The date is not valid." (code
//! DateInvalid), and which results from the closing date C9999-12-31 stay closing dates. Each cell is pinned by
//! types/date-time/CalcDate.al:458-480 or measured by corpus PR 1374 (types/date-time/CalcDateOverflow.al, the same on all
//! three majors; assumption `calcdate-past-date-range`). Every other shape at the ends is refused.

use super::{LAST_DAY, Term, Unit};
use crate::assumptions::{self, Assumption};
use crate::native::{Signal, raise, unsupported};

/// The overload of CalcDate a call went through: a text formula (with or without a date) or a DateFormula.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Overload {
    Text,
    Formula,
}

/// The date a CalcDate call counts from: its day number (1 = 0001-01-01) and whether it is a closing date.
#[derive(Debug, Clone, Copy)]
pub(super) struct Base {
    pub day: i32,
    pub closing: bool,
}

/// 0001-01-01 and 0001-01-02.
const FIRST_DAY: i32 = 1;
const SECOND_DAY: i32 = 2;

fn date_invalid<T>(assumed: bool) -> Result<T, Signal> {
    if assumed {
        assumptions::assume(Assumption::CalcDatePastDateRange);
    }
    raise("The date is not valid.", "DateInvalid")
}

/// CalcDate on the base 0001-01-01 or 0001-01-02 (a date, not a closing date, with a text formula): the measured formulas raise
/// DateInvalid, also those whose result is in range. Any other formula, overload or closing date on these two bases is refused.
/// `Ok(())` for every later base.
pub(super) fn low_base(terms: &[Term], base: Base, overload: Overload) -> Result<(), Signal> {
    use Term::{Edge, Step};
    use Unit::{Day, Year};
    if base.day > SECOND_DAY {
        return Ok(());
    }
    match (base.closing, overload, base.day, terms) {
        // CalcDate.al:466-467 (`<-1D>` from 0001-01-01).
        (false, Overload::Text, FIRST_DAY, [Step(Day, -1)]) => date_invalid(false),
        // CalcDateOverflow.al:285, :421 (`<+0D>`, `<+1D>`, `<-CY>`) and :250, :394 (`<-1Y>`).
        (false, Overload::Text, FIRST_DAY, [Step(Day, 0 | 1) | Edge { unit: Year, end: false } | Step(Year, -1)]) => {
            date_invalid(true)
        }
        // CalcDateOverflow.al:313, :483 (`<+0D>`, `<-1D>`, `<-2D>` from 0001-01-02).
        (false, Overload::Text, SECOND_DAY, [Step(Day, -2..=0)]) => date_invalid(true),
        _ => unsupported(
            "CalcDate on the base 0001-01-01 or 0001-01-02 with this formula, overload or a closing date (not measured)",
        ),
    }
}

/// The last term of `terms`, a step, took the date past 9999-12-31 (a positive step) or before 0001-01-01 (a negative one).
/// The measured cells raise DateInvalid; the rest stays refused.
pub(super) fn past_edge<T>(terms: &[Term], base: Base, overload: Overload) -> Result<T, Signal> {
    use Term::{Edge, Step};
    use Unit::{Day, Month, Week, Year};
    match (base.closing, overload, terms) {
        // CalcDate.al:470 (`<1D>`), :474 (`<1Y>` from 9999-01-01), :478 (`<1M>`), :480 (`<1W>`) from 9999-12-31.
        (false, Overload::Text, [Step(Day | Week | Month | Year, 1)]) => date_invalid(false),
        // CalcDate.al:476 (`<-1Y>` from 0001-06-01).
        (false, Overload::Text, [Step(Year, -1)]) => date_invalid(false),
        // CalcDateOverflow.al:313, :483 (`<+9999Y>` from Today). Above 9999 the formula itself is the error (`<+10000Y>`,
        // `numbers::year_step_too_large`); other numbers above 9999 fall through to the refusal.
        (false, Overload::Text, [Step(Day | Week | Month | Year, 2..=9999)]) => date_invalid(true),
        // CalcDateOverflow.al:250, :367 (`<CM+1D>` from 9999-12-31).
        (false, Overload::Text, [Edge { unit: Month, end: true }, Step(Day, 1)]) => date_invalid(true),
        // CalcDateOverflow.al:250, :394 (the DateFormula `<+1Y>` from 9999-12-31).
        (false, Overload::Formula, [Step(Year, 1)]) => date_invalid(true),
        // CalcDateOverflow.al:250, :394 (`<+1D>` from C9999-12-31).
        (true, Overload::Text, [Step(Day, 1)]) if base.day == LAST_DAY => date_invalid(true),
        _ => unsupported("CalcDate result outside 0001-9999 (error text not pinned)"),
    }
}

/// A result of 0001-01-01 from a base after 0001-01-02 is not measured (from 0001-01-02, `<-1D>` raises DateInvalid,
/// CalcDateOverflow.al:313): refused. Every other result in range is kept.
pub(super) fn first_day_result(day: i32) -> Result<(), Signal> {
    if day == FIRST_DAY {
        return unsupported("CalcDate result 0001-01-01 from a later base (not measured)");
    }
    Ok(())
}

/// Whether the result is a closing date: from C9999-12-31, the text formulas `<+0D>`, `<-1D>` and `<CY>` give closing dates
/// (CalcDateOverflow.al:512). Every other result is a plain date, as before (not measured for other closing dates).
pub(super) fn closing_result(terms: &[Term], base: Base, overload: Overload) -> bool {
    use Term::{Edge, Step};
    let measured = base.closing
        && base.day == LAST_DAY
        && overload == Overload::Text
        && matches!(
            terms,
            [Step(Unit::Day, 0 | -1)
                | Edge {
                    unit: Unit::Year,
                    end: true
                }]
        );
    measured && assumptions::hit(Assumption::CalcDatePastDateRange)
}

#[cfg(test)]
mod tests {
    use crate::assumptions::{self, Assumption};
    use crate::format::day_from_civil;
    use crate::native::Signal;
    use crate::native::tests::{call, t};
    use crate::value::Value;

    fn date(y: i32, m: u32, d: u32) -> Value {
        Value::Date(day_from_civil(y, m, d), false)
    }
    fn closing(y: i32, m: u32, d: u32) -> Value {
        Value::Date(day_from_civil(y, m, d), true)
    }
    /// CalcDate with a text formula, and whether it went through the assumption.
    fn calc(formula: &str, base: Value) -> (Result<Value, Signal>, bool) {
        assumptions::clear_hits();
        let r = call("System.CalcDate#2", vec![t(formula), base]);
        (r, assumptions::take_hits().contains(&Assumption::CalcDatePastDateRange))
    }
    #[track_caller]
    fn date_invalid(r: (Result<Value, Signal>, bool), assumed: bool) {
        match r {
            (Err(Signal::Error(e)), flag) => {
                assert_eq!(
                    (e.message.as_str(), e.code.as_str()),
                    ("The date is not valid.", "DateInvalid")
                );
                assert_eq!(flag, assumed, "assumption flag");
            }
            other => panic!("{other:?}"),
        }
    }
    #[track_caller]
    fn refused(r: (Result<Value, Signal>, bool)) {
        assert!(
            matches!(r, (Err(Signal::Unsupported(_)), false)),
            "{r:?} should be a refusal"
        );
    }

    #[test]
    fn the_pinned_cells_raise_without_the_assumption() {
        // types/date-time/CalcDate.al:466-480.
        date_invalid(calc("<1D>", date(9999, 12, 31)), false);
        date_invalid(calc("<1W>", date(9999, 12, 31)), false);
        date_invalid(calc("<1M>", date(9999, 12, 31)), false);
        date_invalid(calc("<1Y>", date(9999, 1, 1)), false);
        date_invalid(calc("<-1Y>", date(1, 6, 1)), false);
        date_invalid(calc("<-1D>", date(1, 1, 1)), false);
    }

    #[test]
    fn the_measured_cells_raise_through_the_assumption() {
        // CalcDateOverflow.al:250, :285, :313.
        date_invalid(calc("<CM+1D>", date(9999, 12, 31)), true);
        date_invalid(calc("<+9999Y>", date(2026, 11, 13)), true);
        date_invalid(calc("<+1D>", closing(9999, 12, 31)), true);
        date_invalid(calc("<+0D>", date(1, 1, 1)), true);
        date_invalid(calc("<-CY>", date(1, 1, 1)), true);
        date_invalid(calc("<-2D>", date(1, 1, 2)), true);
        assumptions::clear_hits();
        let r = call(
            "System.CalcDate#1",
            vec![Value::DateFormula("<+1Y>".into()), date(9999, 12, 31)],
        );
        date_invalid(
            (r, assumptions::take_hits().contains(&Assumption::CalcDatePastDateRange)),
            true,
        );
        let (r, flag) = calc("<+10000Y>", date(2026, 11, 13));
        let Err(Signal::Error(e)) = r else { panic!("{r:?}") };
        assert!(flag && e.code == "Evaluate" && e.message.starts_with("The date formula '+10000Y' contains"));
    }

    #[test]
    fn closing_results_from_c9999_12_31_stay_closing_only_for_the_measured_formulas() {
        // CalcDateOverflow.al:512.
        let last = day_from_civil(9999, 12, 31);
        assert!(matches!(calc("<-1D>", closing(9999, 12, 31)), (Ok(Value::Date(d, true)), true) if d == last - 1));
        assert!(matches!(calc("<CY>", closing(9999, 12, 31)), (Ok(Value::Date(d, true)), true) if d == last));
        // Not measured: the earlier answer, a plain date, without the assumption.
        assert!(matches!(
            calc("<-1M>", closing(9999, 12, 31)),
            (Ok(Value::Date(_, false)), false)
        ));
        let d = day_from_civil(2024, 1, 30);
        assert!(matches!(calc("<-1D>", closing(2024, 1, 31)), (Ok(Value::Date(x, false)), false) if x == d));
    }

    #[test]
    fn neighbors_inside_the_range_keep_their_answer() {
        let last = day_from_civil(9999, 12, 31);
        assert!(matches!(calc("<+1D>", date(9999, 12, 30)), (Ok(Value::Date(d, false)), false) if d == last));
        assert!(matches!(calc("<+1Y>", date(9998, 12, 31)), (Ok(Value::Date(d, false)), false) if d == last));
        let second = day_from_civil(1, 1, 2);
        assert!(matches!(calc("<-1D>", date(1, 1, 3)), (Ok(Value::Date(d, false)), false) if d == second));
    }

    #[test]
    fn shapes_not_measured_are_refused() {
        // Other formulas, a closing date or the DateFormula overload on the base 0001-01-01 or 0001-01-02.
        refused(calc("<CM>", date(1, 1, 1)));
        refused(calc("<+1D>", date(1, 1, 2)));
        refused(calc("<-1D>", closing(1, 1, 2)));
        let r = call(
            "System.CalcDate#1",
            vec![Value::DateFormula("<+0D>".into()), date(1, 1, 1)],
        );
        assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
        // A result of 0001-01-01 from a later base.
        refused(calc("<-2D>", date(1, 1, 3)));
        // Past the range: a quarter step, a step that is not the last term, a number above 9999, other closing dates, a
        // negative step of days, and a DateFormula other than `<+1Y>`.
        refused(calc("<+1Q>", date(9999, 12, 31)));
        refused(calc("<+1D-1D>", date(9999, 12, 31)));
        refused(calc("<+10001Y>", date(2026, 11, 13)));
        refused(calc("<+2D>", closing(9999, 12, 30)));
        refused(calc("<+1M>", closing(9999, 12, 31)));
        refused(calc("<-5D>", date(1, 1, 3)));
        let r = call(
            "System.CalcDate#1",
            vec![Value::DateFormula("<+1D>".into()), date(9999, 12, 31)],
        );
        assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
    }
}
