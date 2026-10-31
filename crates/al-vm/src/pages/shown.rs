//! When a page's expression controls are evaluated (al-runner issue 5765). BC's client shows the value an expression control
//! had when its page last refreshed, not the value it has now. The corpus pins it for a Card with two ListParts and a
//! CardPart FactBox that show one codeunit's state (pages/lt-qj-part-propagation/QjPartPropagation.al:4629-4710, codeunit
//! 86889):
//! - every page computes its values when it opens (`opened`: all 0);
//! - an action on the main card refreshes the card and every part, with or without `CurrPage.Update(false)` (:4695
//!   RP_SharedState_MainAction, :4682 `after_main_update`);
//! - an action on the FactBox refreshes the FactBox only, also with `CurrPage.Update(false)` in it (:4629
//!   RP_SharedState_FactBoxAction `after_factbox_action_read`, :4651 RP_SharedState_FactBoxActionWithUpdate);
//! - `First()` on a ListPart that declares OnAfterGetCurrRecord refreshes that part only (:4641 `after_factbox_action`, :4677
//!   `after_set`);
//! - `Next()` on that part and `Next()` on the card (no OnAfterGetRecord, no OnAfterGetCurrRecord, parts without a link)
//!   refresh no other page (:4679 `after_sub_next`, :4681 `after_main_next`), and code of the test that changes the state
//!   refreshes nothing (:4676 `after_set_read`).
//!
//! Only controls whose expression reads no record and runs no code that writes keep a value (`kept_controls`): computing
//! them at a refresh changes nothing else. Any other event leaves the page's kept values in doubt: a read then gives the
//! value as it is now when that is the kept one, and is refused when it changed (whether that event refreshes is not
//! pinned).
//!
//! A control of any other expression is computed when it is read. That is what BC shows only when the page refreshed at
//! each event since its last pinned refresh, which is not pinned. It is kept, as before, after the calls the baseline
//! cases need (`DOUBT_KEPT`): a SetValue on the page (handlers/TestPageFieldSharedBinding_Tests.al:137, misc/
//! lt-protected-var-abs-query-views/P146PageExprSetValue.al:245, :300, :506) or on its host, a GoToRecord of the host and
//! First on a part (pageupdate/TestPageUpdate.al:73, :116, :143), First on the page itself (handlers/
//! TestTempIntegerListPart.al:92, pageextensioncontrol/TestPageExtensionControlTrapped_Tests.al:53) and the reads of a
//! control of any page of the tree: Editable, Enabled, AssertEquals (misc/lt-probes-needed-7/PnGTempControl.al:116), AsDecimal
//! (TestPageUpdate.al:116) and the As* methods of the other types, and `FindFirstField` and `FindNextField` on a page without
//! parts (misc/lt-y13-triage/Y13PageTriage.al:150-186, corpus master). These count also when they raise an error
//! (PnGTempControl.al:116). And an action of the host whose code calls only `CurrPage.Update()` (TestPageUpdate.al:143). After
//! any other such call a read is refused, except after `TestPage.Next`, `TestPage.Filter`, the `TestFilter` calls,
//! `TestField.Activate` and `TestField.Caption` (`DOUBT_ASSUMED`): there the read is computed as it is now under assumption
//! page-expression-control-live-read-after-next-filter-activate, which Microsoft's Tests-ERM need (Price Lists
//! UI.T003_SalesPriceListsDiscountsFromCustomersList reads `SourceType` after `TestPage.Next`).
//!
//! A list computes each control of its repeater whose source is a function or an expression once per row, when it reads its
//! rows: right after the row's OnAfterGetRecord (`row_exprs`). The client then shows the value the row had then, after
//! FindFirstField, FindNextField and Next too, which run no trigger and no control function (misc/lt-y14-triage/
//! Y14PageTriage.al:241 Trapped_FindFirstField_FunctionControl and :274 OpenView_FindFirstField_FunctionControl, `open log`
//! `agr(A);describe(A);agr(B);describe(B);agr(A);`, `log` empty; misc/lt-y13-triage/Y13PageTriage.al:150-186; corpus
//! master). Pinned for a List page with OnAfterGetRecord and no OnFindRecord or OnNextRecord, without parts, that the test
//! opened with OpenView or caught with Trap() (`row_expr_controls`). Any other call on the page than those and a control's
//! Value ends it: reads are then computed as above.

use super::*;
use al_builtins::page_op::{PageMethod, PageOp, PageRecv};

/// What a page's own code did to its page tree during a TestPage call (`PageClients::shown_frames`).
#[derive(Debug, Clone)]
pub(crate) enum ShownNote {
    /// `CurrPage.Update(false)` on this page.
    UpdateFalse(*const RefCell<PageState>),
    /// `CurrPage.Update()` or `CurrPage.Update(true)` on this page.
    UpdateTrue(*const RefCell<PageState>),
    /// Any other call of `CurrPage` or a `Page` variable.
    Other,
}

