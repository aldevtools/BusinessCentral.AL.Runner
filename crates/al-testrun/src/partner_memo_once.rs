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

/// `Ok` when the change `by` of SingleInstance codeunit `memo` is of L1's shape, every fact by equality:
/// - every global of the memo is a Boolean, an Integer, a Text or a temporary record (LmXaLoginCaches.al, the globals of "LT LM XA L1 Memo");
/// - its code is of L1's kinds ([`code_of_l1_kinds`]);
/// - each changed global is a Boolean that holds true now, whose every assignment in the memo is `:= true`, changed inside a
///   raise of exactly one of `GetDatabaseTableTriggerSetup` and `OnDatabaseInsert` (Checked in the ask, WrapperSetup;
///   Loaded on the insert event, WrapperWrite); or a temporary record that holds no row and the view of a fresh one except for exactly one
///   filter, `=''` on one field in the standard filter group, changed inside a raise of `OnDatabaseInsert` only (TempSub,
///   LoadTemps). A changed Integer (LoadTable, WrapperWrite, which holds the asked table) is not of the shape;
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
    code_of_l1_kinds(lowered, memo)?;
    // How many changed globals of each of L1's three kinds: Checked, Loaded, TempSub.
    let (mut asked, mut inserted, mut temps) = (0, 0, 0);
    for &g in by.globals {
        let decl = &obj.globals[g];
        let raised = by.raised(g);
        let kind = match decl.ty {
            RtType::Boolean if holds_true(session, memo, g) && only_set_true(lowered, memo, g) => {
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

/// Whether every assignment of Boolean global `g` in the memo's code is `g := true`, and no call gets it by `var`.
fn only_set_true(lowered: &Lowered, memo: usize, g: usize) -> bool {
    let mut ok = true;
    for r in &lowered.program.objects[memo].routines {
        al_ir::types::for_each_stmt(r.body.stmts(), &mut |s| {
            if let Stmt::Assign(Place::Global(x), e) = s {
                ok &= *x != g || matches!(e, Expr::Const(al_ir::Const::Boolean(true)));
            }
        });
        al_ir::types::for_each_expr_post(r.body.stmts(), &mut |e| {
            let args: &[Arg] = match e {
                Expr::Call { args, .. } | Expr::Builtin { args, .. } => args,
                _ => &[],
            };
            ok &= !args.iter().any(|a| matches!(a, Arg::Ref(Place::Global(x)) if *x == g));
        });
    }
    ok
}

/// `Ok` when the memo's code holds only the kinds L1's memo holds (LmXaLoginCaches.al, "LT LM XA L1 Memo"): no event publisher; the
/// statements `:=`, a call, `if`, `repeat` and `exit`; the expressions constants, loads, unary and binary operators,
/// conversions, calls of its own procedures on its own instance and the [`MEASURED_BUILTINS`]; every assignment of a
/// global in a `local` procedure (L1 writes its globals only in its two subscribers and LoadTemps).
fn code_of_l1_kinds(lowered: &Lowered, memo: usize) -> Result<(), String> {
    let obj = &lowered.program.objects[memo];
    for r in &obj.routines {
        let publisher = r
            .attributes
            .iter()
            .any(|a| al_ir::is_event_publisher_attribute(&a.name));
        if publisher {
            return Err(format!("its procedure {} publishes an event", r.name));
        }
        let mut stmt_ok = true;
        al_ir::types::for_each_stmt(r.body.stmts(), &mut |s| {
            stmt_ok &= match s {
                Stmt::Assign(p, _) => r.local || !matches!(p, Place::Global(_)),
                Stmt::Expr(_) | Stmt::If(..) | Stmt::Repeat(..) | Stmt::Exit(_) | Stmt::Pos(_) => true,
                _ => false,
            };
        });
        let mut other: Option<String> = None;
        al_ir::types::for_each_expr_post(r.body.stmts(), &mut |e| {
            let ok = match e {
                Expr::Const(_) | Expr::Load(_) | Expr::Unary(..) | Expr::Binary(..) | Expr::Convert(..) => true,
                Expr::Call {
                    object, instance: None, ..
                } => *object == memo,
                Expr::Builtin { id, .. } => MEASURED_BUILTINS.contains(&id.name().split('#').next().unwrap_or("")),
                _ => false,
            };
            if !ok && other.is_none() {
                other = Some(match e {
                    Expr::Builtin { id, .. } => id.name().to_string(),
                    _ => "a call or expression".to_string(),
                });
            }
        });
        if !stmt_ok {
            return Err(format!(
                "its procedure {} has a statement the measured memo has not",
                r.name
            ));
        }
        if let Some(what) = other {
            return Err(format!(
                "its procedure {} uses {what}, which the measured memo does not",
                r.name
            ));
        }
    }
    Ok(())
}
