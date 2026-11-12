//! How a page that AL code opened with a non-modal `Page.Run` and a `[PageHandler]` answered closes when a close trigger or
//! the save of its row fails (al-runner issue 7480; corpus pages/close-errors/CemPageHandlerTests.al, codeunit 99522, over the
//! pages of CemFixtures.al).
//!
//! One function decides: `page_run_cell`. A close takes a path of this module only when that function proves every fact of
//! one cell of the corpus matrix (`cell`):
//! - the page and how it was run (`pinned_run`, `PinnedRun`): the page's own code and properties, its table's code, the
//!   record the run was given, the call that ran it (`PageRunCall`) from codeunit code, with no write pending;
//! - what the handler did and how the close was asked (`handler_calls`, `CloseBy`);
//! - the row when the close starts (`RowAtClose`), and that the close saved a typed value before the trigger that failed;
//! - the error and where it came from (`close_error_of`, `CloseError`);
//! - no write to the database since the run started but that save, no Commit, and no error the handler caught.
//!
//! Not facts, taken as the pins' data: the body of the codeunit procedure each page trigger calls (only that the call hands
//! values, `hands_values`) and the bodies of the table's OnInsert and OnModify (only that the table declares exactly
//! those); the values typed and the rows besides the one read (docs/probes-needed.md, row al-runner 7480).
//!
//! Every other close keeps the path it had. What the pinned cells do:
//! - The platform closes the page after the handler returned: the error does not reach the code that ran the page. The
//!   trigger stops at the error, the page closes, `Page.Run` returns.
//! - The handler raised an error (its own, or `Cancel` that these pages do not offer): the platform closes the page all the
//!   same, with the same rule for its errors, and `Page.Run` raises the handler's error.
//! - OK in the handler: an error of OnClosePage comes out wrapped as at `TestPage.Close()`; an error of OnQueryClosePage is
//!   shown as a message no handler takes, and that error comes out with the code `TestWrapped:MissingUIHandler`.
//! - `TestPage.Close()` in the handler: an unhandled Confirm of OnClosePage comes out wrapped (`page_close`).
//!
//! What the last error is after a dropped error is not pinned: it refuses to be read until the next error replaces it.

use super::pinned_shape::{self, TestCall};
use super::*;
use crate::ui::UiKind;
use al_builtins::native::{AlError, ExceptionKind};
use al_builtins::page_op::{PageMethod, PageOp, PageRecv};

/// The refusal of a close after a handler error that left the pinned cells.
const HANDLER_ERROR_CLOSE: &str = "a page left open by a PageHandler that raised an error, whose close then failed outside the pinned cells, was refused or wrote to the database (not pinned)";

/// A non-modal `Page.Run` by AL code that a PageHandler answers (`PageState::page_run`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageRun {
    call: PageRunCall,
    /// The run started in codeunit code with no handler running and no page open (`started_in_codeunit_code`).
    from_codeunit: bool,
    /// No write was pending when the run started: the pins commit before it (CemFixtures.al:549 `Commit()` in
    /// `Setup`, then only reads, CemPageHandlerTests.al:33-37).
    committed: bool,
    /// The session's count of commits when the run started: no pin commits after it.
    commits: u64,
    phase: PageRunPhase,
    /// The store's change counter and the session's write mark (`write_mark`: it moves by one with every insert and
    /// modify) when the run started.
    changes: u64,
    writes: (i64, u64),
    /// The session's count of caught errors when the run started (caught_errors.rs).
    caught: u64,
}

/// How AL code ran the page (`PageRun::call`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PageRunCall {
    /// `Page.Run(Page::"CEM Card", Row)`: the page by its id, a record, no field argument (CemFixtures.al:478, :499).
    StaticWithRecord,
    /// `SetRecord(Row)`, then `Run()`, and no other call on the Page variable (CemFixtures.al:485-486, :493-494).
    VariableAfterSetRecord,
    /// Any other run.
    Other,
}

impl PageRunCall {
    /// The static `Page.Run(n, ...)` of page `id`: `by_id`, the page named by a number; `record`, a Record as the second
    /// argument; `field`, a third argument.
    pub(crate) fn of_static(by_id: bool, record: bool, field: bool, n: i32, id: u32) -> Self {
        match (by_id, record, field, i64::from(n) == i64::from(id)) {
            (true, true, false, true) => Self::StaticWithRecord,
            _ => Self::Other,
        }
    }

    /// A Page variable's `Run()` of page `st`, by the calls AL code made on it (`Session::note_page_code_call`).
    pub(crate) fn of_variable(st: &St) -> Self {
        match st.borrow().code_calls.as_slice() {
            [PageMethod::SetRecord, PageMethod::Run] => Self::VariableAfterSetRecord,
            _ => Self::Other,
        }
    }
}

