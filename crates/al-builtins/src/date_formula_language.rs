//! The letters of a date formula in the global language: `Format(DateFormula, 0, 0)` and `Format(DateFormula, 0, 1)` print them,
//! Evaluate reads a formula without angle brackets in them (format-evaluate/Y62DateFormulaFormat.al, codeunit
//! 93262, writer Y62; the same on the BC Linux legs and the SaaS sandbox).

use crate::native::{Signal, raise, unsupported};
use crate::value::Value;

/// The English letter of a unit and its letter in a language, for the languages and letters measured: Danish (1030) `U` for W,
/// `Å` for Y, `L` for C, D and M unchanged (DateFormulaFormat_Std1And2_Danish: `-1U`, `-1Å`, `LM+1D`, `+LÅ-24M`); German (1031)
/// `J` for Y, `L` for C, `T` for D, W and M unchanged (DateFormulaFormat_Std1_German: `-1W`, `-1M`, `-1J`, `LM+1T`).
fn letters(language: i32) -> Option<&'static [(char, char)]> {
    match language {
        1030 => Some(&[('D', 'D'), ('W', 'U'), ('M', 'M'), ('Y', 'Å'), ('C', 'L')]),
        1031 => Some(&[('D', 'T'), ('W', 'W'), ('M', 'M'), ('Y', 'J'), ('C', 'L')]),
        _ => None,
    }
}

/// Format 0 of the stored formulas no rule above answers, as pinned (localization/global-language/GlobalLanguageDateFormula.al):
/// under 1030 `1K` (:209) and `UD1` (:226, :410); under 1031 the Evaluate cells (:242, :247, :248, :263) and the cells after
/// `Evaluate(DF, text, 9)` (:431-435). Danish `K` is Q and `UD` WD; German `T` is D, `J` Y, `L` C and `WT` WD. `None` for
/// every other text.
fn reported_format_zero(text: &str, language: i32) -> Option<&'static str> {
    Some(match (language, text) {
        (1030, "1Q") => "1K",
        (1030, "WD1") => "UD1",
        (1031, "1D") => "1T",
        (1031, "1Y") => "1J",
        (1031, "1Q") => "1Q",
        (1031, "-1D") => "-1T",
        (1031, "CM+1D") => "LM+1T",
        (1031, "1Y+1M+1D") => "1J+1M+1T",
        (1031, "-CW") => "-LW",
        (1031, "WD1") => "WT1",
        _ => return None,
    })
}

/// The formula `text` (English letters, no brackets) in the letters of `language`, as format 1 prints it. Format 1 in English
/// and the empty formula in any language print the text as it is (DateFormulaFormat_Std1_English). A letter or a language not
/// measured is the error.
pub(crate) fn format_one(text: &str, language: i32) -> Result<String, String> {
    if language == 1033 || text.is_empty() {
        return Ok(text.to_string());
    }
    let Some(map) = letters(language) else {
        return Err(format!(
            "Format of a DateFormula with standard format 1 in the language {language} (not pinned)"
        ));
    };
    if text.contains("WD") {
        return Err(format!(
            "Format of the date formula `{text}` in the language {language}: the word for WD is not pinned"
        ));
    }
    text.chars()
        .map(|c| {
            if !c.is_ascii_alphabetic() {
                return Ok(c);
            }
            map.iter().find(|(en, _)| *en == c).map(|(_, own)| *own).ok_or_else(|| {
                format!(
                    "Format of the date formula `{text}` in the language {language}: the letter for {c} is not pinned"
                )
            })
        })
        .collect()
}

/// Format 0 is format 1 in English and in Danish (DateFormulaFormat_Std0And9_English, _Std1And2_Danish). In another language it is
/// pinned only where every reading gives the same text: a formula with no letter that a language may change.
pub(crate) fn format_zero(text: &str, language: i32) -> Result<String, String> {
    if language == 1033 || text.is_empty() || language == 1030 && !text.contains('Q') && !text.contains("WD") {
        return format_one(text, language);
    }
    if letters(language).is_some_and(|map| {
        text.chars()
            .all(|c| !c.is_ascii_alphabetic() || map.iter().any(|(en, own)| *en == c && *own == c))
    }) {
        return Ok(text.to_string());
    }
    if let Some(shown) = reported_format_zero(text, language) {
        return Ok(shown.to_string());
    }
    Err(format!(
        "Format of a DateFormula with standard format 0 in the language {language} (not pinned)"
    ))
}

