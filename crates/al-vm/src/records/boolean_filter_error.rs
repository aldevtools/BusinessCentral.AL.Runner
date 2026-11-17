//! The error of a Boolean filter value the session language does not accept (al-runner issue 7195), and the Danish filter
//! texts (al-runner issue 8197, shape 3).

use super::*;

impl Session {
    /// A Boolean filter under GlobalLanguage 1030 in the measured shape whose whole text as written is `Ja` (the true rows), `Nej`
    /// (the false rows) or `Yes` (the Danish error) (assumption `boolean-filter-words-danish`, corpus PR 1428
    /// FilterGlobalLanguageDateBool.al:375-391 and :429-441 at 04e1fa0a). `Ok(None)` for every other field, language and text: the parser
    /// reads them as on main.
    pub(super) fn danish_boolean_filter(
        &self,
        shape: &super::measured_setfilter::MeasuredSetFilter,
        t: &TableDef,
        pos: usize,
    ) -> Result<Option<FilterExpr>, Signal> {
        if !(matches!(t.fields[pos].ty, RtType::Boolean) && self.global_language == 1030) {
            return Ok(None);
        }
        let Some(read) = al_builtins::filter_boolean_words::danish_filter_text(shape.text) else {
            return Ok(None);
        };
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::BooleanFilterWordsDanish);
        match read {
            Some(b) => Ok(Some(FilterExpr::Eq(Value::Boolean(b)))),
            None => Err(self.boolean_filter_error(t, pos, shape.text, shape.text)),
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
