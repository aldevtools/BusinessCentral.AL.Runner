//! A partner app's load-once memo that its own login subscriber's writes filled through the `Global Triggers` events
//! (al-runner issue 8945): the shape of cell L1 of corpus PR 1571 (open; tests/al-language-xapp-a/LmXaLoginCaches.al,
//! the memo "LT LM XA L1 Memo", reached only through the wrapper's integration events of LmXaMemoShapes.al; read by
//! L1Memo_AfterLogin, L1Memo_TestAddsSetupRow and AllCaches_SecondCodeunit of LmLoginCacheTlTests.al and
//! LmLoginCacheTests.al). Measured: the setup table is empty, the setup ask sets Checked and leaves Activated false, the first
//! insert event sets Loaded, `Clear`s a second temporary record and sets one `=''` filter on it, and copies no row; a later
//! test reads those values in both kinds of session and does not see a setup row it adds. [`kept`] accepts only that shape,
//! through assumption `partner-login-load-once-memo`; the caller (`partner_login::kept`) asks it for a codeunit of the
//! subscriber's own app that no subscriber changed before. Entry point: [`kept`].

use crate::partner_login::By;
use al_builtins::filter::{FieldFilter, FilterExpr};
use al_builtins::value::{RtType, Value};
use al_ir::{Arg, Expr, Place, Stmt};
use al_sema::lower::Lowered;
use al_vm::Session;
use al_vm::events::Registry;
use std::collections::BTreeSet;

/// The builtins L1's memo calls (LmXaLoginCaches.al, "LT LM XA L1 Memo"), by name without the overload: anything else in the memo's
/// code (a host or company call, a permission or temporary-state check, another object's procedure) is not measured.
const MEASURED_BUILTINS: &[&str] = &[
    "Record.IsEmpty",
    "Record.Get",
    "Record.DeleteAll",
    "Record.FindSet",
    "Record.Next",
    "Record.Insert",
    "Record.SetFilter",
    "Record.Copy",
    "Record.Reset",
    "Record.Count",
    "Record.GetFilters",
    "System.Clear",
    "System.Format",
];

/// The builtins of [`MEASURED_BUILTINS`] that leave their record receiver as it was: L1's public readers call them on its
/// temporary globals (LmXaLoginCaches.al RawSetupFilters, RawSetupRows). Every other builtin moves or fills its receiver.
const READING_BUILTINS: &[&str] = &["Record.IsEmpty", "Record.Count", "Record.GetFilters"];

/// The events of Microsoft's GlobalTriggerManagement (codeunit 49 of a Microsoft app) the probe's wrapper subscribes to
/// (LmXaMemoShapes.al:239-253, "LT LM XA Wrapper"), lower case.
const WRAPPED_EVENTS: &[&str] = &[
    "onaftergetdatabasetabletriggersetup",
    "onafterondatabaseinsert",
    "onafterondatabasemodify",
];

