//! A Worksheet over a temporary source that empties and refills Rec from a filter field's OnValidate or from an action, the
//! shape of page 5530 "Item Availability by Event" (corpus pages/lt-outside-worksheet/RmTempRefill.al, al-runner issues 5779
//! and 5780): what the client reads afterwards and which row it shows.

use super::*;
use al_builtins::record_op::RecordOp;
use al_builtins::value::WindowRow;

impl Session {
    /// The pinned shape: a top-level Worksheet over a temporary source in its primary key without filters, with OnAfterGetRecord
    /// and OnAfterGetCurrRecord and without OnFindRecord or OnNextRecord, standing on a stored row.
    pub(super) fn temp_refill_shape(&mut self, st: &St) -> bool {
        self.declares(st, "onaftergetcurrrecord") && self.temp_refill_shape_any_current(st)
    }

    /// `temp_refill_shape`, or the same shape on a tree page whose only filter on Rec is the tree's cut (`apply_tree_cut`): a
    /// filter field's OnValidate that refills a collapsed tree is read as on the flat page (al-runner issue 6550).
    pub(super) fn temp_refill_shape_or_tree(&mut self, st: &St) -> bool {
        if !self.declares(st, "onaftergetcurrrecord") || !self.has_source(st) || st.borrow().instance.is_none() {
            return false;
        }
        let rec = self.page_rec(st);
        let cut: Vec<_> = rec
            .borrow()
            .filters
            .iter()
            .filter(|f| f.group == super::open::TREE_GROUP)
            .cloned()
            .collect();
        if cut.is_empty() {
            return self.temp_refill_shape_any_current(st);
        }
        rec.borrow_mut().filters.retain(|f| f.group != super::open::TREE_GROUP);
        let shape = self.temp_refill_shape_any_current(st);
        rec.borrow_mut().filters.extend(cut);
        shape
    }

    /// `temp_refill_shape`, with or without OnAfterGetCurrRecord.
    fn temp_refill_shape_any_current(&mut self, st: &St) -> bool {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        if (engine.object_facts[object].page_type != Some(al_ir::PageType::Worksheet)
            && !self.temp_list_without_writes(st))
            || !self.has_source(st)
            || !self.is_list(st)
            || st.borrow().host.is_some()
            || !st.borrow().open
            || st.borrow().cursor != Cursor::Row
            || st.borrow().draft
            || !self.declares(st, "onaftergetrecord")
            || self.declares(st, "onfindrecord")
            || self.declares(st, "onnextrecord")
        {
            return false;
        }
        let rec = self.page_rec(st);
        let r = rec.borrow();
        r.temporary && r.key == 0 && r.ascending && r.filters.is_empty()
    }

