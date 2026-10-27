//! The .NET exception `GetLastErrorObject` answers, and the `System.Type` its `GetType()` gives. The runtime
//! provides them itself (ADR-014 section 1, ADR-038), so a run needs no .NET host.
//!
//! Pinned by tests/al-language-onprem-dotnet/lt-dotnet-interop/GetLastErrorObject.al (BC 27, 28, 29):
//! - `Message` is the text `GetLastErrorText` answers (:80), `InnerException` is null for a dialog and a
//!   record-not-found exception and not null for a `NavALException` (:83, :155, :251, :275);
//! - `GetType().FullName` is the exception class (:81-82);
//! - `Format(Exc)` is `Exc.ToString()` (:85); its first line is `<class>: <message>` and the text has `   at `
//!   stack frames (:86-87).
//!
//! Not pinned, so refused (a registry type refuses every member it does not list): `Name`, `Equals`,
//! `StackTrace`, `Source`, `Data`, `HResult`, the members of the inner exception, `GetBaseException`.
//! The stack frames are .NET's, which AL code can read: only the shapes the corpus pins are written (see `exception_text`);
//! any other trace makes the text unreadable.

use super::registry::{MemberKind, Pin, RegistryMember, RegistryType};
use crate::Session;
use al_builtins::dotnet_sigs::{
    AL_EXCEPTION, AL_INNER_EXCEPTION, DIVIDE_BY_ZERO_EXCEPTION, NCL_DIALOG_EXCEPTION, NCL_EVALUATE_EXCEPTION,
    NULL_EXCEPTION, RECORD_NOT_FOUND_EXCEPTION, RUNTIME_TYPE, TEST_FIELD_EXCEPTION,
};
use al_builtins::native::{EXC_CALL_STACK, EXC_MESSAGE, Signal, UNTRACKED_CALL_STACK};
use al_builtins::value::{DotNetKind, DotNetObject, Value};
use std::cell::RefCell;
use std::rc::Rc;

const PIN: &str = "tests/al-language-onprem-dotnet/lt-dotnet-interop/GetLastErrorObject.al:79-87";

fn pin() -> Pin {
    Pin::Corpus(PIN.into())
}

fn unsupported<T>(why: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(why.into()))
}

fn parts(this: Option<&Rc<DotNetObject>>) -> Result<(&str, Value, Value), Signal> {
    match this.map(|o| &o.kind) {
        Some(DotNetKind::Internal { type_name, state }) => {
            let s = state.borrow();
            Ok((type_name.as_str(), s[EXC_MESSAGE].clone(), s[EXC_CALL_STACK].clone()))
        }
        _ => unsupported("DotNet exception member on something that is not a last-error exception"),
    }
}

/// Whether `type_name` is one of the exception classes `GetLastErrorObject` answers.
pub(crate) fn is_exception(type_name: &str) -> bool {
    [
        NCL_DIALOG_EXCEPTION,
        RECORD_NOT_FOUND_EXCEPTION,
        AL_EXCEPTION,
        TEST_FIELD_EXCEPTION,
        NCL_EVALUATE_EXCEPTION,
        DIVIDE_BY_ZERO_EXCEPTION,
    ]
    .contains(&type_name)
}

/// `GetBaseException()`: the exception itself, except for a `NavALException`, whose base is the .NET exception it wraps. The only
/// wrapped one pinned is a division by zero, `System.DivideByZeroException` with the same message
/// (GetLastErrorObjectKinds.al:Kind_DivisionByZero, and the other kinds `baseTypeFullName`).
fn get_base_exception(_: &mut Session, this: Option<&Rc<DotNetObject>>, _: &[Value]) -> Result<Value, Signal> {
    let (type_name, msg, stack) = parts(this)?;
    if type_name != AL_EXCEPTION {
        return match this {
            Some(o) => Ok(Value::DotNet(o.clone())),
            None => unsupported("GetBaseException without an object"),
        };
    }
    Ok(Value::DotNet(Rc::new(DotNetObject::new(DotNetKind::Internal {
        type_name: DIVIDE_BY_ZERO_EXCEPTION.into(),
        state: RefCell::new(vec![msg, stack]),
    }))))
}

