//! ITestHandler callbacks around a test codeunit (BC 29, runtime 18.0). Lowering lists the handler codeunits of a test
//! codeunit in call order (`al_ir::TEST_HANDLER_CODEUNITS`, al-sema `lower/test_handlers.rs`): DefaultTestHandler handlers
//! first, then those of its `TestHandlers` property.
//!
//! Pins (corpus al-language-testlibraries/test-framework/data-driven-tests-and-handlers/, all observed on BC 29):
//! * BiTestHandlerDefault.al:15-16: OnBeforeTestCodeunitRun, OnRun, then per procedure OnBeforeTestProcedureRun, per case of a
//!   data-driven procedure OnBeforeTestCaseRun, the body and OnAfterTestCaseRun, then OnAfterTestProcedureRun.
//! * BiTestHandlerDefault.al:17: default handlers run before the TestHandlers handlers in every callback, After callbacks too.
//! * BiTestHandlerDefault.al:18-19: one instance of a handler codeunit for the run of one test codeunit.
//! * BiTestHandlerDefault.al:20, :287: what a handler sets in OnBeforeTestProcedureRun is visible to the body.
//! * BiTestHandler.al:7-8, :124-125: `Skip` in OnBeforeTestProcedureRun skips the body and OnAfterTestProcedureRun.
//! * BiTestHandlerBehavior.al:204-219, :353-380, :382-420, :443-465: an error raised in OnBeforeTestCodeunitRun,
//!   OnBeforeTestProcedureRun or OnAfterTestProcedureRun of a TestHandlers handler does not change the test's result and the
//!   next handler still runs; `Skip` in OnBeforeTestCodeunitRun or OnAfterTestProcedureRun has no effect; a second handler
//!   still runs after the first skipped the procedure.
//!
//! Not pinned, refused: an error or a `Skip` in a default handler, an error or a `Skip` in the case callbacks and in
//! OnAfterTestCodeunitRun, a callback that writes to the database.
use super::*;
use al_builtins::value::{HandlerContext, Instance, LibraryValue, Value};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Callback {
    BeforeCodeunit,
    AfterCodeunit,
    BeforeProcedure,
    AfterProcedure,
    BeforeCase,
    AfterCase,
}

impl Callback {
    fn name(self) -> &'static str {
        match self {
            Callback::BeforeCodeunit => "OnBeforeTestCodeunitRun",
            Callback::AfterCodeunit => "OnAfterTestCodeunitRun",
            Callback::BeforeProcedure => "OnBeforeTestProcedureRun",
            Callback::AfterProcedure => "OnAfterTestProcedureRun",
            Callback::BeforeCase => "OnBeforeTestCaseRun",
            Callback::AfterCase => "OnAfterTestCaseRun",
        }
    }
}

struct Handler {
    object: usize,
    default: bool,
    instance: Rc<RefCell<Instance>>,
}

/// The handlers of one test codeunit run, each with its one instance.
pub(crate) struct Handlers {
    list: Vec<Handler>,
    codeunit_id: i32,
    codeunit_name: String,
}

/// The handler codeunits of test codeunit `object` as (object, is a default handler), in call order, or why they cannot
/// be run. Empty when it has none.
pub(crate) fn declared(lowered: &Lowered, object: usize) -> Result<Vec<(usize, bool)>, String> {
    let obj = &lowered.program.objects[object];
    let prop = |name: &str| obj.properties.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str());
    if let Some(why) = prop(al_ir::TEST_HANDLERS_REFUSED) {
        return Err(format!("test handlers (ITestHandler callbacks): {why}"));
    }
    let Some(list) = prop(al_ir::TEST_HANDLER_CODEUNITS) else {
        return Ok(Vec::new());
    };
    let heads = lowered.program.objects.heads();
    let mut out = Vec::new();
    for entry in list.split(',') {
        let (default, id) = (entry.starts_with('D'), entry[1..].parse::<u32>().ok());
        let at = heads
            .iter()
            .position(|o| o.is_kind(al_ir::ObjKind::Codeunit) && o.id.is_some() && o.id == id)
            .ok_or_else(|| format!("the test handler codeunit {entry} is not in the compiled apps"))?;
        if lowered.program.objects[at].uncompiled {
            return Err(format!("the test handler codeunit {} is not compiled", &entry[1..]));
        }
        out.push((at, default));
    }
    Ok(out)
}

/// Why a test handler callback that test codeunit `object` runs cannot run (a routine it reaches is refused).
pub(crate) fn blocked(lowered: &Lowered, handlers: &[(usize, bool)]) -> Option<String> {
    handlers.iter().find_map(|&(h, _)| {
        let o = &lowered.program.objects[h];
        o.routines
            .iter()
            .enumerate()
            .filter(|(_, r)| !r.is_trigger)
            .find_map(|(r, _)| blocked_site(lowered, h, r))
            .map(|b| format!("test handler codeunit {}: {}", o.name, b.text))
    })
}

