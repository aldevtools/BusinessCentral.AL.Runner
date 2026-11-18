//! A subscriber that can change how BC shows a value of a pinned AutoFormatType (al-runner issue 8011). BC resolves an AutoFormat
//! through the AutoFormatTranslate event of `UI Helper Triggers`; the System Application's subscriber raises OnResolveAutoFormat and
//! OnAfterResolveAutoFormat of `Auto Format` from there. The runtime raises none of them and formats the pinned types natively. A
//! subscriber outside Microsoft's apps (Microsoft's app: a dependency app whose publisher is "Microsoft"; the app under test never
//! is one) that writes the event's result could change the text, so where one is active the format is refused, by what the test
//! can see (`Unraised::seen_by`):
//!
//! 1. no writer: the format runs (the corpus subscribers events/platform-triggers/P114GlobalTriggerEvents.al:104-109 and
//!    P118SysEvents.al:163-168 log and never write Translation);
//! 2. a writer the test can see: refused, except the cells events/platform-triggers/PtObserve.al:482-497 and :558-570 measure (the
//!    caller proves them, `measured`): there BC raised no event and the value stayed as without the subscriber;
//! 3. a writer the test cannot see that holds no SingleInstance codeunit: refused (the reporter's shape);
//! 4. a writer the test cannot see that holds a SingleInstance codeunit: the format runs as before this check (the corpus fixture
//!    events/platform-triggers/PtFixtures.al:148-158 writes only in a log mode its own tests set). A known hole until the event is
//!    raised (docs/probes-needed.md, "AutoFormatType of a decimal control").
//!
//! Type 0 and no type have no check: Format of a plain field raises nothing (P114GlobalTriggerEvents.al:341-347).

use crate::Session;
use crate::events::Unraised;
use al_ir::{Arg, Expr, Place, Stmt};

/// The events whose result decides the format: the (lower-case codeunit, lower-case event) key of `Registry::unraised`, and the
/// event's name for the refusal text. Built once, so a check makes no key.
static EVENTS: std::sync::LazyLock<[((String, String), &str); 3]> = std::sync::LazyLock::new(|| {
    let key = |c: &str, e: &str| (c.to_string(), e.to_string());
    [
        (
            key("ui helper triggers", "autoformattranslate"),
            "AutoFormatTranslate of UI Helper Triggers",
        ),
        (
            key("auto format", "onresolveautoformat"),
            "OnResolveAutoFormat of Auto Format",
        ),
        (
            key("auto format", "onafterresolveautoformat"),
            "OnAfterResolveAutoFormat of Auto Format",
        ),
    ]
});

impl Session {
    /// Why a format of AutoFormatType `kind` is refused for a subscriber that can write its result, or `None` (see the module).
    /// `measured`: the caller proved one of the two measured cells of tier 2.
    pub(crate) fn auto_format_writer_refusal(&self, kind: &str, measured: bool) -> Option<String> {
        if !al_builtins::pagefmt::resolved_auto_format_type(kind) {
            return None;
        }
        let program = &self.engine.program;
        let test = self.test_object.and_then(|t| program.objects.get(t));
        for (key, name) in EVENTS.iter() {
            let Some(subs) = self.engine.events.unraised.get(key) else {
                continue;
            };
            for u in subs.iter().filter(|u| self.active_foreign_writer(u)) {
                let refused = if u.seen_by(test, self.test_object) {
                    !measured
                } else {
                    u.witnesses.len() == 1
                };
                if refused {
                    return Some(format!(
                        "AutoFormatType {} while {} subscribes to {name} and can set its result (BC resolves the format through that event, which the runtime does not raise)",
                        kind.trim(),
                        u.subscriber
                    ));
                }
            }
        }
        None
    }

