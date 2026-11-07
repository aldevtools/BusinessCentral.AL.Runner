//! The words a Boolean filter accepts in the session language (al-runner issue 7195).
//!
//! `SetFilter` on a Boolean field takes the session language's words for true and false, the words `true`, `false`, `1` and
//! `0`, in any case (record/filters/BooleanFilterByLanguage.al, the same on the three supported majors):
//! - English (1033): `Yes`/`No` (also record/TestFilterTextBooleanFindContracts.al:103);
//! - German (1031): `Ja`/`Nein`; the English `Yes` and `no` are refused (:164-174 WF3_Words_German);
//! - French (1036): `Oui`/`Non`, and `no` too; `Yes`, `Ja` and `Nein` are refused (:184-194 WF3_Words_French).
//!
//! Any other language is not pinned and is refused.

/// `Some(value)` for an accepted word, `None` for a word the language does not accept, `Err` for a language not pinned.
pub(crate) fn boolean_filter_word(language: Option<i32>, text: &str) -> Result<Option<bool>, String> {
    let word = text.to_lowercase();
    let (yes, no): (&[&str], &[&str]) = match language {
        None | Some(1033) => (&["yes", "true", "1"], &["no", "false", "0"]),
        Some(1031) => (&["ja", "true", "1"], &["nein", "false", "0"]),
        Some(1036) => (&["oui", "true", "1"], &["non", "no", "false", "0"]),
        Some(other) => {
            return Err(format!(
                "a Boolean filter value under GlobalLanguage {other} (the words of that language are not pinned)"
            ));
        }
    };
    Ok(if yes.contains(&word.as_str()) {
        Some(true)
    } else if no.contains(&word.as_str()) {
        Some(false)
    } else {
        None
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
}
