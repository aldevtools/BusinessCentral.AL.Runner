//! The row the server-side Rec of a list page stands on while the test client moves over rows it already holds.
//!
//! A `Next` of a list or list part whose only read trigger is OnAfterGetRecord runs no trigger, and a `Page` variable that holds the
//! page keeps answering for the row the page last read: the held sender of a page extension's event still reads `First` after
//! two `Next` on a list and `Line A` after `Next` on a list part (events/extension-events/PageExtensionSender.al:643
//! List_SenderPerRow `held base after next`, :678 Document_HeaderAndLinesSenders `held lines after next`). A trigger the page
//! runs afterwards reads the row again (`page_call` clears the lag), and so do the rows a refresh loads. The runtime keeps its own
//! Rec on the moved row; only a procedure called through a `Page` variable sees the row the server still has. Other moves and
//! other reads of the held page are not pinned and are not lagged.

use super::*;
use al_builtins::page_op::PageMethod;

impl Session {
    /// `page_move_inner`, noting the row the server keeps when a `Next` ran no trigger.
    pub(super) fn page_move_noting_server_row(&mut self, st: &St, how: PageMethod) -> Result<bool, Signal> {
        st.borrow_mut().next_held_row = None;
        let lagging = how == PageMethod::Next
            && st.borrow().server_lag.is_none()
            && st.borrow().instance.is_some()
            && self.has_source(st)
            && self.is_list(st)
            && self.only_after_get_record(st);
        if lagging {
            let before = self.page_rec(st).borrow().fields.clone();
            st.borrow_mut().server_lag = Some(before);
        }
        // A trigger that runs during the move clears the lag (`page_call`).
        let moved = self.page_move_inner(st, how)?;
        if lagging {
            self.note_next_held_row(st)?;
        }
        Ok(moved)
    }

    /// Whether OnAfterGetRecord, in the page or an extension, is the only read trigger the page and its extensions declare.
    fn only_after_get_record(&self, st: &St) -> bool {
        let engine = self.engine();
        let page = page_of(&engine, st);
        let names = page
            .triggers
            .iter()
            .map(|(n, _)| n.as_str())
            .chain(page.ext_triggers.iter().map(|(n, _, _)| n.as_str()));
        let (mut agr, mut other) = (false, false);
        for n in names {
            match n {
                "onaftergetrecord" => agr = true,
                "onaftergetcurrrecord" | "onfindrecord" | "onnextrecord" => other = true,
                _ => {}
            }
        }
        agr && !other
    }

    /// A procedure is called through a `Page` variable: puts the server's row into the page's Rec for the call. Returns the Rec
    /// and the fields to put back afterwards.
    pub(crate) fn enter_server_row(&mut self, st: &St) -> Option<(Rec, Vec<Value>)> {
        let lag = st.borrow().server_lag.clone()?;
        st.borrow().instance.as_ref()?;
        let object = st.borrow().object as usize;
        self.engine.program.objects[object].rec_slot?;
        let rec = self.page_rec(st);
        let now = std::mem::replace(&mut rec.borrow_mut().fields, lag);
        Some((rec, now))
    }
}
