//! A host's own `CurrPage.Update` while its ListPart stands on a later row of its view: the part stays on that row.
//!
//! Pinned (corpus pages/lt-th-part-update-cursor/WfHostUpdatePartRow.al, al-runner issue 7194, same on all three majors) for a
//! Document, Card or ListPlus host with one linked ListPart of at most a window of rows, standing on its second or third row,
//! that declares only OnAfterGetRecord, after a host field's OnValidate called `CurrPage.Update(true)` or `Update(false)`
//! (WF3_*_FieldUpdateTrue, _FieldUpdateFalse: `Host.AGR;Part.AGR(2);Part.AGR(2);Part.AGR(3);Part.AGR(2);Part.AGR(1)`, row 3:
//! `Part.AGR(3);Part.AGR(3);Part.AGR(2);Part.AGR(1)`). The part reads its row, then walks as after its own Update, without the two
//! reads of the row at the end. After a host action that calls Update, BC reads the part before the action runs; that needs a
//! look into the action, so the Update is refused there (`refuse_update_in_action`).

use super::*;
use super::pinned_shape::TestCall;
use al_builtins::page_op::PageMethod;

/// A part's view (up to a window of rows) and the place of its row in it.
type LaterRow = (Vec<Vec<Value>>, usize);

/// The reread a Document host owes after a CardPart drilldown (`PageState::deferred_part_drilldown_refresh`).
#[derive(Debug, Clone)]
pub(crate) enum DeferredReread {
    /// The measured drilldown changed the stored header of this key: the host's next SetValue rereads it first.
    Pending(Vec<Value>),
    /// A second drilldown on the part came while one was pending: no reread, and it sets no new one (not measured).
    Spent,
}

impl Session {
    /// AL Runner issue 7886, corpus pull request 1348 (pages/parts/TestCardPartDrilldownRefresh.al), assumption
    /// `card-part-drilldown-deferred-host-refresh`: after a drilldown of a CardPart (UpdatePropagation = Both) that changed the
    /// stored header and called Update(false), the Document host still shows the old value, and its next SetValue validates
    /// against the stored header. Measured sequence: the drilldown, reads of host fields, then the host's SetValue; every
    /// other test call on the page tree ends the pending reread (`end_deferred_host_reread`).
    pub(super) fn defer_host_refresh_after_part_drilldown(&mut self, part: &St) -> Result<(), Signal> {
        let (host, index) = {
            let child = part.borrow();
            if !child.open
                || child.cursor != Cursor::Row
                || child.draft
                || child.dirty
                || !child.refresh
                || !child.refresh_by_update
                || !child.refresh_update_false
                || self.engine.object_facts[child.object as usize].page_type_lower() != "cardpart"
            {
                return Ok(());
            }
            let Some((host, index)) = child.host.as_ref().and_then(|(h, i)| h.upgrade().map(|h| (h, *i))) else {
                return Ok(());
            };
            (host, index)
        };
        if !self.clean_document_refresh_row(&host) || !self.unextended_unsubscribed(&host) || !self.has_source(part) {
            return Ok(());
        }
        let engine = self.engine();
        let control = &page_of(&engine, &host).controls[index];
        if !matches!(control.prop("updatepropagation"), Some(ir::PropVal::Lit(v)) if v.eq_ignore_ascii_case("both")) {
            return Ok(());
        }
        let hrec = self.page_rec(&host);
        let crec = self.page_rec(part);
        let table = self.table(&hrec)?;
        if hrec.borrow().temporary
            || hrec.borrow().table != crec.borrow().table
            || !control.part_link.as_ref().is_some_and(|link| {
                matches!((table.keys[0].as_slice(), link.entries.as_slice()), ([key], [ir::LinkEntry::Field(child, parent)]) if key == child && key == parent)
                    && link.provider.is_none()
                    && link.view.is_none()
            })
        {
            return Ok(());
        }
        let key = self.row_key(&hrec)?;
        if Self::same_key(&key, &self.row_key(&crec)?)
            && self
                .stored_row(&hrec)?
                .is_some_and(|stored| !Self::same_values(&stored, &hrec.borrow().fields))
        {
            let mut h = host.borrow_mut();
            if h.deferred_part_drilldown_refresh.is_none() {
                h.deferred_part_drilldown_refresh = Some(DeferredReread::Pending(key));
            }
        }
        Ok(())
    }

    fn clean_document_refresh_row(&self, st: &St) -> bool {
        let state = st.borrow();
        state.open
            && state.cursor == Cursor::Row
            && !state.draft
            && !state.dirty
            && state.host.is_none()
            && self.engine.object_facts[state.object as usize].page_type == Some(ir::PageType::Document)
            && self.has_source(st)
            && !self.declares(st, "onfindrecord")
            && !self.declares(st, "onnextrecord")
    }

