//! `Format` of a Boolean in the standard formats 0, 1, 2 and 9 and the global language.

use crate::format::FormatFail;

/// Format 2 is `1` / `0`, format 9 `true` / `false`; the others the words of the global language: English `Yes` / `No`; German
/// after GlobalLanguage(1031) `Ja` / `Nein` (misc/lt-probes-needed-3/PnC*: Identity_GlobalLanguage_SetsAndRestores `set_ok = Ja`,
/// ObjectId_WithAnotherGlobalLanguage `set 1031 ok = Ja`, P142_Date_Language); French after GlobalLanguage(1036) `Oui` / `Non`
/// (types/lt-conversions-hidden-parts-doctype/P142ToTextLang.al:153); Danish after GlobalLanguage(1030) `Ja` for true
/// (types/lt-y62-dateformula-format/Y62DateFormulaFormat.al:DateFormulaFormat_Std1_Danish_EvaluateBack `da:Ja`), false is not
/// pinned there. The words of other languages are English, as before.
pub(crate) fn boolean_text(b: bool, standard: i32, language: i32) -> Result<String, FormatFail> {
    Ok(match (standard, b) {
        (2, true) => "1".into(),
        (2, false) => "0".into(),
        (9, b) => b.to_string(),
        (_, true) if language == 1031 || language == 1030 => "Ja".into(),
        (_, false) if language == 1031 => "Nein".into(),
        (_, false) if language == 1030 => {
            return Err(FormatFail::Unsupported(
                "Format of false in the language 1030 (not pinned)".into(),
            ));
        }
        (_, true) if language == 1036 => "Oui".into(),
        (_, false) if language == 1036 => "Non".into(),
        (_, true) => "Yes".into(),
        (_, false) => "No".into(),
    })
}