/// `Ok` when the change `by` of SingleInstance codeunit `memo` is of L1's shape, every fact by equality:
/// - every global of the memo is a Boolean, an Integer, a Text or a temporary record (LmXaLoginCaches.al, the globals of "LT LM XA L1 Memo");
/// - every event subscriber of the memo subscribes to an event of a wrapper of the probe's shape ([`wrapper_events`]);
/// - its code is of L1's kinds ([`code_of_l1_kinds`]): only those subscribers and the procedures they alone reach change a global;
/// - each changed global is a Boolean that holds true now, that every statement of the memo sets to `true` and nothing else
///   changes (`settled_flags`), changed inside a raise of exactly one of `GetDatabaseTableTriggerSetup` and
///   `OnDatabaseInsert` (Checked in the ask, WrapperSetup; Loaded on the insert event, WrapperWrite); or a temporary record
///   that holds no row and the view of a fresh one except for exactly one filter, `=''` on one field in the standard filter
///   group, changed inside a raise of `OnDatabaseInsert` only (TempSub, LoadTemps). A changed Integer (LoadTable,
///   WrapperWrite, which holds the asked table) is not of the shape;
/// - the changed globals are exactly L1's: one such Boolean changed in the setup ask, one in the insert event, one such
///   temporary record. A memo whose setup ask changed nothing in this login (an ask made before it) is not of the shape.
///
/// `Err` names the first fact that does not hold.
pub(crate) fn kept(lowered: &Lowered, session: &mut Session, memo: usize, by: &By) -> Result<(), String> {
    let obj = &lowered.program.objects[memo];
    let l1_type = |ty: &RtType| {
        matches!(
            ty,
            RtType::Boolean | RtType::Integer | RtType::Text(None) | RtType::Record(_, true)
        )
    };
    if let Some(g) = obj.globals.iter().find(|g| !l1_type(&g.ty)) {
        return Err(format!(
            "its global {} is not a Boolean, Integer, Text or temporary record",
            g.name
        ));
    }
    let subscribers = wrapper_events(lowered, &session.engine.events, memo)?;
    code_of_l1_kinds(lowered, memo, &subscribers)?;
    let settled = crate::runtime::written::settled_flags(lowered, memo);
    // How many changed globals of each of L1's three kinds: Checked, Loaded, TempSub.
    let (mut asked, mut inserted, mut temps) = (0, 0, 0);
    for &g in by.globals {
        let decl = &obj.globals[g];
        let raised = by.raised(g);
        let kind = match decl.ty {
            RtType::Boolean if holds_true(session, memo, g) && settled.contains(&(g, true)) => {
                match raised.as_slice() {
                    ["getdatabasetabletriggersetup"] => Some(&mut asked),
                    ["ondatabaseinsert"] => Some(&mut inserted),
                    _ => None,
                }
            }
            RtType::Record(_, true) if raised == ["ondatabaseinsert"] && one_blank_filter(session, memo, g) => {
                Some(&mut temps)
            }
            _ => None,
        };
        let Some(count) = kind else {
            return Err(format!("its global {} changed other than measured", decl.name));
        };
        *count += 1;
    }
    if (asked, inserted, temps) == (1, 1, 1) {
        return Ok(());
    }
    Err(format!(
        "it changed {asked} Boolean in the setup ask, {inserted} Boolean in the insert event and {temps} temporary record, not one of each as measured"
    ))
}

/// Whether global `g` of the memo's single instance holds `true`.
fn holds_true(session: &Session, memo: usize, g: usize) -> bool {
    session
        .single_global(memo, g)
        .is_some_and(|v| matches!(v, Value::Boolean(true)))
}

/// Whether temporary record global `g` holds no row and the view of a fresh one except for exactly one filter: `=''` on one
/// field, in the standard filter group (LmXaLoginCaches.al LoadTemps; the test reads "Company Name: ''" and 0 rows).
fn one_blank_filter(session: &mut Session, memo: usize, g: usize) -> bool {
    let Some(filters) = session.single_temp_filters_only(memo, g) else {
        return false;
    };
    matches!(filters.as_slice(), [FieldFilter { expr: FilterExpr::Eq(Value::Text(t)), group: 0, .. }] if t.is_empty())
}

/// The routines of the memo that subscribe to an event, when each subscribes to an event of a codeunit of the probe's
/// wrapper shape ([`wrapper_of_probe_shape`]): the memo is reached from the `Global Triggers` raises only through such a
/// wrapper, as L1's is (LmXaLoginCaches.al WrapperSetup and WrapperWrite on "LT LM XA Wrapper"). `Err` names the first other.
fn wrapper_events(lowered: &Lowered, events: &Registry, memo: usize) -> Result<BTreeSet<usize>, String> {
    let mut out = BTreeSet::new();
    for ((publisher, event), list) in &events.subscribers {
        for s in list.iter().filter(|s| s.object == memo) {
            if !wrapper_of_probe_shape(lowered, events, *publisher) {
                let name = &lowered.program.objects[memo].routines[s.routine].name;
                return Err(format!(
                    "its procedure {name} subscribes to {event} of an object that is not a wrapper of the measured shape"
                ));
            }
            out.insert(s.routine);
        }
    }
    let unbound = lowered.program.objects[memo]
        .routines
        .iter()
        .enumerate()
        .find(|(r, rt)| {
            !out.contains(r)
                && rt
                    .attributes
                    .iter()
                    .any(|a| a.name.eq_ignore_ascii_case("eventsubscriber"))
        });
    if let Some((_, rt)) = unbound {
        return Err(format!(
            "its procedure {} subscribes to an event that is not a wrapper's of the measured shape",
            rt.name
        ));
    }
    Ok(out)
}