/// Kinds of unpinned events (`PageState::shown_doubts`): a SetValue on a control of the page, or of its host; a GoToRecord of
/// the host; First on the page itself; a read of a control (Editable, Enabled, AssertEquals, the As* methods); an action of
/// the host whose code calls only `CurrPage.Update()` of the host; anything else.
const DOUBT_SET_VALUE: u16 = 1;
const DOUBT_HOST_SET_VALUE: u16 = 2;
const DOUBT_GO_TO_RECORD: u16 = 4;
const DOUBT_FIRST: u16 = 8;
const DOUBT_READ: u16 = 16;
const DOUBT_HOST_ACTION_UPDATE: u16 = 32;
const DOUBT_OTHER: u16 = 64;
/// `TestPage.Next`, `TestPage.Filter`, a `TestFilter` call, `TestField.Activate` or `TestField.Caption`: a read of an expression the page does not keep is computed as
/// it is now, under an assumption (module comment).
const DOUBT_ASSUMED: u16 = 128;
/// TestField.Lookup through the control's own OnLookup trigger on the page itself (not a part): a read of an expression the
/// page does not keep is computed as it is now (`Session::lookup_refresh_pinned` lists the pinned shapes).
const DOUBT_LOOKUP: u16 = 256;
/// `FindFirstField` or `FindNextField` on a page without parts that the test opened: its controls of a function call, a string expression and a page
/// variable OnAfterGetRecord sets show the row the search ended on (misc/lt-y13-triage/Y13PageTriage.al:150-186, corpus master).
const DOUBT_FIND_FIELD: u16 = 512;
/// The kinds after which a control of an expression the page does not keep is read as it is now (module comment).
const DOUBT_KEPT: u16 = DOUBT_SET_VALUE
    | DOUBT_HOST_SET_VALUE
    | DOUBT_GO_TO_RECORD
    | DOUBT_FIRST
    | DOUBT_READ
    | DOUBT_HOST_ACTION_UPDATE
    | DOUBT_FIND_FIELD;

/// What a TestPage call does to the kept values of one page of its tree.
#[derive(Clone, Copy, PartialEq)]
enum Effect {
    /// The page computes its values again.
    Refresh,
    /// Pinned to leave the page as it is.
    Keep,
    /// Not pinned.
    Doubt,
}

