//! What the expression-bound controls of a read-only List over a temporary source table show after an action that refills Rec
//! (al-runner issue 7419). The corpus pins it for one page (testpage/TempListExpressionRefill.al, page 66411): every read of a
//! control bound to a method of the record, a built-in call or a concatenation gives the value of the row the page stands on
//! now, after OnOpenPage, after an action that deletes every row and inserts others (three other rows, the same keys with
//! other values, one row, no row; with `CurrPage.Update(false)`; with `Rec.FindFirst()`), after `Filter.SetFilter`, `First`,
//! `Next` and `Last`, with a filter no row matches and after the filter is removed (:297-303, :313-319, :329-335, :345-351,
//! :361-367, :377-383, :393-399, :409-415, :425-431, :441-447), opened with OpenView and run modally into a ModalPageHandler
//! (:456-537).
//!
//! The entry predicate states each fact of that page and requires it, its trigger code included (`refill_body`): any other page
//! keeps the earlier rules of `shown.rs`.

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
        if !self.has_source(st) {
            return false;
        }
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
        let program = &engine.program;
        let triggers_ok = page.triggers.len() == 1
            && page.triggers[0].0 == "onopenpage"
            && refill_body(program, object, page.triggers[0].1, Trigger::OpenPage)
            && page.extensions.is_empty()
            && page.ext_triggers.is_empty();
        let controls_ok = page.controls.iter().all(|c| {
            let kind = match c.kind {
                ir::ControlKind::Repeater | ir::ControlKind::Group => c.binding == ir::Binding::None,
                ir::ControlKind::Field => {
                    matches!(
                        c.binding,
                        ir::Binding::Field(_) | ir::Binding::Var(_) | ir::Binding::Expr(_)
                    ) && c.triggers.iter().all(|(n, _)| n == "__sourceset")
                }
                ir::ControlKind::Action => c
                    .triggers
                    .iter()
                    .all(|(n, r)| n == "onaction" && refill_body(program, object, *r, Trigger::Action)),
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

/// The trigger whose code [`refill_body`] checks.
#[derive(Clone, Copy, PartialEq)]
enum Trigger {
    OpenPage,
    Action,
}

/// Whether routine `routine` of page `object` does only what the triggers of the pinned page do (testpage/TempListExpressionRefill.al,
/// page 66411): it reaches `Rec.DeleteAll()` (the refill), and every call it makes, through the page's own procedures, is one of
/// `Rec.IsEmpty()`, `Rec.DeleteAll()`, `Rec.Init()` and `Rec.Insert()` (the procedure `Fill` and `AddRow` of OnOpenPage and of every
/// action), and in an action also `Rec.FindFirst()` and `CurrPage.Update(false)` (the actions ThreeOtherFindFirst and
/// ThreeOtherUpdate, NoRowUpdate). Its statements assign fields of Rec and page or local variables, and branch. A key, an order, a
/// filter, a find in OnOpenPage, or a change of a row without the refill is code the corpus does not show.
fn refill_body(program: &ir::Program, object: usize, routine: usize, trigger: Trigger) -> bool {
    let o = &program.objects[object];
    let Some(rec) = o.rec_slot else {
        return false;
    };
    let id = |name: &str| ir::BuiltinId::of(name);
    let mut allowed = vec![
        id("Record.IsEmpty#1"),
        id("Record.DeleteAll#1"),
        id("Record.Init#1"),
        id("Record.Insert#1"),
    ];
    if trigger == Trigger::Action {
        allowed.push(id("Record.FindFirst#1"));
    }
    let delete_all = id("Record.DeleteAll#1");
    let update = id("Page.Update#1");
    let on_rec = |r: Option<&ir::Arg>| matches!(r, Some(ir::Arg::Ref(ir::Place::Global(g)) | ir::Arg::Value(ir::Expr::Load(ir::Place::Global(g)))) if *g == rec);
    let update_false = |args: &[ir::Arg]| matches!(args, [ir::Arg::Value(ir::Expr::Const(ir::Const::Boolean(false)))]);
    let (mut ok, mut refills) = (true, false);
    let mut todo = vec![routine];
    let mut seen = std::collections::BTreeSet::new();
    while let Some(r) = todo.pop() {
        if !seen.insert(r) {
            continue;
        }
        let Some(body) = o.routines.get(r).map(|x| x.body.stmts()) else {
            return false;
        };
        ok &= statements_assign_plainly(body, rec, o.xrec_slot);
        ir::types::for_each_expr_post(body, &mut |e| match e {
            ir::Expr::Const(_)
            | ir::Expr::Load(_)
            | ir::Expr::Unary(..)
            | ir::Expr::Binary(..)
            | ir::Expr::In(..)
            | ir::Expr::Convert(..)
            | ir::Expr::Ternary(..) => {}
            ir::Expr::Call {
                object: callee,
                routine,
                instance: None,
                ..
            } if *callee == object => todo.push(*routine),
            ir::Expr::Builtin { id, receiver, args, .. } => {
                let record_call = allowed.contains(&Some(*id)) && args.is_empty() && on_rec(receiver.as_deref());
                let page_update = trigger == Trigger::Action && Some(*id) == update && update_false(args);
                refills |= record_call && Some(*id) == delete_all;
                ok &= record_call || page_update;
            }
            _ => ok = false,
        });
    }
    ok && refills
}

/// Whether every statement of `body` is an assignment of a field of Rec or of a variable other than Rec and xRec, a call, a branch
/// (`if`, `case`), an `exit` or a position marker.
fn statements_assign_plainly(body: &[ir::Stmt], rec: usize, xrec: Option<usize>) -> bool {
    body.iter().all(|s| match s {
        ir::Stmt::Assign(p, _) => match p {
            ir::Place::Field(inner, _) => matches!(inner.as_ref(), ir::Place::Global(g) if *g == rec),
            ir::Place::Global(g) => *g != rec && Some(*g) != xrec,
            ir::Place::Local(_) => true,
            _ => false,
        },
        ir::Stmt::Expr(_) | ir::Stmt::Exit(_) | ir::Stmt::Pos(_) => true,
        ir::Stmt::If(_, a, b) => statements_assign_plainly(a, rec, xrec) && statements_assign_plainly(b, rec, xrec),
        ir::Stmt::Case {
            branches, otherwise, ..
        } => {
            branches.iter().all(|(_, b)| statements_assign_plainly(b, rec, xrec))
                && statements_assign_plainly(otherwise, rec, xrec)
        }
        _ => false,
    })
}
