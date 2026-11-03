//! The refresh of `CurrPage.Update(false)` that a TestField.Lookup leaves for later on a host with row triggers whose ListPart
//! links two or three fields (al-runner issues 6466 and 6519): when the Lookup leaves it (`defer_lookup_refresh`), which calls
//! may run while it is pending, and which call runs it (`lookup_refresh_deferred_call`).
//!
//! The Y54 lines cited here (TestPart moves after the Lookup) are on corpus master (corpus PR 990, BC 27.5, 28.5 and 29.0)
//! and not yet in the pin: the bump after it turns them into ratcheted cases.

use super::shown::{Effect, shown_root, shown_tree};
use super::*;
use al_builtins::page_op::{PageMethod, PageOp};

impl Session {
    /// Before the refresh after the SetValue of TestField.Lookup on `st` (`control_set_value`): on a page with OnAfterGetRecord or OnAfterGetCurrRecord whose OnValidate
    /// asked for `CurrPage.Update(false)` and whose one part is a ListPart it hosts with a SubPageLink of two or three
    /// `field(..)` entries, BC leaves the refresh for later. The Lookup runs no OnAfterGetRecord or OnAfterGetCurrRecord on the host or
    /// the part, and reading a control after it runs no trigger: the field shows the picked value, the globals the row
    /// triggers set and the part keep what they showed before (corpus writer Y26,
    /// pages/lt-y15/Y15LookupThreeFieldLink.al:314 Y26_ThreeLinkCurr_LookupRefresh `lookup log`
    /// `lookup;h-fn R1;validate R2;h-fn R1`, `current no` `R1/`, `part current` `R1/`, every read log empty; :574
    /// Y26_ThreeLinkCurr_RegionsSet; two entries the same: :379 Y26_TwoLinkCurr_LookupRefresh, :509 Y26_TwoLinkPlain_LookupRefresh;
    /// al-runner issue 6519). Y26 measured one part, Y63 a second one (below); with others the Lookup is left as it was. BC computes the host's function controls within the Lookup (`h-fn`),
    /// which this runtime does not: a host with an expression control that calls anything is left as it was. The refresh
    /// runs at the next Lookup, Next or TestPart move (:639 Y26_ThreeLinkCurr_LookupTwice, :314 `next log`, :722 Y54 `part next
    /// log`); `lookup_refresh_deferred_call` runs it where that is built and refuses the other calls.
    pub(super) fn defer_lookup_refresh(&mut self, st: &St) {
        let shape = {
            let s = st.borrow();
            s.host.is_none()
                && s.open
                && !s.draft
                && s.cursor == Cursor::Row
                && matches!(s.parts.len(), 1..=4)
                && s.refresh
                && s.refresh_by_update
                && s.refresh_update_false
        };
        if !shape
            || !self.has_source(st)
            || !(self.declares(st, "onaftergetrecord") || self.declares(st, "onaftergetcurrrecord"))
            || self.declares(st, "onfindrecord")
            || self.declares(st, "onnextrecord")
        {
            return;
        }
        let object = st.borrow().object as usize;
        let fn_controls = self.fn_log_controls(st);
        let engine = self.engine();
        let page = page_of(&engine, st);
        let calls_nothing = page.controls.iter().enumerate().all(|(i, c)| match c.binding {
            // A function control runs after the Lookup's OnLookup and after its refresh (`fn_controls.rs`).
            ir::Binding::Expr(r) => {
                c.owner.is_none() && (call_free_routine(&engine.program, object, r) || fn_controls.contains(&i))
            }
            _ => true,
        });
        let part = |i: usize, q: &St, page_type: &str, links: usize| {
            let q = q.borrow();
            let c = &page.controls[i];
            matches!(&engine.object_facts[q.object as usize].page_type, Some(ir::PageType::Other(o)) if o == page_type)
                && q.parts.is_empty()
                && match &c.part_link {
                    None => links == 0,
                    Some(l) => {
                        l.view.is_none()
                            && l.provider.is_none()
                            && l.entries.len() == links
                            && l.entries.iter().all(|e| matches!(e, ir::LinkEntry::Field(..)))
                    }
                }
        };
        let factbox = |i: usize| page.controls[i].factbox;
        let parts: Vec<(usize, St)> = st.borrow().parts.iter().map(|(i, q)| (*i, q.clone())).collect();
        let parts_ok = match parts.as_slice() {
            [(i, q)] => part(*i, q, "listpart", 2) || part(*i, q, "listpart", 3),
            // A second part (corpus writer Y63, SaaS, Y15LookupThreeFieldLink.al Y63_TwoPart_LookupReadEachControl,
            // Y63_CardPart_LookupReadEachControl, al-runner issue 6639): `lookup log` `lookup;h-fn R1;validate R2;h-fn R1`, `read log`
            // and `close log` empty, every part `R1|R1/|R1/` as before. Measured with an unlinked ListPart declared before the
            // linked one, with it declared after (Y63_PartThenOther_LookupReadEachControl: the first read of a part that no call
            // reached before runs the refresh, `lookup_refresh_deferred_call`), and with a CardPart in the FactBoxes area linked
            // by one field.
            // Three and four parts (corpus writer R6806, al-runner issue 6806, R6806LookupParts.al R6806_*_LookupReadEachControl):
            // the same, with the layouts in `many_parts_pinned`.
            _ => {
                let kinds: Option<String> = parts
                    .iter()
                    .map(|(i, q)| {
                        if factbox(*i) {
                            part(*i, q, "cardpart", 1).then_some('K')
                        } else if part(*i, q, "listpart", 3) {
                            Some('L')
                        } else {
                            part(*i, q, "listpart", 0).then_some('O')
                        }
                    })
                    .collect();
                kinds.is_some_and(|k| Self::many_parts_pinned(&k))
            }
        };
        drop(engine);
        if calls_nothing && parts_ok {
            {
                let mut s = st.borrow_mut();
                s.refresh = false;
                s.lookup_refresh_deferred = true;
            }
            // The host's function controls run once more within the Lookup, after the OnValidate (Y15LookupThreeFieldLink.al:373
            // Y26_ThreeLinkCurr_LookupRefresh `lookup log` `lookup;h-fn R1;validate R2;h-fn R1`); a part's stay as they were
            // (`part fn` `R1/`).
            let _ = self.run_fn_controls(st, false);
        }
    }