impl Session {
    /// Runs TestPage call `f` on page `st` (the page of a TestPage, TestPart or TestField) and then applies what the call does
    /// to the kept expression values of the pages of its tree. `arg_false`: the call's first argument is `false`.
    pub(super) fn shown_client_call(
        &mut self,
        method: PageOp,
        st: &St,
        target: Option<&ControlTarget>,
        arg_false: bool,
        f: impl FnOnce(&mut Self) -> Result<Value, Signal>,
    ) -> Result<Value, Signal> {
        if method.recv() == PageRecv::Page {
            let update = (target.is_none() && method.method() == PageMethod::Update).then_some(arg_false);
            self.shown_page_code(method, st, update);
            return f(self);
        }
        let root = shown_root(st);
        self.pages.shown_frames.push((Rc::as_ptr(&root), Vec::new()));
        let r = f(self);
        let notes = self.pages.shown_frames.pop().map(|(_, n)| n).unwrap_or_default();
        let tree = shown_tree(&root);
        for p in &tree {
            let effect = match r {
                Ok(_) => self.shown_effect(method, st, target, &root, p, &notes, &tree),
                Err(_) => Effect::Doubt,
            };
            if p.borrow().row_exprs_live && !row_exprs_kept(method, target, r.is_ok() && notes.is_empty()) {
                p.borrow_mut().row_exprs_live = false;
            }
            if effect == Effect::Refresh {
                self.shown_refresh(p);
                continue;
            }
            // A call pinned to leave the kept values as they are still says nothing about the expressions the page does not
            // keep: those see the kind of the call (`DOUBT_KEPT`).
            let on_control = matches!(target, Some(ControlTarget::Control(_)));
            let is_p = Rc::ptr_eq(st, p);
            let on_root = Rc::ptr_eq(st, &root);
            let kind = match method.method() {
                // A closed page shows nothing; it computes everything again when it opens (`shown_opened`).
                PageMethod::Close | PageMethod::OpenEdit | PageMethod::OpenView | PageMethod::OpenNew
                    if on_root && target.is_none() && r.is_ok() =>
                {
                    0
                }
                // Visible of a field or an action runs no trigger and changes no value (pages/lt-y17-expr-refresh/
                // Y17ExprRefresh.al:517 RF_Visible_PartControl, :549 RF_Visible_CardControl `visible log` empty, totals kept).
                PageMethod::Value | PageMethod::Visible if on_control && r.is_ok() => 0,
                PageMethod::SetValue if is_p && on_control => DOUBT_SET_VALUE,
                PageMethod::SetValue if on_root && on_control => DOUBT_HOST_SET_VALUE,
                // GoToKey of the host as GoToRecord: on the row shown it runs the host's OnAfterGetCurrRecord only and the part's
                // values stay (Y17ExprRefresh.al:517 `gotokey log` = `card agcr`, `totals before` 15.00/2).
                PageMethod::GoToRecord | PageMethod::GoToKey if on_root && target.is_none() => DOUBT_GO_TO_RECORD,
                PageMethod::First if is_p && target.is_none() => DOUBT_FIRST,
                // A page the test caught with Trap() is pinned only for the values a list keeps per row (`row_exprs`, read before
                // this is reached); a read computed now is pinned only on a page the test opened.
                PageMethod::FindFirstField | PageMethod::FindNextField
                    if is_p && on_root && tree.len() == 1 && r.is_ok() && !st.borrow().caught_by_trap =>
                {
                    DOUBT_FIND_FIELD
                }
                PageMethod::Invoke
                    if on_root
                        && on_control
                        && !notes.is_empty()
                        && notes
                            .iter()
                            .all(|n| matches!(n, ShownNote::UpdateTrue(x) if *x == Rc::as_ptr(&root))) =>
                {
                    DOUBT_HOST_ACTION_UPDATE
                }
                PageMethod::Value
                | PageMethod::Editable
                | PageMethod::Enabled
                | PageMethod::AssertEquals
                | PageMethod::AsBoolean
                | PageMethod::AsDate
                | PageMethod::AsDateTime
                | PageMethod::AsDecimal
                | PageMethod::AsInteger
                | PageMethod::AsTime
                    if on_control =>
                {
                    DOUBT_READ
                }
                PageMethod::Lookup
                    if is_p
                        && on_root
                        && r.is_ok()
                        && matches!(target, Some(ControlTarget::Control(i))
                            if page_of(&self.engine(), st).controls[*i].triggers.iter().any(|(n, _)| n == "onlookup"))
                        && self.lookup_refresh_pinned(&root, &tree, &notes) =>
                {
                    DOUBT_LOOKUP
                }
                _ if method.recv() == PageRecv::TestFilter
                    || matches!(
                        method,
                        PageOp::TestPageNext
                            | PageOp::TestPageFilter
                            | PageOp::TestFieldActivate
                            | PageOp::TestFieldCaption
                    ) =>
                {
                    DOUBT_ASSUMED
                }
                _ => DOUBT_OTHER,
            };
            let mut s = p.borrow_mut();
            if effect == Effect::Doubt {
                s.shown_current = false;
            }
            s.shown_doubts |= kind;
            if kind == DOUBT_OTHER && s.shown_doubt_call.is_none() {
                s.shown_doubt_call = Some(method.name());
            }
        }
        r
    }

    /// A call of `CurrPage` (or a `Page` variable) on page `st`: noted for the TestPage call that runs on its tree; outside
    /// one, a `CurrPage.Update` leaves the tree's kept values in doubt.
    /// `update`: the call is `CurrPage.Update`, with `false` (`Some(true)`) or not.
    fn shown_page_code(&mut self, method: PageOp, st: &St, update: Option<bool>) {
        let root = Rc::as_ptr(&shown_root(st));
        let note = match update {
            Some(true) => ShownNote::UpdateFalse(Rc::as_ptr(st)),
            Some(false) => ShownNote::UpdateTrue(Rc::as_ptr(st)),
            None => ShownNote::Other,
        };
        if let Some((_, notes)) = self.pages.shown_frames.iter_mut().rev().find(|(r, _)| *r == root) {
            notes.push(note);
        } else if method.method() == PageMethod::Update {
            for p in shown_tree(&shown_root(st)) {
                let mut s = p.borrow_mut();
                s.row_exprs_live = false;
                s.shown_current = false;
                s.shown_doubts |= DOUBT_OTHER;
                s.shown_doubt_call
                    .get_or_insert("CurrPage.Update outside a TestPage call");
            }
        }
    }