impl PageRun {
    /// `TestPage.Trap()` variable `trap` takes the page AL code runs on `from` (`run_page_ui_with`): it is caught by the trap,
    /// and `trapped_static_run` says whether the run was the static `Page.Run(Page::X, Rec)`.
    pub(crate) fn catch_by_trap(trap: &St, from: &St) {
        let static_run = from
            .borrow()
            .page_run
            .is_some_and(|r| r.call == PageRunCall::StaticWithRecord);
        let mut t = trap.borrow_mut();
        t.caught_by_trap = true;
        t.trapped_static_run = static_run;
    }
}

/// More calls than this are not kept in `PageState::code_calls`: a longer list is no pinned run.
const KEPT_CODE_CALLS: usize = 3;

/// Where the run of a `PageRun` page is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PageRunPhase {
    /// The page is open: before or inside its handler.
    Open,
    /// The handler returned and left the page open: the platform closes it.
    Returned,
    /// The handler raised an error and left the page open: the platform closes it.
    HandlerFailed(HandlerEnd),
}

/// The error a pinned handler ends with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HandlerEnd {
    /// Its own `Error('CEM handler error')` (CemPageHandlerTests.al:1846-1849, mode 7).
    Error,
    /// `P.Cancel().Invoke()` on a page that has no Cancel (:1824-1827, mode 2, `TestActionNotFound`).
    Cancel,
}

/// The page and run of the pins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PinnedRun {
    /// `Page.Run(Page::"CEM Card", Row)` (CemFixtures.al:476-479).
    CardStatic,
    /// `CardPage.SetRecord(Row); CardPage.Run()` (CemFixtures.al:481-487).
    CardVariable,
    /// `Page.Run(Page::"CEM List", Row)` (CemFixtures.al:497-500).
    ListStatic,
    /// `Wizard.SetRecord(Row); Wizard.Run()` of a NavigatePage (CemFixtures.al:489-495).
    NavigateVariable,
}

/// How the close was asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CloseBy {
    /// The platform, after the handler returned (modes 0, 4, 8).
    Returned,
    /// The platform, after the handler raised its own error (mode 7).
    HandlerError,
    /// The platform, after `Cancel` in the handler was not found (mode 2).
    HandlerCancel,
    /// `OK().Invoke()` in the handler (modes 1, 5).
    Ok,
    /// `TestPage.Close()` in the handler (mode 3).
    TestClose,
}

/// The row of the page when the close starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowAtClose {
    /// The stored row the run was given, nothing typed (handler modes 0, 1, 2, 3, 7).
    Unchanged,
    /// That row with one value typed into a Text field, not saved yet (modes 4, 5, 8:
    /// `P.Name.SetValue(...)`).
    Edited,
}

/// The error a close trigger or the save raised.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CloseError {
    /// OnQueryClosePage raised `Error('CEM query close error')` (CemFixtures.al:150-155).
    Query,
    /// OnClosePage raised `Error('CEM close page error')` (:163).
    ClosePage,
    /// OnClosePage asked `Confirm('CEM close confirm?')` and no handler took it (:165).
    ClosePageConfirm,
    /// The save of the typed value: the page's OnModifyRecord raised `Error('CEM page modify error')` in the codeunit it calls
    /// (:174, :206-210), or the table's OnModify raised `Error('CEM table modify refused')` (:45-52).
    Save,
}

/// What a close trigger or the save raised, as `close_with` and `close_modal_left_open` see it.
pub(super) enum CloseFailure<'a> {
    /// OnQueryClosePage raised the error.
    Query(&'a AlError),
    /// OnClosePage raised the error.
    ClosePage(&'a AlError),
    /// The save of the row raised the error; `trigger`: the page trigger it came out of (`None`: the table's own write),
    /// `raised_in`: the object whose routine raised it.
    Save {
        e: &'a AlError,
        trigger: Option<&'a str>,
        raised_in: Option<usize>,
    },
}

/// A cell of the matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cell {
    /// Pinned, and this module gives its result.
    Runs,
    /// Pinned, with a result this module does not give yet: refused.
    NotBuilt,
}

/// The facts of a close of a pinned page and run, taken when the close starts (`Session::page_run_close`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PageRunClose {
    run: PinnedRun,
    by: CloseBy,
    row: RowAtClose,
    /// The page's source table.
    table: u32,
    /// `PageRun::changes`, `PageRun::writes`, `PageRun::caught` and `PageRun::commits`.
    changes: u64,
    writes: (i64, u64),
    caught: u64,
    commits: u64,
}

const CLOSE_ERRORS: [Option<CloseError>; 5] = [
    None,
    Some(CloseError::Query),
    Some(CloseError::ClosePage),
    Some(CloseError::ClosePageConfirm),
    Some(CloseError::Save),
];