    /// The layouts of a host's parts (in layout order: `L` the ListPart linked by three fields, `O` an unlinked ListPart, `K` a
    /// CardPart in the FactBoxes area linked by one field) on which the Lookup is measured to leave the refresh for later:
    /// two parts (Y63: `OL`, `LO`, `LK`), three (R6806, R6806LookupParts.al: `OOL`, `LOO`, `OLO`, with a card `OOLK`) and four with the
    /// linked part last (`OOOL`). Every other layout (five parts, four with the linked part not last, a second card) is not pinned.
    fn many_parts_pinned(kinds: &str) -> bool {
        matches!(kinds, "OL" | "LO" | "LK" | "OOL" | "LOO" | "OLO" | "OOLK" | "OOOL")
    }

    /// TestPage call `method` on page `st` (of control `target`) while a Lookup of its tree left its refresh for later
    /// (`defer_lookup_refresh`): reading a control that is a field, a variable or (on the host) an expression that calls nothing
    /// runs no trigger (Y15LookupThreeFieldLink.al:314, every `.. log` of a read empty), and Close runs none either (`close log` empty);
    /// any other call runs the refresh first (:639 `second lookup log`, :314 `next log`, :722 `part next log`), built for a TestPart
    /// move and a second Lookup on the measured shape below, refused otherwise.
    pub(super) fn lookup_refresh_deferred_call(
        &mut self,
        method: PageOp,
        st: &St,
        target: Option<&ControlTarget>,
    ) -> Result<(), Signal> {
        let root = shown_root(st);
        if method == PageOp::TestPageClose && Rc::ptr_eq(st, &root) {
            self.close_after_host_followed_part(&root)?;
        }
        if matches!(
            method,
            PageOp::TestPartNext | PageOp::TestPartFirst | PageOp::TestPartPrevious | PageOp::TestPartLast
        ) && target.is_none()
            && !Rc::ptr_eq(st, &root)
            && self.unlinked_part_of_two(st, &root)
            && !(method == PageOp::TestPartNext && self.host_follows_part_pinned(&root))
        {
            // Y63_TwoPart_PartsNextAfterCancel `other next log`, Y63_TwoPart_LinkedPartNextFirst: Next on the part without a link
            // reads the host's row and the linked part too (`host_follows_part`); pinned after a Lookup, and for Next only.
            return unsupported(format!(
                "{method} on the ListPart without a SubPageLink of a host with a second ListPart and function controls (the reads of the host that follows are pinned after a TestField.Lookup, for Next)"
            ));
        }
        if !root.borrow().lookup_refresh_deferred {
            return Ok(());
        }
        if root.borrow().parts.len() >= 3
            && matches!(
                method,
                PageOp::TestPartNext | PageOp::TestPartFirst | PageOp::TestPartPrevious | PageOp::TestPartLast
            )
            && !Rc::ptr_eq(st, &root)
            && target.is_none()
        {
            // R6806_ABL_OtherBNextFirst `OtherB next log`, R6806_ABL_ResourcesNextFirst: after the refresh the move adds reads of the host
            // and the parts that follow, which are not built for three or more parts.
            return unsupported(format!(
                "{method} on a part of a host with three or more parts after TestField.Lookup (the reads of the host and parts that follow the move are not built)"
            ));
        }
        if method == PageOp::TestPageClose && Rc::ptr_eq(st, &root) {
            root.borrow_mut().lookup_refresh_deferred = false;
            return Ok(());
        }
        if method == PageOp::TestFieldValue
            && let Some(ControlTarget::Control(i)) = target
        {
            let fn_list = self.fn_log_controls(st);
            let engine = self.engine();
            let c = &page_of(&engine, st).controls[*i];
            let read_ok = match c.binding {
                // A function control shows what its page last ran it with (`fn_controls.rs`); the host's other expressions it
                // computed within the Lookup (`h-fn`); a part did not (`part fn` `R1/`, before the pick).
                ir::Binding::Expr(r) => {
                    fn_list.contains(i)
                        || (Rc::ptr_eq(st, &root)
                            && c.owner.is_none()
                            && call_free_routine(&engine.program, st.borrow().object as usize, r))
                }
                _ => true,
            };
            if read_ok {
                // The first read of a part's control after the Lookup, a part no TestPage call reached before: BC runs the refresh
                // then, with the open's reads again (Y15LookupThreeFieldLink.al:1509 Y63_CardPart_CardReadAfterLookup `card no log`,
                // :1224 Y63_PartThenOther_LookupReadEachControl `part no log`, :1422 Y63_TwoPart_FirstPartReadAfterLookup `other no
                // log`, :1478 Y63_PartThenOther_PartReadBeforeLookup `other no log`: the part read before the Lookup shows
                // `R1|R1/` with no log). A part a call reached before the Lookup runs nothing (:1041 Y63_TwoPart_LookupReadEachControl,
                // :1287 Y63_CardPart_LookupReadEachControl: `before` read every control, `read log` empty). Pinned with a second
                // part; with one part the reads run nothing (Y26).
                if !Rc::ptr_eq(st, &root) && root.borrow().parts.len() > 1 && !st.borrow().lookup_part_touched {
                    root.borrow_mut().lookup_refresh_deferred = false;
                    self.replay_open_reads(&root)?;
                    root.borrow_mut().lookup_refresh_used = true;
                }
                return Ok(());
            }
        }
        // Measured on a host with OnAfterGetCurrRecord only, whose part declares no row trigger: there the refresh is the
        // host's OnAfterGetCurrRecord (Y15LookupThreeFieldLink.al:991 Y54_Y15Host_PartWalkAfterLookup `part next log` `agcr R1`,
        // the page's `open log` at :213). With OnAfterGetRecord BC reads the host's rows as when the page opens (:722 `part next
        // log`), which `run_refresh` does not.
        // A host with a second part (Y63): the first call on any part runs the refresh with every part, as the page opens
        // (`part next log`, `other next log`, `card next log`), which `run_refresh` does not: refused.
        let shape = root.borrow().parts.len() == 1
            && !self.declares(&root, "onaftergetrecord")
            && shown_tree(&root)
                .iter()
                .skip(1)
                .all(|q| !self.declares(q, "onaftergetrecord") && !self.declares(q, "onaftergetcurrrecord"));
        let part_move = matches!(
            method,
            PageOp::TestPartNext | PageOp::TestPartFirst | PageOp::TestPartPrevious | PageOp::TestPartLast
        ) && !Rc::ptr_eq(st, &root)
            && target.is_none();
        if part_move && !self.fn_log_controls(&root).is_empty() {
            // On a host with function controls the first TestPart move runs the refresh as the open's reads again, then moves the
            // part (Y15LookupThreeFieldLink.al:736 Y54_ThreeLinkCurr_PartNextAfterLookup `part next log` = `open log`, :1451
            // Y63_TwoPart_PartsReadBeforeThenNext); every control then shows the values as they are now.
            root.borrow_mut().lookup_refresh_deferred = false;
            self.replay_open_reads(&root)?;
            root.borrow_mut().lookup_refresh_used = true;
            return Ok(());
        }
        if shape && part_move {
            // The first TestPart move runs the refresh, with the parts, then moves the part (:991 `part next` `false|R1|G1`);
            // every control shows the values as they are now (`lookup_refresh_used_effect`).
            self.run_lookup_refresh_left(&root, true)?;
            root.borrow_mut().lookup_refresh_used = true;
            return Ok(());
        }
        if shape && method == PageOp::TestFieldLookup && Rc::ptr_eq(st, &root) {
            // A second Lookup runs it first, on the host only (:297 Y15_ThreeLink_LookupTwice `log`
            // `lookup;validate R2;agcr R1;lookup;validate R3`; no part trigger: :649 Y26_ThreeLinkCurr_LookupTwice
            // `second lookup log`).
            return self.run_lookup_refresh_left(&root, false);
        }
        if method == PageOp::TestFieldLookup && Rc::ptr_eq(st, &root) && self.lookup_next_refresh_shape(&root) {
            // A second Lookup runs it first, on the host only (Y26_ThreeLinkCurr_LookupTwice `second lookup log`).
            root.borrow_mut().lookup_refresh_deferred = false;
            return self.replay_open_reads_host(&root);
        }
        if method == PageOp::TestPageNext && Rc::ptr_eq(st, &root) && self.lookup_next_refresh_shape(&root) {
            root.borrow_mut().lookup_refresh_deferred = false;
            root.borrow_mut().lookup_next_refresh = true;
            return Ok(());
        }
        if shape && method == PageOp::TestPageNext && Rc::ptr_eq(st, &root) {
            // The host's own Next as the first call after the Lookup: the refresh runs first, then the host moves. Pinned for the
            // values only (Y15_ThreeLink_RegionsSetPartFollowsRow `next`); the refresh's log is not observed there.
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::LookupPendingRefreshAtHostNext);
            return self.run_lookup_refresh_left(&root, true);
        }
        unsupported(format!(
            "{method} after TestField.Lookup on a page with OnAfterGetRecord or OnAfterGetCurrRecord whose ListPart links two or \
             three fields: BC runs the refresh of the OnValidate's CurrPage.Update(false) first (not built)"
        ))
    }

    /// `st` is a ListPart without a SubPageLink of a host `root` with function controls and two ListParts.
    pub(super) fn unlinked_part_of_two(&mut self, st: &St, root: &St) -> bool {
        let Some((_, idx)) = st.borrow().host.as_ref().map(|(h, i)| (h.clone(), *i)) else {
            return false;
        };
        let engine = self.engine();
        let list_parts = root
            .borrow()
            .parts
            .iter()
            .filter(|(_, q)| {
                matches!(&engine.object_facts[q.borrow().object as usize].page_type,
                    Some(ir::PageType::Other(o)) if o == "listpart")
            })
            .count();
        let unlinked = page_of(&engine, root).controls[idx]
            .part_link
            .as_ref()
            .is_none_or(|l| l.entries.is_empty());
        list_parts == 2 && unlinked && !self.fn_log_controls(root).is_empty()
    }

    /// First or Last on a part of one row, after the refresh a Lookup left for later ran as the open's reads again
    /// (`replay_open_reads`): the part stays on its row and reads nothing (Y15LookupThreeFieldLink.al:774
    /// Y54_ThreeLinkCurr_PartFirstAfterLookup, :838 _PartLastAfterLookup `part first log` and `part last log`: only the refresh's
    /// reads; :806 _PartPreviousAfterLookup `part last log` empty).
    pub(super) fn part_stays_after_lookup_refresh(&self, st: &St) -> bool {
        let Some(root) = st.borrow().host.as_ref().map(|_| shown_root(st)) else {
            return false;
        };
        let s = st.borrow();
        root.borrow().lookup_refresh_used
            && s.open
            && s.cursor == Cursor::Row
            && !s.draft
            && !s.dirty
            && s.window.len() == 1
    }

    /// Runs the refresh of `CurrPage.Update(false)` that a TestField.Lookup on `root` left for later (`defer_lookup_refresh`),
    /// as the Lookup would have run it (`run_refresh`); `with_parts`: the parts read their rows again after it.
    fn run_lookup_refresh_left(&mut self, root: &St, with_parts: bool) -> Result<(), Signal> {
        {
            let mut s = root.borrow_mut();
            s.lookup_refresh_deferred = false;
            s.refresh = true;
            s.refresh_by_update = true;
            s.refresh_without_parts = !with_parts;
        }
        self.run_refresh(root)
    }

    /// What a TestPage call on `st` (of control `target`) does to the values every page of the tree of `root` shows, once the
    /// first TestPart move ran the refresh a Lookup left (`lookup_refresh_deferred_call`); `None` when the usual rules apply.
    /// `clean`: the call succeeded and the page's code called no `CurrPage` method.
    /// - That move, and the part's later Next, First, Previous and Last, leave every control showing the values as they are now
    ///   (Y15LookupThreeFieldLink.al:991 Y54_Y15Host_PartWalkAfterLookup `part walk log` empty, `host` `R2|R1`; :722
    ///   Y54_ThreeLinkCurr_PartNextAfterLookup `after part next`, `part next again log` empty).
    /// - The host's Next then moves as usual and its controls show the new row (:991 `next` `true|R2|R2|R2`, `next log`
    ///   `agcr R2`; :722 `after next`). It ends the rule, as does any other call but a control's Value or Visible.
    pub(super) fn lookup_refresh_used_effect(
        &mut self,
        method: PageOp,
        st: &St,
        target: Option<&ControlTarget>,
        root: &St,
        clean: bool,
    ) -> Option<Effect> {
        if !root.borrow().lookup_refresh_used {
            return None;
        }
        let part_move = matches!(
            method,
            PageOp::TestPartNext | PageOp::TestPartFirst | PageOp::TestPartPrevious | PageOp::TestPartLast
        ) && !Rc::ptr_eq(st, root)
            && target.is_none();
        let read = matches!(method.method(), PageMethod::Value | PageMethod::Visible)
            && matches!(target, Some(ControlTarget::Control(_)));
        if read && clean {
            return None;
        }
        if clean && (part_move || (method == PageOp::TestPageNext && Rc::ptr_eq(st, root))) {
            if method == PageOp::TestPageNext {
                root.borrow_mut().lookup_refresh_used = false;
            }
            return Some(Effect::Refresh);
        }
        root.borrow_mut().lookup_refresh_used = false;
        None
    }
}

/// Routine `r` of `object` only returns an expression of constants, variables, fields and operators: it calls nothing.
pub(super) fn call_free_routine(program: &ir::Program, object: usize, r: usize) -> bool {
    fn call_free(e: &ir::Expr) -> bool {
        match e {
            ir::Expr::Const(_) | ir::Expr::Load(_) => true,
            ir::Expr::Unary(_, a) | ir::Expr::Convert(_, a) | ir::Expr::FieldOf(a, _) => call_free(a),
            ir::Expr::Binary(_, a, b) => call_free(a) && call_free(b),
            ir::Expr::Ternary(a, b, c) => call_free(a) && call_free(b) && call_free(c),
            _ => false,
        }
    }
    let Some(routine) = program.objects[object].routines.get(r) else {
        return false;
    };
    let mut body = routine
        .body
        .stmts()
        .iter()
        .filter(|s| !matches!(s, ir::Stmt::Pos(_)))
        .peekable();
    body.peek().is_some() && body.all(|s| matches!(s, ir::Stmt::Exit(Some(e)) if call_free(e)))
}