    /// What TestPage call `method` on page `st` did to page `p` of its tree (see the module comment for the pins).
    #[allow(clippy::too_many_arguments)]
    fn shown_effect(
        &self,
        method: PageOp,
        st: &St,
        target: Option<&ControlTarget>,
        root: &St,
        p: &St,
        notes: &[ShownNote],
        tree: &[St],
    ) -> Effect {
        let engine = self.engine();
        let m = method.method();
        let on_root = Rc::ptr_eq(st, root);
        let is_p = Rc::ptr_eq(st, p);
        let page_type = |s: &St| engine.object_facts[s.borrow().object as usize].page_type.clone();
        let other_type = |s: &St, t: &str| matches!(page_type(s), Some(ir::PageType::Other(o)) if o == t);
        // No part of the tree takes its link from `st`, and none is linked to the host's row.
        let unlinked = |s: &St| {
            tree.iter().all(
                |q| match q.borrow().host.as_ref().and_then(|(h, i)| h.upgrade().map(|h| (h, *i))) {
                    None => true,
                    Some((h, i)) => match &page_of(&engine, &h).controls[i].part_link {
                        None => true,
                        Some(l) => {
                            l.entries.is_empty()
                                && l.view.is_none()
                                && l.provider.is_none_or(|pr| {
                                    !s.borrow().host.as_ref().is_some_and(|(sh, si)| {
                                        sh.upgrade().is_some_and(|sh| Rc::ptr_eq(&sh, &h)) && *si == pr
                                    })
                                })
                        }
                    },
                },
            )
        };
        // Every other page of the tree is a ListPart that `root` hosts, linked by fields only (no view, no provider).
        let linked_list_parts = |root: &St| {
            tree.iter().filter(|q| !Rc::ptr_eq(q, root)).all(|q| {
                let host = q.borrow().host.as_ref().and_then(|(h, i)| h.upgrade().map(|h| (h, *i)));
                let Some((h, i)) = host else { return false };
                Rc::ptr_eq(&h, root)
                    && other_type(q, "listpart")
                    && page_of(&engine, &h).controls[i].part_link.as_ref().is_some_and(|l| {
                        l.view.is_none()
                            && l.provider.is_none()
                            && !l.entries.is_empty()
                            && l.entries.iter().all(|e| matches!(e, ir::LinkEntry::Field(..)))
                    })
            })
        };
        // A Value read changes nothing: the reads of the corpus cases agree with each other (:4676-4682). Nor does Visible
        // (Y17ExprRefresh.al:517, :549).
        if matches!(m, PageMethod::Value | PageMethod::Visible) && matches!(target, Some(ControlTarget::Control(_))) {
            return Effect::Keep;
        }
        // Only `CurrPage.Update(false)` of the page whose action runs is pinned inside its action (:4651).
        let only_update_false_of = |s: &St| {
            notes
                .iter()
                .all(|n| matches!(n, ShownNote::UpdateFalse(x) if *x == Rc::as_ptr(s)))
        };
        match (m, target) {
            // The open computed the values (`shown_opened`).
            (PageMethod::OpenEdit | PageMethod::OpenView | PageMethod::OpenNew, None) => Effect::Keep,
            (PageMethod::Invoke, Some(ControlTarget::Control(idx)))
                if page_of(&engine, st).controls[*idx].kind == ir::ControlKind::Action
                    && page_of(&engine, st).controls[*idx].run_object.is_none() =>
            {
                if on_root && page_type(st) == Some(ir::PageType::Card) && only_update_false_of(st) {
                    return Effect::Refresh;
                }
                // An action of a Document page whose parts are ListParts linked to it by fields refreshes the page and its
                // parts, also when it calls a procedure of a part that writes the part's rows, with or without
                // `CurrPage.Update(false)` (Y17ExprRefresh.al:653 RF_CardAction_CallsPartProcedure, :682 _Update: `invoke log`
                // `proc;card agcr;find;agr 10000;...;agcr 10000`, totals recomputed).
                if on_root
                    && page_type(st) == Some(ir::PageType::Document)
                    && only_update_false_of(st)
                    && linked_list_parts(st)
                {
                    return Effect::Refresh;
                }
                let propagates = st
                    .borrow()
                    .host
                    .as_ref()
                    .and_then(|(h, i)| h.upgrade().map(|h| (h, *i)))
                    .is_none_or(|(h, i)| page_of(&engine, &h).controls[i].prop("updatepropagation").is_some());
                if !on_root && other_type(st, "cardpart") && !propagates && only_update_false_of(st) {
                    return if is_p { Effect::Refresh } else { Effect::Keep };
                }
                Effect::Doubt
            }
            (PageMethod::First | PageMethod::Next, None) if !on_root && notes.is_empty() && unlinked(st) => {
                if !is_p {
                    return Effect::Keep;
                }
                if m == PageMethod::First && other_type(st, "listpart") && self.declares(st, "onaftergetcurrrecord") {
                    return Effect::Refresh;
                }
                Effect::Doubt
            }
            // First, Next and Last on a ListPart linked by fields to its Document or Card host, the only part: the part refreshes
            // (its OnAfterGetCurrRecord recomputed the values when it ran; First on the row shown runs nothing and keeps them),
            // the host stays (Y17ExprRefresh.al:583 RF_PartNext_NewLineSaved `22.00/3`, :626 RF_PartNext_ExistingRows `60.00/2`,
            // :711 RF_PartFirst_Linked `first log` empty, `15.00/2`).
            (PageMethod::First | PageMethod::Next | PageMethod::Last, None)
                if !on_root
                    && notes.is_empty()
                    && tree.len() == 2
                    && other_type(st, "listpart")
                    && matches!(page_type(root), Some(ir::PageType::Document | ir::PageType::Card))
                    && linked_list_parts(root) =>
            {
                if is_p {
                    Effect::Refresh
                } else {
                    Effect::Keep
                }
            }
            (PageMethod::Next, None)
                if on_root
                    && notes.is_empty()
                    && page_type(st) == Some(ir::PageType::Card)
                    && !self.declares(st, "onaftergetrecord")
                    && !self.declares(st, "onaftergetcurrrecord")
                    && tree.iter().all(|q| q.borrow().host.is_none() || unlinked(q)) =>
            {
                Effect::Keep
            }
            _ => Effect::Doubt,
        }
    }

