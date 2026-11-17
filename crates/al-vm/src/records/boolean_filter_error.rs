//! The error of a Boolean filter value the session language does not accept (al-runner issue 7195), and the Danish filter
//! texts (al-runner issue 8197, shape 3).

use super::*;

impl Session {
    /// `SetFilter` on field `pos` with `text` and `arg_count` arguments under GlobalLanguage 1030: `None` for a field that is no
    /// Boolean or another language (the parser decides). Only the shape the reporter measured runs (assumption
    /// `boolean-filter-words-danish`): Record.SetFilter in AL code, no placeholder argument, the whole text as written `Ja` (the
    /// true rows), `Nej` (the false rows) or `Yes` (the Danish error). Every other Danish Boolean filter keeps main's refusal.
    pub(crate) fn danish_boolean_filter(
        &self,
        from_al: bool,
        t: &TableDef,
        pos: usize,
        text: &str,
        arg_count: usize,
    ) -> Result<Option<FilterExpr>, Signal> {
        if !(matches!(t.fields[pos].ty, RtType::Boolean) && self.global_language == 1030) {
            return Ok(None);
        }
        let read = al_builtins::filter_boolean_words::danish_filter_text(text);
        let Some(read) = read.filter(|_| from_al && arg_count == 2) else {
            return Err(Signal::unsupported(format!(
                "SetFilter of `{text}` on a Boolean field under GlobalLanguage 1030 (not measured)"
            )));
        };
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::BooleanFilterWordsDanish);
        match read {
            Some(b) => Ok(Some(FilterExpr::Eq(Value::Boolean(b)))),
            None => Err(self.boolean_filter_error(t, pos, text, text)),
        }
    }

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
            // Assumption boolean-filter-words-danish (al-runner issue 8197): the reporter's measured text, reached only from
            // `danish_boolean_filter`.
            1030 => format!(
                "Filteret \"{text}\" er ikke gyldigt for feltet {field} i tabellen {table}. V\u{e6}rdien \"{word}\" kan ikke evalueres til typen Boolean."
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