    /// Subscriber `u` is outside Microsoft's apps, is static or bound now, and may write a `var` parameter (every `var` parameter
    /// of the three events is a result: Translation, Result, Resolved).
    ///
    /// Microsoft's apps: a dependency app whose publisher is "Microsoft". The app under test is never one, whatever publisher it
    /// names (review of PR 8015: a test app published as "Microsoft" runs its subscriber in BC). A known hole: a partner
    /// dependency package that names "Microsoft" as its publisher counts as Microsoft's (docs/probes-needed.md, "AutoFormatType
    /// of a decimal control").
    fn active_foreign_writer(&self, u: &Unraised) -> bool {
        let program = &self.engine.program;
        // The head only: a subscriber of Microsoft's apps is not made to be read (ADR-054).
        let microsoft = program
            .objects
            .heads()
            .get(u.object)
            .and_then(|h| program.apps.get(h.app as usize))
            .is_some_and(|a| a.dependency && a.publisher == "Microsoft");
        if microsoft || (u.manual && !self.is_bound_object(u.object)) {
            return false;
        }
        !self.reads_only(u.object, u.routine)
    }

    /// Whether routine `routine` of `object` is known to write no `var` parameter: it can run (lowering blocked nothing in it),
    /// no statement of it was lowered to a refusal, a failure or a stub (the statement lowering could not read may be the
    /// write), and it neither assigns a `var` parameter nor hands one on by `var`.
    fn reads_only(&self, object: usize, routine: usize) -> bool {
        let blocked = self
            .engine
            .blocked
            .get(object)
            .and_then(|o| o.get(routine))
            .is_some_and(|b| b.is_some());
        let Some(r) = self
            .engine
            .program
            .objects
            .get(object)
            .and_then(|o| o.routines.get(routine))
        else {
            return false;
        };
        !blocked && !lowered_away(r.body.stmts()) && !writes_var_parameter(r)
    }
}

/// Whether `body` holds an expression lowering put where it could not lower the source: a refusal, a failure, a `--tdd` stub.
fn lowered_away(body: &[Stmt]) -> bool {
    let mut found = false;
    al_ir::types::for_each_expr_post(body, &mut |e| {
        found |= matches!(
            e,
            Expr::Refused(_) | Expr::Fail { .. } | Expr::TddStub { .. } | Expr::FitOrRefuse { .. }
        );
    });
    found
}

/// Whether routine `r` assigns a `var` parameter (also an element of it, or as a loop variable) or hands it on by `var`.
fn writes_var_parameter(r: &al_ir::Routine) -> bool {
    (0..r.params)
        .filter(|&i| r.by_var.get(i).copied().unwrap_or(false))
        .any(|slot| {
            let is_slot = |p: &Place| root_local(p) == Some(slot);
            let body = r.body.stmts();
            !crate::effects::stmts_keep_local(body, &is_slot) || passes_by_var(body, &is_slot)
        })
}

/// The local a place is rooted in.
fn root_local(p: &Place) -> Option<usize> {
    match p {
        Place::Local(n) => Some(*n),
        Place::Index(base, _) | Place::Field(base, _) | Place::RecGlobal(base, ..) => root_local(base),
        Place::Global(_) | Place::BaseGlobal(_) | Place::ExtGlobal(..) => None,
    }
}

/// Whether an argument or a receiver of a call in `body` hands a place `is_slot` names on by `var`.
fn passes_by_var(body: &[Stmt], is_slot: &dyn Fn(&Place) -> bool) -> bool {
    let by_var = |a: &Arg| matches!(a, Arg::Ref(p) if is_slot(p));
    let mut found = false;
    al_ir::types::for_each_expr_post(body, &mut |e| match e {
        Expr::Call { args, .. } | Expr::InterfaceCall { args, .. } | Expr::DotNet { args, .. } => {
            found |= args.iter().any(by_var);
        }
        Expr::Builtin { receiver, args, .. } => {
            found |= receiver.as_deref().is_some_and(by_var) || args.iter().any(by_var);
        }
        _ => {}
    });
    found
}
