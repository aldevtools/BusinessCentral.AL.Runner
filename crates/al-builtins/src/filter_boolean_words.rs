//! The words a Boolean filter accepts in the session language (al-runner issue 7195).
//!
//! `SetFilter` on a Boolean field takes the session language's words for true and false, the words `true`, `false`, `1` and
//! `0`, in any case (record/filters/BooleanFilterByLanguage.al, the same on the three supported majors):
//! - English (1033): `Yes`/`No` (also record/TestFilterTextBooleanFindContracts.al:103);
//! - German (1031): `Ja`/`Nein`; the English `Yes` and `no` are refused (:164-174 WF3_Words_German);
//! - French (1036): `Oui`/`Non`, and `no` too; `Yes`, `Ja` and `Nein` are refused (:184-194 WF3_Words_French).
//!
//! Any other language is not pinned and is refused.

/// The words of the session language for true and false, besides `true`, `false`, `1` and `0` (lower case).
fn language_words(language: Option<i32>) -> Option<(&'static [&'static str], &'static [&'static str])> {
    Some(match language {
        None | Some(1033) => (&["yes", "true", "1"], &["no", "false", "0"]),
        Some(1031) => (&["ja", "true", "1"], &["nein", "false", "0"]),
        Some(1036) => (&["oui", "true", "1"], &["non", "no", "false", "0"]),
        _ => return None,
    })
}

/// `Some(value)` for an accepted word, `None` for a word the language does not accept, `Err` for a language not pinned.
pub(crate) fn boolean_filter_word(language: Option<i32>, text: &str) -> Result<Option<bool>, String> {
    let word = text.to_lowercase();
    let Some((yes, no)) = language_words(language) else {
        return Err(format!(
            "a Boolean filter value under GlobalLanguage {} (the words of that language are not pinned)",
            language.unwrap_or(1033)
        ));
    };
    Ok(if yes.contains(&word.as_str()) {
        Some(true)
    } else if no.contains(&word.as_str()) {
        Some(false)
    } else {
        None
    })
}

/// What `Evaluate` into a Boolean makes of a text under a language other than English.
#[derive(Debug, PartialEq)]
pub(crate) enum EvaluateWord {
    Value(bool),
    Rejected,
    /// A text no corpus case pins in that language.
    NotPinned,
}

/// `Evaluate(Boolean, Text)` under GlobalLanguage 1030, 1031, 1036, 1043 or 2067 (`None` for another language), with blanks
/// around the text dropped and any case (format-evaluate/EvalLanguageBoolean.al EL_Words_*, EL_Errors_*, the same on the
/// three supported majors). The language's own words are `Ja`/`Nein` (1031), `Oui`/`Non` and `No` (1036), `Ja`/`Nej` (1030)
/// and `Ja`/`Nee` (1043, 2067); `true`, `false`, `1`, `0` and `N` (false) read in all of them. `Y`, `Yes`, and the words of
/// the other languages are rejected, except `No`, which French reads as false. Texts outside that list are not pinned,
/// except the German ones errors/WcUnpinnedSingles.al:79-113 pins (`vielleicht`, `Wahr`, `Falsch`, `x`).
pub(crate) fn evaluate_boolean_word(language: i32, text: &str) -> Option<EvaluateWord> {
    let (yes, no): (&[&str], &[&str]) = match language {
        1031 => (&["ja"], &["nein"]),
        1036 => (&["oui"], &["non", "no"]),
        1030 => (&["ja"], &["nej"]),
        1043 | 2067 => (&["ja"], &["nee"]),
        _ => return None,
    };
    let word = text.trim_matches(' ').to_lowercase();
    let w = word.as_str();
    Some(if matches!(w, "true" | "1") || yes.contains(&w) {
        EvaluateWord::Value(true)
    } else if matches!(w, "false" | "0" | "n") || no.contains(&w) {
        EvaluateWord::Value(false)
    } else if matches!(w, "yes" | "no" | "y" | "ja" | "nee" | "nein" | "nej" | "oui" | "non")
        // The empty text and a blank in German (format-evaluate/format-matrix/FMEvalBoolean.al:274-279, corpus PR 1256).
        || (language == 1031 && w.is_empty())
        || (language == 1031 && matches!(w, "vielleicht" | "wahr" | "falsch" | "x"))
    {
        EvaluateWord::Rejected
    } else {
        EvaluateWord::NotPinned
    })
}

#[cfg(test)]
mod tests {
    use super::boolean_filter_word as w;

    #[test]
    fn english_words() {
        assert_eq!(w(Some(1033), "Yes"), Ok(Some(true)));
        assert_eq!(w(Some(1033), "no"), Ok(Some(false)));
        assert_eq!(w(Some(1033), "Ja"), Ok(None));
    }

    #[test]
    fn german_words() {
        assert_eq!(w(Some(1031), "ja"), Ok(Some(true)));
        assert_eq!(w(Some(1031), "Nein"), Ok(Some(false)));
        assert_eq!(w(Some(1031), "False"), Ok(Some(false)));
        assert_eq!(w(Some(1031), "Yes"), Ok(None));
        assert_eq!(w(Some(1031), "no"), Ok(None));
    }

    #[test]
    fn french_words() {
        assert_eq!(w(Some(1036), "Oui"), Ok(Some(true)));
        assert_eq!(w(Some(1036), "no"), Ok(Some(false)));
        assert_eq!(w(Some(1036), "Yes"), Ok(None));
        assert_eq!(w(Some(1036), "Nein"), Ok(None));
    }

    #[test]
    fn other_languages_are_refused() {
        assert!(w(Some(1043), "Ja").is_err());
    }

    use super::{EvaluateWord::*, evaluate_boolean_word as e};

    #[test]
    fn evaluate_words_by_language() {
        assert_eq!(e(2067, " ja "), Some(Value(true)));
        assert_eq!(e(2067, "Nee"), Some(Value(false)));
        assert_eq!(e(2067, "Nein"), Some(Rejected));
        assert_eq!(e(1031, "N"), Some(Value(false)));
        assert_eq!(e(1031, "No"), Some(Rejected));
        assert_eq!(e(1036, "No"), Some(Value(false)));
        assert_eq!(e(1036, "Yes"), Some(Rejected));
        assert_eq!(e(1030, "Nej"), Some(Value(false)));
        assert_eq!(e(1030, "Y"), Some(Rejected));
        assert_eq!(e(1030, "t"), Some(NotPinned));
        assert_eq!(e(1033, "Yes"), None);
    }
}