    /// The test makes call `call` on page `st`: a pending reread of its page tree's host stays only for the measured next
    /// steps, a read of a host field and a value typed into a host control (whose SetValue takes it,
    /// `refresh_host_before_set_value`). A drilldown on a part leaves it `Spent`; any other call ends it.
    pub(super) fn end_deferred_host_reread(st: &St, call: Option<TestCall>) {
        let Some(call) = call else { return };
        let (root, part) = Self::tree_root(st);
        let mut r = root.borrow_mut();
        if r.deferred_part_drilldown_refresh.is_none() {
            return;
        }
        r.deferred_part_drilldown_refresh = match (part, call) {
            (None, TestCall::Read | TestCall::Write(_)) => r.deferred_part_drilldown_refresh.take(),
            (Some(_), TestCall::Other(PageMethod::DrillDown)) => Some(DeferredReread::Spent),
            _ => None,
        };
    }

    pub(super) fn refresh_host_before_set_value(&mut self, st: &St) -> Result<(), Signal> {
        let pending = st.borrow_mut().deferred_part_drilldown_refresh.take();
        let Some(DeferredReread::Pending(key)) = pending else { return Ok(()) };
        if self.clean_document_refresh_row(st) && Self::same_key(&key, &self.row_key(&self.page_rec(st))?) {
            al_builtins::assumptions::assume(
                al_builtins::assumptions::Assumption::CardPartDrilldownDeferredHostRefresh,
            );
            st.borrow_mut().refresh = true;
            self.run_refresh(st)?;
        }
        Ok(())
    }

    /// Whether `child` is a linked ListPart on a stored row of its view other than the first, within a window, and the view
    /// and its row: `None` when it is not (it then reads as before).
    fn part_on_later_row(&mut self, child: &St) -> Result<Option<LaterRow>, Signal> {
        if !child.borrow().open || !self.has_source(child) || !self.is_list(child) {
            return Ok(None);
        }
        let (cursor, draft, dirty) = {
            let c = child.borrow();
            (c.cursor, c.draft, c.dirty)
        };
        if cursor != Cursor::Row || draft || dirty {
            return Ok(None);
        }
        let rec = self.page_rec(child);
        let rows = self.view_rows(&rec, Self::WINDOW + 1)?;
        let table = self.table(&rec)?;
        let held = self.row_key(&rec)?;
        let at = rows.iter().position(|r| Self::same_key(&table.primary_key(r), &held));
        Ok(match at {
            Some(at) if at > 0 => Some((rows, at)),
            _ => None,
        })
    }

    /// The part's row triggers are OnAfterGetRecord alone (the only shape the corpus pins; a part with OnAfterGetCurrRecord,
    /// OnFindRecord or OnNextRecord reads differently or is not pinned).
    fn declares_only_after_get_record(&self, child: &St) -> bool {
        self.declares(child, "onaftergetrecord")
            && !self.declares(child, "onaftergetcurrrecord")
            && !self.declares(child, "onfindrecord")
            && !self.declares(child, "onnextrecord")
    }

    /// The host's refresh after its own `CurrPage.Update` (not from an action) with one open ListPart on a later row: the part
    /// reads as pinned in the module comment. Returns whether it handled the parts; any other shape reads as before.
    pub(super) fn host_update_keeps_part_row(&mut self, st: &St) -> Result<bool, Signal> {
        let parts: Vec<St> = st
            .borrow()
            .parts
            .iter()
            .filter(|(_, c)| c.borrow().open)
            .map(|(_, c)| c.clone())
            .collect();
        let host_type = self.engine.object_facts[st.borrow().object as usize].page_type.clone();
        if parts.len() != 1
            || !st.borrow().hidden_parts.is_empty()
            || !matches!(
                host_type,
                Some(al_ir::PageType::Document | al_ir::PageType::Card | al_ir::PageType::ListPlus)
            )
        {
            return Ok(false);
        }
        let child = &parts[0];
        if !self.declares_only_after_get_record(child) {
            return Ok(false);
        }
        let Some((rows, at)) = self.part_on_later_row(child)? else {
            return Ok(false);
        };
        if rows.len() > Self::WINDOW {
            return Ok(false);
        }
        self.read_part_rows_after_host_update(child, &rows, at)?;
        Ok(true)
    }

    /// `CurrPage.Update` inside a host action while a part stands on a later row: BC reads the part before the action runs,
    /// which is known only by looking into the action (WF3_*_Action `Part.AGR(2);Host.AGR;Double.A;...`). Refused.
    pub(super) fn refuse_update_in_action(&mut self, st: &St) -> Result<(), Signal> {
        if !st.borrow().in_action {
            return Ok(());
        }
        let parts: Vec<St> = st.borrow().parts.iter().map(|(_, c)| c.clone()).collect();
        for child in &parts {
            // Only the shape the field cases pin; other parts read as before (Y37LinkFilterTests.al:419
            // LT_C311_Link_ValueChangedOnPage and QjPartPropagation.al:4668 RP_SharedState_TestCodeChangesState pass through
            // an action's Update with parts that declare OnAfterGetCurrRecord).
            if self.declares_only_after_get_record(child) && self.part_on_later_row(child)?.is_some() {
                return unsupported(
                    "CurrPage.Update in a host action while a part stands on a row other than its first (BC reads the part before the action runs; not modelled)",
                );
            }
        }
        Ok(())
    }
}
