//! The page triggers of the List hosts of corpus pull request 1378 (pages/parts/EmptyPartRecListHostTriggers.al, merged, not yet
//! in the pinned corpus; assumption `empty-part-rec-list-host-triggers`): the reporter's host of al-runner issue 6924 has an
//! OnOpenPage that sets CurrPage.Editable from a value read at run time and sends a notification, and an OnAfterGetCurrRecord that
//! sends the same notification. The probe measured Rec of the empty part on hosts with exactly those triggers, both or one of them
//! (pages 76865-76870). `host_trigger_cell` compares each trigger body, statement by statement, with the probe's, and each called
//! procedure with the probe's procedure; the SendNotificationHandler of the test with the probe's (`notification_handlers_measured`).

use super::pinned_shape::{code_stmts, text_log_routine, unconverted, without_pos};
use al_builtins::builtin_id::ids;
use al_builtins::library_kind::LibraryKind;
use al_builtins::value::RtType;
use al_ir as ir;

/// A statement of a probe host trigger.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum HostStmt {
    /// `Log.AddSeq('open')`: a procedure of a codeunit variable of the trigger that only appends its text argument to a Text
    /// global of the codeunit (EmptyPartRecListHostTriggers.al:503-508, `text_log_routine`).
    Log,
    /// `CurrPage.Editable := Setup.IsLicensed()` (:236): the result of `licensed_routine` on a Record variable of the trigger.
    Editable,
    /// `Setup.ShowDisabledNotification()` (:237): `notify_routine` on a Record variable of the trigger.
    Notify,
}

use HostStmt::{Editable, Log, Notify};

/// The measured hosts: their OnOpenPage and OnAfterGetCurrRecord bodies (`None`: no such trigger). Each host form has the three.
const CELLS: [(Option<&[HostStmt]>, Option<&[HostStmt]>); 3] = [
    // Pages 76865 "PG EPT List Both" (:224) and 76868 "PG EPT Rep Both" (:352).
    (Some(&[Log, Editable, Notify]), Some(&[Log, Notify])),
    // Pages 76866 "PG EPT List OpenOnly" (:273) and 76869 "PG EPT Rep OpenOnly" (:403).
    (Some(&[Log, Editable]), None),
    // Pages 76867 "PG EPT List CurrOnly" (:312) and 76870 "PG EPT Rep CurrOnly" (:444).
    (None, Some(&[Log, Notify])),
];

/// Whether the page triggers of List page `host` are those of a host of probe 1378 (`CELLS`): exactly OnOpenPage,
/// OnAfterGetCurrRecord or both, each body the probe's statements in the probe's order, the Record variables of every trigger on
/// one table that is neither `host_table` nor `part_table` (table 76862 "PG EPT Setup", :63) and not temporary.
pub(super) fn host_trigger_cell(program: &ir::Program, host: usize, host_table: u32, part_table: u32) -> bool {
    let Some(page) = program.objects[host].page.as_ref() else {
        return false;
    };
    let mut open = None;
    let mut curr = None;
    let mut setup = None;
    for (name, r) in &page.triggers {
        let slot = match name.as_str() {
            "onopenpage" => &mut open,
            "onaftergetcurrrecord" => &mut curr,
            _ => return false,
        };
        let body: Option<Vec<HostStmt>> = code_stmts(program, host, *r)
            .into_iter()
            .map(|s| host_stmt(program, host, page, *r, s, &mut setup))
            .collect();
        let Some(body) = body else { return false };
        if slot.replace(body).is_some() {
            return false;
        }
    }
    let third_table = setup.is_some_and(|t| t != host_table && t != part_table);
    third_table
        && CELLS
            .iter()
            .any(|(o, c)| open.as_deref() == *o && curr.as_deref() == *c)
}

/// What statement `s` of trigger `r` of host `host` is, when it is one the probe hosts have (`HostStmt`). The table of a Record
/// variable it calls a procedure on is kept in `setup`: every such call must be on the same table.
fn host_stmt(
    program: &ir::Program,
    host: usize,
    page: &ir::Page,
    r: usize,
    s: &ir::Stmt,
    setup: &mut Option<u32>,
) -> Option<HostStmt> {
    let locals = &program.objects[host].routines[r].locals;
    let ir::Stmt::Expr(e) = s else { return None };
    if text_log_call(program, locals, e) {
        return Some(Log);
    }
    if let Some((t, callee)) = setup_call(locals, e)
        && notify_routine(program, t, callee)
        && same_table(setup, t)
    {
        return Some(Notify);
    }
    let ir::Expr::Builtin { id, receiver, args, .. } = e else {
        return None;
    };
    let Some(ir::Place::Global(g)) = by_ref(receiver) else {
        return None;
    };
    let [ir::Arg::Value(value)] = args.as_slice() else {
        return None;
    };
    let (t, callee) = setup_call(locals, unconverted(value))?;
    (*id == ids::PAGE_EDITABLE_1
        && *g == page.curr_page_slot
        && licensed_routine(program, t, callee)
        && same_table(setup, t))
    .then_some(Editable)
}

