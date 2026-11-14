//! What code the runner stopped leaves of the session language and WorkDate: a test refused while it ran, and install code
//! that did not run to its end (al-runner issue 8072). One owner for the roll back and the restore.
//!
//! The pin is reports/runtime/BdRegionTests.al:1327-1339 (`BdSummaryML_1040`): the test saves `GlobalLanguage()` first,
//! switches, and sets the saved value back after the statement the runner refused; the `BdSetterLanguage_*` tests after it
//! (:926) run under the language from before it on real BC. A refused test is restored as on main. Install code is restored
//! only in that shape; install code that changed the state in any other shape leaves a state the runner does not know, and
//! the caller refuses the later code of that install session.

use al_builtins::builtin_id::ids;
use al_ir::{Arg, Expr, Place, Stmt};
use al_sema::lower::Lowered;
use al_vm::{CarriedPart, CarriedState, Session};

/// Rolls back the open transaction and sets the session language and WorkDate to `state`.
fn roll_back_and_restore(session: &mut Session, state: CarriedState) {
    session.rollback_transaction();
    session.restore_carried(state);
}

/// Ends a test refused while it ran: what it wrote and the language and WorkDate it set go back to `carried`, the state at
/// its start (BdRegionTests.al:1327-1339: the refusal came before the test's own `GlobalLanguage(Original)`).
pub(crate) fn roll_back_refused_test(session: &mut Session, carried: CarriedState) {
    roll_back_and_restore(session, carried);
}

/// Ends install code `root` (object, routine) that the runner stopped: its rows are rolled back. Per part of the state
/// (language, WorkDate) that differs from `before`, the state taken right before `root` ran: set back to `before` where
/// [`sets_back`] holds, else kept as it is at the stop. `Some(why)` when a part was kept: the state after `root` is not
/// known, and the caller refuses the later code of the install session with `why`. The caller clears the fault site
/// before `root` runs (`Session::clear_fault_site`). `by_runner`: the runner refused it (`Signal::Unsupported`); an AL
/// error ends it on real BC too, before any set-back, so nothing is set back then.
pub(crate) fn roll_back_stopped_install(
    lowered: &Lowered,
    session: &mut Session,
    root: (usize, usize),
    before: CarriedState,
    by_runner: bool,
) -> Option<String> {
    let now = session.carried_state();
    // The statement of `root` itself that was running at the stop.
    let stop = session
        .refusal_site()
        .filter(|_| by_runner)
        .and_then(|frames| frames.first().copied())
        .filter(|f| (f.object, f.routine) == root)
        .map(|f| f.pos);
    let mut target = now;
    let mut unknown = Vec::new();
    for part in [CarriedPart::Language, CarriedPart::WorkDate] {
        if !before.differs(&now, part) {
            continue;
        }
        if stop.is_some_and(|pos| sets_back(lowered, root, pos, part)) {
            target = target.with_part(&before, part);
        } else {
            unknown.push(match part {
                CarriedPart::Language => "language",
                CarriedPart::WorkDate => "WorkDate",
            });
        }
    }
    roll_back_and_restore(session, target);
    (!unknown.is_empty()).then(|| {
        format!(
            "the session {} after {}, which did not run to its end, is not known (it does not save the value first and set \
             it back as its last statement)",
            unknown.join(" and "),
            crate::install::label(lowered, root.0, root.1)
        )
    })
}

/// Whether routine `root`, stopped in the statement at position `pos`, would have set `part` back to the value it had when
/// the routine started: the shape of BdRegionTests.al:1327-1339. Each of these is required:
/// - the statements before the stopped one start with `V := GlobalLanguage();` (or `WorkDate()`), only such saves before
///   it, so `V` holds the value at the routine's start;
/// - the statements after the stopped one end with `GlobalLanguage(V);` (or `WorkDate(V)`), only such set-backs after it,
///   with no `exit` from the stopped statement on, so the set-back runs and nothing after it changes `part` again;
/// - `V` is a local that is not a parameter and is named nowhere else in the routine.
fn sets_back(lowered: &Lowered, root: (usize, usize), pos: u32, part: CarriedPart) -> bool {
    let (oi, ri) = root;
    let Some(rt) = lowered.program.objects[oi].routines.get(ri) else {
        return false;
    };
    let body: &[Stmt] = &rt.body;
    let marker = |s: &Stmt| matches!(s, Stmt::Pos(_) | Stmt::Assume(_) | Stmt::TddMark(_));
    // The saves at the start; `saved_to`: the index after the last one.
    let mut saves = Vec::new();
    let mut saved_to = 0;
    for (i, s) in body.iter().enumerate() {
        if marker(s) {
            continue;
        }
        let Some(save) = save_of(s) else { break };
        saves.push(save);
        saved_to = i + 1;
    }
    // The set-backs at the end, with their position markers; `set_back_from`: the index of the first one.
    let mut set_backs = Vec::new();
    let mut set_back_from = body.len();
    for (i, s) in body.iter().enumerate().rev() {
        if !marker(s) {
            let Some(set_back) = set_back_of(s) else { break };
            set_backs.push(set_back);
        }
        set_back_from = i;
    }
    let Some(stopped) = crate::runtime::written::statement_at(lowered, oi, ri, pos) else {
        return false;
    };
    if !(saved_to <= stopped && stopped < set_back_from) || has_exit(&body[stopped..set_back_from]) {
        return false;
    }
    let one = |list: &[(CarriedPart, usize)]| {
        let mut of_part = list.iter().filter(|(p, _)| *p == part);
        match (of_part.next(), of_part.next()) {
            (Some(&(_, v)), None) => Some(v),
            _ => None,
        }
    };
    match (one(&saves), one(&set_backs)) {
        (Some(v), Some(w)) => v == w && v >= rt.params && uses(body, v) == 2,
        _ => false,
    }
}

