//! `Format` of a Boolean in the standard formats 0, 1, 2 and 9 and the global language.

use crate::format::FormatFail;

/// Format 2 is `1` / `0`, format 9 `true` / `false`; the others the words of the global language: English `Yes` / `No`; German
/// after GlobalLanguage(1031) `Ja` / `Nein` (misc/lt-probes-needed-3/PnC*: Identity_GlobalLanguage_SetsAndRestores `set_ok = Ja`,
/// ObjectId_WithAnotherGlobalLanguage `set 1031 ok = Ja`, P142_Date_Language); French after GlobalLanguage(1036) `Oui` / `Non`
/// (types/operators-expressions/P142ToTextLang.al:153); Danish after GlobalLanguage(1030) `Ja` for true
/// (format-evaluate/Y62DateFormulaFormat.al:DateFormulaFormat_Std1_Danish_EvaluateBack `da:Ja`), false `Nej`; Dutch (1043, 2067)
/// `Ja` / `Nee` (format-evaluate/EvalLanguageBoolean.al EL_Errors_*). The words of other languages are English, as before.
pub(crate) fn boolean_text(b: bool, standard: i32, language: i32) -> Result<String, FormatFail> {
    Ok(match (standard, b) {
        (2, true) => "1".into(),
        (2, false) => "0".into(),
        (9, b) => b.to_string(),
        (_, true) if matches!(language, 1031 | 1030 | 1043 | 2067) => "Ja".into(),
        (_, false) if language == 1031 => "Nein".into(),
        // format-evaluate/EvalLanguageBoolean.al EL_Errors_DAN_1030, EL_Errors_NLD_1043, EL_Errors_NLB_2067.
        (_, false) if language == 1030 => "Nej".into(),
        (_, false) if matches!(language, 1043 | 2067) => "Nee".into(),
        // format-evaluate/EvalLanguageBoolean.al:1063 WA_BlankEnum1_ES_1034 (`Sí` for true; the word for false is not pinned).
        (_, true) if language == 1034 => "Sí".into(),
        (_, false) if language == 1034 => {
            return Err(FormatFail::Unsupported(
                "Format of false under GlobalLanguage 1034 (only the word for true is pinned)".into(),
            ));
        }
        (_, true) if language == 1036 => "Oui".into(),
        (_, false) if language == 1036 => "Non".into(),
        (_, true) => "Yes".into(),
        (_, false) => "No".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::boolean_text;

    // format-evaluate/EvalLanguageBoolean.al:1063 WA_BlankEnum1_ES_1034: true is `Sí` under 1034; false is not pinned.
    #[test]
    fn spanish_true_is_si_and_false_is_refused() {
        assert_eq!(boolean_text(true, 0, 1034).unwrap(), "Sí");
        assert!(boolean_text(false, 0, 1034).is_err());
        assert_eq!(boolean_text(true, 9, 1034).unwrap(), "true");
    }
}