/// A formula without angle brackets that Evaluate reads in `language`, in English letters: `Ok(None)` when the text needs no
/// change (English, or a text in brackets, which is always English: DateFormulaFormat_Std1And2_Danish evaluates `<-1W>` under
/// 1030). Under 1030 the Danish letters read (`-1U` is -1W, `-1Å` -1Y, `LM+1D` CM+1D, DateFormulaFormat_Std1_Danish_EvaluateBack);
/// the English letters W, Y, C and Q, and another language with letters, are not pinned.
pub(crate) fn evaluate_text(text: &str, language: i32) -> Result<Option<String>, String> {
    let t = text.trim();
    if language == 1033 || t.starts_with('<') || !t.chars().any(char::is_alphabetic) {
        return Ok(None);
    }
    if language != 1030 {
        return Err(format!(
            "Evaluate of the date formula `{text}` without brackets in the language {language} (not pinned)"
        ));
    }
    let map = letters(1030).expect("Danish");
    let mut out = String::with_capacity(t.len());
    for c in t.chars() {
        let up = c.to_uppercase().next().unwrap_or(c);
        if !c.is_alphabetic() {
            out.push(c);
        } else if let Some((en, _)) = map.iter().find(|(_, own)| *own == up) {
            out.push(*en);
        } else {
            return Err(format!(
                "Evaluate of the date formula `{text}` under language 1030: the letter {c} is not pinned"
            ));
        }
    }
    Ok(Some(out))
}

/// `Evaluate(DateFormula, text, 9)` under 1030 of a text without brackets that has a Danish letter (the letters of [`letters`]
/// that are not their English letter: U, Å and L in either case). Format number 9 reads the English letters: the pinned texts
/// `1U` and `1Å`, with the result used (localization/global-language/GlobalLanguageDateFormula.al:33), give false and leave the
/// variable (:418, :420). `None` for every other language, number and text (main's reading); every other text with a Danish
/// letter and the statement form refuse.
fn danish_letter_number_nine(
    text: &str,
    language: i32,
    number: i64,
    ret_used: bool,
) -> Option<Result<crate::datefns::Evaluated, crate::native::Signal>> {
    let danish = letters(1030).expect("Danish");
    let danish_letter = |c: char| {
        let up = c.to_uppercase().next().unwrap_or(c);
        danish.iter().any(|(en, own)| en != own && *own == up)
    };
    let t = text.trim();
    if !(language == 1030 && number == 9) || t.starts_with('<') || !t.chars().any(danish_letter) {
        return None;
    }
    if ret_used && matches!(text, "1U" | "1Å") {
        return Some(Ok(crate::datefns::Evaluated::Rejected(None)));
    }
    Some(Err(crate::native::Signal::unsupported(format!(
        "Evaluate of the date formula `{text}` with format number 9 under language 1030 (not measured for this text or form)"
    ))))
}

/// The Evaluate cells without a format number, with the result used (localization/global-language/GlobalLanguageDateFormula.al:27;
/// 1030 at :204, :206, :207, :209, :219, :220; 1031 at :239-243, :247, :248, :256): `Some(Some(english))` for a text read,
/// `Some(None)` for a text rejected (the variable keeps its value), `None` for every other text.
fn reported_evaluate(text: &str, language: i32) -> Option<Option<&'static str>> {
    Some(match (language, text) {
        (1030, "1K") => Some("1Q"),
        (1030, "1W" | "1Q" | "1Y" | "CW" | "AM") => None,
        (1031, "1T") => Some("1D"),
        (1031, "1W") => Some("1W"),
        (1031, "1M") => Some("1M"),
        (1031, "1J") => Some("1Y"),
        (1031, "1Q") => Some("1Q"),
        (1031, "1D" | "1Y" | "CW") => None,
        _ => return None,
    })
}

