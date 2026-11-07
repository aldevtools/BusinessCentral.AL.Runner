//! The error of a Boolean filter value the session language does not accept (al-runner issue 7195).

use super::*;

impl Session {
    /// `SetFilter` on a Boolean field with `text`, of which `word` is not a word of the session language.
    ///
    /// English: record/TestFilterTextBooleanFindContracts.al:130 (SetFilter_BooleanField_UnknownWord_Throws); a leading blank stays
    /// in the quoted filter (format-evaluate/EvalLastError.al:501-539, BT_Evaluate_Boolean_LeadingBlank `filter [ Da]`); a
    /// trailing one is not pinned. German and French: the platform's words, the quotes and the table name differ
    /// (record/filters/BooleanFilterByLanguage.al:164-174 WF3_Words_German, :184-194 WF3_Words_French, the French quotes hold
    /// no-break spaces). A blank around the word under another language is not pinned there.
    pub(crate) fn boolean_filter_error(&self, t: &TableDef, pos: usize, text: &str, word: &str) -> Signal {
        let language = al_builtins::format::FormatCtx::language(self);
        if text.ends_with(char::is_whitespace) || language != 1033 && text.trim() != text {
            return Signal::Unsupported(
                "the error of a Boolean filter with a blank around the word (its quoted filter is not pinned)".into(),
            );
        }
        let (field, table) = (self.field_caption(t, pos), self.caption(t));
        let message = match language {
            1031 => format!(
                "Der Filter '{text}' ist für das Feld '{field}' der Tabelle '{table}' nicht gültig. Der Wert „{word}“ kann nicht in Typ Boolean ausgewertet werden."
            ),
            1036 => format!(
                "Le filtre «\u{a0}{text}\u{a0}» n'est pas valide pour le champ {field} de la table {table}. La valeur «\u{a0}{word}\u{a0}» ne peut pas être évaluée en type Boolean."
            ),
            _ => format!(
                "The filter \"{text}\" is not valid for the {field} field on the {table} table. The value \"{word}\" can't be evaluated into type Boolean."
            ),
        };
        Signal::Error(Box::new(AlError {
            message,
            code: "Filter:CSide".into(),
            call_stack: String::new(),
            ..Default::default()
        }))
    }
}