    /// Whether the rows the client holds are no longer what the table has (a row gone or changed).
    pub(super) fn held_rows_changed(&mut self, st: &St) -> Result<bool, Signal> {
        let rec = self.page_rec(st);
        let held: Vec<WindowRow> = st.borrow().window.clone();
        for w in &held {
            let probe = Rc::new(RefCell::new(rec.borrow().clone()));
            probe.borrow_mut().filters.clear();
            let mut pk = w.key.clone();
            self.ret_used = true;
            if !matches!(self.record_op(RecordOp::Get, &probe, &mut pk)?, Value::Boolean(true))
                || !Self::same_values(&probe.borrow().fields, &w.stored)
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// The rows of the view from its start, up to `limit` of them.
    pub(super) fn view_rows(&mut self, rec: &Rec, limit: usize) -> Result<Vec<Vec<Value>>, Signal> {
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        let mut rows = Vec::new();
        self.ret_used = true;
        if !matches!(
            self.record_op(RecordOp::FindFirst, &probe, &mut [])?,
            Value::Boolean(true)
        ) {
            return Ok(rows);
        }
        loop {
            rows.push(probe.borrow().fields.clone());
            if rows.len() >= limit {
                return Ok(rows);
            }
            self.ret_used = true;
            if !matches!(self.record_op(RecordOp::Next, &probe, &mut [Value::Integer(1)])?, Value::Integer(n) if n != 0)
            {
                return Ok(rows);
            }
        }
    }

    /// Reads row `fields` with OnAfterGetRecord and gives the row the client then holds.
    fn read_refilled_row(&mut self, st: &St, rec: &Rec, fields: &[Value]) -> Result<WindowRow, Signal> {
        rec.borrow_mut().fields = fields.to_vec();
        let key = self.row_key(rec)?;
        self.populate_current(st)?;
        Ok(WindowRow {
            key,
            stored: fields.to_vec(),
            shown: rec.borrow().fields.clone(),
        })
    }

    /// After a filter field's OnValidate refilled the rows: the client stays on the row it stood on when the refill has its key,
    /// else on the row Rec was left on. From the first row of the view it reads the rows from there on (up to a window), then the
    /// row twice and runs OnAfterGetCurrRecord (RmTempRefill.al:215 RM_Filter_B_FromOpen `R:B1;R:B2;R:B1;R:B1;C:B1;`, :254 from
    /// row 23, :302 RM_Filter_BThenCleared `R:A1;...;R:A20;R:A1;R:A1;C:A1;` then 27 rows). From a later row of a view the window
    /// holds whole it reads the row, the rows from it on, the rows before it, then the row twice (:279 RM_Filter_B_FromRow2_Previous
    /// `R:B2;R:B2;R:B1;R:B2;R:B2;C:B2;`, shows B2). A held row that is gone while Rec stands on a row other than the first of the
    /// view, and a later row of a longer view, are not pinned.
    /// `expanded` is the rows of a tree the client had expanded before the field was validated.
    pub(super) fn reread_after_refill_by_validate(
        &mut self,
        st: &St,
        held: &[Value],
        expanded: Vec<Vec<Value>>,
    ) -> Result<(), Signal> {
        let rec = self.page_rec(st);
        // A collapsed tree: the client reads every row of the tree as the flat page reads its view, keeps the row it stood on
        // when its key came back (else Rec's row), keeps the rows it had expanded by key, shows the parent of a kept row below the
        // top level expanded, and holds the rows the cut shows (al-runner issue 6550; corpus writer Y34, RmTempRefill.al
        // Y34_Tree_Fewer_AB_To_B `R:B;R:B-1;R:B-2;R:B;R:B;C:B;` rows `B`, Y34_Tree_SameKeys_AB_To_CD_FromSecondRoot
        // `R:D;R:D;R:D-1;R:D-2;R:D;R:C-2;R:C-1;R:C;R:D;R:D;C:D;` rows `C D`, Y34_Tree_ExpandedBefore_AB_To_B rows `B B-1 B-2`,
        // Y34_Tree_EndOnLastRow_A_To_AB rows `A A-1 A-2 B` (Rec's own row, B-2, does not expand B),
        // Y34_Tree_KeptKeyIsChild_AB_To_aBc_FromSecondRoot rows `a B B-1 B-2 c`). A refill to no rows is not built.
        if rec.borrow().filters.iter().any(|f| f.group == super::open::TREE_GROUP) {
            // Awaits the Y34 pin (al-runner issue 6550).
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::TreeRefillByValidateRereads);
            let Some(level) = self.tree_level_field(st)? else {
                return unsupported("a tree whose IndentationColumn is not an Integer field (not pinned)");
            };
            rec.borrow_mut().filters.retain(|f| f.group != super::open::TREE_GROUP);
            if self.view_rows(&rec, 1)?.is_empty() {
                return unsupported(
                    "a refill of a collapsed tree Worksheet by a field's OnValidate that left no rows (its reads are not built)",
                );
            }
            self.reread_view_after_refill_by_validate(st, held)?;
            st.borrow_mut().expanded = expanded;
            self.apply_tree_cut(st)?;
            self.split_tree_window(st, level);
            return Ok(());
        }
        self.reread_view_after_refill_by_validate(st, held)
    }

    /// `reread_after_refill_by_validate` over the rows of Rec's view.
    fn reread_view_after_refill_by_validate(&mut self, st: &St, held: &[Value]) -> Result<(), Signal> {
        let rec = self.page_rec(st);
        let rows = self.view_rows(&rec, Self::WINDOW + 1)?;
        let table = self.table(&rec)?;
        let at_key = |k: &[Value]| rows.iter().position(|r| Self::same_key(&table.primary_key(r), k));
        let at = match at_key(held) {
            Some(i) => i,
            None => {
                let now = self.row_key(&rec)?;
                match at_key(&now) {
                    Some(0) => 0,
                    _ => {
                        return unsupported(
                            "a refill of a temporary Worksheet by a field's OnValidate that left the row the client stood on gone and Rec on a row other than the first of the view (not pinned)",
                        );
                    }
                }
            }
        };
        if at > 0 && rows.len() > Self::WINDOW {
            return unsupported(
                "a refill of a temporary Worksheet by a field's OnValidate that keeps the client on a later row of a view longer than a window (not pinned)",
            );
        }
        self.read_refilled_rows(st, &rows, at)
    }

    /// After an action that refilled the rows (or moved Rec) and left Rec on another row: after the read of the row the client
    /// stood on that comes before every action, the client reads Rec's row, then reads as from the first row of the view
    /// (RmTempRefill.al:323 RM_Action_Recalculate_FromRow23 `R:A23;R:A1;R:A1;R:A2;...;R:A20;R:A1;R:A1;C:A1;`, :367 with
    /// CurrPage.Update(false) the same, :388 from row 2 `R:A2;R:A1;...`, :407 after filter B `R:B2;R:B1;R:B1;R:B2;R:B1;R:B1;C:B1;`).
    /// It shows Rec's row. Rec on a row other than the first of the view, and a row the client stood on that the table no longer
    /// has, are not pinned.
    pub(super) fn reread_after_refill_by_action(&mut self, st: &St, held: &[Value]) -> Result<(), Signal> {
        let rec = self.page_rec(st);
        let rows = self.view_rows(&rec, Self::WINDOW + 1)?;
        let table = self.table(&rec)?;
        let now = self.row_key(&rec)?;
        if rows
            .first()
            .is_none_or(|r| !Self::same_key(&table.primary_key(r), &now))
        {
            return unsupported(
                "an action on a temporary Worksheet that left Rec on a row other than the first of its view (not pinned)",
            );
        }
        // A view longer than a window refilled while the client stood on its first row: the reads of a later walk differ from
        // the ones built (RmTempRefill.al:798 TD_RefillMany_WithCurrRecord_3To25 `walk log length` 444, :812 _25To22 372);
        // from another row they match (:826 _30To21_FromRow25).
        if rows.len() > Self::WINDOW
            && self.declares(st, "onaftergetcurrrecord")
            && Self::same_key(&table.primary_key(&rows[0]), held)
        {
            return unsupported(
                "an action on a temporary Worksheet that refilled it with more rows than a window while the client stood on its first row (the reads of later moves are not built)",
            );
        }
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        probe.borrow_mut().filters.clear();
        let mut pk = held.to_vec();
        self.ret_used = true;
        if !matches!(self.record_op(RecordOp::Get, &probe, &mut pk)?, Value::Boolean(true)) {
            return unsupported(
                "an action on a temporary Worksheet after which the row the client stood on is gone (not pinned)",
            );
        }
        self.read_refilled_row(st, &rec, &rows[0])?;
        self.read_refilled_rows(st, &rows, 0)
    }

    /// An action that refilled the rows of a temporary Worksheet, a row the client held being gone, Rec left on the first row of
    /// the view. With one row: after the read of the row the client stood on that comes before every action, the client reads the
    /// row three times and runs OnAfterGetCurrRecord, or, without that trigger, reads it twice; it then holds that one row
    /// (pages/lt-outside-worksheet/RmTempRefill.al SH_RefillFewer_WithCurrRecord_3To1 `R:R1;R:R1;R:R1;R:R1;C:R1;`, _25To1, _25To1_FromRow23 `R:R23;R:R1;R:R1;R:R1;C:R1;`,
    /// SH_RefillFewer_NoCurrRecord_3To1 and _25To1 `R:R1;R:R1;R:R1;`). With two rows up to a window it holds only those rows
    /// (al-runner issue 6029; RmTempRefill.al TD_RefillFewer_WithCurrRecord_*, the reads below). Without OnAfterGetCurrRecord,
    /// Rec on another row of the view, and more rows than a window, are refused.
    /// Returns whether it handled the action.
    pub(super) fn reread_after_refill_to_fewer_rows(&mut self, st: &St) -> Result<bool, Signal> {
        if !self.temp_refill_shape_any_current(st) {
            return Ok(false);
        }
        // On the List shape a held row the action replaced under the same key counts as gone (UjTempList.al
        // UJ_Temp_List_OneRowRebuilt `same key`: `invoke;R:1;R:1;R:1;R:1;C:1;`).
        let list = self.temp_list_without_writes(st);
        let gone = if list {
            self.held_rows_changed(st)?
        } else {
            self.held_row_gone(st)?
        };
        if !gone {
            return Ok(false);
        }
        let rec = self.page_rec(st);
        let rows = self.view_rows(&rec, Self::WINDOW + 1)?;
        if rows.is_empty() {
            return Ok(false);
        }
        // Pinned by pages/lt-uj-outside-leads/UjTempList.al (was assumption `list-action-rebuilds-rows`).
        let table = self.table(&rec)?;
        let now = self.row_key(&rec)?;
        // On the List shape Rec left on a later row of a view the window holds whole: the client reads the row, the rows from it
        // on, the rows before it, then the row twice and OnAfterGetCurrRecord, and shows it (UjTempList.al UJ_Temp_List_ThreeNewRows
        // `no find` `invoke;R:1;R:23;R:23;R:22;R:21;R:23;R:23;C:23;`, cur 23).
        let at = rows.iter().position(|r| Self::same_key(&table.primary_key(r), &now));
        if list && rows.len() <= Self::WINDOW && self.declares(st, "onaftergetcurrrecord") {
            if let Some(at) = at.filter(|a| *a > 0) {
                self.read_refilled_rows(st, &rows, at)?;
                return Ok(true);
            }
        }
        if !Self::same_key(&table.primary_key(&rows[0]), &now) {
            return unsupported(
                "an action on a temporary Worksheet that refilled it, a row the client held being gone, and left Rec on a row other than the first of its view (not pinned)",
            );
        }
        if rows.len() > Self::WINDOW {
            return unsupported(
                "an action on a temporary Worksheet that refilled it with more rows than the client reads at once, a row the client held being gone (not pinned)",
            );
        }
        if self.declares(st, "onaftergetcurrrecord") {
            // With two rows or more the client first reads Rec's row (the first), then each row once, the first twice more, then
            // OnAfterGetCurrRecord (RmTempRefill.al:616 TD_RefillFewer_WithCurrRecord_3To2 `R:R1;R:R1;R:R1;R:R2;R:R1;R:R1;C:R1;`,
            // :629 _25To2, :642 _25To3, :681 _25To2_FromRow23 `R:R23;R:R1;R:R1;R:R2;R:R1;R:R1;C:R1;`, :694 _3To2_FromRow3).
            // With one row: each row once, the first twice more (`R:R1;R:R1;R:R1;C:R1;`).
            if rows.len() > 1 {
                self.read_refilled_row(st, &rec, &rows[0])?;
            }
            self.read_refilled_rows(st, &rows, 0)?;
            return Ok(true);
        }
        // Without OnAfterGetCurrRecord and with two rows or more, the row the client stood on is read once more when it is not
        // the first row (RmTempRefill.al:707 TD_RefillFewer_NoCurrRecord_3To2 `R:R1;R:R1;R:R1;R:R2;R:R1;`, :772 _25To2_FromRow23
        // `R:R23;R:R23;R:R1;R:R1;R:R2;R:R1;`): the rule behind it is not built.
        if rows.len() > 1 {
            return unsupported(
                "an action on a temporary Worksheet without OnAfterGetCurrRecord that refilled it to two rows or more, a row the client held being gone (its reads are not built)",
            );
        }
        // Without OnAfterGetCurrRecord: each row once, the first once more (with one row: `R:R1;R:R1;`).
        let mut held = Vec::new();
        for r in &rows {
            held.push(self.read_refilled_row(st, &rec, r)?);
        }
        let first = self.read_refilled_row(st, &rec, &rows[0])?;
        {
            let mut s = st.borrow_mut();
            held[0].shown = first.shown;
            s.window = held;
            s.window_from_start = true;
            s.window_eof = true;
            s.window_whole = true;
            s.view_read_to_end = false;
            s.window_read_up = false;
            s.window_top_unread = false;
            s.read_up_from_last = None;
            s.opened_on_moved_rec = false;
            s.settled_first_row = false;
            s.refresh = false;
            s.cursor = Cursor::Row;
        }
        Ok(true)
    }

    /// A List with InsertAllowed, ModifyAllowed and DeleteAllowed false: an action that rebuilds its temporary rows is read as on a
    /// temporary Worksheet (pages/lt-uj-outside-leads/UjTempList.al UJ_Temp_List_ThreeNewRows, UJ_Temp_List_OneRowRebuilt, corpus
    /// branch al-runtime-rs-staging-uj 1dc037a8; former assumption `list-action-rebuilds-rows` (pinned on corpus master since bump 1030) until the bump).
    fn temp_list_without_writes(&mut self, st: &St) -> bool {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        engine.object_facts[object].page_type == Some(al_ir::PageType::List)
            && ["InsertAllowed", "ModifyAllowed", "DeleteAllowed"].iter().all(|p| {
                engine.program.objects[object]
                    .properties
                    .iter()
                    .any(|(k, v)| k.eq_ignore_ascii_case(p) && v.trim().eq_ignore_ascii_case("false"))
            })
    }

    /// Whether a row the client holds is no longer in the table.
    fn held_row_gone(&mut self, st: &St) -> Result<bool, Signal> {
        let rec = self.page_rec(st);
        let held: Vec<WindowRow> = st.borrow().window.clone();
        for w in &held {
            let probe = Rc::new(RefCell::new(rec.borrow().clone()));
            probe.borrow_mut().filters.clear();
            let mut pk = w.key.clone();
            self.ret_used = true;
            if !matches!(self.record_op(RecordOp::Get, &probe, &mut pk)?, Value::Boolean(true)) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// The reads from row `at` of `rows` (the view from its start), then OnAfterGetCurrRecord on it; the client then holds the
    /// rows it read.
    fn read_refilled_rows(&mut self, st: &St, rows: &[Vec<Value>], at: usize) -> Result<(), Signal> {
        self.read_refilled_rows_as(st, rows, at, false)
    }

    /// A ListPart whose control's OnValidate ran `CurrPage.Update` (with or without `false`) on a Document host whose row did not
    /// change, standing on a later row of its view than the first: it stays on that row. It reads from the row to the end of the
    /// view, the row again when it moved, the rows before it back to the first, then the row twice and OnAfterGetCurrRecord
    /// (writer TH, pages/lt-th-part-update-cursor/ThPartUpdateCursor.al:357 TH_Line2_UpdateTrue `R2 R3 R2 R1 R2 R2 C2`, :373
    /// TH_Line2_UpdateFalse, :437 TH_Line3_UpdateFalse `R3 R2 R1 R3 R3 C3`, :453 TH_Walk_*; on branch al-runtime-rs-staging-pub-th,
    /// assumption part-keeps-row-after-update-in-validate). `rows` is the view from its start, up to a window.
    pub(super) fn read_part_rows_around(&mut self, st: &St, rows: &[Vec<Value>], at: usize) -> Result<(), Signal> {
        self.read_refilled_rows_as(st, rows, at, true)
    }

    fn read_refilled_rows_as(&mut self, st: &St, rows: &[Vec<Value>], at: usize, part: bool) -> Result<(), Signal> {
        let rec = self.page_rec(st);
        let before = self.xrec_before_move(st);
        let window_end = rows.len().min(at + Self::WINDOW);
        let mut held = Vec::new();
        if at > 0 && !part {
            self.read_refilled_row(st, &rec, &rows[at])?;
        }
        for r in &rows[at..window_end] {
            held.push(self.read_refilled_row(st, &rec, r)?);
        }
        if at > 0 {
            if window_end > at + 1 {
                self.read_refilled_row(st, &rec, &rows[at])?;
            }
            let mut above = Vec::new();
            for r in rows[..at].iter().rev() {
                above.push(self.read_refilled_row(st, &rec, r)?);
            }
            above.reverse();
            above.extend(held);
            held = above;
        }
        if !(part && at == 0 && window_end == 1) {
            self.read_refilled_row(st, &rec, &rows[at])?;
        }
        let current = self.read_refilled_row(st, &rec, &rows[at])?;
        let whole = rows.len() <= Self::WINDOW && held.len() == rows.len();
        {
            let mut s = st.borrow_mut();
            if let Some(w) = held.get_mut(at) {
                w.shown = current.shown;
            }
            s.window = held;
            s.window_from_start = true;
            s.window_eof = whole;
            s.window_whole = whole;
            s.view_read_to_end = false;
            s.window_read_up = false;
            s.window_top_unread = false;
            s.read_up_from_last = None;
            s.opened_on_moved_rec = false;
            s.settled_first_row = false;
            s.refresh = false;
            s.cursor = Cursor::Row;
        }
        self.current_record_trigger(st, before)
    }
}