    /// Page `st` computes its kept expression values again (or for the first time, as it opens).
    pub(super) fn shown_refresh(&mut self, st: &St) {
        if !st.borrow().open || st.borrow().instance.is_none() {
            let mut s = st.borrow_mut();
            s.shown_current = false;
            s.shown_exprs.clear();
            s.shown_doubts = DOUBT_OTHER;
            s.shown_doubt_call = Some("a refresh of a page that is not open");
            return;
        }
        let controls = self.kept_controls(st);
        let mut shown = Vec::with_capacity(controls.len());
        for &idx in controls.iter() {
            let ir::Binding::Expr(r) = page_of(&self.engine(), st).controls[idx].binding else {
                continue;
            };
            // What BC shows for an expression that fails when the page refreshes is not pinned: its reads are refused.
            shown.push((idx, self.control_routine(st, idx, r).ok()));
        }
        let mut s = st.borrow_mut();
        s.shown_exprs = shown;
        s.shown_current = true;
        s.shown_doubts = 0;
        s.shown_doubt_call = None;
    }

    /// Every page of the tree of `st` computes its kept values as it opens.
    pub(super) fn shown_opened(&mut self, st: &St) {
        for p in shown_tree(st) {
            self.shown_refresh(&p);
        }
    }

    /// The value expression control `idx` of `st` shows, given `live`, its value now (see the module comment).
    pub(super) fn shown_value(
        &mut self,
        st: &St,
        idx: usize,
        live: impl FnOnce(&mut Self) -> Result<Value, Signal>,
    ) -> Result<Value, Signal> {
        let kept = st
            .borrow()
            .shown_exprs
            .iter()
            .find(|(i, _)| *i == idx)
            .map(|(_, v)| v.clone());
        let Some(kept) = kept else {
            let doubts = st.borrow().shown_doubts;
            if doubts & !(DOUBT_KEPT | DOUBT_ASSUMED | DOUBT_LOOKUP) != 0 {
                return unsupported(format!(
                    "control {} shows an expression computed when it is read, after {}, whose refresh of the page is not pinned",
                    page_of(&self.engine(), st).controls[idx].name,
                    st.borrow().shown_doubt_call.unwrap_or("a call")
                ));
            }
            // DOUBT_LOOKUP: the pinned shapes of a Lookup through OnLookup show the picked value (`lookup_refresh_pinned`).
            if doubts & DOUBT_ASSUMED != 0 {
                al_builtins::assumptions::assume(
                    al_builtins::assumptions::Assumption::PageExpressionControlLiveReadAfterNextFilterActivate,
                );
            }
            return live(self);
        };
        let Some(kept) = kept else {
            return unsupported(format!(
                "control {} shows an expression that failed when its page last refreshed (what BC shows then is not pinned)",
                page_of(&self.engine(), st).controls[idx].name
            ));
        };
        if st.borrow().shown_current {
            return Ok(kept);
        }
        let now = live(self)?;
        if ops::compare(&now, &kept).is_ok_and(|o| o.is_eq()) {
            return Ok(now);
        }
        unsupported(format!(
            "control {} shows an expression whose value changed since its page last refreshed, after a call whose refresh is not pinned",
            page_of(&self.engine(), st).controls[idx].name
        ))
    }

