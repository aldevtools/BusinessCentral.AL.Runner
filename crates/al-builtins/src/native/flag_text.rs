//! The text `GetLastErrorText(true)` answers for a runtime error whose raise site set no [`ErrorOrigin`]: decided from the
//! error's code and text, for the codes and text shapes errors/last-error/LastErrorFlagRuntimeCodes.al measured (the same on
//! BC 27.5, 28.5 and 29.0). Each rule matches its measured text exactly or by its fixed parts, so an error of the same code
//! with another text stays refused.
//!
//! [`ErrorOrigin`]: super::ErrorOrigin

use super::AlError;

/// The text BC answers for an error .NET raised without a privacy classification (LastErrorFlagRuntimeCodes.al:
/// ArgumentOutOfRange_ExcludeFlag `listget_true`, `substring_true`, `padleft_true`; Dictionary_ExcludeFlag `get_true`).
pub const NOT_CLASSIFIED: &str =
    "Message not shown because the NavBaseException constructor was used without privacy classification";

/// The flag's text for `e`, or `None` when no measured rule covers its code and text.
pub(super) fn flag_text(e: &AlError) -> Option<String> {
    let m = e.message.as_str();
    match e.code.as_str() {
        // List.Get past the end, Text.Substring past the end, Text.PadLeft with a negative width (ArgumentOutOfRange_ExcludeFlag).
        "ArgumentOutOfRange"
            if m == "An invalid argument was passed to a 'List' data type method."
                || m == "startIndex cannot be larger than length of string. (Parameter 'startIndex')"
                || (m.starts_with("totalWidth ('")
                    && m.contains("') must be a non-negative value. (Parameter 'totalWidth')")) =>
        {
            Some(NOT_CLASSIFIED.into())
        }
        // Dictionary.Get of a missing key (Dictionary_ExcludeFlag `get_true`).
        "KeyNotFound" if m == "The given key was not present in the dictionary." => Some(NOT_CLASSIFIED.into()),
        // The texts that hold no value are left as they are: an array index and JsonArray.RemoveAt past the end
        // (IndexOutOfBounds_ExcludeFlag), Dictionary.Add of a key it holds (Dictionary_ExcludeFlag `addtwice_true`), a
        // Variant holding a text into an Integer, Evaluate of a text that is no GUID (Conversions_ExcludeFlag `variantint_true`,
        // `evalguid_true`), a List changed in its own foreach (ListChangedInForeach_ExcludeFlag `foreach_true`).
        "IndexOutOfBounds" if m == "Index out of bounds." => Some(m.into()),
        "Argument" if m == "An item with the same key has already been added." => Some(m.into()),
        "TypeConversion:Format" if m == "The expression Variant cannot be type-converted to a Integer value." => {
            Some(m.into())
        }
        "InvalidGuidFormat"
            if m.starts_with("Invalid format of GUID string. The correct format of the GUID string is: ") =>
        {
            Some(m.into())
        }
        "AL:InvalidOperation" if m == "Collection was modified; enumeration operation may not execute." => {
            Some(m.into())
        }
        // DMY2Date out of range hides the value it was given (Conversions_ExcludeFlag `dmy2date_true`).
        "OutsidePermittedRange" if m.starts_with("The value of DMY2Date parameter ") => {
            let (head, rest) = m.split_once(" The current value is: ")?;
            let (_, tail) = rest.split_once(". The permitted range is: ")?;
            Some(format!(
                "{head} The current value is: <redacted>. The permitted range is: {tail}"
            ))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn err(code: &str, message: &str) -> AlError {
        AlError {
            message: message.into(),
            code: code.into(),
            ..Default::default()
        }
    }

    #[test]
    fn dmy2date_hides_the_value_and_keeps_the_range() {
        let e = err(
            "OutsidePermittedRange",
            "The value of DMY2Date parameter 2 is outside of the permitted range. The current value is: 13. The permitted range is: from 1 to 12.",
        );
        assert_eq!(
            flag_text(&e).as_deref(),
            Some(
                "The value of DMY2Date parameter 2 is outside of the permitted range. The current value is: <redacted>. The permitted range is: from 1 to 12."
            )
        );
    }

    #[test]
    fn a_code_with_an_unmeasured_text_has_no_rule() {
        assert_eq!(flag_text(&err("ArgumentOutOfRange", "something else")), None);
        let position = "The value of Position parameter 1 is outside of the permitted range. The current value is: 1. The permitted range is: from 1 to 0.";
        assert_eq!(flag_text(&err("OutsidePermittedRange", position)), None);
    }
}
