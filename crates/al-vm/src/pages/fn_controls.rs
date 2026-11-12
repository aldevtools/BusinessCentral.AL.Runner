//! The function controls of a page in a tree with parts (al-runner issue 6639). BC runs a control whose source is a function
//! right after each OnAfterGetRecord and OnAfterGetCurrRecord of its page, and the client shows the value that run gave until
//! a later run: a read of the control runs no code (testpage/Y15LookupThreeFieldLink.al:373 Y26_ThreeLinkCurr_LookupRefresh
//! `open log` `h-agr R1;h-fn R1;...;p-agcr R1;p-fn R1`, every `.. log` of a read empty). `PageState::fn_rows` holds the values
//! per row key.
//!
//! The open of such a host reads like this (the same test, `open log`; Y26_ThreeLinkPlain_LookupRefresh at :527 for a host
//! without OnAfterGetCurrRecord; Y63_TwoPart_LookupReadEachControl, Y63_PartThenOther_LookupReadEachControl and
//! Y63_CardPart_LookupReadEachControl for two ListParts and a CardPart):
//! - a list that reads more than one row reads its first row once more right after the window, and that read runs no
//!   function control (`fn_open_skipped`);
//! - after the parts opened, the host reads its current row again (OnAfterGetRecord, then OnAfterGetCurrRecord when it declares
//!   it), once for each ListPart; with two ListParts the first one declared then reads its current row again too
//!   (`fn_open_tail`). Other counts of ListParts are not pinned and read as before;
//! - a TestField.Lookup runs the host's function controls once when OnLookup returns, picked or not, and once more after the
//!   OnValidate when the Lookup left the refresh for later (`lookup_trigger_call`, `defer_lookup_refresh`).

use super::lookup_refresh::call_free_routine;
use super::shown::shown_tree;
use super::*;
use al_builtins::page_op::{PageMethod, PageOp};

