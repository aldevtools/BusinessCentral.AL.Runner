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
}