/// The cell of the corpus matrix for `error` at the close `by` of `run` with `row`; each arm names the tests that measured
/// it (CemPageHandlerTests.al, the line of the test). `None`: no test measured it.
fn cell(run: PinnedRun, by: CloseBy, row: RowAtClose, error: Option<CloseError>) -> Option<Cell> {
    use CloseBy as B;
    use CloseError as E;
    use PinnedRun as P;
    use RowAtClose as R;
    let pinned = match (by, error, run, row) {
        // The handler returned: :250 CardStaticRun_HandlerReturns_QueryCloseError, :386 _HandlerEditReturns_QueryCloseError,
        // :883 CardVariableRun_HandlerReturns_QueryCloseError, :965 _HandlerEditReturns_QueryCloseError, :1214
        // ListStaticRun_HandlerReturns_QueryCloseError, :1571 WizardRun_HandlerReturns_QueryCloseError.
        (B::Returned, Some(E::Query), P::CardStatic | P::CardVariable, R::Unchanged | R::Edited)
        | (B::Returned, Some(E::Query), P::ListStatic | P::NavigateVariable, R::Unchanged) => true,
        // :469, :993, :1323, :1626 _HandlerReturns_ClosePageError; :578, :1048, :1378, :1681 _HandlerReturns_ClosePageConfirmUnhandled.
        (B::Returned, Some(E::ClosePage | E::ClosePageConfirm), _, R::Unchanged) => true,
        // :687 CardStaticRun_HandlerEditReturns_PageModifyRecordError, :222 _HandlerTableRefusesReturns_NoError, :441
        // _HandlerTableRefusesReturns_QueryCloseError, :743 _HandlerTableRefusesReturns_PageModifyRecordError.
        (B::Returned, Some(E::Save), P::CardStatic, R::Edited) => true,
        // The handler raised its own error: :139 CardStaticRun_HandlerError_NoError, :1187 ListStaticRun_HandlerError_NoError,
        // :1544 WizardRun_HandlerError_NoError.
        (B::HandlerError, None, P::CardStatic | P::ListStatic | P::NavigateVariable, R::Unchanged) => true,
        // :359 CardStaticRun_HandlerError_QueryCloseError, :1296 ListStaticRun_HandlerError_QueryCloseError.
        (B::HandlerError, Some(E::Query), P::CardStatic | P::ListStatic, R::Unchanged) => true,
        // :1790 WizardRun_HandlerError_ClosePageConfirmUnhandled.
        (B::HandlerError, Some(E::ClosePageConfirm), P::NavigateVariable, R::Unchanged) => true,
        // Cancel was not found: :84 CardStaticRun_HandlerCancel_NoError, :1489 WizardRun_HandlerCancel_NoError.
        (B::HandlerCancel, None, P::CardStatic | P::NavigateVariable, R::Unchanged) => true,
        // :305 CardStaticRun_HandlerCancel_QueryCloseError.
        (B::HandlerCancel, Some(E::Query), P::CardStatic, R::Unchanged) => true,
        // :1736 WizardRun_HandlerCancel_ClosePageConfirmUnhandled.
        (B::HandlerCancel, Some(E::ClosePageConfirm), P::NavigateVariable, R::Unchanged) => true,
        // OK: :278, :497, :606 CardStaticRun_HandlerOk_*; :414, :551, :660 _HandlerEditOk_*; :911, :1021, :1076
        // CardVariableRun_HandlerOk_*; :1242, :1351, :1406 ListStaticRun_HandlerOk_*; :1599, :1654, :1709 WizardRun_HandlerOk_*
        // (QueryCloseError, ClosePageError, ClosePageConfirmUnhandled).
        (B::Ok, Some(E::Query | E::ClosePage | E::ClosePageConfirm), P::CardStatic, R::Unchanged | R::Edited)
        | (B::Ok, Some(E::Query | E::ClosePage | E::ClosePageConfirm), _, R::Unchanged) => true,
        // :715 CardStaticRun_HandlerEditOk_PageModifyRecordError: `Page.Run` returns and BC logs neither close trigger
        // (`returned` `M-card EDIT;`). Which close that is, is not built.
        (B::Ok, Some(E::Save), P::CardStatic, R::Edited) => return Some(Cell::NotBuilt),
        // TestPage.Close(): :633 CardStaticRun_HandlerClose_ClosePageConfirmUnhandled, :1763 WizardRun_HandlerClose_ClosePageConfirmUnhandled.
        (B::TestClose, Some(E::ClosePageConfirm), P::CardStatic | P::NavigateVariable, R::Unchanged) => true,
        _ => false,
    };
    pinned.then_some(Cell::Runs)
}

/// `e` is an AL `Error` with a text (the fixtures' form: code Dialog, `ExceptionKind::Dialog`, a text that is not empty).
fn error_with_text(e: &AlError) -> bool {
    e.code == "Dialog" && e.exception == ExceptionKind::Dialog && !e.message.is_empty()
}

impl Session {
    /// Starts a run of page `st` that a handler answers. A modal run or a trapped page is no `PageRun`; a non-modal one keeps
    /// what `note_page_run` noted.
    pub(super) fn begin_handled_run(&self, st: &St, modal: bool, trapped: bool) {
        let mut s = st.borrow_mut();
        s.modal = modal && !trapped;
        if modal || trapped {
            s.page_run = None;
        }
    }