/// `Equals` on an exception is the identity of the object (GetLastErrorObjectKinds.al:Type_Equals_SameKinds `dialog_dialog_sameObject`
/// No for two errors of one kind, `dialog_self_sameObject` Yes), and on a `System.Type` the identity of the type, which is its name
/// (`dialog_dialog` Yes, `dialog_notFound` No).
fn equals(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    let (Some(this), Some(Value::DotNet(other))) = (this, args.first()) else {
        return unsupported("Equals of a DotNet exception with something that is no DotNet object");
    };
    if Rc::ptr_eq(this, other) {
        return Ok(Value::Boolean(true));
    }
    Ok(Value::Boolean(match (&this.kind, &other.kind) {
        (
            DotNetKind::Internal {
                type_name: a,
                state: sa,
            },
            DotNetKind::Internal {
                type_name: b,
                state: sb,
            },
        ) if a == RUNTIME_TYPE && b == RUNTIME_TYPE => {
            matches!((&sa.borrow()[0], &sb.borrow()[0]), (Value::Text(x), Value::Text(y)) if x == y)
        }
        _ => false,
    }))
}

fn message(_: &mut Session, this: Option<&Rc<DotNetObject>>, _: &[Value]) -> Result<Value, Signal> {
    Ok(parts(this)?.1)
}

fn inner_exception(_: &mut Session, this: Option<&Rc<DotNetObject>>, _: &[Value]) -> Result<Value, Signal> {
    let (type_name, ..) = parts(this)?;
    if type_name != AL_EXCEPTION {
        return Ok(Value::Null);
    }
    Ok(Value::DotNet(Rc::new(DotNetObject::new(DotNetKind::Internal {
        type_name: AL_INNER_EXCEPTION.into(),
        state: RefCell::new(Vec::new()),
    }))))
}

fn get_type(_: &mut Session, this: Option<&Rc<DotNetObject>>, _: &[Value]) -> Result<Value, Signal> {
    let (type_name, ..) = parts(this)?;
    Ok(Value::DotNet(Rc::new(DotNetObject::new(DotNetKind::Internal {
        type_name: RUNTIME_TYPE.into(),
        state: RefCell::new(vec![Value::Text(type_name.into())]),
    }))))
}

/// The codeunit id and routine name of one AL frame of an error's call stack (`"Name"(CodeUnit 61480).OnRun(Trigger) line 3 - ...`,
/// see `callstack.rs`); `None` for a frame of another object type.
fn codeunit_frame(frame: &str) -> Option<(u32, &str)> {
    let (_, rest) = frame.split_once("(CodeUnit ")?;
    let (id, rest) = rest.split_once(").")?;
    let name = rest.split([' ', '(']).next()?;
    Some((id.parse().ok()?, name))
}

const DO_RUN_ASYNC: &str =
    "\n   at Microsoft.Dynamics.Nav.Runtime.NavCodeunit.DoRunAsync(DataError errorLevel, NavRecord record)";