impl Handlers {
    /// One instance per handler for the run of test codeunit `object` (BiTestHandlerDefault.al:18-19).
    pub(crate) fn start(
        session: &mut Session,
        lowered: &Lowered,
        object: usize,
        declared: &[(usize, bool)],
    ) -> Handlers {
        let obj = &lowered.program.objects[object];
        Handlers {
            list: declared
                .iter()
                .map(|&(o, default)| Handler {
                    object: o,
                    default,
                    instance: session.codeunit_instance(o),
                })
                .collect(),
            codeunit_id: obj.id.unwrap_or(0) as i32,
            codeunit_name: match &obj.namespace {
                Some(ns) => format!("{ns}.{}", obj.name),
                None => obj.name.clone(),
            },
        }
    }

    /// The handlers of test codeunit `object` (none for a scratch run, `execute`), after OnBeforeTestCodeunitRun.
    /// `run_codeunit_core` refused a codeunit whose handlers cannot run.
    pub(crate) fn begin(
        session: &mut Session,
        lowered: &Lowered,
        object: usize,
        execute: bool,
    ) -> Result<Handlers, String> {
        let list = if execute {
            Vec::new()
        } else {
            declared(lowered, object).unwrap_or_default()
        };
        let th = Handlers::start(session, lowered, object, &list);
        th.call(session, lowered, Callback::BeforeCodeunit, "", "", None)?;
        Ok(th)
    }

    /// OnAfterTestCodeunitRun, after the last test, where no test of the codeunit sees it (BiTestHandler.al:11-12): its
    /// result changes no row.
    /// `Err(reason)`: the callback raised an error, skipped or wrote to the database, which is not pinned; the caller
    /// reports the passing tests as not run.
    pub(crate) fn end(
        &self,
        session: &mut Session,
        lowered: &Lowered,
        out: &[(String, Outcome)],
    ) -> Result<(), String> {
        let passed = out.iter().all(|(_, o)| *o == Outcome::Pass);
        self.call(session, lowered, Callback::AfterCodeunit, "", "", Some(passed))
            .map(|_| ())
    }

    /// [`Handlers::end`] and its refusal: the passing tests of the codeunit are reported as not run (not pinned).
    pub(crate) fn end_codeunit(&self, session: &mut Session, lowered: &Lowered, out: &mut [(String, Outcome)]) {
        if let Err(why) = self.end(session, lowered, out) {
            for (_, o) in out.iter_mut().filter(|(_, o)| *o == Outcome::Pass) {
                *o = Outcome::NotRun(why.clone());
            }
        }
    }