/// The variable a builtin method runs on, when it is passed by reference.
fn by_ref(receiver: &Option<Box<ir::Arg>>) -> Option<&ir::Place> {
    match receiver.as_deref() {
        Some(ir::Arg::Ref(p)) => Some(p),
        _ => None,
    }
}

/// `t` is the table of the first Record variable seen (`setup`), or the first.
fn same_table(setup: &mut Option<u32>, t: u32) -> bool {
    *setup.get_or_insert(t) == t
}

/// `e` calls, with no argument and not as a try call, a procedure of the table of a non-temporary Record variable in `locals`:
/// that table and the routine.
fn setup_call(locals: &[ir::Slot], e: &ir::Expr) -> Option<(u32, usize)> {
    let ir::Expr::Call {
        object,
        routine,
        instance: Some(instance),
        args,
        try_call: false,
    } = e
    else {
        return None;
    };
    let ir::Expr::Load(ir::Place::Local(l)) = &**instance else {
        return None;
    };
    match locals.get(*l)?.ty {
        RtType::Record(t, false) if t as usize == *object && args.is_empty() => Some((t, *routine)),
        _ => None,
    }
}

/// `e` calls, not as a try call, a procedure of a codeunit variable in `locals` with one text constant, and the procedure only
/// appends its argument to a Text global of its codeunit (`text_log_routine`; `Log.AddSeq('open')`, :234).
fn text_log_call(program: &ir::Program, locals: &[ir::Slot], e: &ir::Expr) -> bool {
    let ir::Expr::Call {
        object,
        routine,
        instance: Some(instance),
        args,
        try_call: false,
    } = e
    else {
        return false;
    };
    let codeunit = matches!(&**instance, ir::Expr::Load(ir::Place::Local(l))
        if locals.get(*l).is_some_and(|s| s.ty == RtType::Codeunit(*object as u32)));
    let text =
        matches!(args.as_slice(), [ir::Arg::Value(v)] if matches!(unconverted(v), ir::Expr::Const(ir::Const::Text(_))));
    codeunit && text && text_log_routine(program, *object, *routine)
}

/// The routine is a procedure with no parameter and the result `ret`.
fn signature(routine: &ir::Routine, ret: Option<RtType>) -> bool {
    !routine.is_trigger && routine.params == 0 && routine.ret == ret
}

/// `e` is `Get()` on the table's own Rec (global `rec`) with no argument.
fn get_on_rec(e: &ir::Expr, rec: usize) -> bool {
    matches!(e, ir::Expr::Builtin { id, receiver, args, .. }
        if *id == ids::RECORD_GET_1 && by_ref(receiver) == Some(&ir::Place::Global(rec)) && args.is_empty())
}

/// `e` reads a Boolean field of the table's own Rec (global `rec`) of table object `t`.
fn rec_boolean_field(program: &ir::Program, t: u32, rec: usize, e: &ir::Expr) -> bool {
    let ir::Expr::Load(ir::Place::Field(base, f)) = e else {
        return false;
    };
    let table = program.objects[t as usize].table.as_ref();
    matches!(**base, ir::Place::Global(g) if g == rec)
        && table
            .and_then(|t| t.fields.get(*f))
            .is_some_and(|f| f.ty == RtType::Boolean)
}

/// Procedure `r` of table `t` is the probe's `IsLicensed` (EmptyPartRecListHostTriggers.al:78-83): no parameter, a Boolean
/// result, no local, and the body `if not Get() then exit(false); exit(<Boolean field>);`.
fn licensed_routine(program: &ir::Program, t: u32, r: usize) -> bool {
    let o = &program.objects[t as usize];
    let (Some(routine), Some(rec)) = (o.routines.get(r), o.rec_slot) else {
        return false;
    };
    let no_locals = routine.locals.len() == usize::from(routine.ret_local.is_some());
    let body = match code_stmts(program, t as usize, r).as_slice() {
        [
            ir::Stmt::If(ir::Expr::Unary(ir::UnOp::Not, get), then, otherwise),
            ir::Stmt::Exit(Some(field)),
        ] => {
            get_on_rec(get, rec)
                && otherwise.is_empty()
                && matches!(
                    without_pos(then).as_slice(),
                    [ir::Stmt::Exit(Some(ir::Expr::Const(ir::Const::Boolean(false))))]
                )
                && rec_boolean_field(program, t, rec, unconverted(field))
        }
        _ => false,
    };
    signature(routine, Some(RtType::Boolean)) && no_locals && body
}

