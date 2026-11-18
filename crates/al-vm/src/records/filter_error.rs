//! The English Filter:CSide error of a filter text `Record.SetFilter` rejects: one sentence that names the filter, the field and the
//! table, then the reason; and the German sentence of a Date filter text.

use super::*;

impl Session {
    /// `The filter "<shown>" is not valid for the <field> field on the <table> table. <reason>` with code Filter:CSide (the
    /// reasons and their pins are at the callers in `op_inner.rs` and `setfilter_empty_or_side.rs`).
    pub(crate) fn filter_error(&self, t: &TableDef, pos: usize, shown: &str, reason: &str) -> Signal {
        Signal::Error(Box::new(AlError {
            message: format!(
                "The filter \"{shown}\" is not valid for the {} field on the {} table. {reason}",
                self.field_caption(t, pos),
                self.caption(t)
            ),
            code: "Filter:CSide".into(),
            call_stack: String::new(),
            ..Default::default()
        }))
    }

    /// The German error of a date no Date filter reads, `bad` the failing value (types/date-time/P143Date_DePlain.al:27,
    /// _DeRanges.al, _DeFormulas.al).
    pub(crate) fn german_date_filter_error(&self, t: &TableDef, pos: usize, shown: &str, bad: &str) -> Signal {
        Signal::Error(Box::new(AlError {
            message: format!(
                "Der Filter '{shown}' ist für das Feld '{}' der Tabelle '{}' nicht gültig. Das Format Date „{bad}“ stimmt nicht mit den Datumseinstellungen Ihres Geräts überein.\\\\Stellen Sie sicher, dass Tag, Monat und Jahr die richtige Reihenfolge aufweisen und das richtige Trennzeichen verwendet wird (z. b. - oder /). Alternativ können Sie das Format „jjjj-mm-tt“ verwenden.",
                self.field_caption(t, pos),
                self.caption(t)
            ),
            code: "Filter:CSide".into(),
            call_stack: String::new(),
            ..Default::default()
        }))
    }

    /// The date-format error of a Date filter whose whole text `value` no Date filter reads under the session language, for the
    /// closing-date text of another language's letter (`closing_letter_date_filter`, its caller and shape). 1031: the pinned German
    /// text. 1030, 1036, 1044, 1053: the language's text as measured (assumption `date-filter-error-by-language`, corpus PR 1428
    /// FilterGlobalLanguageDateBool.al:229-372 at 04e1fa0a), which quotes the filter as written. Every other language refuses.
    pub(crate) fn date_filter_error_in_language(&self, t: &TableDef, pos: usize, value: &str) -> Signal {
        if self.global_language.get() == 1031 {
            return self.german_date_filter_error(t, pos, value, value);
        }
        let (shown, bad) = (value, value); // the filter as written is the one value
        let (field, table) = (self.field_caption(t, pos), self.caption(t));
        let message = match self.global_language.get() {
            1030 => format!(
                "Filteret \"{shown}\" er ikke gyldigt for feltet {field} i tabellen {table}. Formatet for Date '{bad}' stemmer ikke overens med enhedens datoindstillinger.\\\\Kontroller, at dagen, måneden og året er i den rigtige rækkefølge, og brug den korrekte separator (f.eks. - eller /). Du kan også altid bruge formatet 'åååå-mm-dd'."
            ),
            1036 => format!(
                "Le filtre \u{ab}\u{a0}{shown}\u{a0}\u{bb} n'est pas valide pour le champ {field} de la table {table}. Le format de Date \u{ab}\u{a0}{bad}\u{a0}\u{bb} ne correspond pas aux paramètres de date de votre appareil.\\\\Assurez-vous que le jour, le mois et l'année sont dans le bon ordre et utilisez le séparateur correct (par exemple, - ou /). Sinon, vous pouvez également utiliser le format \u{ab}\u{a0}aaaa-mm-jj\u{a0}\u{bb}."
            ),
            1044 => format!(
                "Filteret {shown} er ikke gyldig for feltet {field} i tabellen {table}. Formatet på Date {bad} samsvarer ikke med enhetens datoinnstillinger.\\\\Kontroller at dagen, måneden og året er i riktig rekkefølge, og at du bruker riktig skilletegn (for eksempel - eller /). Alternativt kan du alltids bruke formatet åååå-mm-dd."
            ),
            1053 => format!(
                "Filtret {shown} är inte giltigt för fältet {field} i tabellen {table}. Formatet på Date \"{bad}\" stämmer inte överens med enhetens datuminställningar.\\\\Kontrollera att dag, månad och år är i rätt ordning och har rätt avgränsare (som \u{2013} eller /). Alternativt kan du alltid använda formatet \"åååå\u{2013}mm\u{2013}dd\"."
            ),
            lang => {
                return Signal::unsupported(format!(
                    "SetFilter: the Date filter error for `{shown}` under GlobalLanguage {lang} (not measured)"
                ));
            }
        };
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::DateFilterErrorByLanguage);
        Signal::Error(Box::new(AlError {
            message,
            code: "Filter:CSide".into(),
            call_stack: String::new(),
            ..Default::default()
        }))
    }

    /// The error of a Text filter text `Record.SetFilter` and `FieldRef.SetFilter` reject in an English session
    /// (`al_builtins::filter::set_filter_text_error`, record/crud/RcAFilters.al:654-658).
    pub(crate) fn check_set_filter_text(
        &self,
        t: &TableDef,
        pos: usize,
        text: &str,
        ty: &RtType,
    ) -> Result<(), Signal> {
        match al_builtins::filter::set_filter_text_error(text, ty) {
            Some(reason) if self.global_language.get() == 1033 => Err(self.filter_error(t, pos, text, reason)),
            _ => Ok(()),
        }
    }
}
