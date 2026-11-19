//! Two Worksheet opens where BC runs the DataCaptionExpression once at the open, after OnOpenPage and before the Trap's first
//! read or the handler, and main keeps its own open, which does not run it: the assumption
//! `data-caption-two-microsoft-opens-skipped`. Its probe is corpus PR 1417 (merged, not pinned),
//! tests/al-language/pages/properties/DataCaptionKeptOpen.al :553, :577, :640, :652, which also measures that the test's reads
//! after the open equal those of the same page without the expression, on pages whose only trigger is OnOpenPage. Microsoft's
//! tests that pass on real BC through these two shapes are the evidence for their two pages. Each shape is required here by
//! equality:
//! - `TrappedVariableRun`: Microsoft's Navigate page as a posted document's Navigate opens it (a Page variable's procedures, then
//!   `Run()`, into a TestPage Trap); its expression is a page procedure that returns StrSubstNo of a label with one `%1` and a
//!   Text field of Rec when that field is not empty, else ''.
//! - `ModalVariableAfterView`: Microsoft's Order Promising Lines as a sales line opens it (a Page variable's procedure, then
//!   `SetTableView` and `RunModal()`, for a ModalPageHandler); its expression returns, through a codeunit variable of the page, a
//!   Text variable of that codeunit.
//!
//! Both pages are temporary Worksheets with InsertAllowed = false (`kept_page` lists every page fact). Neither expression can
//! raise an error or write anything, so skipping its run at the open changes only the caption; a TestPage's Caption() on such
//! a page refuses (`refuse_kept_open_caption`). Every other page keeps the refusal of `data_caption_open_neighbor`.

use super::page_run_close::PageRunCall;
use super::*;
use crate::ui::UiKind;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::builtin_id::ids;
use al_builtins::page_op::PageMethod;
use al_builtins::value::RtType;
use al_ir::{Arg, Const, Expr, Place, Stmt};

/// The run of an evidence test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeptRun {
    /// A Page variable's `Run()` after only calls of the page's own procedures, caught by a TestPage Trap (Microsoft's SCM
    /// Navigate tests and ERM Purchase Order's PostedPurchaseReceiptNavigateDocumentNo).
    TrappedVariableRun,
    /// A Page variable's `SetTableView`, then its `RunModal()`, answered by a ModalPageHandler (Microsoft's SCM Inventory - Orders
    /// and SCM Bugfixes order promising tests).
    ModalVariableAfterView,
}

/// The expression of an evidence page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeptExpression {
    /// `Proc()`, a procedure of the page whose body is `if Rec.F <> '' then exit(StrSubstNo(Lbl, Rec.F)); exit('');`.
    PageProcedureLabel,
    /// `Cu.Proc()`, a codeunit variable of the page, whose procedure's body is `exit(V);` with V a Text variable of the codeunit.
    CodeunitTextVariable,
}

impl Session {
    /// Whether page `st`, opened by AL code with `bind`'s handler bound (or caught by a Trap, `bind` empty), is in one of the two
    /// shapes of the module doc: then main's open stands, the assumption is marked, and a caption read will refuse.
    pub(super) fn data_caption_open_kept(&self, st: &St, bind: Option<UiKind>) -> bool {
        let Some(run) = Self::kept_run(st, bind) else {
            return false;
        };
        if !self.kept_page(st) {
            return false;
        }
        let expected = match run {
            KeptRun::TrappedVariableRun => KeptExpression::PageProcedureLabel,
            KeptRun::ModalVariableAfterView => KeptExpression::CodeunitTextVariable,
        };
        if self.kept_expression(st) != Some(expected) {
            return false;
        }
        assume(Assumption::DataCaptionTwoMicrosoftOpensSkipped);
        st.borrow_mut().data_caption_open_kept = true;
        true
    }

    /// A TestPage's Caption() of page `st`: refused where its expression's run at the open was skipped
    /// (`data_caption_open_kept`), as the caption is what that run would show.
    pub(super) fn refuse_kept_open_caption(&self, st: &St) -> Result<(), Signal> {
        if st.borrow().data_caption_open_kept {
            return unsupported(
                "the caption of a Worksheet whose DataCaptionExpression did not run at the open, where BC runs it \
                 (assumption data-caption-two-microsoft-opens-skipped covers no caption read)",
            );
        }
        Ok(())
    }

