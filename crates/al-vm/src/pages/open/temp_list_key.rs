//! A List over a temporary source whose OnOpenPage filled it and changed its key (gap G5-19).

use super::*;
use al_builtins::record_op::RecordOp;

/// How a temporary List in a key other than the primary key opens (`temp_list_key_open`).
pub(super) enum TempListKeyOpen {
    /// Rec's primary key row exists: it was read, and the list reads each row once more as a list over a database table in
    /// such a key does.
    ReadOnceMore,
    /// Rec's buffer is a key no row has, and the view has rows: the page opens `=><` from Rec and reads as `ReadOnceMore`
    /// without the first read.
    FromBuffer,
    /// The view has no row: the page was opened on no row.
    Opened,
}

impl Session {
    /// A List over a temporary source, opened without a record, whose OnOpenPage filled it, changed its key (and maybe its
    /// direction) and left Rec on a row that is not the first of the view, or on a buffer no row has. Pinned by
    /// pages/lt-y50-temp-list-key-desc/Y50TempListKeyDesc.al, by Page.Run after TestPage.Trap (and TestPage.OpenView for an
    /// empty view), a page with OnAfterGetRecord and OnAfterGetCurrRecord:
    /// - :293 G519_Rows_LastInserted_KeyDesc: Rec on the last inserted row 4, the view 1,4,2,3: opens on 4, reading it first
    ///   (`R:4;R:4;R:2;R:3;R:4;R:1;R:4;R:4;R:4;C:4`); :364 G519_Rows_LastInserted_KeyAsc the same in ascending order;
    /// - :309 G519_Rows_BufferNotInserted_KeyDesc: the buffer holds row 2, not inserted, the view 1,3: opens on 3, `=><` from
    ///   the buffer (`R:3;R:1;R:3;R:3;R:3;C:3`);
    /// - :271 G519_EmptyView_BufferFilled (Page.Run) and :325 _OpenView: no row is inserted: the page shows no row, GoToRecord
    ///   and First answer false; Page.Run runs OnAfterGetCurrRecord on a blank Rec (`C:0`), OpenView runs no trigger.
    ///
    /// Run modally, in lookup mode, with OnFindRecord or OnNextRecord, or another page type is not pinned and is refused.
    ///
    /// A tree Worksheet (`temp_tree_worksheet_in_key`) is pinned on a row its Rec's primary key has only. So is a Worksheet without
    /// a tree, by OpenView, Page.Run, RunModal and in lookup mode, with OnNextRecord too: SetCurrentKey(Name) and Get(B) open on
    /// B and read as above (pages/lt-qr-onopenpage-moves-rec/QrMovedTests.al QR_TempWs_OpenView_KeyGet
    /// `O:B;R:B;R:B;R:A;R:B;R:C;R:D;R:B;R:B;R:B;C:B;`, _PageRun_KeyGet, _RunModal_KeyGet, _LookupMode_KeyGet,
    /// QR_TempWsNext_OpenView_KeyGet, QR_TempWsNext_LookupMode_KeyGet, writer Y57).
    pub(super) fn temp_list_key_open(&mut self, st: &St) -> Result<TempListKeyOpen, Signal> {
        let tree_ws = self.temp_tree_worksheet_in_key(st);
        let flat_ws = !tree_ws
            && matches!(
                self.engine.object_facts[st.borrow().object as usize].page_type,
                Some(al_ir::PageType::Worksheet)
            )
            && st.borrow().host.is_none()
            && !self.declares(st, "onfindrecord");
        let pinned = tree_ws || flat_ws || {
            let s = st.borrow();
            !s.modal && !s.lookup && s.host.is_none()
        } && matches!(
            self.engine.object_facts[st.borrow().object as usize].page_type,
            Some(al_ir::PageType::List)
        ) && !self.declares(st, "onfindrecord")
            && !self.declares(st, "onnextrecord");
        if !pinned {
            return unsupported(
                "a list over a temporary source opened without a record whose OnOpenPage changed its key and left Rec on a row other than the first of its view (not pinned)",
            );
        }
        let rec = self.page_rec(st);
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        self.ret_used = true;
        let has_rows = matches!(
            self.record_op(RecordOp::FindFirst, &probe, &mut [])?,
            Value::Boolean(true)
        );
        let mut pk = self.row_key(&rec)?;
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        self.ret_used = true;
        let on_row = has_rows && matches!(self.record_op(RecordOp::Get, &probe, &mut pk)?, Value::Boolean(true));
        if (tree_ws || flat_ws) && !on_row {
            return unsupported(
                "a temporary Worksheet whose OnOpenPage changed its key and left Rec on a key no row has (not pinned)",
            );
        }
        if !has_rows {
            let fields = self.initial_fields(rec.borrow().table as usize);
            rec.borrow_mut().fields = fields;
            self.page_first_row(st)?;
            // Page.Run runs OnAfterGetCurrRecord on the blank Rec (:271 `C:0`); TestPage.OpenView runs nothing (:325).
            if st.borrow().caught_by_trap {
                self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
            }
            return Ok(TempListKeyOpen::Opened);
        }
        // From a buffer no row has the row it lands on is read once, then as in `ReadOnceMore` (:309 `R:3;R:1;R:3;R:3;R:3;C:3`).
        st.borrow_mut().temp_reads_once_more = true;
        if !on_row {
            return Ok(TempListKeyOpen::FromBuffer);
        }
        self.ret_used = true;
        self.record_op(RecordOp::Get, &rec, &mut pk)?;
        self.populate_current(st)?;
        Ok(TempListKeyOpen::ReadOnceMore)
    }