/// `<class>: <message>`, then the .NET frames BC writes. Pinned only for a dialog exception (Error()) caught by the Codeunit.Run
/// that started the codeunit (the trace ends at that run, whatever calls it):
/// - raised in the OnRun itself: GetLastErrorObjectKinds.al:384 ToString_DialogException_Lines (the NavDialog.ALError frame, then
///   OnRunAsync and DoRunAsync);
/// - raised in a procedure without parameters or return value of the same codeunit that the OnRun calls: AzLastErrorObject.al:180
///   ToString_NestedProcedure (`Codeunit99472.Nested()`, no ALError frame);
/// - raised in such a procedure of another codeunit that the OnRun calls: AzLastErrorObject.al:199 ToString_OtherCodeunit
///   (`Codeunit99471.Raise()`, from BC 29 a ValueTaskExtensions frame after it).
///
/// Any other trace is not pinned, and BC's frames are .NET's, not AL's. Refusing those reads would lose four baseline cases that
/// read only the first line and look for `   at ` (GetLastErrorObject.al:188 _AfterErrorInTryFunction, :212 _AfterErrorInAsserterror,
/// :233 _AfterGetWithoutRow, :258 _AfterDivisionByZero), so the text keeps one AL frame per line there under the registered
/// assumption `exception-tostring-stack-frames`; a trace with a frame the runtime does not track carries the untracked call-stack
/// mark instead, so every read of it is refused.
pub(crate) fn exception_text(session: &Session, this: &Rc<DotNetObject>) -> Result<String, Signal> {
    let (type_name, msg, stack) = parts(Some(this))?;
    let (Value::Text(msg), Value::Text(stack)) = (msg, stack) else {
        return unsupported("a last-error exception without its message");
    };
    let mut out = format!("{type_name}: {msg}");
    if type_name == NCL_DIALOG_EXCEPTION {
        let frames: Vec<&str> = stack.split('\n').filter(|f| !f.is_empty()).collect();
        let first = frames.first().and_then(|f| codeunit_frame(f));
        let second = frames.get(1).and_then(|f| codeunit_frame(f));
        let on_run_async = |id: u32| {
            format!(
                "\n   at Microsoft.Dynamics.Nav.BusinessApplication.Codeunit{id}.OnRunAsync(INavRecordHandle \u{3b5}rec)"
            )
        };
        match (first, second) {
            (Some((id, "OnRun")), _) => {
                out.push_str("\n   at Microsoft.Dynamics.Nav.Runtime.NavDialog.ALError(NavSession session, Guid automationId, String message, NavValue[] values)");
                out.push_str(&on_run_async(id));
                out.push_str(DO_RUN_ASYNC);
                return Ok(out);
            }
            (Some((inner, proc)), Some((outer, "OnRun"))) if plain_procedure(session, inner, proc) => {
                out.push_str(&format!(
                    "\n   at Microsoft.Dynamics.Nav.BusinessApplication.Codeunit{inner}.{proc}()"
                ));
                if inner != outer && session.has(al_version::Gate::ExceptionTraceValueTaskFrameFrom29) {
                    out.push_str("\n   at Microsoft.Dynamics.Nav.Types.ValueTaskExtensions.ManualResetValueTaskSource`1.GetResult(Int16 token)");
                }
                out.push_str(&on_run_async(outer));
                out.push_str(DO_RUN_ASYNC);
                return Ok(out);
            }
            _ => {}
        }
    }
    let frames: Vec<&str> = stack.split('\n').filter(|f| !f.is_empty()).collect();
    if frames.is_empty() || frames.contains(&UNTRACKED_CALL_STACK) {
        out.push_str("\n   at ");
        out.push_str(UNTRACKED_CALL_STACK);
        return Ok(out);
    }
    al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ExceptionToStringStackFrames);
    for f in frames {
        out.push_str("\n   at ");
        out.push_str(f);
    }
    Ok(out)
}

/// Whether codeunit `id` has a procedure `name` with no parameters and no return value, whose name is a plain identifier: the
/// shape whose .NET frame the corpus pins (`Codeunit99472.Nested()`, `Codeunit99471.Raise()`).
fn plain_procedure(session: &Session, id: u32, name: &str) -> bool {
    if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        return false;
    }
    // The codeunits with the id are found from the heads, so only they are made (ADR-054).
    let objects = &session.engine.program.objects;
    objects.heads().iter().enumerate().any(|(oi, h)| {
        h.kind == al_ir::ObjKind::Codeunit
            && h.id == Some(id)
            && objects[oi]
                .routines
                .iter()
                .any(|r| r.name == name && !r.is_trigger && r.params == 0 && r.ret.is_none())
    })
}

fn to_string(session: &mut Session, this: Option<&Rc<DotNetObject>>, _: &[Value]) -> Result<Value, Signal> {
    let Some(this) = this else {
        return unsupported("DotNet exception ToString without an object");
    };
    Ok(Value::Text(exception_text(session, this)?.into()))
}