    /// The run of `st`, when it is one of `KeptRun`, every fact required: open at the top level, not opened by the test.
    fn kept_run(st: &St, bind: Option<UiKind>) -> Option<KeptRun> {
        let s = st.borrow();
        if !s.open || s.host.is_some() || s.opened_by_test || s.lookup {
            return None;
        }
        match bind {
            None if s.caught_by_trap && !s.modal && s.trapped_run == Some(PageRunCall::VariableRunOnly) => {
                Some(KeptRun::TrappedVariableRun)
            }
            Some(UiKind::ModalPage)
                if s.modal
                    && s.modal_run == ModalRun::Variable
                    && s.code_calls.as_slice() == [PageMethod::SetTableView, PageMethod::RunModal] =>
            {
                Some(KeptRun::ModalVariableAfterView)
            }
            _ => None,
        }
    }

    /// The page facts both evidence pages share (Microsoft's Navigate and Order Promising Lines): a Worksheet with a source
    /// table and a DataCaptionExpression of its own, SourceTableTemporary = true, InsertAllowed = false, no Editable and no
    /// DelayedInsert, page triggers among OnInit, OnOpenPage and OnClosePage; no page extension sets one of these properties
    /// (an extension's value is the one BC applies, pages/extensions/UPageProps.al:275) or adds a page trigger, and no
    /// subscriber listens to the page's OnOpenPageEvent.
    fn kept_page(&self, st: &St) -> bool {
        const GATED: [&str; 5] = [
            "datacaptionexpression",
            "insertallowed",
            "sourcetabletemporary",
            "editable",
            "delayedinsert",
        ];
        const TRIGGERS: [&str; 3] = ["oninit", "onopenpage", "onclosepage"];
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let props = &engine.program.objects[object].properties;
        let prop = |name: &str| {
            props
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(name))
                .map(|(_, v)| v.trim().to_ascii_lowercase())
        };
        let Some(page) = engine.program.objects[object].page.as_ref() else {
            return false;
        };
        let extensions_keep_page = page.extensions.iter().all(|&e| {
            engine.program.objects[e]
                .properties
                .iter()
                .all(|(k, _)| GATED.iter().all(|g| !k.eq_ignore_ascii_case(g)))
        }) && page.ext_triggers.is_empty();
        engine.object_facts[object].page_type == Some(al_ir::PageType::Worksheet)
            && prop("sourcetabletemporary").as_deref() == Some("true")
            && prop("insertallowed").as_deref() == Some("false")
            && prop("editable").is_none()
            && prop("delayedinsert").is_none()
            && page.triggers.iter().all(|(n, _)| TRIGGERS.contains(&n.as_str()))
            && extensions_keep_page
            && !engine.events.is_subscribed(object, "onopenpageevent")
            && self.has_source(st)
            && self.has_own_data_caption_expression(st)
    }

    /// The form of the page's compiled DataCaptionExpression, when it is one of `KeptExpression`.
    fn kept_expression(&self, st: &St) -> Option<KeptExpression> {
        let engine = self.engine();
        let page = st.borrow().object as usize;
        let object = &engine.program.objects[page];
        let routine = self.data_caption_routine(st)?;
        let expression = plain(object.routines[routine].body.stmts());
        let [
            Stmt::Exit(Some(Expr::Call {
                object: callee,
                routine: r,
                instance,
                args,
                try_call: false,
            })),
        ] = expression.as_slice()
        else {
            return None;
        };
        if !args.is_empty() || !self.kept_callee(*callee, *r) {
            return None;
        }
        let body = plain(engine.program.objects[*callee].routines[*r].body.stmts());
        match instance.as_deref() {
            None if *callee == page => {
                let rec = object.rec_slot?;
                let table = match object.globals.get(rec)?.ty {
                    RtType::Record(t, _) => t as usize,
                    _ => return None,
                };
                label_body(&body, rec, &engine.program.objects.get(table)?.table.as_ref()?.fields)
                    .then_some(KeptExpression::PageProcedureLabel)
            }
            Some(Expr::Load(Place::Global(g))) => {
                if !matches!(object.globals.get(*g)?.ty, RtType::Codeunit(c) if c as usize == *callee) {
                    return None;
                }
                let [Stmt::Exit(Some(Expr::Load(Place::Global(v))))] = body.as_slice() else {
                    return None;
                };
                matches!(engine.program.objects[*callee].globals.get(*v)?.ty, RtType::Text(None))
                    .then_some(KeptExpression::CodeunitTextVariable)
            }
            _ => None,
        }
    }

    /// Routine `r` of object `o` is a procedure without parameters that returns an unbounded Text, that lowering did not block
    /// and that is no event publisher.
    fn kept_callee(&self, o: usize, r: usize) -> bool {
        let engine = self.engine();
        let Some(routine) = engine.program.objects.get(o).and_then(|x| x.routines.get(r)) else {
            return false;
        };
        routine.params == 0
            && !routine.is_trigger
            && routine.ret == Some(RtType::Text(None))
            && engine
                .blocked
                .get(o)
                .and_then(|x| x.get(r))
                .is_some_and(Option::is_none)
            && !engine.events.publishers.contains_key(&(o, r))
    }
}