/// The texts pinned with `Evaluate(DF, text, 9)` under 1030 and 1031, which read the English letters
/// (localization/global-language/GlobalLanguageDateFormula.al:407-410 for 1030, :431-436 for 1031). Under 1030 `1D`, `1M` and the
/// empty text take the path of [`evaluate_text`].
fn reported_number_nine(text: &str, language: i32) -> bool {
    match language {
        1030 => matches!(text, "CM+1D" | "1Y+1M+1D" | "-CW" | "WD1"),
        1031 => matches!(text, "1M" | "-1D" | "CM+1D" | "1Y+1M+1D" | "-CW" | "WD1"),
        _ => false,
    }
}

/// `Evaluate` of a DateFormula text in `language` (see [`evaluate_text`]) with format `number`; `ret_used`: the call's result
/// is used.
pub(crate) fn evaluate(
    text: &str,
    language: i32,
    number: i64,
    ret_used: bool,
) -> Result<crate::datefns::Evaluated, crate::native::Signal> {
    if let Some(read) = danish_letter_number_nine(text, language, number, ret_used) {
        return read;
    }
    if number == 9 && reported_number_nine(text, language) {
        return crate::datefns::evaluate_formula(text);
    }
    if let Some(read) = reported_evaluate(text, language).filter(|_| number == 0) {
        if !ret_used {
            return Err(crate::native::Signal::unsupported(format!(
                "Evaluate of the date formula `{text}` under language {language} as a statement (only the result is measured)"
            )));
        }
        return match read {
            Some(english) => crate::datefns::evaluate_formula(english),
            None => Ok(crate::datefns::Evaluated::Rejected(None)),
        };
    }
    match evaluate_text(text, language) {
        // A Danish letter is no unit in English: `-1Å` evaluates to false under 1033 (DateFormulaFormat_Std1_Danish_EvaluateBack
        // `en:No|`).
        Ok(None) if language == 1033 && text.contains(['Å', 'å']) => Ok(crate::datefns::Evaluated::Rejected(None)),
        Ok(None) => crate::datefns::evaluate_formula(text),
        Ok(Some(english)) => crate::datefns::evaluate_formula(&english),
        Err(why) => Err(crate::native::Signal::unsupported(why)),
    }
}

/// CalcDate with a Text or Code formula, with or without a base date, under GlobalLanguage 1030, 1031, 1036, 1044 or 1053
/// (localization/global-language/GlobalLanguageDateFormula.al:540-771; the errors at :647, :659, :671, :683, :695 and
/// :719-769, the text `1D` at :553, :571, :589, :607, :625): the answer the language gives before or instead of the English
/// reading, or `None` to keep it. `unit_error`: the English reading raised the should-include-a-time-unit error. Measured: a
/// formula `<1X>` with X one of U, K, Å, T, J, Z raises the language's text that lists the English units; the text `1D` without
/// brackets reads the language's letters, the next day under 1030, 1044 and 1053, the language's text that lists its own units
/// under 1031 and 1036. Every other text without brackets that has a letter refuses; a bracketed formula keeps the English reading.
/// `text_with_base`: the formula is a Text value and a base date is given, the only call pinned (:37-39); a Code formula or a call
/// without a base date refuses where the language would answer.
pub(crate) fn calcdate_text(
    formula: &str,
    language: i32,
    unit_error: bool,
    text_with_base: bool,
) -> Option<Result<Value, Signal>> {
    let answer = language_answer(formula, language, unit_error)?;
    if text_with_base {
        return Some(answer);
    }
    Some(unsupported(format!(
        "CalcDate of `{formula}` under GlobalLanguage {language} with a Code formula or without a base date (not pinned)"
    )))
}

