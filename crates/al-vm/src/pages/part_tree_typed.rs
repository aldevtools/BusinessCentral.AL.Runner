//! A ListPart whose repeater is a tree keeps the value typed on a row after the part's `CurrPage.Update(false)`; any other
//! part drops it. Pinned by pages/parts/R6889RealShape.al (objects 96130-96147, identical on BC 27.5, 28.5 and 29.0, al-runner
//! issue 6889): the tree parts of steps 2, 2b, 3 and 4 show the typed Type and store it (the Type SetValue writes the row, log
//! `M30:Resource/N30`, before the part reads its rows), steps 0, 1, 2a and 2c without a tree drop it (:1364 PG6889RS_Step2_,
//! :1438 _Step2b_TreeOnly, :1420 _Step2a_, :1456 _Step2c_). What the file does not split (ShowAsTree alone, another
//! IndentationColumn or TreeInitialState, a part without UpdatePropagation Both on its host) is refused.

use super::*;

impl Session {
    /// A part over a table, standing on no row, whose repeater is a tree and whose view is sorted by another key than the primary
    /// key. Its action that leaves rows is read by `land_empty_tree_part_after_action` in the one shape
    /// pages/parts/R6889RealShape.al:1213 RunStep4, :1400 PG6889RS_Step4_ pin; any other is refused.
    pub(super) fn empty_tree_part_in_key(&self, st: &St) -> bool {
        if st.borrow().host.is_none() || st.borrow().cursor != Cursor::None || !self.has_source(st) {
            return false;
        }
        let rec = self.page_rec(st);
        let other_key = {
            let r = rec.borrow();
            r.key != 0 || r.adhoc.is_some()
        };
        other_key && self.part_is_tree(st)
    }

    pub(super) fn refuse_empty_tree_part_action(&self) -> Result<Value, Signal> {
        unsupported(
            "an action of a tree part in another key than the primary key that showed no row before it and left rows (the reads are not modelled)",
        )
    }

    /// The action of an `empty_tree_part_in_key` left rows: the part reads the row the action left in Rec, then the rows around
    /// it as a part in another key does (the row twice more, the row before it, the row three times, OnAfterGetCurrRecord), and
    /// then goes to the first row of its view: that row three times and OnAfterGetCurrRecord on it. Pinned for an action that
    /// leaves two rows with Rec on the second, on a part with both row triggers and without OnFindRecord or OnNextRecord
    /// (al-runner issue 6889; pages/parts/R6889RealShape.al:1213 RunStep4, :1400 PG6889RS_Step4_ `line after action`
    /// `10000/O10000/L0`, `log action` `...;T.INS(20);R20;R20;R20;R10;R20;R20;R20;C20;R10;R10;R10;C10`). Pinned on the host and
    /// tree of that step (`on_pinned_tree_host`) and for an action that called neither CurrPage.Update nor CurrPage.SaveRecord
    /// (:430 NewChildCodeunit: `Lines.NewChildLine(Rec); Rec.Next();`). Another row count or place of Rec, another host, open mode
    /// or tree, and an action that asked for an update are refused.
    pub(super) fn land_empty_tree_part_after_action(&mut self, st: &St) -> Result<(), Signal> {
        let rec = self.page_rec(st);
        let rows = self.view_rows(&rec, 3)?;
        let table = self.table(&rec)?;
        let held = self.row_key(&rec)?;
        let update_asked = st.borrow().refresh || st.borrow().save_record_called;
        let pinned = rows.len() == 2
            && Self::same_key(&table.primary_key(&rows[1]), &held)
            && self.on_pinned_tree_host(st)
            && !update_asked
            && self.is_list(st)
            && !st.borrow().dirty
            && !st.borrow().draft
            && self.declares(st, "onaftergetrecord")
            && self.declares(st, "onaftergetcurrrecord")
            && !self.declares(st, "onfindrecord")
            && !self.declares(st, "onnextrecord");
        if !pinned {
            return self.refuse_empty_tree_part_action().map(|_| ());
        }
        st.borrow_mut().cursor = Cursor::Row;
        let before = self.xrec_before_move(st);
        for _ in 0..3 {
            self.read_refilled_row(st, &rec, &rows[1])?;
        }
        self.read_refilled_row(st, &rec, &rows[0])?;
        let mut second = self.read_refilled_row(st, &rec, &rows[1])?;
        for _ in 0..2 {
            second = self.read_refilled_row(st, &rec, &rows[1])?;
        }
        self.current_record_trigger(st, before)?;
        let before = self.xrec_before_move(st);
        let mut first = self.read_refilled_row(st, &rec, &rows[0])?;
        for _ in 0..2 {
            first = self.read_refilled_row(st, &rec, &rows[0])?;
        }
        Self::hold_whole_view(st, vec![first, second]);
        st.borrow_mut().empty_tree_part_landed = true;
        self.current_record_trigger(st, before)
    }