/// Whether codeunit `w` is of the probe's wrapper shape (LmXaMemoShapes.al:237-266, "LT LM XA Wrapper"): no globals; every
/// routine either an event it publishes, with an empty body, or a subscriber of one of the [`WRAPPED_EVENTS`] of Microsoft's
/// GlobalTriggerManagement whose body is one statement: a call of one of its own events, each argument a constant, one of
/// the subscriber's parameters or the `Number` of one of them.
fn wrapper_of_probe_shape(lowered: &Lowered, events: &Registry, w: usize) -> bool {
    let Some(obj) = lowered.program.objects.get(w) else {
        return false;
    };
    let own_event = |r: usize| events.publishers.contains_key(&(w, r));
    let param = |p: &Place, rt: &al_ir::Routine| matches!(p, Place::Local(i) if *i < rt.params);
    let arg_ok = |a: &Arg, rt: &al_ir::Routine| match a {
        Arg::Ref(p) | Arg::Value(Expr::Load(p)) => param(p, rt),
        Arg::Value(Expr::Const(_)) => true,
        Arg::Value(Expr::Builtin {
            id,
            receiver: Some(recv),
            args,
            ..
        }) => {
            id.name() == "RecordRef.Number#1"
                && args.is_empty()
                && matches!(&**recv, Arg::Ref(p) | Arg::Value(Expr::Load(p)) if param(p, rt))
        }
        _ => false,
    };
    obj.is_kind(al_ir::ObjKind::Codeunit)
        && obj.globals.is_empty()
        && obj.routines.iter().enumerate().all(|(r, rt)| {
            if own_event(r) {
                return rt.body.read(|b| b.iter().all(|s| matches!(s, Stmt::Pos(_))));
            }
            let wrapped = events.subscribers.iter().any(|((p, e), list)| {
                list.iter().any(|s| (s.object, s.routine) == (w, r))
                    && WRAPPED_EVENTS.contains(&e.as_str())
                    && global_trigger_management(lowered, *p)
            });
            wrapped
                && rt.body.read(|b| {
                    let mut calls = b.iter().filter(|s| !matches!(s, Stmt::Pos(_)));
                    matches!(
                        (calls.next(), calls.next()),
                        (Some(Stmt::Expr(Expr::Call { object, routine, instance: None, args, try_call: false })), None)
                            if *object == w && own_event(*routine) && args.iter().all(|a| arg_ok(a, rt))
                    )
                })
        })
}

/// Whether object `o` is Microsoft's GlobalTriggerManagement: codeunit 49 of a dependency app published by Microsoft.
fn global_trigger_management(lowered: &Lowered, o: usize) -> bool {
    let objs = &lowered.program.objects;
    objs.get(o)
        .is_some_and(|obj| obj.is_kind(al_ir::ObjKind::Codeunit) && obj.id == Some(49))
        && crate::login_memo::microsoft_dependency(lowered, o)
}

/// The global a place is rooted in: through array elements and fields (as `settled_flags` walks them).
fn global_root(p: &Place) -> Option<usize> {
    let mut p = p;
    while let Place::Index(b, _) | Place::Field(b, _) = p {
        p = b;
    }
    match p {
        Place::Global(g) => Some(*g),
        _ => None,
    }
}

fn arg_place(a: &Arg) -> Option<&Place> {
    match a {
        Arg::Ref(p) | Arg::Value(Expr::Load(p)) => Some(p),
        _ => None,
    }
}

/// What one routine of the memo does, for [`code_of_l1_kinds`].
#[derive(Default)]
struct RoutineFacts {
    /// A statement or expression kind L1's memo has not, by name.
    other: Option<String>,
    /// The first global it changes: assigned (also a field or element of it), the receiver of a builtin other than the
    /// [`READING_BUILTINS`], or handed to a `var` parameter other than the source of `Record.Copy`.
    writes: Option<usize>,
    /// The memo's own procedures it calls.
    calls: Vec<usize>,
}

