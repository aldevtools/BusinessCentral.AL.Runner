//! The letters of a date formula in the global language: `Format(DateFormula, 0, 0)` and `Format(DateFormula, 0, 1)` print them,
//! Evaluate reads a formula without angle brackets in them (types/lt-y62-dateformula-format/Y62DateFormulaFormat.al, codeunit
//! 93262, writer Y62; the same on the BC Linux legs and the SaaS sandbox).

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
    if language == 1033 || language == 1030 || text.is_empty() {
        return format_one(text, language);
    }
    if letters(language).is_some_and(|map| {
        text.chars()
            .all(|c| !c.is_ascii_alphabetic() || map.iter().any(|(en, own)| *en == c && *own == c))
    }) {
        return Ok(text.to_string());
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

/// `Evaluate` of a DateFormula text in `language` (see [`evaluate_text`]).
pub(crate) fn evaluate(text: &str, language: i32) -> Result<crate::datefns::Evaluated, crate::native::Signal> {
    match evaluate_text(text, language) {
        // A Danish letter is no unit in English: `-1Å` evaluates to false under 1033 (DateFormulaFormat_Std1_Danish_EvaluateBack
        // `en:No|`).
        Ok(None) if language == 1033 && text.contains(['Å', 'å']) => Ok(crate::datefns::Evaluated::Rejected(None)),
        Ok(None) => crate::datefns::evaluate_formula(text),
        Ok(Some(english)) => crate::datefns::evaluate_formula(&english),
        Err(why) => Err(crate::native::Signal::Unsupported(why)),
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
}