    /// A TestField.Lookup through OnLookup on `root`, whose code noted `notes`, is pinned to show the picked value in a control
    /// the page does not keep (pages/lt-sc-outside-pages/ScLookupGlobalRecordField.al, codeunit 89008, every read
    /// `shows=[R2]`): the code calls no `CurrPage` method but `CurrPage.Update` of `root`, and
    /// - without parts: no Update or `Update(false)` (:382 LookupShowsThePickedResource, :434
    ///   LookupWithoutUpdateShowsThePickedResource); `Update(true)` there is not pinned;
    /// - with parts: no Update, `Update(false)` or `Update(true)` (:469, :493, :517, :541, :565, :589), when every other page
    ///   of the tree is a ListPart hosted by `root` with no SubPageView or provider and a SubPageLink that is absent or one
    ///   `field(..)` entry (pages 89631 and 89632) or two (pages/lt-wb-6290/WbLookupListPartTests.al:21-31
    ///   OnePartWithTwoLinkedFieldsShowsThePickedResource, al-runner issue 6290), or three (al-runner issue 6466, assumption
    ///   `lookup-three-field-link-shows-picked`). A filter entry, four or more entries, a view or a provider are not pinned.
    fn lookup_refresh_pinned(&self, root: &St, tree: &[St], notes: &[ShownNote]) -> bool {
        let with_parts = tree.len() > 1;
        let root_ptr = Rc::as_ptr(root);
        // `Update(false)` of a part of the tree, run by a procedure of the part that OnValidate called, is pinned with parts too
        // (al-runner issue 6118; corpus writer TW, branch al-runtime-rs-staging-tw, ScLookupGlobalRecordField.al:644
        // TW_LookupOnValidateUpdatesThePart `shows=[R2]`), as an assumption until that pin is in the corpus.
        let part_update = |x: *const std::cell::RefCell<PageState>| {
            with_parts && tree.iter().any(|q| !Rc::ptr_eq(q, root) && Rc::as_ptr(q) == x)
        };
        let notes_ok = notes.iter().all(|n| match n {
            ShownNote::UpdateFalse(x) if *x != root_ptr && part_update(*x) => {
                // Pinned by pages/lt-sc-outside-pages/ScLookupGlobalRecordField.al TW_LookupOnValidateUpdatesThePart (was assumption `lookup-part-update-false-shows-picked`).
                true
            }
            ShownNote::UpdateFalse(x) => *x == root_ptr,
            ShownNote::UpdateTrue(x) => with_parts && *x == root_ptr,
            ShownNote::Other => false,
        });
        if !notes_ok {
            return false;
        }
        let engine = self.engine();
        let mut three = false;
        let pinned = tree.iter().filter(|q| !Rc::ptr_eq(q, root)).all(|q| {
            let host = q.borrow().host.as_ref().and_then(|(h, i)| h.upgrade().map(|h| (h, *i)));
            let Some((h, i)) = host else { return false };
            let list_part = matches!(
                &engine.object_facts[q.borrow().object as usize].page_type,
                Some(ir::PageType::Other(o)) if o == "listpart"
            );
            let link_ok = page_of(&engine, &h).controls[i].part_link.as_ref().is_none_or(|l| {
                l.view.is_none()
                    && l.provider.is_none()
                    && (l.entries.is_empty()
                        || matches!(
                            l.entries.as_slice(),
                            [ir::LinkEntry::Field(..)] | [ir::LinkEntry::Field(..), ir::LinkEntry::Field(..)]
                        )
                        || {
                            let t = matches!(
                                l.entries.as_slice(),
                                [
                                    ir::LinkEntry::Field(..),
                                    ir::LinkEntry::Field(..),
                                    ir::LinkEntry::Field(..)
                                ]
                            );
                            three |= t;
                            t
                        })
            });
            Rc::ptr_eq(&h, root) && list_part && link_ok
        });
        if !(pinned && three) {
            return pinned;
        }
        // With three entries BC runs the refresh of the OnValidate's `CurrPage.Update(false)` later, not within the Lookup: the
        // host's OnAfterGetCurrRecord is not logged after it, and runs before the next Lookup (corpus writer Y15,
        // pages/lt-y15/Y15LookupThreeFieldLink.al:194 `lookup log` `lookup;validate R2`, :278 `log`
        // `lookup;validate R2;agcr R1;lookup;validate R3`). Without row triggers the refresh shows nothing (al-runner issue
        // 6466, the reporter's page). With them the Lookup left the refresh for later (`defer_lookup_refresh`) and the reads
        // after it show the values as they are now (al-runner issue 6519; corpus writer Y26, branch al-runtime-rs-staging-y26,
        // Y15LookupThreeFieldLink.al:314 Y26_ThreeLinkCurr_LookupRefresh); a Lookup that did not leave it is refused.
        if self.declares(root, "onaftergetrecord") || self.declares(root, "onaftergetcurrrecord") {
            if !root.borrow().lookup_refresh_deferred {
                return false;
            }
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::LookupThreeFieldLinkDefersRefresh);
            return true;
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::LookupThreeFieldLinkShowsPicked);
        true
    }

    /// The controls whose value list `st` keeps per row as it reads its rows (module comment), when the page's shape is pinned:
    /// the page's own controls of the repeater whose source is an expression, but those the page keeps across its refreshes
    /// (`kept_controls`) and elements of a page array.
    pub(super) fn row_expr_controls(&mut self, st: &St) -> Option<Rc<Vec<usize>>> {
        {
            let s = st.borrow();
            if s.host.is_some() || !s.parts.is_empty() || !(s.caught_by_trap || s.mode == PageMode::View) {
                return None;
            }
        }
        if !self.has_source(st)
            || !self.is_list_page(st)
            || !self.declares(st, "onaftergetrecord")
            || self.declares(st, "onfindrecord")
            || self.declares(st, "onnextrecord")
        {
            return None;
        }
        let kept = self.kept_controls(st);
        let engine = self.engine();
        let page = page_of(&engine, st);
        let controls: Vec<usize> = (0..page.controls.len())
            .filter(|&i| {
                let c = &page.controls[i];
                matches!(c.binding, ir::Binding::Expr(_))
                    && c.owner.is_none()
                    && c.array_element.is_none()
                    && !kept.contains(&i)
                    && self.in_repeater(st, i)
            })
            .collect();
        (!controls.is_empty()).then(|| Rc::new(controls))
    }

    /// Computes `controls` for the row with `key` the list just read (after its OnAfterGetRecord) and notes their values.
    pub(super) fn note_row_exprs(&mut self, st: &St, key: Vec<Value>, controls: &[usize]) -> Result<(), Signal> {
        if st.borrow().cursor != Cursor::Row || st.borrow().draft {
            return Ok(());
        }
        let mut values = Vec::with_capacity(controls.len());
        for &idx in controls {
            let ir::Binding::Expr(r) = page_of(&self.engine(), st).controls[idx].binding else {
                continue;
            };
            // What BC shows for an expression that fails when the row is read is not pinned: its reads are refused.
            values.push((idx, self.control_routine(st, idx, r).ok()));
        }
        let mut s = st.borrow_mut();
        match s.row_exprs.iter_mut().find(|(k, _)| Self::same_key(k, &key)) {
            Some((_, held)) => *held = values,
            None => s.row_exprs.push((key, values)),
        }
        Ok(())
    }

    /// The value control `idx` of `st` showed when the list read the row the cursor stands on (module comment); `None` when the
    /// page keeps no such value for it.
    pub(super) fn row_expr_value(&mut self, st: &St, idx: usize) -> Result<Option<Value>, Signal> {
        {
            let s = st.borrow();
            if !s.row_exprs_live || s.row_exprs.is_empty() || s.cursor != Cursor::Row || s.draft {
                return Ok(None);
            }
        }
        let key = self.row_key(&self.page_rec(st))?;
        let held = st
            .borrow()
            .row_exprs
            .iter()
            .find(|(k, _)| Self::same_key(k, &key))
            .and_then(|(_, v)| v.iter().find(|(c, _)| *c == idx).map(|(_, v)| v.clone()));
        match held {
            None => Ok(None),
            Some(Some(v)) => Ok(Some(v)),
            Some(None) => unsupported(format!(
                "control {} shows an expression that failed when the list read the row (what BC shows then is not pinned)",
                page_of(&self.engine(), st).controls[idx].name
            )),
        }
    }

    /// Before the refresh after the SetValue of TestField.Lookup on `st` (`control_set_value`): on a page with OnAfterGetRecord or OnAfterGetCurrRecord whose OnValidate
    /// asked for `CurrPage.Update(false)` and whose every part is a ListPart it hosts with a SubPageLink of three `field(..)`
    /// entries, BC leaves the refresh for later. The Lookup runs no OnAfterGetRecord or OnAfterGetCurrRecord on the host or
    /// the part, and reading a control after it runs no trigger: the field shows the picked value, the globals the row
    /// triggers set and the part keep what they showed before (corpus writer Y26, branch al-runtime-rs-staging-y26,
    /// pages/lt-y15/Y15LookupThreeFieldLink.al:314 Y26_ThreeLinkCurr_LookupRefresh `lookup log`
    /// `lookup;h-fn R1;validate R2;h-fn R1`, `current no` `R1/`, `part current` `R1/`, every read log empty; :574
    /// Y26_ThreeLinkCurr_RegionsSet; al-runner issue 6519). BC computes the host's function controls within the Lookup (`h-fn`),
    /// which this runtime does not: a host with an expression control that calls anything is left as it was. The refresh
    /// runs at the next Lookup or Next (:639 Y26_ThreeLinkCurr_LookupTwice, :314 `next log`), which is not built:
    /// `lookup_refresh_deferred_call` refuses every call but a read and Close.
    pub(super) fn defer_lookup_refresh(&mut self, st: &St) {
        let shape = {
            let s = st.borrow();
            s.host.is_none()
                && s.open
                && !s.draft
                && s.cursor == Cursor::Row
                && !s.parts.is_empty()
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
                            [
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
    /// any other call runs the refresh first (:639 `second lookup log`, :314 `next log`), which is not built.
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
        unsupported(format!(
            "{method} after TestField.Lookup on a page with OnAfterGetRecord or OnAfterGetCurrRecord whose ListPart links three \
             fields: BC runs the refresh of the OnValidate's CurrPage.Update(false) first (not built)"
        ))
    }

    /// The expression controls of `st` whose value the page keeps: the page's own controls (not an extension's) whose
    /// expression reads no record and calls only codeunit procedures that only return a value (`pure_expr`).
    fn kept_controls(&mut self, st: &St) -> Rc<Vec<usize>> {
        let object = st.borrow().object;
        if let Some(c) = self.pages.shown_controls.get(&object) {
            return c.clone();
        }
        let engine = self.engine();
        let program = &engine.program;
        let kept: Vec<usize> = page_of(&engine, st)
            .controls
            .iter()
            .enumerate()
            .filter_map(|(i, c)| match c.binding {
                ir::Binding::Expr(r) if c.owner.is_none() && pure_routine(program, object as usize, r, 0) => Some(i),
                _ => None,
            })
            .collect();
        let kept = Rc::new(kept);
        self.pages.shown_controls.insert(object, kept.clone());
        kept
    }
}

/// The page at the top of the part tree `st` belongs to.
fn shown_root(st: &St) -> St {
    let mut cur = st.clone();
    loop {
        let host = cur.borrow().host.as_ref().and_then(|(h, _)| h.upgrade());
        match host {
            Some(h) => cur = h,
            None => return cur,
        }
    }
}

/// `root` and every part below it.
fn shown_tree(root: &St) -> Vec<St> {
    let mut out = vec![root.clone()];
    let mut i = 0;
    while i < out.len() {
        let parts: Vec<St> = out[i].borrow().parts.iter().map(|(_, p)| p.clone()).collect();
        out.extend(parts);
        i += 1;
    }
    out
}

/// Routine `r` of `object` only returns a value computed by `pure_expr`.
fn pure_routine(program: &ir::Program, object: usize, r: usize, depth: usize) -> bool {
    if depth > 4 {
        return false;
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
    body.peek().is_some()
        && body.all(|s| matches!(s, ir::Stmt::Exit(Some(e)) if pure_expr(program, object, routine, e, depth)))
}

/// An expression that reads no record and runs nothing that writes: constants, operators, variables that hold no record,
/// and calls of procedures of a codeunit variable that are themselves `pure_routine` (QjPartPropagation.al:4767
/// `State.GetValue()`).
fn pure_expr(program: &ir::Program, object: usize, routine: &ir::Routine, e: &ir::Expr, depth: usize) -> bool {
    let not_record = |ty: &RtType| !matches!(ty, RtType::Record(..));
    let pure = |e: &ir::Expr| pure_expr(program, object, routine, e, depth);
    match e {
        ir::Expr::Const(_) => true,
        ir::Expr::Load(ir::Place::Global(g)) => program.objects[object]
            .globals
            .get(*g)
            .is_some_and(|s| not_record(&s.ty)),
        ir::Expr::Load(ir::Place::Local(l)) => routine.locals.get(*l).is_some_and(|s| not_record(&s.ty)),
        ir::Expr::Unary(_, a) | ir::Expr::Convert(_, a) => pure(a),
        ir::Expr::Binary(_, a, b) => pure(a) && pure(b),
        ir::Expr::Ternary(a, b, c) => pure(a) && pure(b) && pure(c),
        ir::Expr::Call {
            object: callee,
            routine: r,
            instance: Some(inst),
            args,
            try_call: false,
        } => {
            let codeunit_var = match &**inst {
                ir::Expr::Load(ir::Place::Global(g)) => program.objects[object]
                    .globals
                    .get(*g)
                    .is_some_and(|s| matches!(s.ty, RtType::Codeunit(_))),
                _ => false,
            };
            codeunit_var
                && program.objects[*callee].is_kind(ir::ObjKind::Codeunit)
                && args.iter().all(|a| matches!(a, ir::Arg::Value(v) if pure(v)))
                && pure_routine(program, *callee, *r, depth + 1)
        }
        _ => false,
    }
}

/// Routine `r` of `object` only returns an expression of constants, variables, fields and operators: it calls nothing.
fn call_free_routine(program: &ir::Program, object: usize, r: usize) -> bool {
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

/// A TestPage call (`ok`: it succeeded and the page's code called no `CurrPage` method) pinned to leave the values a list
/// noted per row as they are (module comment): a control's Value, FindFirstField, FindNextField and Next; an open reads the
/// rows again and a close ends the page.
fn row_exprs_kept(method: PageOp, target: Option<&ControlTarget>, ok: bool) -> bool {
    let on_control = matches!(target, Some(ControlTarget::Control(_)));
    ok && match method.method() {
        PageMethod::Value => on_control,
        PageMethod::FindFirstField | PageMethod::FindNextField => method.recv() == PageRecv::TestPage,
        PageMethod::Next => target.is_none() && method.recv() == PageRecv::TestPage,
        PageMethod::OpenView | PageMethod::OpenEdit | PageMethod::Close => target.is_none(),
        _ => false,
    }
}