    /// A Worksheet with a tree repeater (ShowAsTree) over a temporary source, run modally outside lookup mode, without
    /// OnFindRecord or OnNextRecord, whose OnOpenPage left Rec in a key other than the primary key. Its OnOpenPage filled it and
    /// left Rec on the last row it inserted; the page opens on that row and reads as a list in such a key does: the row first,
    /// the rows above it, then the row three times (pages/lt-y50-tree-expand-leaf/Y50TreeExpandLeaf.al:263
    /// G517_Flat_ExpandTrueOnFirst_ThenNext `No/30:agr 30;agr 30;agr 20;agr 10;agr 30;agr 30;agr 30;agcr 30`, :296 MIXED on 40,
    /// :351 the tree without row triggers or TreeInitialState on 30). Without a tree see `temp_list_key_open`.
    pub(super) fn temp_tree_worksheet_in_key(&self, st: &St) -> bool {
        let rec = self.page_rec(st);
        let s = st.borrow();
        rec.borrow().temporary
            && rec.borrow().key != 0
            && s.modal
            && !s.lookup
            && s.host.is_none()
            && matches!(
                self.engine.object_facts[s.object as usize].page_type,
                Some(al_ir::PageType::Worksheet)
            )
            && {
                drop(s);
                self.tree_indentation(st).is_some()
            }
            && !self.declares(st, "onfindrecord")
            && !self.declares(st, "onnextrecord")
    }

    /// A temporary Worksheet whose OnOpenPage left Rec on the last row of the view (one row read at the open) serves the open read
    /// from OnNextRecord as on another row: it steps back over the rows above and reads Rec's row twice
    /// (pages/lt-qr-onopenpage-moves-rec/QrMovedTests.al QR_TempWsNext_OpenView_NoMove
    /// `O:D;R:D;N:D|1>0;N:C|-1>-1;R:C;N:B|-1>-1;R:B;N:A|-1>-1;R:A;N:A|-1>0;R:D;R:D;C:D;`, writer Y57). `rows_read` 1 also asks
    /// whether Next from the last row onto the new-row line skips OnNextRecord: it does there (`next log` `C:;`).
    pub(in crate::pages) fn temp_worksheet_last_row_open(&self, st: &St, rows_read: usize) -> bool {
        rows_read == 1
            && self.page_rec(st).borrow().temporary
            && self.engine.object_facts[st.borrow().object as usize].page_type == Some(al_ir::PageType::Worksheet)
    }

    /// A temporary Worksheet with OnNextRecord whose OnOpenPage changed its key and left Rec on a row (`temp_list_key_open`):
    /// the open reads Rec's row once more after the rows above it, and Next and Previous read each row once more, as a list
    /// without OnNextRecord does in such a key (QrMovedTests.al QR_TempWsNext_OpenView_KeyGet `...;N:D|-1>0;R:B;R:B;R:B;C:B;`,
    /// `next log` `R:A;R:A;R:A;C:A;`, QR_TempWsNext_LookupMode_KeyGet, writer Y57).
    pub(in crate::pages) fn temp_worksheet_next_in_key(&self, st: &St) -> bool {
        st.borrow().temp_reads_once_more
            && self.page_rec(st).borrow().key != 0
            && self.declares(st, "onnextrecord")
            && self.temp_worksheet_last_row_open(st, 1)
    }
}
