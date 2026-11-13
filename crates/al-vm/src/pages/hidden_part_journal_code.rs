//! The code of the pages of the journal shape of a hidden part (`hidden_part_journal.rs`), compared statically with the code
//! of the measured pages (open corpus PR 1367, testpage/parts/HiddenPartJournalShape.al at commit 32b29320; assumption
//! `hidden-part-journal-shape`): the host's triggers and actions (pages 75315 to 75317) and the part page's one procedure
//! (pages 75313, 75314). Each check lists the statements and expressions those pages have and requires that the page has no
//! other.

use super::pinned_shape::{code_stmts, codeunit_var_call};
use super::*;
use al_builtins::builtin_id::ids;

/// The Record methods the part page's procedure calls (`Fill`, HiddenPartJournalShape.al:118-134 on a temporary Rec and a
/// local record, :180-185 on the stored Rec).
const FILL_RECORD_METHODS: [al_builtins::builtin_id::BuiltinId; 8] = [
    ids::RECORD_DELETEALL_1,
    ids::RECORD_SETRANGE_1,
    ids::RECORD_FINDSET_1,
    ids::RECORD_INIT_1,
    ids::RECORD_INSERT_1,
    ids::RECORD_NEXT_1,
    ids::RECORD_COUNT_1,
    ids::RECORD_ISEMPTY_1,
];

/// The global of host page `host` that the Visible expression of its part `control` returns, when that expression is one
/// Boolean variable of the page (`Visible = ShowStatus`, HiddenPartJournalShape.al:239).
pub(super) fn visible_variable(program: &ir::Program, host: usize, control: &ir::Control) -> Option<usize> {
    let (_, value) = control.props.iter().find(|(k, _)| k.eq_ignore_ascii_case("visible"))?;
    let ir::PropVal::Routine(r) = value else {
        return None;
    };
    match code_stmts(program, host, *r).as_slice() {
        [ir::Stmt::Exit(Some(ir::Expr::Load(ir::Place::Global(g))))]
            if program.objects[host]
                .globals
                .get(*g)
                .is_some_and(|s| s.ty == RtType::Boolean) =>
        {
            Some(*g)
        }
        _ => None,
    }
}

/// `p` is Rec (global `rec`) or a field of it.
fn rec_place(p: &ir::Place, rec: usize) -> bool {
    match p {
        ir::Place::Global(g) => *g == rec,
        ir::Place::Field(base, _) => matches!(&**base, ir::Place::Global(g) if *g == rec),
        _ => false,
    }
}

/// `e` is a field of Rec, as it is or converted to the parameter's type (`Rec."No."`, :252).
fn rec_field(e: &ir::Expr, rec: usize) -> bool {
    let e = match e {
        ir::Expr::Convert(_, inner) => inner,
        e => e,
    };
    matches!(e, ir::Expr::Load(p @ ir::Place::Field(..)) if rec_place(p, rec))
}

/// `e` is a value the measured host code hands to a call: a constant, Rec or a field of it, a conversion or a binary
/// operation of those (`'host-agcr(' + Rec."No." + ')'`, :265; `Rec`, :222).
fn host_value(e: &ir::Expr, rec: usize) -> bool {
    match e {
        ir::Expr::Const(_) => true,
        ir::Expr::Load(p) => rec_place(p, rec),
        ir::Expr::Convert(_, e) => host_value(e, rec),
        ir::Expr::Binary(_, a, b) => host_value(a, rec) && host_value(b, rec),
        _ => false,
    }
}

/// The routines of page `object` that are procedures: no trigger and no routine of an expression property of a control.
fn procedures(program: &ir::Program, object: usize, page: &ir::Page) -> Vec<usize> {
    let property = |r: usize| {
        page.controls
            .iter()
            .flat_map(|c| &c.props)
            .any(|(_, v)| matches!(v, ir::PropVal::Routine(p) if *p == r))
    };
    program.objects[object]
        .routines
        .iter()
        .enumerate()
        .filter(|(r, routine)| !routine.is_trigger && !property(*r))
        .map(|(r, _)| r)
        .collect()
}

