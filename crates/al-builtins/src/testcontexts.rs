//! `TestHandlerContext` and `DataSourceContext` (BC 29, runtime 18.0) on a variable the test framework did not hand
//! over: a fresh variable of the type (corpus lt-ac/ACContexts.al). A context handed to an `ITestHandler` or an
//! `ITestDataSource` callback is not built by the runtime yet, so any other receiver is refused.

use crate::describe::Describe;
use crate::native::{Ctx, NativeFn, Signal};
use crate::signature::Signature;
use crate::value::{LibraryValue, Value};

type Out = Result<Value, Signal>;

/// Refuses a receiver that is not a fresh variable of the type.
fn fresh(r: Option<&mut Value>, ty: &str) -> Result<(), Signal> {
    match r {
        Some(Value::Library(_, cell)) if matches!(&*cell.borrow(), LibraryValue::Empty) => Ok(()),
        other => Err(Signal::Unsupported(format!(
            "{ty} method on a value the test framework handed over ({})",
            other.map(|v| v.describe()).unwrap_or_else(|| "no receiver".into())
        ))),
    }
}

// ACContexts.al:47 (codeunit 77541): `Format(Ctx.CodeunitId())` of a fresh variable is '0'.
fn th_codeunit_id(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    fresh(r, "TestHandlerContext")?;
    Ok(Value::Integer(0))
}

// ACContexts.al:68 (CodeunitName, 77543), :89 (ProcedureName, 77544), :110 (TestCaseName, 77545): empty text.
fn th_empty_text(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    fresh(r, "TestHandlerContext")?;
    Ok(Value::Text(String::new().into()))
}

// ACContexts.al:131 (codeunit 77546): `Format(Ctx.Success())` of a fresh variable is 'No'.
fn th_success(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    fresh(r, "TestHandlerContext")?;
    Ok(Value::Boolean(false))
}

// ACContexts.al:151-155, :167 (codeunit 77542): Skip on a fresh variable returns without an error. Nothing else of
// its effect outside a handler is observable from the probe.
fn th_skip(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
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
        ("TestHandlerContext", "CodeunitName" | "ProcedureName" | "TestCaseName") => th_empty_text,
        ("TestHandlerContext", "Success") => th_success,
        ("TestHandlerContext", "Skip") => th_skip,
        ("DataSourceContext", "CodeunitId") => ds_codeunit_id,
        _ => return None,
    })
}