fn language_answer(formula: &str, language: i32, unit_error: bool) -> Option<Result<Value, Signal>> {
    if !matches!(language, 1030 | 1031 | 1036 | 1044 | 1053) || !formula.chars().any(char::is_alphabetic) {
        return None;
    }
    if let Some(f) = formula.strip_prefix('<').and_then(|f| f.strip_suffix('>')) {
        let measured = matches!(f, "1U" | "1K" | "1Å" | "1T" | "1J" | "1Z");
        if !(unit_error && measured) {
            return None;
        }
        let message = match language {
            1030 => format!(
                "Der skal indgå en tidsenhed i datoformlen '{f}'.\\\\Tidsenheder kan være: D,WD,W,M,Q eller Y.\\C betyder den aktuelle tidsenhed baseret på dato, kan specificeres som et præfiks for tidsenheden."
            ),
            1031 => format!(
                "Die Datumsformel \u{201e}{f}\u{201c} muss eine Zeiteinheit enthalten.\\\\Mögliche Zeiteinheiten: D,WD,W,M,Q oder Y.\\C, die laufende Zeiteinheit basierend auf dem Datum, kann als Präfix der Zeiteinheit angegeben werden."
            ),
            1036 => format!(
                "La formule de date \u{ab}\u{a0}{f}\u{a0}\u{bb} devrait inclure une unité de temps.\\\\Les unités de temps peuvent être\u{a0}: D,WD,W,M,Q ou Y.\\C, signifiant unité de temps actuelle basée sur la date, peut être spécifié comme préfixe de l\u{2019}unité de temps."
            ),
            1044 => format!(
                "Datoformelen {f} må inneholde en tidsenhet.\\\\Tidsenheter kan være: D,WD,W,M,Q eller Y.\\C, som betyr nåværende tidsenhet basert på dato, kan angis som et prefiks til tidsenheten."
            ),
            _ => format!(
                "Datumformeln '{f}' ska inkludera en tidsenhet.\\\\Tidsenheter kan vara: D,WD,W,M,Q eller Y.\\C, avser aktuell tidsenhet baserad på datum och kan anges som ett prefix till tidsenheten."
            ),
        };
        return Some(raise(message, "Evaluate"));
    }
    if formula != "1D" {
        return Some(unsupported(format!(
            "CalcDate of `{formula}` without angle brackets under GlobalLanguage {language} (not measured for this text)"
        )));
    }
    let f = formula;
    match language {
        1031 => Some(raise(
            format!(
                "Die Datumsformel \u{201e}{f}\u{201c} muss eine Zeiteinheit enthalten.\\\\Mögliche Zeiteinheiten: T,WT,W,M,Q oder J.\\L, die laufende Zeiteinheit basierend auf dem Datum, kann als Präfix der Zeiteinheit angegeben werden."
            ),
            "Evaluate",
        )),
        1036 => Some(raise(
            format!(
                "La formule de date \u{ab}\u{a0}{f}\u{a0}\u{bb} devrait inclure une unité de temps.\\\\Les unités de temps peuvent être\u{a0}: J,JS,S,M,T ou A.\\F, signifiant unité de temps actuelle basée sur la date, peut être spécifié comme préfixe de l\u{2019}unité de temps."
            ),
            "Evaluate",
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn danish_and_german_letters() {
        assert_eq!(format_one("+CY-24M", 1030).unwrap(), "+LÅ-24M");
        assert_eq!(format_one("CM+1D", 1031).unwrap(), "LM+1T");
        assert!(format_one("-1Q", 1030).is_err());
        assert_eq!(evaluate_text("LM+1D", 1030).unwrap().as_deref(), Some("CM+1D"));
        assert_eq!(evaluate_text("<-1W>", 1030).unwrap(), None);
        assert!(evaluate_text("-1W", 1030).is_err());
    }

    // GlobalLanguageDateFormula.al:571 pins `1D` under 1031 only for a Text formula with a base date (:37-39).
    #[test]
    fn the_language_answer_needs_a_text_formula_with_a_base_date() {
        assert!(matches!(
            calcdate_text("1D", 1031, false, true),
            Some(Err(Signal::Error(_)))
        ));
        assert!(matches!(
            calcdate_text("1D", 1031, false, false),
            Some(Err(Signal::Unsupported(_)))
        ));
        assert!(matches!(
            calcdate_text("<1U>", 1030, true, false),
            Some(Err(Signal::Unsupported(_)))
        ));
        // Out of scope: a formula without a letter keeps main's reading.
        assert!(calcdate_text("-1", 1031, false, false).is_none());
    }
}
