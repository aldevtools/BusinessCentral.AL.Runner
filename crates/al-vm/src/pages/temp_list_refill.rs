//! What the expression-bound controls of a read-only List over a temporary source table show after an action that refills Rec
//! (al-runner issue 7419). The corpus pins it for one page (testpage/TempListExpressionRefill.al, page 66411): every read of a
//! control bound to a method of the record, a built-in call or a concatenation gives the value of the row the page stands on
//! now, after OnOpenPage, after an action that deletes every row and inserts others (three other rows, the same keys with
//! other values, one row, no row; with `CurrPage.Update(false)`; with `Rec.FindFirst()`), after `Filter.SetFilter`, `First`,
//! `Next` and `Last`, with a filter no row matches and after the filter is removed (:297-303, :313-319, :329-335, :345-351,
//! :361-367, :377-383, :393-399, :409-415, :425-431, :441-447), opened with OpenView and run modally into a ModalPageHandler
//! (:456-537).
//!
//! The entry predicate states each fact of that page and requires it: any other page keeps the earlier rules of `shown.rs`.

use super::*;
use al_builtins::page_op::{PageMethod, PageOp, PageRecv};

/// The properties the pinned page declares, lower case, with their value (`None`: any value; they run nothing).
const PROPERTIES: [(&str, Option<&str>); 6] = [
    ("pagetype", Some("list")),
    ("sourcetable", None),
    ("sourcetabletemporary", Some("true")),
    ("editable", Some("false")),
    ("applicationarea", None),
    ("usagecategory", None),
];

impl Session {
    /// Whether `st` is a page of the pinned shape (module comment): a List declaring exactly the properties of
    /// [`PROPERTIES`], with OnOpenPage as its only trigger, a repeater of controls bound to fields of Rec, page variables or
    /// expressions, actions with only OnAction, no part, no page extension, no subscriber of the page or of its table, opened
    /// by the test with OpenView or run modally (not as a lookup) into a handler.
    pub(in crate::pages) fn temp_list_refill_pinned(&self, st: &St) -> bool {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let page = page_of(&engine, st);
        let properties = &engine.program.objects[object].properties;
        let props_ok = properties.iter().all(|(k, v)| {
            PROPERTIES.iter().any(|(name, want)| {
                k.eq_ignore_ascii_case(name) && want.is_none_or(|w| v.trim().eq_ignore_ascii_case(w))
            })
        }) && PROPERTIES
            .iter()
            .filter(|(_, want)| want.is_some())
            .all(|(name, _)| properties.iter().any(|(k, _)| k.eq_ignore_ascii_case(name)));
        let triggers_ok = page.triggers.len() == 1
            && page.triggers[0].0 == "onopenpage"
            && page.extensions.is_empty()
            && page.ext_triggers.is_empty();
        let controls_ok = page.controls.iter().all(|c| {
            let kind = match c.kind {
                ir::ControlKind::Repeater | ir::ControlKind::Group => c.binding == ir::Binding::None,
                ir::ControlKind::Field => {
                    matches!(c.binding, ir::Binding::Field(_) | ir::Binding::Var(_) | ir::Binding::Expr(_))
                        && c.triggers.iter().all(|(n, _)| n == "__sourceset")
                }
                ir::ControlKind::Action => c.triggers.iter().all(|(n, _)| n == "onaction"),
                _ => false,
            };
            kind && c.owner.is_none() && c.ext_triggers.is_empty()
        });
        let table = self.page_rec(st).borrow().table as usize;
        let subscribers = engine.events.any_subscriber_of(object) || engine.events.any_subscriber_of(table);
        let s = st.borrow();
        let opened = !s.caught_by_trap
            && s.host.is_none()
            && ((s.opened_by_test && s.mode == PageMode::View && !s.modal) || (s.modal && !s.lookup));
        props_ok && triggers_ok && controls_ok && !subscribers && opened
    }

    /// Whether `method` on the pinned page is one of the calls the corpus pins (module comment): an action whose code called no
    /// `CurrPage` method but `CurrPage.Update(false)` of this page, First, Next, Last, and a filter set through the TestPage.
    pub(super) fn temp_list_refill_call(
        method: PageOp,
        target: Option<&ControlTarget>,
        notes: &[shown::ShownNote],
        page: &St,
    ) -> bool {
        let action = method.method() == PageMethod::Invoke
            && matches!(target, Some(ControlTarget::Control(_)))
            && notes
                .iter()
                .all(|n| matches!(n, shown::ShownNote::UpdateFalse(x) if *x == Rc::as_ptr(page)));
        let moves = target.is_none()
            && notes.is_empty()
            && matches!(method.method(), PageMethod::First | PageMethod::Next | PageMethod::Last);
        let filter = notes.is_empty() && (method.recv() == PageRecv::TestFilter || method == PageOp::TestPageFilter);
        action || moves || filter
    }
}