    /// Marks the unprobed assumption of OnAfterTestCodeunitRun when a handler implements it (a test of the codeunit
    /// then relies on its effects staying invisible to, or visible to, later codeunits: not pinned).
    pub(crate) fn mark_after_codeunit(&self, lowered: &Lowered) {
        let declares = self.list.iter().any(|h| {
            lowered.program.objects[h.object]
                .routines
                .iter()
                .any(|r| !r.is_trigger && r.name.eq_ignore_ascii_case(Callback::AfterCodeunit.name()))
        });
        if declares {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::TestHandlerAfterCodeunitEffects);
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// Runs callback `cb` of every handler in order. `Ok(Some(reason))`: a handler skipped the procedure.
    pub(crate) fn call(
        &self,
        session: &mut Session,
        lowered: &Lowered,
        cb: Callback,
        procedure: &str,
        case: &str,
        success: Option<bool>,
    ) -> Result<Option<String>, String> {
        let mut skipped = None;
        for h in &self.list {
            let obj = &lowered.program.objects[h.object];
            // A callback the codeunit does not declare runs ITestHandler's empty body (System.app ITestHandler.Interface.al).
            let Some(r) = obj
                .routines
                .iter()
                .position(|r| !r.is_trigger && r.name.eq_ignore_ascii_case(cb.name()))
            else {
                continue;
            };
            let ctx = match obj.routines[r].locals.first().map(|l| Value::default_for(&l.ty)) {
                Some(Value::Library(kind, cell)) => {
                    *cell.borrow_mut() = LibraryValue::HandlerContext(Box::new(HandlerContext {
                        codeunit_id: self.codeunit_id,
                        codeunit_name: self.codeunit_name.clone(),
                        procedure: procedure.to_string(),
                        case: case.to_string(),
                        // BiTestHandlerSuccess.al:116: Success() is false in every Before callback.
                        success: success.unwrap_or(false),
                        success_assumed: success.is_none() && h.default,
                        skip: None,
                    }));
                    Value::Library(kind, cell)
                }
                _ => {
                    return Err(format!(
                        "{} of {} has no TestHandlerContext parameter",
                        cb.name(),
                        obj.name
                    ));
                }
            };
            let cell = match &ctx {
                Value::Library(_, cell) => cell.clone(),
                _ => unreachable!(),
            };
            let changes = session.store_mut().change_counter();
            let called = session.call(h.object, r, h.instance.clone(), vec![ctx]);
            if session.store_mut().change_counter() != changes {
                return Err(format!(
                    "{} of test handler {} wrote to the database: whether the test framework keeps it is not pinned",
                    cb.name(),
                    obj.name
                ));
            }
            let whose = if h.default {
                "a default test handler"
            } else {
                "a TestHandlers handler"
            };
            match called {
                Ok(_) => {}
                // BiTestHandlerBehavior.al:206-213 with the logs at :378 and :418: the error does not reach the test.
                Err(Signal::Error(_))
                    if !h.default
                        && matches!(
                            cb,
                            Callback::BeforeCodeunit | Callback::BeforeProcedure | Callback::AfterProcedure
                        ) => {}
                Err(Signal::Error(e)) => {
                    return Err(format!(
                        "an error in {} of {whose} ({}): what the test framework does then is not pinned",
                        cb.name(),
                        e.message
                    ));
                }
                Err(Signal::Unsupported(why)) => return Err(why.into_reason()),
                Err(other) => {
                    return Err(format!(
                        "{} of test handler {}: {}",
                        cb.name(),
                        obj.name,
                        other.describe()
                    ));
                }
            }
            let skip = match &*cell.borrow() {
                LibraryValue::HandlerContext(c) => c.skip.clone(),
                _ => None,
            };
            if let Some(reason) = skip {
                match cb {
                    _ if h.default => {
                        return Err(format!("Skip in {} of a default test handler (not pinned)", cb.name()));
                    }
                    // BiTestHandler.al:55-56, :124; BiTestHandlerBehavior.al:212-214, :418 (the next handler still runs).
                    Callback::BeforeProcedure => skipped = skipped.or(Some(reason)),
                    // BiTestHandlerBehavior.al:216-218, :463: no effect.
                    Callback::BeforeCodeunit | Callback::AfterProcedure => {}
                    _ => return Err(format!("Skip in {} (not pinned)", cb.name())),
                }
            }
        }
        Ok(skipped)
    }
}

/// The result of a procedure a handler skipped. BC reports it as skipped (BiTestHandler.al:124, the JUnit file of the
/// corpus CI); the runner has no skipped result and reports it as not run, with the reason.
fn skipped_reason(reason: &str) -> String {
    format!(
        "skipped by a test handler (TestHandlerContext.Skip in OnBeforeTestProcedureRun: {reason}); BC reports the test as skipped"
    )
}

/// Why the test handlers of test codeunit `object` keep its tests from running (`None`: they run, or it has none).
pub(crate) fn refusal(lowered: &Lowered, object: usize) -> Option<String> {
    match declared(lowered, object) {
        Err(why) => Some(why),
        Ok(list) => blocked(lowered, &list),
    }
}

type Called = Result<(Value, Vec<Value>), Signal>;

/// Runs `body` of `procedure` between the callbacks of `handlers` (BiTestHandlerDefault.al:15-16). `case` is the case
/// identifier of a data-driven procedure's case, whether it is the first and whether it is the last case. `all_ok` carries whether every case
/// of a data-driven procedure so far succeeded, for its OnAfterTestProcedureRun. A refused callback refuses the test; a
/// skipped procedure is a refusal that says so ([`skipped_reason`]).
pub(crate) fn around(
    session: &mut Session,
    lowered: &Lowered,
    handlers: &Handlers,
    procedure: &str,
    case: Option<(&str, bool, bool)>,
    all_ok: &mut bool,
    body: impl FnOnce(&mut Session) -> Called,
) -> Called {
    if handlers.is_empty() {
        return body(session);
    }
    let refuse = |why: String| Err(Signal::unsupported(why));
    handlers.mark_after_codeunit(lowered);
    match case {
        None => {
            let name = procedure;
            match handlers.call(session, lowered, Callback::BeforeProcedure, name, "", None) {
                Err(why) => return refuse(why),
                Ok(Some(reason)) => return refuse(skipped_reason(&reason)),
                Ok(None) => {}
            }
            let result = body(session);
            let ok = result.is_ok();
            if let Err(why) = handlers.call(session, lowered, Callback::AfterProcedure, name, "", Some(ok)) {
                return refuse(why);
            }
            result
        }
        Some((id, first, last)) => {
            if first {
                *all_ok = true;
                match handlers.call(session, lowered, Callback::BeforeProcedure, procedure, "", None) {
                    Err(why) => return refuse(why),
                    Ok(Some(_)) => return refuse("Skip of a data-driven procedure (not pinned)".into()),
                    Ok(None) => {}
                }
            }
            if let Err(why) = handlers.call(session, lowered, Callback::BeforeCase, procedure, id, None) {
                return refuse(why);
            }
            let result = body(session);
            let ok = result.is_ok();
            *all_ok &= ok;
            if let Err(why) = handlers.call(session, lowered, Callback::AfterCase, procedure, id, Some(ok)) {
                return refuse(why);
            }
            if last {
                if let Err(why) =
                    handlers.call(session, lowered, Callback::AfterProcedure, procedure, "", Some(*all_ok))
                {
                    return refuse(why);
                }
            }
            result
        }
    }
}