/// Procedure `r` of table `t` is the probe's `ShowDisabledNotification` (EmptyPartRecListHostTriggers.al:85-97): no parameter,
/// no result, a codeunit and a Notification local, and the body `if Get() then if <Boolean field> then exit;`, a log call
/// (`text_log_call`), `N.Message := <text constant>; N.Scope := NotificationScope::LocalScope; N.Send();`. A notification with
/// an Id, an action or GlobalScope is not measured.
fn notify_routine(program: &ir::Program, t: u32, r: usize) -> bool {
    let o = &program.objects[t as usize];
    let (Some(routine), Some(rec)) = (o.routines.get(r), o.rec_slot) else {
        return false;
    };
    let locals = &routine.locals;
    let notification = |l: &usize| {
        locals
            .get(*l)
            .is_some_and(|s| s.ty == RtType::Library(LibraryKind::Notification))
    };
    let kinds: Vec<&RtType> = locals.iter().map(|s| &s.ty).collect();
    let two_locals = matches!(
        kinds.as_slice(),
        [RtType::Codeunit(_), RtType::Library(LibraryKind::Notification)]
            | [RtType::Library(LibraryKind::Notification), RtType::Codeunit(_)]
    );
    let call = |s: &ir::Stmt, id: al_builtins::builtin_id::BuiltinId, arg: &dyn Fn(&ir::Expr) -> bool| {
        matches!(s, ir::Stmt::Expr(ir::Expr::Builtin { id: i, receiver, args, .. })
        if *i == id && matches!(by_ref(receiver), Some(ir::Place::Local(l)) if notification(l)) && match args.as_slice() {
            [] => false,
            [ir::Arg::Value(v)] => arg(unconverted(v)),
            _ => false,
        })
    };
    let send = |s: &ir::Stmt| {
        matches!(s, ir::Stmt::Expr(ir::Expr::Builtin { id, receiver, args, .. })
            if *id == ids::NOTIFICATION_SEND_1 && matches!(by_ref(receiver), Some(ir::Place::Local(l)) if notification(l))
                && args.is_empty())
    };
    let text = |e: &ir::Expr| matches!(e, ir::Expr::Const(ir::Const::Text(_)));
    let local_scope = |e: &ir::Expr| {
        matches!(e, ir::Expr::Const(ir::Const::Option(0, names))
            if names.first().is_some_and(|n| n.eq_ignore_ascii_case("LocalScope")))
    };
    let enabled_exit = |then: &[ir::Stmt]| match without_pos(then).as_slice() {
        [ir::Stmt::If(field, inner, otherwise)] => {
            rec_boolean_field(program, t, rec, unconverted(field))
                && otherwise.is_empty()
                && matches!(without_pos(inner).as_slice(), [ir::Stmt::Exit(None)])
        }
        _ => false,
    };
    let body = match code_stmts(program, t as usize, r).as_slice() {
        [
            ir::Stmt::If(get, then, otherwise),
            ir::Stmt::Expr(log),
            message,
            scope,
            send_stmt,
        ] => {
            get_on_rec(get, rec)
                && otherwise.is_empty()
                && enabled_exit(then)
                && text_log_call(program, locals, log)
                && call(message, ids::NOTIFICATION_MESSAGE_1, &text)
                && call(scope, ids::NOTIFICATION_SCOPE_1, &local_scope)
                && send(send_stmt)
        }
        _ => false,
    };
    signature(routine, None) && two_locals && body
}

/// Whether routine `r` of test codeunit `object` is the probe's SendNotificationHandler (EmptyPartRecListHostTriggers.al:556-561):
/// a log call on a codeunit global of the test codeunit (`text_log_routine`), then `exit(true)`.
pub(super) fn notification_handler_measured(program: &ir::Program, object: usize, r: usize) -> bool {
    let o = &program.objects[object];
    let log = |e: &ir::Expr| {
        let ir::Expr::Call {
            object: callee,
            routine,
            instance: Some(instance),
            args,
            try_call: false,
        } = e
        else {
            return false;
        };
        let global = matches!(&**instance, ir::Expr::Load(ir::Place::Global(g))
            if o.globals.get(*g).is_some_and(|s| s.ty == RtType::Codeunit(*callee as u32)));
        let text = matches!(args.as_slice(), [ir::Arg::Value(v)] if matches!(unconverted(v), ir::Expr::Const(ir::Const::Text(_))));
        global && text && text_log_routine(program, *callee, *routine)
    };
    matches!(code_stmts(program, object, r).as_slice(),
        [ir::Stmt::Expr(e), ir::Stmt::Exit(Some(ir::Expr::Const(ir::Const::Boolean(true))))] if log(e))
}