/// The builtin `e` calls with no receiver, under conversions, and its arguments.
fn builtin_call(e: &Expr) -> Option<(al_ir::BuiltinId, &[Arg])> {
    match strip(e) {
        Expr::Builtin {
            id,
            receiver: None,
            args,
            ..
        } => Some((*id, args)),
        _ => None,
    }
}

fn strip(mut e: &Expr) -> &Expr {
    while let Expr::Convert(_, x) = e {
        e = x;
    }
    e
}

fn part_of(id: al_ir::BuiltinId) -> Option<CarriedPart> {
    match id {
        ids::SYSTEM_GLOBALLANGUAGE_1 => Some(CarriedPart::Language),
        ids::SYSTEM_WORKDATE_1 => Some(CarriedPart::WorkDate),
        _ => None,
    }
}

/// `V := GlobalLanguage();` or `V := WorkDate();` with `V` a local: the part and `V`.
fn save_of(s: &Stmt) -> Option<(CarriedPart, usize)> {
    let Stmt::Assign(Place::Local(v), e) = s else {
        return None;
    };
    let (id, args) = builtin_call(e)?;
    if !args.is_empty() {
        return None;
    }
    Some((part_of(id)?, *v))
}

/// `GlobalLanguage(V);` or `WorkDate(V);` with `V` a local: the part and `V`.
fn set_back_of(s: &Stmt) -> Option<(CarriedPart, usize)> {
    let Stmt::Expr(e) = s else { return None };
    let (id, args) = builtin_call(e)?;
    let [Arg::Value(x)] = args else { return None };
    let Expr::Load(Place::Local(v)) = strip(x) else {
        return None;
    };
    Some((part_of(id)?, *v))
}

fn has_exit(body: &[Stmt]) -> bool {
    body.iter().any(|s| match s {
        Stmt::Exit(_) => true,
        Stmt::If(_, a, b) => has_exit(a) || has_exit(b),
        Stmt::While(_, b) | Stmt::Repeat(b, _) | Stmt::AssertError(b) => has_exit(b),
        Stmt::For { body, .. } | Stmt::Foreach { body, .. } => has_exit(body),
        Stmt::Case {
            branches, otherwise, ..
        } => branches.iter().any(|(_, b)| has_exit(b)) || has_exit(otherwise),
        _ => false,
    })
}

/// The local a place stands in, through indexes, fields and record globals.
fn local_of(mut p: &Place) -> Option<usize> {
    loop {
        match p {
            Place::Index(b, _) | Place::Field(b, _) | Place::RecGlobal(b, ..) => p = b,
            Place::Local(i) => return Some(*i),
            _ => return None,
        }
    }
}

/// How often `body` names local `v`: as an assignment or loop target, a read, or a `var` argument.
fn uses(body: &[Stmt], v: usize) -> usize {
    fn targets(body: &[Stmt], v: usize) -> usize {
        body.iter()
            .map(|s| match s {
                Stmt::Assign(p, _) => usize::from(local_of(p) == Some(v)),
                Stmt::If(_, a, b) => targets(a, v) + targets(b, v),
                Stmt::While(_, b) | Stmt::Repeat(b, _) | Stmt::AssertError(b) => targets(b, v),
                Stmt::For { var, body, .. } | Stmt::Foreach { var, body, .. } => {
                    usize::from(local_of(var) == Some(v)) + targets(body, v)
                }
                Stmt::Case {
                    branches, otherwise, ..
                } => branches.iter().map(|(_, b)| targets(b, v)).sum::<usize>() + targets(otherwise, v),
                _ => 0,
            })
            .sum()
    }
    let mut n = targets(body, v);
    let by_ref = |args: &[Arg]| {
        args.iter()
            .filter(|a| matches!(a, Arg::Ref(p) if local_of(p) == Some(v)))
            .count()
    };
    al_ir::types::for_each_expr_post(body, &mut |e| {
        n += match e {
            Expr::Load(p) => usize::from(local_of(p) == Some(v)),
            Expr::Call { args, .. } | Expr::InterfaceCall { args, .. } | Expr::DotNet { args, .. } => by_ref(args),
            Expr::Builtin { receiver, args, .. } => {
                by_ref(args) + receiver.as_deref().map_or(0, |r| by_ref(std::slice::from_ref(r)))
            }
            _ => 0,
        };
    });
    n
}