fn routine_facts(lowered: &Lowered, memo: usize, rt: &al_ir::Routine) -> RoutineFacts {
    let globals = &lowered.program.objects[memo].globals;
    let mut f = RoutineFacts::default();
    rt.body.read(|body| {
        al_ir::types::for_each_stmt(body, &mut |s| match s {
            Stmt::Assign(p, _) => f.writes = f.writes.or(global_root(p)),
            Stmt::Expr(_) | Stmt::If(..) | Stmt::Repeat(..) | Stmt::Exit(_) | Stmt::Pos(_) => {}
            _ => {
                f.other.get_or_insert_with(|| "a statement".into());
            }
        });
        al_ir::types::for_each_expr_post(body, &mut |e| match e {
            Expr::Const(_) | Expr::Load(_) | Expr::Unary(..) | Expr::Binary(..) | Expr::Convert(..) => {}
            Expr::Call {
                object,
                routine,
                instance: None,
                args,
                ..
            } if *object == memo => {
                f.calls.push(*routine);
                for p in args
                    .iter()
                    .filter_map(|a| if let Arg::Ref(p) = a { Some(p) } else { None })
                {
                    f.writes = f.writes.or(global_root(p));
                }
            }
            Expr::Builtin { id, receiver, args, .. } => {
                let name = id.name().split('#').next().unwrap_or("");
                if !MEASURED_BUILTINS.contains(&name) {
                    f.other.get_or_insert_with(|| id.name().to_string());
                    return;
                }
                let recv = receiver.as_deref().and_then(arg_place);
                if !READING_BUILTINS.contains(&name) {
                    f.writes = f.writes.or(recv.and_then(global_root));
                }
                // L1 inserts into and empties only its temporary globals, without RunTrigger (LoadTemps).
                if matches!(name, "Record.Insert" | "Record.DeleteAll") {
                    let temp_global = recv.and_then(|p| match p {
                        Place::Global(g) => globals.get(*g),
                        _ => None,
                    });
                    if !(args.is_empty() && temp_global.is_some_and(|g| matches!(g.ty, RtType::Record(_, true)))) {
                        f.other.get_or_insert_with(|| {
                            format!("{name} on a record that is not a temporary global, or with RunTrigger")
                        });
                    }
                }
                if name != "Record.Copy" {
                    for p in args
                        .iter()
                        .filter_map(|a| if let Arg::Ref(p) = a { Some(p) } else { None })
                    {
                        f.writes = f.writes.or(global_root(p));
                    }
                }
            }
            _ => {
                f.other.get_or_insert_with(|| "a call or expression".into());
            }
        });
    });
    f
}

/// `Ok` when the memo's code holds only the kinds L1's memo holds (LmXaLoginCaches.al, "LT LM XA L1 Memo"): no event publisher; the
/// statements `:=`, a call, `if`, `repeat` and `exit`; the expressions constants, loads, unary and binary operators,
/// conversions, calls of its own procedures on its own instance and the [`MEASURED_BUILTINS`], `Insert` and `DeleteAll`
/// only on a temporary global without RunTrigger; and a global changed only in the wrapper `subscribers` and the procedures
/// that only they reach (L1 changes its globals only in its two subscribers and LoadTemps; its public procedures read).
fn code_of_l1_kinds(lowered: &Lowered, memo: usize, subscribers: &BTreeSet<usize>) -> Result<(), String> {
    let obj = &lowered.program.objects[memo];
    let mut facts = Vec::with_capacity(obj.routines.len());
    for rt in &obj.routines {
        if rt
            .attributes
            .iter()
            .any(|a| al_ir::is_event_publisher_attribute(&a.name))
        {
            return Err(format!("its procedure {} publishes an event", rt.name));
        }
        let f = routine_facts(lowered, memo, rt);
        if let Some(what) = &f.other {
            return Err(format!(
                "its procedure {} uses {what}, which the measured memo does not",
                rt.name
            ));
        }
        facts.push(f);
    }
    // Every routine another object can enter other than through a wrapper subscriber, and what it reaches.
    let mut reached: Vec<usize> = (0..obj.routines.len())
        .filter(|r| !subscribers.contains(r) && (!obj.routines[*r].local || obj.routines[*r].is_trigger))
        .collect();
    let mut seen: BTreeSet<usize> = reached.iter().copied().collect();
    while let Some(r) = reached.pop() {
        if let Some(g) = facts[r].writes {
            return Err(format!(
                "its procedure {} changes a global ({}) and is reached other than through the wrapper's events",
                obj.routines[r].name, obj.globals[g].name
            ));
        }
        for &c in &facts[r].calls {
            if seen.insert(c) {
                reached.push(c);
            }
        }
    }
    Ok(())
}
