//! A part that reads its rows last to first (moved out of rows.rs).

use super::*;
use al_builtins::record_op::RecordOp;

impl Session {
    /// The window of a part that reads its rows last to first, once each, without reading the first row again. Pinned
    /// for 2 and 3 rows (pages/lt-part-reads/PartReadsOpen.al:OpenView_SixParts_ReadsPerPartLogged `open-Same`;
    /// misc/lt-probes-needed-7/PnGPartTriggers.al:OpenOrderOfHostWithSixParts); one row or more than 3 is refused.
    pub(super) fn populate_window_backwards(
        &mut self,
        st: &St,
        start: Vec<Value>,
        why: al_builtins::value::ReadBackwards,
    ) -> Result<bool, Signal> {
        let rec = self.page_rec(st);
        let mut rows: Vec<al_builtins::value::WindowRow> = Vec::new();
        loop {
            let key = self.row_key(&rec)?;
            let stored = rec.borrow().fields.clone();
            rows.push(al_builtins::value::WindowRow {
                key,
                stored: stored.clone(),
                shown: stored,
            });
            if rows.len() > 3 || !self.nav_next(st, 1)? {
                break;
            }
        }
        // At open 2 and 3 rows are pinned (see `view_names_link_field`); after a move back 1, 2 and 3
        // (pages/lt-part-reads/PartReadsOpen.al:358-379).
        let after_move = why == al_builtins::value::ReadBackwards::AfterMove;
        if after_move && rows.len() > 3 {
            // Kept, not pinned: a part of more than 3 rows after its host moved back reads first to last, as every
            // refill did before the backward order was pinned (keeps misc/lt-probes-needed-7/PnGPartRows.al:
            // PendingPartRowSavedWhenHostMoves, which does not log those reads).
            self.ret_used = true;
            self.record_op(RecordOp::Get, &rec, &mut start.clone())?;
            return Ok(false);
        }
        if (rows.len() == 1 && !after_move) || rows.len() > 3 {
            return unsupported(
                "a part viewing its linked field with a number of rows other than 2 or 3 (reads not pinned)",
            );
        }
        for row in rows.iter_mut().rev() {
            rec.borrow_mut().fields = row.stored.clone();
            self.populate_current(st)?;
            row.shown = rec.borrow().fields.clone();
        }
        self.ret_used = true;
        self.record_op(RecordOp::Get, &rec, &mut start.clone())?;
        // A part with OnAfterGetCurrRecord stands on the row it read first (the last one), which it reads twice more before
        // OnAfterGetCurrRecord, as a forward read does on its first row (pages/lt-y80-subpageview/Y80SubPageViewTests.al
        // Y80_SV_F3_OpenView `F3.agr(H1/20000);F3.agr(H1/10000);F3.agr(H1/20000);F3.agr(H1/20000);F3.agcr(H1/20000)`,
        // Y80_SV_F1_HostMoves `previous log`; needs corpus bump).
        if self.declares(st, "onaftergetcurrrecord") && rows.len() > 1 {
            let last = rows.len() - 1;
            rec.borrow_mut().fields = rows[last].stored.clone();
            self.populate_current(st)?;
            rows[last].shown = rec.borrow().fields.clone();
            self.page_trigger(st, "onaftergetrecord", vec![])?;
            let stored = rows[last].stored.clone();
            self.set_xrec_fields(st, stored);
            st.borrow_mut().window = rows;
            return Ok(true);
        }
        rec.borrow_mut().fields = rows[0].shown.clone();
        let first_stored = rows[0].stored.clone();
        self.set_xrec_fields(st, first_stored);
        st.borrow_mut().window = rows;
        Ok(true)
    }
}
