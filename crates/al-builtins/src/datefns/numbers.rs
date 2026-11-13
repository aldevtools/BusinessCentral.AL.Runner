//! A date formula that is only a number above 9999 (types/date-time/P128CalcDateForms.al:111, :127
//! P128_CalcDate_DateTextIsNotAFormula: `161026` raises "contains a number that is too large", not the missing time unit
//! error a shorter number gets).

use crate::native::{Signal, raise};

/// The error of `body` (the normalized formula) when it is digits only and their value is above 9999.
pub(super) fn too_large(body: &str) -> Result<(), Signal> {
    let digits_only = !body.is_empty() && body.bytes().all(|b| b.is_ascii_digit());
    if digits_only && body.trim_start_matches('0').len() > 4 {
        return raise(message(body), "Evaluate");
    }
    Ok(())
}

/// CalcDate with the text formula `<+10000Y>`: the year step is above 9999 and BC raises the same error, with the formula's
/// sign in the message (code Evaluate; corpus PR 1374, types/date-time/CalcDateOverflow.al:250, :367, assumption
/// `calcdate-past-date-range`). Only this text is measured; a step above 9999 in any other formula is refused (`range::past_edge`).
pub(super) fn year_step_too_large(formula: &str) -> Result<(), Signal> {
    if formula == "<+10000Y>" {
        crate::assumptions::assume(crate::assumptions::Assumption::CalcDatePastDateRange);
        return raise(message("+10000Y"), "Evaluate");
    }
    Ok(())
}

fn message(shown: &str) -> String {
    format!(
        "The date formula '{shown}' contains a number that is too large.\\\\The number must be in the range 0 - 9999."
    )
}

#[cfg(test)]
mod tests {
    use crate::format::day_from_civil;
    use crate::native::Signal;
    use crate::native::tests::{call, t};
    use crate::value::Value;

    #[test]
    fn a_number_above_9999_alone_is_too_large() {
        let r = call(
            "System.CalcDate#2",
            vec![t("161026"), Value::Date(day_from_civil(2024, 1, 15), false)],
        );
        let Err(Signal::Error(e)) = r else { panic!("{r:?}") };
        assert_eq!(
            (e.message.as_str(), e.code.as_str()),
            (
                "The date formula '161026' contains a number that is too large.\\\\The number must be in the range 0 - 9999.",
                "Evaluate"
            )
        );
    }
}
