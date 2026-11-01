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
    /// al-runner issue 6519). Y26 measured one part; with more the Lookup is left as it was. BC computes the host's function controls within the Lookup (`h-fn`),
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
                && s.parts.len() == 1
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
        let engine = self.engine();
        let page = page_of(&engine, st);
        let object = st.borrow().object as usize;
        let calls_nothing = page.controls.iter().all(|c| match c.binding {
            ir::Binding::Expr(r) => c.owner.is_none() && call_free_routine(&engine.program, object, r),
            _ => true,
        });
        let parts_ok = st.borrow().parts.iter().all(|(i, q)| {
            let q = q.borrow();
            let list_part = matches!(
                &engine.object_facts[q.object as usize].page_type,
                Some(ir::PageType::Other(o)) if o == "listpart"
            );
            list_part
                && q.parts.is_empty()
                && page.controls[*i].part_link.as_ref().is_some_and(|l| {
                    l.view.is_none()
                        && l.provider.is_none()
                        && matches!(
                            l.entries.as_slice(),
                            [ir::LinkEntry::Field(..), ir::LinkEntry::Field(..)]
                                | [
                                    ir::LinkEntry::Field(..),
                                    ir::LinkEntry::Field(..),
                                    ir::LinkEntry::Field(..)
                                ]
                        )
                })
        });
        drop(engine);
        if calls_nothing && parts_ok {
            let mut s = st.borrow_mut();
            s.refresh = false;
            s.lookup_refresh_deferred = true;
        }
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
        if !root.borrow().lookup_refresh_deferred {
            return Ok(());
        }
        if method == PageOp::TestPageClose && Rc::ptr_eq(st, &root) {
            root.borrow_mut().lookup_refresh_deferred = false;
            return Ok(());
        }
        if method == PageOp::TestFieldValue
            && let Some(ControlTarget::Control(i)) = target
        {
            let engine = self.engine();
            let c = &page_of(&engine, st).controls[*i];
            let read_ok = match c.binding {
                // The host computed its expressions within the Lookup (`h-fn`); a part did not (`part fn` `R1/`, before the pick).
                ir::Binding::Expr(r) => {
                    Rc::ptr_eq(st, &root)
                        && c.owner.is_none()
                        && call_free_routine(&engine.program, st.borrow().object as usize, r)
                }
                _ => true,
            };
            if read_ok {
                return Ok(());
            }
        }
        // Measured on a host with OnAfterGetCurrRecord only, whose part declares no row trigger: there the refresh is the
        // host's OnAfterGetCurrRecord (Y15LookupThreeFieldLink.al:991 Y54_Y15Host_PartWalkAfterLookup `part next log` `agcr R1`,
        // the page's `open log` at :213). With OnAfterGetRecord BC reads the host's rows as when the page opens (:722 `part next
        // log`), which `run_refresh` does not.
        let shape = !self.declares(&root, "onaftergetrecord")
            && shown_tree(&root)
                .iter()
                .skip(1)
                .all(|q| !self.declares(q, "onaftergetrecord") && !self.declares(q, "onaftergetcurrrecord"));
        let part_move = matches!(
            method,
            PageOp::TestPartNext | PageOp::TestPartFirst | PageOp::TestPartPrevious | PageOp::TestPartLast
        ) && !Rc::ptr_eq(st, &root)
            && target.is_none();
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
        unsupported(format!(
            "{method} after TestField.Lookup on a page with OnAfterGetRecord or OnAfterGetCurrRecord whose ListPart links two or \
             three fields: BC runs the refresh of the OnValidate's CurrPage.Update(false) first (not built)"
        ))
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
