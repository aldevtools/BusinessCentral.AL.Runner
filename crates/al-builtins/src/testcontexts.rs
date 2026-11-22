//! `TestHandlerContext` and `DataSourceContext` (BC 29, runtime 18.0) on a variable the test framework did not hand
//! over: a fresh variable of the type (corpus permissions/ACContexts.al). A context the runner hands to an `ITestDataSource`
//! callback (`LibraryValue::HandedContext`, ADR-059) has its members refused: on BC a call of one there ends the session
//! (al-language-testlibraries/test-framework/data-driven-tests-and-handlers/BiDataDriven.al:29-34, corpus PR 861).

use crate::describe::Describe;
use crate::native::{Ctx, NativeFn, Signal};
use crate::signature::Signature;
use crate::value::{HandlerContext, LibraryValue, Value};

type Out = Result<Value, Signal>;

/// What `read` gives from the context the test runner handed to an ITestHandler callback, or `None` for another receiver.
fn handed<T>(r: &mut Option<&mut Value>, read: impl FnOnce(&mut HandlerContext) -> T) -> Option<T> {
    match r.as_deref_mut() {
        Some(Value::Library(_, cell)) => match &mut *cell.borrow_mut() {
            LibraryValue::HandlerContext(c) => Some(read(c)),
            _ => None,
        },
        _ => None,
    }
}

/// Refuses a receiver that is not a fresh variable of the type.
fn fresh(r: Option<&mut Value>, ty: &str) -> Result<(), Signal> {
    match r {
        Some(Value::Library(_, cell)) if matches!(&*cell.borrow(), LibraryValue::Empty) => Ok(()),
        other => Err(Signal::unsupported(format!(
            "{ty} method on a value the test framework handed over ({})",
            other.map(|v| v.describe()).unwrap_or_else(|| "no receiver".into())
        ))),
    }
}

// ACContexts.al:47 (codeunit 77541): `Format(Ctx.CodeunitId())` of a fresh variable is '0'.
fn th_codeunit_id(_: &mut dyn Ctx, mut r: Option<&mut Value>, _: &mut [Value]) -> Out {
    // BiTestHandler.al:79, :125: the id of the test codeunit in every callback.
    if let Some(id) = handed(&mut r, |c| c.codeunit_id) {
        return Ok(Value::Integer(id));
    }
    fresh(r, "TestHandlerContext")?;
    Ok(Value::Integer(0))
}

// BiTestHandler.al:79, :125 and BiTestHandlerBehaviorNs.al:26: the test codeunit's name (namespace first) in every callback.
fn th_codeunit_name(_: &mut dyn Ctx, mut r: Option<&mut Value>, _: &mut [Value]) -> Out {
    if let Some(n) = handed(&mut r, |c| c.codeunit_name.clone()) {
        return Ok(Value::Text(n.into()));
    }
    th_empty_text_of(r)
}

// BiTestHandler.al:125, BiTestHandlerDefault.al:286: empty in the codeunit callbacks, the procedure's name in the others.
fn th_procedure_name(_: &mut dyn Ctx, mut r: Option<&mut Value>, _: &mut [Value]) -> Out {
    if let Some(n) = handed(&mut r, |c| c.procedure.clone()) {
        return Ok(Value::Text(n.into()));
    }
    th_empty_text_of(r)
}

// BiTestHandlerDefault.al:286: the case identifier in the case callbacks, empty in the others.
fn th_test_case_name(_: &mut dyn Ctx, mut r: Option<&mut Value>, _: &mut [Value]) -> Out {
    if let Some(n) = handed(&mut r, |c| c.case.clone()) {
        return Ok(Value::Text(n.into()));
    }
    th_empty_text_of(r)
}

// ACContexts.al:68 (CodeunitName, 77543), :89 (ProcedureName, 77544), :110 (TestCaseName, 77545): empty text.
fn th_empty_text_of(r: Option<&mut Value>) -> Out {
    fresh(r, "TestHandlerContext")?;
    Ok(Value::Text(String::new().into()))
}

// ACContexts.al:131 (codeunit 77546): `Format(Ctx.Success())` of a fresh variable is 'No'.
fn th_success(_: &mut dyn Ctx, mut r: Option<&mut Value>, _: &mut [Value]) -> Out {
    // BiTestHandlerSuccess.al:116 (codeunit 81599): No in every Before callback, Yes in the After callbacks of a passing
    // procedure or case (also BiTestHandler.al:125, BiTestHandlerDefault.al:286).
    if let Some((b, assumed)) = handed(&mut r, |c| (c.success, c.success_assumed)) {
        if assumed {
            crate::assumptions::assume(crate::assumptions::Assumption::TestHandlerDefaultBeforeSuccess);
        }
        return Ok(Value::Boolean(b));
    }
    fresh(r, "TestHandlerContext")?;
    Ok(Value::Boolean(false))
}

// ACContexts.al:151-155, :167 (codeunit 77542): Skip on a fresh variable returns without an error. Nothing else of
// its effect outside a handler is observable from the probe.
fn th_skip(_: &mut dyn Ctx, mut r: Option<&mut Value>, args: &mut [Value]) -> Out {
    // In a callback: the runner reads the reason when the callback returns (al-testrun `runtime/test_handlers.rs`).
    let reason = args
        .first()
        .and_then(crate::native::text_of)
        .map(|t| t.to_string())
        .unwrap_or_default();
    if handed(&mut r, |c| c.skip = Some(reason)).is_some() {
        return Ok(Value::Null);
    }
    fresh(r, "TestHandlerContext")?;
    Ok(Value::Null)
}

// ACContexts.al:22-25 (codeunit 77506): CodeunitId of a fresh DataSourceContext raises this error, code TypeConversion.
fn ds_codeunit_id(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    fresh(r, "DataSourceContext")?;
    Err(Signal::Error(Box::new(crate::native::AlError {
        message: "The expression System.Func`1[System.Int32] cannot be type-converted to a Microsoft.Dynamics.Nav.Runtime.NavValue value."
            .into(),
        code: "TypeConversion".into(),
        call_stack: String::new(),
        ..Default::default()
    })))
}

/// `DataSourceContext.AppId` has no implementation: on a fresh variable it ends the test session on BC 29 and is not
/// probed (ACContexts.al:8), so a call is refused.
pub fn native_for(sig: &Signature) -> Option<NativeFn> {
    Some(match (sig.receiver.as_str(), sig.name.as_str()) {
        ("TestHandlerContext", "CodeunitId") => th_codeunit_id,
        ("TestHandlerContext", "CodeunitName") => th_codeunit_name,
        ("TestHandlerContext", "ProcedureName") => th_procedure_name,
        ("TestHandlerContext", "TestCaseName") => th_test_case_name,
        ("TestHandlerContext", "Success") => th_success,
        ("TestHandlerContext", "Skip") => th_skip,
        ("DataSourceContext", "CodeunitId") => ds_codeunit_id,
        _ => return None,
    })
}