    /// AL code calls `method` on page `st` through its Page variable or `CurrPage`: the first calls are kept
    /// (`KEPT_CODE_CALLS`, `PageRunCall::of_variable`).
    pub(super) fn note_page_code_call(st: &St, method: PageOp) {
        let mut s = st.borrow_mut();
        if method.recv() == PageRecv::Page && s.code_calls.len() < KEPT_CODE_CALLS {
            s.code_calls.push(method.method());
        }
    }

    /// AL code runs page `st` without a modal, by `call`.
    pub(crate) fn note_page_run(&mut self, st: &St, call: PageRunCall) {
        let changes = self.store_mut().change_counter();
        let from_codeunit = self.started_in_codeunit_code();
        let committed = !self.store.has_uncommitted();
        st.borrow_mut().page_run = Some(PageRun {
            call,
            from_codeunit,
            committed,
            commits: self.commits,
            phase: PageRunPhase::Open,
            changes,
            writes: self.write_mark(),
            caught: self.caught_errors(),
        });
    }

    /// The code running now is codeunit code only (every frame a codeunit), no handler runs, and no page is open: where the
    /// pins start their runs (CemPageHandlerTests.al:37 `Runner.RunCardStatic(Row)` in the test, CemFixtures.al:478 in
    /// codeunit CEM Runner).
    fn started_in_codeunit_code(&self) -> bool {
        let engine = self.engine();
        self.frames
            .iter()
            .all(|o| engine.program.objects[*o].kind == al_ir::ObjKind::Codeunit)
            && self.ui_state.page_handler_depth == 0
            && self.ui_state.scope.as_ref().is_some_and(|s| !s.in_handler)
            && self
                .events
                .pages_with_instance
                .iter()
                .filter_map(std::rc::Weak::upgrade)
                .all(|p| p.try_borrow().is_ok_and(|p| !p.open))
    }

