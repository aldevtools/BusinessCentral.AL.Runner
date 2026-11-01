//! A date formula that is only a number above 9999 (errors/lt-ms-refusals-128/P128CalcDateForms.al:111, :127
//! P128_CalcDate_DateTextIsNotAFormula: `161026` raises "contains a number that is too large", not the missing time unit
//! error a shorter number gets).

use crate::native::{Signal, raise};

/// The error of `body` (the normalized formula) when it is digits only and their value is above 9999.
pub(super) fn too_large(body: &str) -> Result<(), Signal> {
    let digits_only = !body.is_empty() && body.bytes().all(|b| b.is_ascii_digit());
    if digits_only && body.trim_start_matches('0').len() > 4 {
        return raise(
            format!(
                "The date formula '{body}' contains a number that is too large.\\\\The number must be in the range 0 - 9999."
            ),
            "Evaluate",
        );
    }
    Ok(())
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