impl Session {
    /// The controls of `st` run after its row triggers: its own expression controls the page does not keep (`kept_controls`)
    /// whose expression calls something, when the page has parts or is one, and declares OnAfterGetRecord or
    /// OnAfterGetCurrRecord.
    pub(super) fn fn_log_controls(&mut self, st: &St) -> Vec<usize> {
        {
            let s = st.borrow();
            if s.parts.is_empty() && s.host.is_none() {
                return Vec::new();
            }
        }
        if !self.has_source(st) || !(self.declares(st, "onaftergetrecord") || self.declares(st, "onaftergetcurrrecord"))
        {
            return Vec::new();
        }
        let kept = self.kept_controls(st);
        let engine = self.engine();
        let object = st.borrow().object as usize;
        page_of(&engine, st)
            .controls
            .iter()
            .enumerate()
            .filter(|(i, c)| match c.binding {
                ir::Binding::Expr(r) => {
                    c.owner.is_none()
                        && c.array_element.is_none()
                        && !kept.contains(i)
                        && !call_free_routine(&engine.program, object, r)
                }
                _ => false,
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Runs the function controls of `st` for the row it stands on and notes what they show. `agr`: right after its
    /// OnAfterGetRecord.
    pub(super) fn run_fn_controls(&mut self, st: &St, agr: bool) -> Result<(), Signal> {
        let controls = self.fn_log_controls(st);
        if controls.is_empty() || st.borrow().draft || st.borrow().cursor != Cursor::Row {
            return Ok(());
        }
        let key = self.row_key(&self.page_rec(st))?;
        if agr {
            let mut s = st.borrow_mut();
            if std::mem::take(&mut s.fn_skip_agr) {
                return Ok(());
            }
            if !s.fn_open_skipped && s.window.len() > 1 && s.fn_rows.iter().any(|(k, _)| Self::same_key(k, &key)) {
                s.fn_open_skipped = true;
                return Ok(());
            }
        }
        let mut values = Vec::with_capacity(controls.len());
        for idx in controls {
            let ir::Binding::Expr(r) = page_of(&self.engine(), st).controls[idx].binding else {
                continue;
            };
            values.push((idx, self.control_routine(st, idx, r).ok()));
        }
        let mut s = st.borrow_mut();
        match s.fn_rows.iter_mut().find(|(k, _)| Self::same_key(k, &key)) {
            Some((_, held)) => *held = values,
            None => s.fn_rows.push((key, values)),
        }
        Ok(())
    }

    /// Runs the function controls of `st` on the blank line it stands on, noting nothing (`part_filter_runs_fn_on_new_line`).
    pub(super) fn run_fn_controls_on_new_line(&mut self, st: &St) -> Result<(), Signal> {
        for idx in self.fn_log_controls(st) {
            let ir::Binding::Expr(r) = page_of(&self.engine(), st).controls[idx].binding else {
                continue;
            };
            // What BC shows for an expression that fails there is not observed: the error is dropped as at a row read.
            let _ = self.control_routine(st, idx, r);
        }
        Ok(())
    }

    /// `populate_current` for the first of the two reads a list makes of a row it moves onto (`read_client_row_in`): that read
    /// runs no function control (Y15LookupThreeFieldLink.al:736 Y54_ThreeLinkCurr_PartNextAfterLookup `next log`
    /// `h-agr R2;h-agr R2;h-fn R2;h-agcr R2;h-fn R2`).
    pub(super) fn populate_current_pair_first(&mut self, st: &St) -> Result<(), Signal> {
        // A host that already read the row for its unlinked ListPart (`host_follows_part`) reads it once (Y63_TwoPart_LinkedPartNextFirst
        // `next log`).
        if st.borrow().host_followed_part {
            return Ok(());
        }
        if !self.fn_log_controls(st).is_empty() {
            st.borrow_mut().fn_skip_agr = true;
        }
        let r = self.populate_current(st);
        st.borrow_mut().fn_skip_agr = false;
        r
    }

    /// What function control `idx` of `st` showed when its page last ran it for the row the cursor stands on; `None` when the
    /// page keeps no such value.
    pub(super) fn fn_control_value(&mut self, st: &St, idx: usize) -> Result<Option<Value>, Signal> {
        {
            let s = st.borrow();
            if s.fn_rows.is_empty() || s.cursor != Cursor::Row || s.draft {
                return Ok(None);
            }
        }
        let key = self.row_key(&self.page_rec(st))?;
        let held = st
            .borrow()
            .fn_rows
            .iter()
            .find(|(k, _)| Self::same_key(k, &key))
            .and_then(|(_, v)| v.iter().find(|(c, _)| *c == idx).map(|(_, v)| v.clone()));
        match held {
            None => Ok(None),
            Some(Some(v)) => Ok(Some(v)),
            Some(None) => unsupported(format!(
                "control {} shows an expression that failed when its page ran it (what BC shows then is not pinned)",
                page_of(&self.engine(), st).controls[idx].name
            )),
        }
    }

    /// The host's current row once more, as the client reads it after the parts opened: OnAfterGetRecord, then
    /// OnAfterGetCurrRecord when the page declares it (each runs the page's function controls).
    pub(super) fn fn_current_stage(&mut self, st: &St) -> Result<(), Signal> {
        if st.borrow().draft || st.borrow().cursor != Cursor::Row {
            return Ok(());
        }
        self.page_trigger(st, "onaftergetrecord", vec![])?;
        if self.declares(st, "onaftergetcurrrecord") {
            self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
        }
        Ok(())
    }

    /// After the open of a host with function controls and one to four ListParts (module comment).
    pub(super) fn fn_open_tail(&mut self, st: &St) -> Result<(), Signal> {
        if self.fn_log_controls(st).is_empty() {
            return Ok(());
        }
        let engine = self.engine();
        let list_parts: Vec<St> = st
            .borrow()
            .parts
            .iter()
            .filter(|(_, q)| {
                matches!(&engine.object_facts[q.borrow().object as usize].page_type,
                    Some(ir::PageType::Other(o)) if o == "listpart")
            })
            .map(|(_, q)| q.clone())
            .collect();
        let n = list_parts.len();
        if !(1..=4).contains(&n) {
            return Ok(());
        }
        // The host's current row n times, then each ListPart but the last of the layout once less than the one before
        // (Y63 with one and two ListParts; R6806LookupParts.al R6806_*_LookupReadEachControl `open log` with three and four, in
        // every position of the linked part measured there).
        for _ in 0..n {
            self.fn_current_stage(st)?;
        }
        for (i, part) in list_parts.iter().enumerate().take(n - 1) {
            for _ in 0..(n - 1 - i) {
                self.fn_current_stage(part)?;
            }
        }
        Ok(())
    }

    /// After the host's Next with function controls and one ListPart (and a CardPart): the host reads its current row once more
    /// after the parts followed (Y15LookupThreeFieldLink.al:736 Y54_ThreeLinkCurr_PartNextAfterLookup `next log`, :1329
    /// Y63_CardPart_LinkedPartNextFirst). The moves with two ListParts read differently (:1083) and are left as they were.
    pub(super) fn fn_next_tail(&mut self, st: &St) -> Result<(), Signal> {
        if self.fn_log_controls(st).is_empty() {
            return Ok(());
        }
        let engine = self.engine();
        let list_parts = st
            .borrow()
            .parts
            .iter()
            .filter(|(_, q)| {
                matches!(&engine.object_facts[q.borrow().object as usize].page_type,
                    Some(ir::PageType::Other(o)) if o == "listpart")
            })
            .count();
        if list_parts == 2 && st.borrow().host_followed_part {
            // The host, then the unlinked ListPart read the row the host moved to (Y63_TwoPart_LinkedPartNextFirst `next log`).
            self.fn_current_stage(st)?;
            let unlinked: Vec<St> = st.borrow().parts.iter().map(|(_, q)| q.clone()).collect();
            for q in unlinked {
                if self.unlinked_part_of_two(&q, st) {
                    self.fn_current_stage(&q)?;
                }
            }
            return Ok(());
        }
        if list_parts == 1 {
            // After the refresh a Lookup left, a host without OnAfterGetCurrRecord reads its row as the client does a row it
            // holds (`lookup_next_refresh`).
            if st.borrow().lookup_next_plain {
                self.page_trigger_skipping_fn(st)?;
            }
            self.fn_current_stage(st)?;
        }
        Ok(())
    }

    /// Notes whether the tree of `root` stands as a cancelled TestField.Lookup left it (`PageState::lookup_cancelled`), on a host
    /// with function controls: reads and part moves keep it, any other call ends it. `clean`: the call succeeded and the
    /// page's code called no `CurrPage` method.
    pub(super) fn note_lookup_cancelled(
        &mut self,
        method: PageOp,
        st: &St,
        target: Option<&ControlTarget>,
        root: &St,
        clean: bool,
    ) {
        use al_builtins::page_op::PageMethod as M;
        if method == PageOp::TestFieldLookup {
            let cancelled = clean
                && Rc::ptr_eq(st, root)
                && !root.borrow().lookup_value_set
                && !root.borrow().lookup_refresh_deferred
                && !self.fn_log_controls(root).is_empty();
            root.borrow_mut().lookup_cancelled = cancelled;
            return;
        }
        let read =
            matches!(method.method(), M::Value | M::Visible) && matches!(target, Some(ControlTarget::Control(_)));
        let part_move = matches!(method.method(), M::First | M::Next | M::Last | M::Previous)
            && !Rc::ptr_eq(st, root)
            && target.is_none();
        if !(clean && (read || part_move)) {
            root.borrow_mut().lookup_cancelled = false;
        }
    }

    /// `refresh_parts_asked_by_host` at the end of the open, then the host's reads after the parts (`fn_open_tail`).
    pub(super) fn refresh_parts_then_fn_tail(&mut self, st: &St, waiting: &[(usize, St)]) -> Result<(), Signal> {
        self.refresh_parts_asked_by_host(st, waiting, true, false)?;
        self.fn_open_tail(st)
    }

    /// `sync_parts_after_move` after the host's Next or Previous, then the host's reads after the parts for Next
    /// (`fn_next_tail`).
    pub(super) fn sync_parts_after_move_fn(&mut self, st: &St, name: PageMethod) -> Result<(), Signal> {
        self.lookup_next_refresh(st)?;
        self.sync_parts_after_move(st, name == PageMethod::Previous)?;
        if name == PageMethod::Next {
            self.fn_next_tail(st)?;
        }
        st.borrow_mut().lookup_next_plain = false;
        Ok(())
    }

    /// The OnLookup trigger of a control: when it returns, picked or not, the host's function controls run (module comment).
    pub(super) fn lookup_trigger_call(
        &mut self,
        st: &St,
        object: usize,
        routine: usize,
        inst: Rc<RefCell<al_builtins::value::Instance>>,
        args: Vec<Value>,
    ) -> Result<(Value, Vec<Value>), Signal> {
        let r = self.page_call(st, object, routine, inst, args)?;
        self.run_fn_controls(st, false)?;
        Ok(r)
    }
}

impl Session {
    /// The refresh a TestField.Lookup left for later (`defer_lookup_refresh`), run by the first read of a control of the part
    /// laid out first: the host and its parts read their rows as they do when the page opens, the log of the read is the
    /// open's (Y15LookupThreeFieldLink.al:1509 Y63_CardPart_CardReadAfterLookup `card no log` = `open log`, :1224
    /// Y63_PartThenOther_LookupReadEachControl `part no log`, :1422 Y63_TwoPart_FirstPartReadAfterLookup `other no log`), and
    /// every control then shows the values as they are now (`after` `R2|R1/R2|R1|R1/R2|R1/R2|...`).
    pub(super) fn replay_open_reads(&mut self, root: &St) -> Result<(), Signal> {
        for p in shown_tree(root) {
            let mut s = p.borrow_mut();
            s.window.clear();
            s.fn_rows.clear();
            s.fn_open_skipped = false;
        }
        let mode = root.borrow().mode;
        let waiting = self.parts_waiting_refresh(root);
        root.borrow_mut().open_read = true;
        let loaded = self.load_first_row(root, mode, false);
        root.borrow_mut().open_read = false;
        loaded?;
        self.sync_parts_at_open(root)?;
        self.refresh_parts_asked_by_host(root, &waiting, true, false)?;
        self.fn_open_tail(root)?;
        self.shown_opened(root);
        Ok(())
    }
}