    /// The pinned page and run of `st`, with its source table; `None` for every other page and run. Every fact of the pins is
    /// required (CemFixtures.al:185-275 pages CEM Card and CEM List, :427-470 CEM Wizard, :18-53 table CEM Row, :474-500 the
    /// runs):
    /// - run by AL code in the form of the pins (`PageRunCall`), from codeunit code with no page open and no handler running,
    ///   with no write pending; not modal, not a lookup, not opened by the test, with no host and no part; Rec sorted by its
    ///   primary key of one Code field, with one filter, on that field;
    /// - no page property but PageType, SourceTable, ApplicationArea, UsageCategory and Caption; no page extension, no
    ///   subscriber;
    /// - the page's own triggers exactly those of its fixture page, each with the fixture's body (`cem_trigger_call`) and a
    ///   call that hands values only (`hands_values`): on the Card and the List OnModifyRecord, OnInsertRecord,
    ///   OnQueryClosePage and OnClosePage, on the NavigatePage the two close triggers;
    /// - its controls fields of Rec without a trigger and with ApplicationArea only, on the List in one repeater, on the
    ///   NavigatePage with one action that has an OnAction trigger, ApplicationArea and InFooterBar (no pinned handler invokes it);
    /// - a stored source table whose own code is exactly OnInsert and OnModify: no field trigger, no procedure, no table
    ///   extension, no subscriber.
    fn pinned_run(&self, st: &St, run: PageRun) -> Result<Option<(PinnedRun, u32)>, Signal> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let shape = {
            let s = st.borrow();
            run.from_codeunit
                && run.committed
                && !s.modal
                && !s.lookup
                && !s.opened_by_test
                && s.host.is_none()
                && s.parts.is_empty()
                && s.hidden_parts.is_empty()
        };
        let pinned = match (engine.object_facts[object].page_type_lower(), run.call) {
            ("card", PageRunCall::StaticWithRecord) => PinnedRun::CardStatic,
            ("card", PageRunCall::VariableAfterSetRecord) => PinnedRun::CardVariable,
            ("list", PageRunCall::StaticWithRecord) => PinnedRun::ListStatic,
            ("navigatepage", PageRunCall::VariableAfterSetRecord) => PinnedRun::NavigateVariable,
            _ => return Ok(None),
        };
        if !shape || !Self::cem_page_code(&engine, object, pinned) {
            return Ok(None);
        }
        let rec = self.page_rec(st);
        if rec.borrow().temporary {
            return Ok(None);
        }
        let table = self.table(&rec)?;
        // The record of every pinned run: sorted by a primary key of one Code field, with one filter, on that field
        // (CemFixtures.al:26, :33; CemPageHandlerTests.al:33-34 `Row.Get('R1'); Row.SetRange("Code", 'R1')`).
        let view = {
            let r = rec.borrow();
            r.key == 0
                && r.adhoc.is_none()
                && r.ascending
                && r.field_descending == 0
                && !r.marked_only
                && matches!((table.keys[0].as_slice(), r.filters.as_slice()), ([k], [f])
                    if f.group == 0 && f.field == *k && matches!(table.fields[*k].ty, RtType::Code(_)))
        };
        if !view {
            return Ok(None);
        }
        let table = table.object;
        let table_code = pinned_shape::table_code(&engine, table as usize)
            .is_some_and(|(triggers, more)| !more && triggers == ["oninsert", "onmodify"]);
        Ok(table_code.then_some((pinned, table)))
    }

    /// Whether page `object` declares what the fixture page of `run` declares (see `pinned_run`).
    fn cem_page_code(engine: &Engine, object: usize, run: PinnedRun) -> bool {
        let program = &engine.program;
        let Some(page) = program.objects[object].page.as_ref() else {
            return false;
        };
        let lit = |c: &ir::Control, allowed: &[&str]| {
            c.props
                .iter()
                .all(|(k, v)| allowed.contains(&k.as_str()) && matches!(v, ir::PropVal::Lit(_)))
        };
        let mut triggers: Vec<&str> = page.triggers.iter().map(|(n, _)| n.as_str()).collect();
        triggers.sort_unstable();
        let (want_triggers, repeaters, actions): (&[&str], usize, usize) = match run {
            PinnedRun::CardStatic | PinnedRun::CardVariable => (
                &["onclosepage", "oninsertrecord", "onmodifyrecord", "onqueryclosepage"],
                0,
                0,
            ),
            PinnedRun::ListStatic => (
                &["onclosepage", "oninsertrecord", "onmodifyrecord", "onqueryclosepage"],
                1,
                0,
            ),
            PinnedRun::NavigateVariable => (&["onclosepage", "onqueryclosepage"], 0, 1),
        };
        let count = |kind: ir::ControlKind| page.controls.iter().filter(|c| c.kind == kind).count();
        triggers == want_triggers
            && page.triggers.iter().all(|(n, r)| {
                pinned_shape::cem_trigger_call(program, object, n, *r)
                    .is_some_and(|call| Self::hands_values(program, object, *r, call))
            })
            && page.extensions.is_empty()
            && page.ext_triggers.is_empty()
            && !engine.events.any_subscriber_of(object)
            && program.objects[object]
                .properties
                .iter()
                .all(|(k, _)| pinned_shape::PLAIN_PROPERTIES.contains(&k.to_ascii_lowercase().as_str()))
            && count(ir::ControlKind::Repeater) == repeaters
            && count(ir::ControlKind::Action) == actions
            && page.controls.iter().all(|c| {
                c.owner.is_none()
                    && c.ext_triggers.is_empty()
                    && match c.kind {
                        ir::ControlKind::Field => {
                            matches!(c.binding, ir::Binding::Field(_))
                                && c.triggers.is_empty()
                                && lit(c, &["applicationarea"])
                        }
                        ir::ControlKind::Repeater => c.binding == ir::Binding::None && c.triggers.is_empty(),
                        ir::ControlKind::Action => {
                            matches!(c.triggers.as_slice(), [(n, _)] if n == "onaction")
                                && c.run_object.is_none()
                                && lit(c, &["applicationarea", "infooterbar"])
                        }
                        _ => false,
                    }
            })
    }

    /// The codeunit call `call` of trigger routine `r` of page `object` hands values only, as every pinned trigger does
    /// (CemFixtures.al:208 `Cfg.ModRec('card', Rec.Name)`, :214 `Cfg.InsRec('card', Rec.Name)`, :220 `Cfg.Query('card',
    /// CloseAction)`, :225 `Cfg.Closed('card')`): each argument a constant, a field of Rec or a parameter of the trigger, none
    /// passed to a `var` parameter. So the row the page saves is the row as the handler typed it.
    fn hands_values(program: &ir::Program, object: usize, r: usize, call: &ir::Expr) -> bool {
        fn unwrap(e: &ir::Expr) -> &ir::Expr {
            match e {
                ir::Expr::Convert(_, inner) => inner,
                e => e,
            }
        }
        let ir::Expr::Call { args, .. } = unwrap(call) else {
            return false;
        };
        let rec = program.objects[object].rec_slot;
        let params = program.objects[object].routines[r].params;
        args.iter().all(|a| match a {
            ir::Arg::Value(e) => match unwrap(e) {
                ir::Expr::Const(_) => true,
                ir::Expr::Load(ir::Place::Local(i)) => *i < params,
                ir::Expr::Load(ir::Place::Field(base, _)) => {
                    matches!(base.as_ref(), ir::Place::Global(g) if Some(*g) == rec)
                }
                _ => false,
            },
            ir::Arg::Ref(_) => false,
        })
    }

    /// What the handler of page `st` did so far, when it is what a pinned handler does (CemPageHandlerTests.al:1814-1941
    /// `CemCardRun`, `CemListRun`, `CemWizardRun`): nothing, or one value typed into a Text field of Rec
    /// (`P.Name.SetValue(...)`, modes 4, 5, 8); then nothing or `OK().Invoke()`, or with nothing typed `Cancel()` or `Close()`. The second
    /// value: the call the handler made last. `None` after any other call.
    fn handler_calls(&self, st: &St) -> Result<Option<(RowAtClose, Option<PageMethod>)>, Signal> {
        use PageMethod::{Cancel, Close, Invoke, OK};
        use TestCall::{Other, Page, Write};
        let s = st.borrow();
        if s.tree_calls_unknown || s.tree_calls.iter().any(|c| c.part.is_some() || c.no) {
            return Ok(None);
        }
        let calls: Vec<TestCall> = s.tree_calls.iter().map(|c| c.call).collect();
        // Only the last call may have raised: the close it asked, or the Cancel that was not found.
        if s.tree_calls.iter().rev().skip(1).any(|c| c.failed) {
            return Ok(None);
        }
        let (row, rest) = match calls.as_slice() {
            [Write(i), rest @ ..] => {
                let engine = self.engine();
                let table = self.table(&self.page_rec(st))?;
                // Field 2, Name, of CEM Row (CemFixtures.al:27): a Text field; the primary key is the Code field alone
                // (`pinned_run`).
                match page_of(&engine, st).controls.get(*i).map(|c| &c.binding) {
                    Some(ir::Binding::Field(f)) if matches!(table.fields[*f].ty, RtType::Text(_)) => {
                        (RowAtClose::Edited, rest)
                    }
                    _ => return Ok(None),
                }
            }
            all => (RowAtClose::Unchanged, all),
        };
        Ok(match rest {
            [] => Some((row, None)),
            [Page(OK), Other(Invoke)] => Some((row, Some(OK))),
            [Page(Cancel)] if row == RowAtClose::Unchanged => Some((row, Some(Cancel))),
            [Page(Close)] if row == RowAtClose::Unchanged => Some((row, Some(Close))),
            _ => None,
        })
    }

    /// The facts of the close of page `st` that `close_with` (`action`, `via_action`, `test_close`) starts, when the page, the
    /// run, the handler's calls and the row are those of a cell of the matrix; `None` for every other close.
    pub(super) fn page_run_close(
        &self,
        st: &St,
        action: &str,
        via_action: bool,
        test_close: bool,
    ) -> Result<Option<PageRunClose>, Signal> {
        let Some(run) = st.borrow().page_run else {
            return Ok(None);
        };
        let Some((pinned, table)) = self.pinned_run(st, run)? else {
            return Ok(None);
        };
        let Some((row, last)) = self.handler_calls(st)? else {
            return Ok(None);
        };
        let by = match (via_action, test_close, run.phase, last) {
            (true, false, PageRunPhase::Open, Some(PageMethod::OK)) if action == "OK" => CloseBy::Ok,
            (false, true, PageRunPhase::Open, Some(PageMethod::Close)) => CloseBy::TestClose,
            (false, false, PageRunPhase::Returned, None) => CloseBy::Returned,
            (false, false, PageRunPhase::HandlerFailed(HandlerEnd::Error), None) => CloseBy::HandlerError,
            (false, false, PageRunPhase::HandlerFailed(HandlerEnd::Cancel), Some(PageMethod::Cancel)) => {
                CloseBy::HandlerCancel
            }
            _ => return Ok(None),
        };
        let s = st.borrow();
        // The stored row the run was given is under the cursor, and it is unsaved exactly when the handler typed a value.
        let row_as_typed = !s.draft
            && s.pending.is_none()
            && s.cursor == Cursor::Row
            && s.mode == PageMode::Edit
            && s.errors.is_empty()
            && !s.close_error
            && s.dirty == (row == RowAtClose::Edited);
        let measured = CLOSE_ERRORS.iter().any(|e| cell(pinned, by, row, *e).is_some());
        Ok((row_as_typed && measured).then_some(PageRunClose {
            run: pinned,
            by,
            row,
            table,
            changes: run.changes,
            writes: run.writes,
            caught: run.caught,
            commits: run.commits,
        }))
    }

    /// The pinned error a close trigger or the save raised, by where it came from; `None` for every other error.
    fn close_error_of(&self, c: &PageRunClose, failure: &CloseFailure) -> Option<CloseError> {
        match failure {
            CloseFailure::Query(e) => error_with_text(e).then_some(CloseError::Query),
            CloseFailure::ClosePage(e) if error_with_text(e) => Some(CloseError::ClosePage),
            CloseFailure::ClosePage(e) => self
                .unhandled_ui_is(e, UiKind::Confirm)
                .then_some(CloseError::ClosePageConfirm),
            CloseFailure::Save { e, trigger, raised_in } => {
                let source = match (trigger, raised_in) {
                    (Some("onmodifyrecord"), Some(o)) => {
                        self.engine.program.objects[*o].kind == al_ir::ObjKind::Codeunit
                    }
                    (None, Some(o)) => *o == c.table as usize,
                    _ => false,
                };
                (error_with_text(e) && source).then_some(CloseError::Save)
            }
        }
    }

    /// THE decision of this module: the cell of the matrix that the close `c` of page `st` is in, now that `failure` was
    /// raised (`None`: nothing was). Every fact of the cell is proved here or in `page_run_close`, which built `c`:
    /// - the error and its source (`close_error_of`);
    /// - the handler caught no error and nothing committed since the run started (no pin commits after `Setup`,
    ///   CemFixtures.al:549), and the test binds no handler but PageHandlers (every test of CemPageHandlerTests.al binds only
    ///   its PageHandler);
    /// - a typed value was saved by this close before a close trigger failed: the page holds nothing unsaved, and the only
    ///   write since the run started is that one modify of a row of the page's table (CemPageHandlerTests.al:386 `M-card EDIT; T-mod R1
    ///   EDIT; Q-card OK;`); in every other cell nothing was written since the run started.
    ///
    /// `None`: not a cell the corpus measured; the caller keeps the path it had.
    fn page_run_cell(
        &mut self,
        st: &St,
        c: Option<PageRunClose>,
        failure: Option<CloseFailure>,
    ) -> Option<(Cell, Option<CloseError>)> {
        let c = c?;
        let error = match &failure {
            Some(f) => Some(self.close_error_of(&c, f)?),
            None => None,
        };
        let saved = c.row == RowAtClose::Edited && error != Some(CloseError::Save);
        let dirty = st.borrow().dirty;
        let mark = self.write_mark();
        let store = self.store_mut();
        let writes = if saved {
            !dirty
                && mark == (c.writes.0 + 1, c.writes.1)
                && store.tables_changed_since(c.changes).into_iter().eq([c.table])
        } else {
            mark == c.writes && store.change_counter() == c.changes
        };
        let same = self.caught_errors() == c.caught && self.commits == c.commits;
        if !writes || !same || !self.every_ui_handler_is(UiKind::Page) {
            return None;
        }
        cell(c.run, c.by, c.row, error).map(|cell| (cell, error))
    }

    /// An error `e` of OnQueryClosePage in the close `c`. At the platform's close the page closes without OnClosePage and
    /// `Page.Run` returns (CemPageHandlerTests.al:250 `returned` `Q-card OK;`); at OK the error is shown as a message no
    /// handler takes and comes out with the code `TestWrapped:MissingUIHandler` (:278 `Unhandled UI: Message CEM query close
    /// error`). `None` outside the pinned cells.
    pub(super) fn page_run_query_close_error(
        &mut self,
        c: Option<PageRunClose>,
        st: &St,
        e: &AlError,
    ) -> Option<Result<Value, Signal>> {
        let (Cell::Runs, _) = self.page_run_cell(st, c, Some(CloseFailure::Query(e)))? else {
            return None;
        };
        match c?.by {
            CloseBy::Returned | CloseBy::HandlerError | CloseBy::HandlerCancel => {
                return Some(Ok(self.page_run_closed_without_error(st)));
            }
            CloseBy::Ok => {}
            CloseBy::TestClose => return None,
        }
        // The cell has no MessageHandler bound (`page_run_cell`), so the message comes back as the error nobody handled.
        let shown = self.ui_message(e.message.clone());
        self.mark_closed(st);
        Some(match shown {
            Err(Signal::Error(mut m)) if m.code == "MissingUIHandler" => {
                m.code = "TestWrapped:MissingUIHandler".into();
                Err(Signal::Error(m))
            }
            other => other.map(|()| Value::Null),
        })
    }

    /// An error of OnClosePage `r` in the close `c`: `Some(Ok)` when the platform closes the page (the error is dropped and
    /// the close goes on, CemPageHandlerTests.al:469 `returned` `Q-card OK; C-card;`), `Some(Err)` with the wrapped error at OK
    /// (:497 `One or more errors occurred while closing the page.\\CEM close page error` `TestWrapped`). `None` outside the
    /// pinned cells.
    pub(super) fn page_run_close_page_error(
        &mut self,
        c: Option<PageRunClose>,
        st: &St,
        r: &Result<Option<Value>, Signal>,
    ) -> Option<Result<(), Signal>> {
        let Err(Signal::Error(e)) = r else {
            return None;
        };
        let (Cell::Runs, _) = self.page_run_cell(st, c, Some(CloseFailure::ClosePage(e)))? else {
            return None;
        };
        match c?.by {
            CloseBy::Returned | CloseBy::HandlerError | CloseBy::HandlerCancel => {
                self.last_error = al_builtins::native::unpinned_error();
                Some(Ok(()))
            }
            CloseBy::Ok => {
                self.mark_closed(st);
                Some(Err(super::relation_lookup_close::closing_error(&e.message)))
            }
            // `page_close` wraps the error of its own close (`page_run_test_close_confirm`).
            CloseBy::TestClose => None,
        }
    }

    /// Whether the error `e` of OnClosePage at `TestPage.Close()` in the close `c` is the unhandled Confirm the corpus pins
    /// there (CemPageHandlerTests.al:633 `...page.\\Unhandled UI: Confirm CEM close confirm?` `TestWrapped`).
    pub(super) fn page_run_test_close_confirm(&mut self, c: Option<PageRunClose>, st: &St, e: &AlError) -> bool {
        c.is_some_and(|c| c.by == CloseBy::TestClose)
            && self.page_run_cell(st, c, Some(CloseFailure::ClosePage(e)))
                == Some((Cell::Runs, Some(CloseError::ClosePageConfirm)))
    }

    /// The page closes and `Page.Run` returns; the last error is not pinned.
    fn page_run_closed_without_error(&mut self, st: &St) -> Value {
        self.last_error = al_builtins::native::unpinned_error();
        self.mark_closed(st);
        Value::Null
    }

    /// The platform starts the close of page `st` that its handler returned from and left open (`action`:
    /// `unattended_action`): the facts of that close.
    pub(super) fn page_run_returned(&self, st: &St, action: &str) -> Result<Option<PageRunClose>, Signal> {
        if let Some(run) = st.borrow_mut().page_run.as_mut() {
            run.phase = PageRunPhase::Returned;
        }
        self.page_run_close(st, action, false, false)
    }

    /// The save of the typed value failed in the close `c` of page `st`, which stayed open (`close_modal_left_open`). In the
    /// pinned cell the page closes, the row as it was, and `Page.Run` returns (CemPageHandlerTests.al:687 `returned` `M-card
    /// EDIT;`, :222 `returned` `M-card FAILMOD; T-mod R1 FAILMOD;`). `false` outside it.
    pub(super) fn page_run_save_error_dropped(
        &mut self,
        c: Option<PageRunClose>,
        st: &St,
        failure: CloseFailure,
        errors_before: usize,
    ) -> bool {
        if !matches!(self.page_run_cell(st, c, Some(failure)), Some((Cell::Runs, _))) {
            return false;
        }
        {
            let mut s = st.borrow_mut();
            s.dirty = false;
            s.errors.truncate(errors_before);
        }
        self.page_run_closed_without_error(st);
        true
    }

    /// The save of the typed value failed with `e` at an action in the close `c`: the cell of OK on the pinned card is
    /// measured (CemPageHandlerTests.al:715 CardStaticRun_HandlerEditOk_PageModifyRecordError: `Page.Run` returns, neither
    /// close trigger is logged), and its close is not built, so it is refused. Every other close goes on as it did.
    pub(super) fn refuse_page_run_save_error_at_action(
        &mut self,
        c: Option<PageRunClose>,
        st: &St,
        e: &AlError,
    ) -> Result<(), Signal> {
        let object = st.borrow().object;
        let trigger = self
            .pages
            .failed_trigger
            .as_ref()
            .filter(|(o, _)| *o == object)
            .map(|(_, n)| n.clone());
        let failure = CloseFailure::Save {
            e,
            trigger: trigger.as_deref(),
            raised_in: self.pages.close_save_raised_in,
        };
        if matches!(self.page_run_cell(st, c, Some(failure)), Some((Cell::NotBuilt, _))) {
            return unsupported(
                "an error raised while the page saved its edited row at OK in a PageHandler of Page.Run (measured, not built)",
            );
        }
        Ok(())
    }

    /// A handler of page `st` raised `e` and left the page open. A modal page closes as `close_modal_after_handler_error`
    /// says (`caught_before`: its argument). In a pinned cell of a page run by `Page.Run` the platform closes the page as after
    /// a return, and the caller raises the handler's error (CemPageHandlerTests.al:139 `CEM handler error` `log` `Q-card OK;
    /// C-card;`). Every other page is left as it was. Once the close of a pinned cell runs, an end outside the cells (another
    /// error, a refused close, a write) is refused: the close triggers already ran.
    pub(super) fn close_page_after_handler_error(
        &mut self,
        st: &St,
        e: &Signal,
        modal: bool,
        caught_before: u64,
    ) -> Result<(), Signal> {
        if modal {
            return self.close_modal_after_handler_error(st, e, caught_before);
        }
        let end = match e {
            Signal::Error(e) if error_with_text(e) => HandlerEnd::Error,
            Signal::Error(e) if e.code == "TestActionNotFound" => HandlerEnd::Cancel,
            _ => return Ok(()),
        };
        if let Some(run) = st.borrow_mut().page_run.as_mut() {
            run.phase = PageRunPhase::HandlerFailed(end);
        }
        let a = self.unattended_action(st);
        let c = self.page_run_close(st, a, false, false)?;
        if self.page_run_cell(st, c, None).is_none() {
            return Ok(());
        }
        match self.close_with(st, a, false, false) {
            Ok(_) if !st.borrow().open && self.page_run_cell(st, c, None).is_some() => Ok(()),
            Ok(_) | Err(Signal::Error(_)) => unsupported(HANDLER_ERROR_CLOSE),
            Err(other) => Err(other),
        }
    }
}