fn full_name(_: &mut Session, this: Option<&Rc<DotNetObject>>, _: &[Value]) -> Result<Value, Signal> {
    match this.map(|o| &o.kind) {
        Some(DotNetKind::Internal { state, .. }) => Ok(state.borrow()[0].clone()),
        _ => unsupported("DotNet Type member on something that is not a runtime type"),
    }
}

fn member(name: &str, kind: MemberKind, f: super::registry::MemberFn) -> RegistryMember {
    RegistryMember {
        name: name.into(),
        kind,
        pin: pin(),
        call: f,
        call_ref: None,
    }
}

fn exception(clr_name: &str) -> RegistryType {
    RegistryType {
        clr_name: clr_name.into(),
        members: vec![
            member("Message", MemberKind::Get, message),
            member("InnerException", MemberKind::Get, inner_exception),
            member("GetType", MemberKind::Call, get_type),
            member("ToString", MemberKind::Call, to_string),
            member("GetBaseException", MemberKind::Call, get_base_exception),
            member("Equals", MemberKind::Call, equals),
        ],
    }
}

/// A member of the "no last error" object: the error of an unassigned DotNet variable, naming the declared
/// type System.Exception (GetLastErrorObject.al:296-299, `Message`, `GetType` and `ToString`).
fn not_instantiated(s: &mut Session, member: &str) -> Result<Value, Signal> {
    let Some((&object, &frame)) = s.frames.last().zip(s.call_stack.last()) else {
        return unsupported(format!(
            "System.Exception.{member} on a null exception outside a routine"
        ));
    };
    let o = &s.engine.program.objects[object];
    if o.kind != al_ir::ObjKind::Codeunit || o.uncompiled {
        return unsupported(format!(
            "System.Exception.{member} on a null exception outside a codeunit (the object kind's spelling in the error is not pinned)"
        ));
    }
    let routine = o.routines.get(frame.routine).map_or("", |r| r.name.as_str());
    let message = format!(
        "A DotNet variable has not been instantiated. Attempting to call System.Exception.{member} in CodeUnit {}: {routine}",
        o.name
    );
    al_builtins::native::raise(message, "DotNetVariableNotCreated")
}

fn null_message(s: &mut Session, _: Option<&Rc<DotNetObject>>, _: &[Value]) -> Result<Value, Signal> {
    not_instantiated(s, "Message")
}

fn null_get_type(s: &mut Session, _: Option<&Rc<DotNetObject>>, _: &[Value]) -> Result<Value, Signal> {
    not_instantiated(s, "GetType")
}

fn null_to_string(s: &mut Session, _: Option<&Rc<DotNetObject>>, _: &[Value]) -> Result<Value, Signal> {
    not_instantiated(s, "ToString")
}

pub fn types() -> Vec<RegistryType> {
    vec![
        exception(NCL_DIALOG_EXCEPTION),
        exception(RECORD_NOT_FOUND_EXCEPTION),
        exception(AL_EXCEPTION),
        exception(TEST_FIELD_EXCEPTION),
        exception(NCL_EVALUATE_EXCEPTION),
        // The base exception of a division by zero: its message and its class name are pinned.
        RegistryType {
            clr_name: DIVIDE_BY_ZERO_EXCEPTION.into(),
            members: vec![
                member("Message", MemberKind::Get, message),
                member("GetType", MemberKind::Call, get_type),
            ],
        },
        // The inner exception of a NavALException: only that it exists is pinned.
        RegistryType {
            clr_name: AL_INNER_EXCEPTION.into(),
            members: Vec::new(),
        },
        RegistryType {
            clr_name: NULL_EXCEPTION.into(),
            members: vec![
                member("Message", MemberKind::Get, null_message),
                member("GetType", MemberKind::Call, null_get_type),
                member("ToString", MemberKind::Call, null_to_string),
            ],
        },
        RegistryType {
            clr_name: RUNTIME_TYPE.into(),
            members: vec![
                member("FullName", MemberKind::Get, full_name),
                member("Equals", MemberKind::Call, equals),
            ],
        },
    ]
}
