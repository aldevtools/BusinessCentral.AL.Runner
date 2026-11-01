//! Which row an unlinked list part over a temporary source opens on when the host's code left the part's Rec on a row.

use super::*;
use al_builtins::record_op::RecordOp;

impl Session {
    /// A ListPart over a temporary source, without SubPageLink, SubPageView, OnOpenPage, OnFindRecord and OnNextRecord, in a
    /// host without a source table whose OnOpenPage filled the part and left its Rec on a row other than the first of the
    /// view: true when the part opens on that row, as a list whose OnOpenPage left Rec there does
    /// (`open_on_rec_moved_by_onopenpage`). Pinned by pages/lt-y15/Y15WorksheetPartRefresh.al:287
    /// Y15_PartRefresh_TwoRowsPartOnFirst (`open log` `AGR2;AGR1;AGR2;AGR2;AGCR2`, `at open` `2/B`: rows 1 and 2 inserted, the
    /// last insert leaves Rec on row 2). Another host or part keeps the first row, as before (not pinned).
    pub(super) fn part_rec_left_by_host(&mut self, st: &St) -> Result<bool, Signal> {
        let Some((host, control)) = st.borrow().host.as_ref().and_then(|(h, c)| h.upgrade().map(|h| (h, *c))) else {
            return Ok(false);
        };
        let host_has_source = {
            let object = host.borrow().object as usize;
            self.engine.program.objects[object].rec_slot.is_some()
        };
        let unlinked = page_of(&self.engine(), &host).controls[control]
            .part_link
            .as_ref()
            .is_none_or(|l| l.entries.is_empty() && l.view.is_none() && l.provider.is_none());
        let list_part = matches!(
            &self.engine.object_facts[st.borrow().object as usize].page_type,
            Some(al_ir::PageType::Other(t)) if t == "listpart"
        );
        let rec = self.page_rec(st);
        if host_has_source
            || !unlinked
            || !list_part
            || !rec.borrow().temporary
            || rec.borrow().key != 0
            || ["onopenpage", "onfindrecord", "onnextrecord"].iter().any(|t| self.declares(st, t))
        {
            return Ok(false);
        }
        let key = self.row_key(&rec)?;
        let table = self.table(&rec)?;
        let blank: Vec<Value> = table.keys[0]
            .iter()
            .map(|&f| self.default_value(&table.fields[f].ty))
            .collect();
        if Self::same_key(&key, &blank) {
            return Ok(false);
        }
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        self.ret_used = true;
        if !matches!(
            self.record_op(RecordOp::Find, &probe, &mut [Value::Text("=".into())])?,
            Value::Boolean(true)
        ) {
            return Ok(false);
        }
        Ok(!self.on_first_row_of_view(&rec)?)
    }

    /// Opens the part on the row the host's code left its Rec on (`part_rec_left_by_host`), as a list opens on the row its
    /// OnOpenPage left Rec on.
    pub(super) fn open_part_on_host_row(&mut self, st: &St) -> Result<(), Signal> {
        st.borrow_mut().part_on_host_row = true;
        let r = self.open_on_rec_moved_by_onopenpage(st);
        st.borrow_mut().part_on_host_row = false;
        r
    }

    /// The open read of `st` follows the rules of a part: a part, unless it opens on the row the host's code left its Rec on,
    /// which reads as a list opened on the row its OnOpenPage left Rec on (Y15WorksheetPartRefresh.al:287 `AGR2;AGR1;AGR2;AGR2;
    /// AGCR2`, as QrMovedTests.al:331 QR_List_OpenView_SetRangeFindLast `O:D;R:D;R:C;R:B;R:D;R:D;C:D;`).
    pub(in crate::pages) fn reads_as_part(&self, st: &St) -> bool {
        st.borrow().host.is_some() && !st.borrow().part_on_host_row
    }
}