/// `body` without its position markers.
fn plain(body: &[Stmt]) -> Vec<&Stmt> {
    body.iter().filter(|s| !matches!(s, Stmt::Pos(_))).collect()
}

/// `body` is `if Rec.F <> '' then exit(StrSubstNo(Lbl, Rec.F)); exit('');`: F one stored Text field of the page's Rec (global
/// `rec`, its table's `fields`), Lbl a label whose text passes `one_placeholder`.
fn label_body(body: &[&Stmt], rec: usize, fields: &[al_ir::Field]) -> bool {
    let [
        Stmt::If(cond, then, otherwise),
        Stmt::Exit(Some(Expr::Const(Const::Text(empty)))),
    ] = body
    else {
        return false;
    };
    let Expr::Binary(al_ir::BinOp::Ne, field, blank) = cond else {
        return false;
    };
    let rec_field = |e: &Expr| match e {
        Expr::Load(Place::Field(inner, f)) if matches!(**inner, Place::Global(g) if g == rec) => Some(*f),
        _ => None,
    };
    let Some(f) = rec_field(field) else {
        return false;
    };
    let text_field = fields
        .get(f)
        .is_some_and(|x| x.stored && matches!(x.ty, RtType::Text(_)));
    let then = plain(then);
    let [
        Stmt::Exit(Some(Expr::Builtin {
            id,
            receiver: None,
            args,
            ..
        })),
    ] = then.as_slice()
    else {
        return false;
    };
    let [Arg::Value(label), Arg::Value(value)] = args.as_slice() else {
        return false;
    };
    empty.is_empty()
        && matches!(&**blank, Expr::Const(Const::Text(t)) if t.is_empty())
        && otherwise.is_empty()
        && text_field
        && *id == ids::TEXT_STRSUBSTNO_1
        && rec_field(value) == Some(f)
        && label_text(label).is_some_and(one_placeholder)
}

/// `text` has one `%1`, not followed by a digit (`%10` is placeholder ten), and no other `%` or `#`.
fn one_placeholder(text: &str) -> bool {
    text.matches('%').count() == 1
        && !text.contains('#')
        && text
            .split_once("%1")
            .is_some_and(|(_, after)| after.chars().next().is_none_or(|c| !c.is_ascii_digit()))
}

/// The text of a label loaded as an expression: the translated-text builtin over constants, the first one the text.
fn label_text(e: &Expr) -> Option<&str> {
    let Expr::Builtin {
        id,
        receiver: None,
        args,
        ..
    } = e
    else {
        return None;
    };
    if *id != ids::SYSTEM_TRANSLATEDTEXT_1
        || !args
            .iter()
            .all(|a| matches!(a, Arg::Value(Expr::Const(Const::Text(_)))))
    {
        return None;
    }
    match args.first() {
        Some(Arg::Value(Expr::Const(Const::Text(t)))) => Some(t),
        _ => None,
    }
}