    /// The part holds `window`, its whole view read from the start, and stands on a stored row of it.
    fn hold_whole_view(st: &St, window: Vec<al_builtins::value::WindowRow>) {
        let mut s = st.borrow_mut();
        s.window = window;
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

    /// A tree part in another key than the primary key that saved its typed row on the first of the two rows of its view
    /// (`save_typed_tree_row`): it reads the row twice, the row after it, the row three times, then OnAfterGetCurrRecord
    /// (pages/parts/R6889RealShape.al:1400 PG6889RS_Step4_ `log type` `...;SE;M10:Resource/N10;R10;R10;R20;R10;R10;R10;C10`,
    /// `after type` `10000 Resource`). Another view, host, open mode or tree (`on_pinned_tree_host`) is refused.
    pub(super) fn read_typed_tree_first_row(&mut self, st: &St, rows: &[Vec<Value>]) -> Result<(), Signal> {
        let rec = self.page_rec(st);
        let other_key = {
            let r = rec.borrow();
            r.key != 0 || r.adhoc.is_some()
        };
        let pinned = rows.len() == 2
            && other_key
            && self.on_pinned_tree_host(st)
            && self.declares(st, "onaftergetrecord")
            && self.declares(st, "onaftergetcurrrecord")
            && !self.declares(st, "onfindrecord")
            && !self.declares(st, "onnextrecord");
        if !pinned {
            return unsupported(
                "a tree part on its first row whose code ran CurrPage.Update after a typed value, other than on the first of two rows in another key than the primary key, on a ListPlus host opened for edit (not pinned)",
            );
        }
        let before = self.xrec_before_move(st);
        self.read_refilled_row(st, &rec, &rows[0])?;
        self.read_refilled_row(st, &rec, &rows[0])?;
        let second = self.read_refilled_row(st, &rec, &rows[1])?;
        let mut first = self.read_refilled_row(st, &rec, &rows[0])?;
        for _ in 0..2 {
            first = self.read_refilled_row(st, &rec, &rows[0])?;
        }
        Self::hold_whole_view(st, vec![first, second]);
        self.current_record_trigger(st, before)
    }

    /// Whether part `st` has the host and the tree of pages/parts/R6889RealShape.al step 4 (Part4 at :370, Host4 at :605), the
    /// only ones the reads of an empty tree part's action and of a typed first row are pinned on: a ListPlus host opened for
    /// edit (:1220 `P.OpenEdit()`) whose part control says UpdatePropagation = Both, and a tree with IndentationColumn = Rec.Level
    /// and TreeInitialState = ExpandAll. A Card or Document host, a host opened for view, a part without Both and another tree
    /// are not pinned.
    fn on_pinned_tree_host(&self, st: &St) -> bool {
        let host = st
            .borrow()
            .host
            .as_ref()
            .and_then(|(h, c)| h.upgrade().map(|h| (h, *c)));
        let Some((host, c)) = host else {
            return false;
        };
        let engine = self.engine();
        let both = matches!(
            page_of(&engine, &host).controls[c].prop("updatepropagation"),
            Some(ir::PropVal::Lit(v)) if v.eq_ignore_ascii_case("both")
        );
        let list_plus = engine.object_facts[host.borrow().object as usize].page_type == Some(al_ir::PageType::ListPlus);
        let edit = host.borrow().mode == PageMode::Edit;
        both && list_plus && edit && self.pinned_tree_properties(st) == Some(true)
    }

    /// Whether the tree of page `st` says IndentationColumn = Rec.Level and TreeInitialState = ExpandAll, the properties of the
    /// pinned parts (R6889RealShape.al:277, :386, :768). `None`: the page's repeater is no tree.
    fn pinned_tree_properties(&self, st: &St) -> Option<bool> {
        let engine = self.engine();
        let tree = page_of(&engine, st).controls.iter().find(|c| {
            c.kind == ir::ControlKind::Repeater
                && matches!(c.prop("showastree"), Some(ir::PropVal::Lit(v)) if unquote(v).eq_ignore_ascii_case("true"))
        })?;
        let lit = |name: &str| match tree.prop(name) {
            Some(ir::PropVal::Lit(v)) => Some(unquote(v)),
            _ => None,
        };
        Some(
            lit("indentationcolumn").is_some_and(|v| v.replace(' ', "").eq_ignore_ascii_case("Rec.Level"))
                && lit("treeinitialstate").is_some_and(|v| v.eq_ignore_ascii_case("expandall")),
        )
    }

    /// Whether the part's repeater is a tree (`ShowAsTree = true`).
    pub(super) fn part_is_tree(&self, st: &St) -> bool {
        let engine = self.engine();
        page_of(&engine, st).controls.iter().any(|c| {
            c.kind == ir::ControlKind::Repeater
                && matches!(c.prop("showastree"), Some(ir::PropVal::Lit(v)) if unquote(v).eq_ignore_ascii_case("true"))
        })
    }

    /// The row `st` (a part) holds as typed, when its tree keeps it across the part's update: returns the stored row (xRec) the
    /// save compares with. `None`: the part drops the typed row as before (no typed row, or no tree).
    pub(super) fn tree_part_keeps_typed_row(&self, st: &St, both: bool) -> Result<Option<Vec<Value>>, Signal> {
        if !st.borrow().dirty || st.borrow().draft {
            return Ok(None);
        }
        let Some(pinned) = self.pinned_tree_properties(st) else {
            return Ok(None);
        };
        if !pinned || !both || !self.is_list(st) {
            return unsupported(
                "a tree part (ShowAsTree) other than IndentationColumn = Rec.Level and TreeInitialState = ExpandAll on a host part with UpdatePropagation Both, whose code ran CurrPage.Update after a typed value (whether it keeps the value is not pinned)",
            );
        }
        Ok(Some(self.page_xrec_fields(st)))
    }

    /// The tree part saves the row it holds as typed (`before` is the stored row), then reads its rows around it again: the view
    /// as the table has it now and the row's place in it.
    pub(super) fn save_typed_tree_row(
        &mut self,
        st: &St,
        before: Vec<Value>,
    ) -> Result<(Vec<Vec<Value>>, usize), Signal> {
        self.set_xrec_fields(st, before);
        st.borrow_mut().dirty = true;
        self.save_row(st)?;
        let rec = self.page_rec(st);
        let rows = self.view_rows(&rec, Self::WINDOW + 1)?;
        let table = self.table(&rec)?;
        let held = self.row_key(&rec)?;
        match rows.iter().position(|r| Self::same_key(&table.primary_key(r), &held)) {
            Some(at) if rows.len() <= Self::WINDOW => Ok((rows, at)),
            _ => unsupported(
                "a tree part that saved its typed row and whose view no longer holds it or is longer than a window (not pinned)",
            ),
        }
    }
}