/// The one procedure of part page `object` (`page`), when the page has exactly one and its body holds only what the measured
/// `Fill` procedures hold (HiddenPartJournalShape.al:118-134, :180-185): the methods of [`FILL_RECORD_METHODS`] on Rec or on a
/// local record with value arguments, `Format`, calls of a procedure of a codeunit variable, constants, Rec, locals, their
/// fields and operations on them. Not in it: a method of `CurrPage`, a call of another procedure, any other builtin.
pub(super) fn part_procedure_is(program: &ir::Program, object: usize, page: &ir::Page) -> Option<usize> {
    let &[fill] = procedures(program, object, page).as_slice() else {
        return None;
    };
    let o = &program.objects[object];
    let (rec, routine) = (o.rec_slot?, &o.routines[fill]);
    let record_local = |l: usize| {
        routine
            .locals
            .get(l)
            .is_some_and(|s| matches!(s.ty, RtType::Record(..)))
    };
    let place = |p: &ir::Place| {
        let base = match p {
            ir::Place::Field(base, _) => base,
            p => p,
        };
        match base {
            ir::Place::Local(_) => true,
            // Rec, or the codeunit variable a call runs on (`Log`).
            ir::Place::Global(g) => *g == rec || o.globals.get(*g).is_some_and(|s| matches!(s.ty, RtType::Codeunit(_))),
            _ => false,
        }
    };
    let record = |a: &ir::Arg| match a {
        ir::Arg::Ref(ir::Place::Global(g)) | ir::Arg::Value(ir::Expr::Load(ir::Place::Global(g))) => *g == rec,
        ir::Arg::Ref(ir::Place::Local(l)) | ir::Arg::Value(ir::Expr::Load(ir::Place::Local(l))) => record_local(*l),
        _ => false,
    };
    let values = |args: &[ir::Arg]| args.iter().all(|a| matches!(a, ir::Arg::Value(_)));
    let mut measured = true;
    al_ir::types::for_each_expr_post(routine.body.stmts(), &mut |e| {
        measured &= match e {
            ir::Expr::Const(_) | ir::Expr::Unary(..) | ir::Expr::Binary(..) | ir::Expr::Convert(..) => true,
            ir::Expr::Load(p) => place(p),
            ir::Expr::Builtin {
                id,
                receiver: Some(receiver),
                args,
                ..
            } => FILL_RECORD_METHODS.contains(id) && record(receiver) && values(args),
            ir::Expr::Builtin {
                id,
                receiver: None,
                args,
                ..
            } => *id == ids::SYSTEM_FORMAT_1 && values(args),
            ir::Expr::Call { args, .. } => codeunit_var_call(program, object, e) && values(args),
            _ => false,
        };
    });
    measured.then_some(fill)
}

/// Whether the code of host page `host` (`page`) is the measured hosts' (HiddenPartJournalShape.al pages 75315 to 75317), for
/// its part control `part`:
/// - the page has one procedure, whose body is `Variable := CurrPage.<part>.Page.<procedure>(Rec.<field>)` with the Visible
///   variable of the part control and the part page's one procedure (`RefreshStatus`, :250-253);
/// - each page trigger and each control trigger holds only calls of that procedure and calls of a procedure of a codeunit
///   variable, a global or a local of the trigger, whose arguments are constants, Rec and fields of Rec (:219-223, :255-270).
pub(super) fn host_bodies_are(program: &ir::Program, host: usize, page: &ir::Page, part: usize) -> bool {
    let o = &program.objects[host];
    let control = &page.controls[part];
    let facts = || {
        let rec = o.rec_slot?;
        let part_page = control.part_page?;
        let fill = part_procedure_is(program, part_page, program.objects[part_page].page.as_ref()?)?;
        let visible = visible_variable(program, host, control)?;
        let &[refresh] = procedures(program, host, page).as_slice() else {
            return None;
        };
        Some((rec, part_page, fill, visible, refresh))
    };
    let Some((rec, part_page, fill, visible, refresh)) = facts() else {
        return false;
    };
    let sets_visible = matches!(code_stmts(program, host, refresh).as_slice(),
        [ir::Stmt::Assign(ir::Place::Global(v), ir::Expr::Call { object, routine, instance: Some(instance), args, try_call: false })]
            if *v == visible
                && (*object, *routine) == (part_page, fill)
                && matches!(&**instance, ir::Expr::Control { page: p, control: c, runtime: true }
                    if *c == part && matches!(&**p, ir::Expr::Load(ir::Place::Global(g)) if *g == page.curr_page_slot))
                && matches!(args.as_slice(), [ir::Arg::Value(a)] if rec_field(a, rec)));
    let measured_stmt = |r: usize, s: &ir::Stmt| {
        let ir::Stmt::Expr(ir::Expr::Call {
            object,
            routine,
            instance,
            args,
            try_call: false,
        }) = s
        else {
            return false;
        };
        let Some(instance) = instance else {
            return (*object, *routine) == (host, refresh) && args.is_empty();
        };
        let codeunit = |s: Option<&ir::Slot>| s.is_some_and(|s| matches!(s.ty, RtType::Codeunit(_)));
        let variable = match &**instance {
            ir::Expr::Load(ir::Place::Global(g)) => codeunit(o.globals.get(*g)),
            ir::Expr::Load(ir::Place::Local(l)) => codeunit(o.routines[r].locals.get(*l)),
            _ => false,
        };
        variable
            && args.iter().all(|a| match a {
                ir::Arg::Value(e) => host_value(e, rec),
                ir::Arg::Ref(p) => matches!(p, ir::Place::Global(g) if *g == rec),
            })
    };
    let triggers = page
        .triggers
        .iter()
        .chain(page.controls.iter().flat_map(|c| &c.triggers));
    sets_visible
        && triggers
            .into_iter()
            .all(|(_, r)| code_stmts(program, host, *r).iter().all(|s| measured_stmt(*r, s)))
}
