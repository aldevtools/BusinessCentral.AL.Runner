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

/// A part's view (up to a window of rows) and the place of its row in it.
type LaterRow = (Vec<Vec<Value>>, usize);

impl Session {
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
