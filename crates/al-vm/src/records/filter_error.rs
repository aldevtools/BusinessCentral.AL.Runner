//! The English Filter:CSide error of a filter text `Record.SetFilter` rejects: one sentence that names the filter, the field and the
//! table, then the reason.

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
}
