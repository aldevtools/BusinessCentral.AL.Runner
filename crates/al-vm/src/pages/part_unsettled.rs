//! A new ListPart row after a page-variable control's OnValidate ran `CurrPage.Update()` on a Document host (al-runner issue
//! 7481): the part reads the row and the row before it, nothing more, and reads its row once more when the host closes, before it
//! saves (pages/parts/PartNewRowPageVariable.al:1218 Split_NewRow_RecordFieldsThenVariableWithUpdate `ActUpd` `... AGR 20000 |
//! AGR 10000`, `closed` `[AGR 20000]`; :1248 Split_NewRow_VariableWithUpdateFirst, a SetValue between the update and the close,
//! `closed` `[AGR 20000 | MOD 20000 1 | T-M 20000 1]`; :1470, :1500, :1722, :1752 with DelayedInsert).
//!
//! This module owns the shape (`part_update_unsettled`), the mark it leaves (`PageState::reread_at_close`), what ends the mark
//! (`end_part_reread_at_close`) and the read at the close (`reread_part_at_close`). Every other part row keeps the walk of
//! ThPartUpdateCursor.al (`read_part_rows_around`), and a mark that a move, an action or a host call ended reads nothing at the
//! close.

use super::*;
use al_builtins::page_op::PageMethod;

impl Session {
    /// The pinned shape, every fact required: `new_row_part_pinned`; the part declares no OnAfterGetCurrRecord; `CurrPage.Update()` (it saves, not `Update(false)`); the view holds
    /// two rows and the part stands on the second; that row is the new row the part's `New()` started below the first
    /// (`PageState::after_new`), and the part has not left it since. `rows` is the part's view from its start.
    pub(super) fn part_update_unsettled(&self, st: &St, rows: &[Vec<Value>], at: usize) -> Result<bool, Signal> {
        let shape = self.new_row_part_pinned(st)
            && !self.declares(st, "onaftergetcurrrecord")
            && !st.borrow().refresh_update_false
            && rows.len() == 2
            && at == 1;
        if !shape {
            return Ok(false);
        }
        let Some(anchor) = st.borrow().after_new.clone() else {
            return Ok(false);
        };
        let table = self.table(&self.page_rec(st))?;
        Ok(Self::same_key(
            &table.primary_key(&anchor),
            &table.primary_key(&rows[0]),
        ))
    }

    /// The pages of PartNewRowPageVariable.al: a ListPart on a Document host, neither with a page extension or an event
    /// subscriber. Every behavior this file measured on a new or untouched part row asks this (`dirty_step.rs`, `save_record_now`,
    /// `part_update_unsettled`); other hosts and parts keep the rules before it.
    pub(super) fn new_row_part_pinned(&self, st: &St) -> bool {
        self.list_part_on_document_host(st)
            && self.unextended_unsubscribed(st)
            && st
                .borrow()
                .host
                .as_ref()
                .and_then(|(h, _)| h.upgrade())
                .is_some_and(|h| self.unextended_unsubscribed(&h))
    }

    /// The part read as `part_update_unsettled` pins: it reads its row once more at the close (`reread_part_at_close`).
    pub(super) fn read_part_rows_unsettled(&mut self, st: &St, rows: &[Vec<Value>], at: usize) -> Result<(), Signal> {
        self.read_part_rows_unsettled_walk(st, rows, at)?;
        st.borrow_mut().reread_at_close = true;
        Ok(())
    }

    /// A client call on `st` ends the mark of `read_part_rows_unsettled` on `st`, on its parts and on its host's parts, except the
    /// calls the corpus shows between the update and the close: a read of a field and a SetValue on the part (`on_control`,
    /// PartNewRowPageVariable.al:1218 `shown`, :1248 `Code`), and the host's Close that reads. Any other call (a move of the part
    /// or of the host, an action, New, a host field) is not pinned: the close then reads nothing, as before the mark existed.
    pub(super) fn end_part_reread_at_close(&self, st: &St, name: PageMethod, on_control: bool) {
        if name == PageMethod::Close
            || (on_control && (Self::field_read(name) || (name == PageMethod::SetValue && st.borrow().host.is_some())))
        {
            return;
        }
        let mut pages = vec![st.clone()];
        pages.extend(st.borrow().parts.iter().map(|(_, c)| c.clone()));
        if let Some(host) = st.borrow().host.as_ref().and_then(|(h, _)| h.upgrade()) {
            pages.extend(host.borrow().parts.iter().map(|(_, c)| c.clone()));
        }
        for p in pages {
            p.borrow_mut().reread_at_close = false;
        }
    }

    /// At the host's close, before the part saves: a part that still holds the mark reads its row once more; the read keeps what
    /// was typed on the row since, and the save after it stores it (:1248 `closed`).
    pub(super) fn reread_part_at_close(&mut self, child: &St) -> Result<(), Signal> {
        if !std::mem::take(&mut child.borrow_mut().reread_at_close) {
            return Ok(());
        }
        let rec = self.page_rec(child);
        let (typed, dirty, x) = (
            rec.borrow().fields.clone(),
            child.borrow().dirty,
            self.page_xrec_fields(child),
        );
        self.populate_current(child)?;
        rec.borrow_mut().fields = typed;
        child.borrow_mut().dirty = dirty;
        self.set_xrec_fields(child, x);
        Ok(())
    }
}
