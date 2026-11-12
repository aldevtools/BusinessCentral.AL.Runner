//! A .NET exception thrown by a DotNet call the host made becomes an AL error (al-runner issue 7924). Measured by corpus PR 1365
//! (open; onprem-dotnet dotnet/dotnet/DotNetExceptionOutcome.al, every major its CI runs) for four calls: an instance method
//! (`Uri.IsBaseOf` of an unassigned Uri, :66, :194), a static method (`Convert.ToInt32('abc')`, :71, :205), a property get on an
//! object (`AbsolutePath` of a relative Uri, :76, :216) and a constructor (`StringBuilder(-1)`, :79, :224). For each: Codeunit.Run
//! answers No (:229-256), asserterror catches it (:258-282), a TryFunction answers No (:285-302, :388-430), the text is
//! `A call to <Type>.<Member> failed with this message: <.NET message>` (a constructor names the type only, :251) with code
//! `DotNetInvoke:<class without Exception>`, and GetLastErrorObject is a NavNCLDotNetInvokeException whose Message is the text and
//! whose InnerException is the .NET exception (:305-386).
//!
//! The exceptions whose text an older pin covers are raised by `pinned_exceptions::signal` first; this module answers what is
//! left, and only for the four bindings above (`shape`). Everything else keeps the refusal of the caller.

use super::Target;
use super::last_error::{get_type, member, message};
use super::registry::{MemberKind, Pin, RegistryType};
use crate::Session;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::dotnet_sigs::{DOTNET_INVOKE_INNER_EXCEPTION, NCL_DOTNET_INVOKE_EXCEPTION, RUNTIME_TYPE};
use al_builtins::native::{AlError, EXC_CODE, EXC_MESSAGE, ExceptionKind, Signal, unpinned_error_for, unsupported};
use al_builtins::value::{DotNetKind, DotNetObject, Value};
use al_host::dotnet::DotNetException;
use std::cell::RefCell;
use std::rc::Rc;

/// The exception classes whose text and code the probe measured, with their code (DotNetExceptionOutcome.al:234-235, :237-238,
/// :248-249, :251-252).
const MEASURED: [(&str, &str); 4] = [
    ("System.ArgumentNullException", "DotNetInvoke:ArgumentNull"),
    ("System.FormatException", "DotNetInvoke:Format"),
    ("System.InvalidOperationException", "DotNetInvoke:InvalidOperation"),
    ("System.ArgumentOutOfRangeException", "DotNetInvoke:ArgumentOutOfRange"),
];

/// The four measured bindings, each by the request kind, what it was made on and what the sidecar says threw. `None` for any
/// other (a setter, an indexer, a property reached through a call, a call on a boxed value, a static property): not measured.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    /// :66, `Base.IsBaseOf(Other)` on an object.
    InstanceMethod,
    /// :71, `Convert.ToInt32(Input)` through a variable that holds no object.
    StaticMethod,
    /// :76, `Relative.AbsolutePath` on an object.
    InstanceGet,
    /// :79, `Builder.StringBuilder(-1)`.
    Constructor,
}

fn shape(kind: MemberKind, target: &Target, bound: &str) -> Option<Shape> {
    match (kind, target, bound) {
        (MemberKind::Call, Target::Host(..), "method") => Some(Shape::InstanceMethod),
        (MemberKind::Call, Target::Static(_), "method") => Some(Shape::StaticMethod),
        (MemberKind::Get, Target::Host(..), "property") => Some(Shape::InstanceGet),
        (MemberKind::Construct, Target::Static(_), "constructor") => Some(Shape::Constructor),
        _ => None,
    }
}

/// A type name as the measured texts spell it: dotted plain identifiers (`System.Uri`, `System.Text.StringBuilder`). A generic or
/// nested type is spelled otherwise by BC (`Dictionary`2[System.Object,System.Object]`, Y17DotNetDictionary.al:126) than the host
/// reports it.
fn plain_type_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .split('.')
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'))
}

/// The AL error of the exception `e` thrown by `clr_type.member`, or `None` when the binding is not one of the measured four
/// (the caller then refuses). The text and code are the measured rule's for a measured class, a type spelled plainly, a member
/// written in its .NET letter case and an exception without an inner one (assumption `dotnet-exception-measured-text`); any other
/// exception of these bindings is an error whose text and code are not pinned, so a read of them refuses (assumption
/// `dotnet-exception-other-class`).
pub(super) fn signal(
    kind: MemberKind,
    target: &Target,
    clr_type: &str,
    member: &str,
    e: &DotNetException,
) -> Option<Signal> {
    let shape = shape(kind, target, &e.bound)?;
    let code = MEASURED
        .iter()
        .find(|(class, _)| *class == e.clr_type)
        .map(|(_, code)| *code);
    match code {
        Some(code) if plain_type_name(clr_type) && !e.folded && e.inner.is_empty() => {
            assume(Assumption::DotnetExceptionMeasuredText);
            let called = if shape == Shape::Constructor {
                clr_type.to_string()
            } else {
                format!("{clr_type}.{member}")
            };
            Some(Signal::Error(Box::new(AlError {
                message: format!("A call to {called} failed with this message: {}", e.message),
                code: code.into(),
                exception: ExceptionKind::DotNetInvoke,
                ..Default::default()
            })))
        }
        _ => {
            assume(Assumption::DotnetExceptionOtherClass);
            Some(Signal::Error(Box::new(unpinned_error_for(
                "dotnet-exception-other-class",
            ))))
        }
    }
}

/// The text before the .NET message in the error text (DotNetExceptionOutcome.al:234).
const BEFORE_MESSAGE: &str = " failed with this message: ";

/// `InnerException` of a NavNCLDotNetInvokeException: the .NET exception, its class from the code and its message from the text
/// (DotNetExceptionOutcome.al:320-322, :340-342, :360-362, :380-382).
fn inner_exception(_: &mut Session, this: Option<&Rc<DotNetObject>>, _: &[Value]) -> Result<Value, Signal> {
    let Some(DotNetKind::Internal { state, .. }) = this.map(|o| &o.kind) else {
        return unsupported("InnerException of something that is not a NavNCLDotNetInvokeException");
    };
    let state = state.borrow();
    let (Some(Value::Text(text)), Some(Value::Text(code))) = (state.get(EXC_MESSAGE), state.get(EXC_CODE)) else {
        return unsupported("a NavNCLDotNetInvokeException without its text and code");
    };
    let class = MEASURED
        .iter()
        .find(|(_, c)| *c == code.as_str())
        .map(|(class, _)| *class);
    let (Some(class), Some((_, msg))) = (class, text.as_str().split_once(BEFORE_MESSAGE)) else {
        return unsupported(format!(
            "InnerException of a NavNCLDotNetInvokeException of code '{code}' (not measured)"
        ));
    };
    assume(Assumption::DotnetExceptionMeasuredText);
    Ok(Value::DotNet(Rc::new(DotNetObject::new(DotNetKind::Internal {
        type_name: DOTNET_INVOKE_INNER_EXCEPTION.into(),
        // The slots `message` and `get_type` read: the message, then the class `GetType().FullName` answers.
        state: RefCell::new(vec![Value::Text(msg.into()), Value::Text(class.into())]),
    }))))
}

/// `GetType()` of the inner exception: its .NET class (DotNetExceptionOutcome.al:321, `inner_type`).
fn inner_get_type(_: &mut Session, this: Option<&Rc<DotNetObject>>, _: &[Value]) -> Result<Value, Signal> {
    let Some(DotNetKind::Internal { state, .. }) = this.map(|o| &o.kind) else {
        return unsupported("GetType of something that is not the inner exception of a NavNCLDotNetInvokeException");
    };
    let class = state.borrow()[1].clone();
    Ok(Value::DotNet(Rc::new(DotNetObject::new(DotNetKind::Internal {
        type_name: RUNTIME_TYPE.into(),
        state: RefCell::new(vec![class]),
    }))))
}

fn pin() -> Pin {
    Pin::Probe(
        "probe tests/al-language-onprem-dotnet/dotnet/dotnet/DotNetExceptionOutcome.al:305-386 (corpus PR 1365, open; assumption \
         dotnet-exception-measured-text)"
            .into(),
    )
}

/// NavNCLDotNetInvokeException: `Message` is the error text (:319 `object_message_is_text`), `GetType().FullName` its class
/// (:318), `InnerException` the .NET exception. No other member is measured.
pub(super) fn outer_type() -> RegistryType {
    RegistryType {
        clr_name: NCL_DOTNET_INVOKE_EXCEPTION.into(),
        members: vec![
            super::registry::RegistryMember {
                pin: pin(),
                ..member("Message", MemberKind::Get, message)
            },
            super::registry::RegistryMember {
                pin: pin(),
                ..member("InnerException", MemberKind::Get, inner_exception)
            },
            super::registry::RegistryMember {
                pin: pin(),
                ..member("GetType", MemberKind::Call, get_type)
            },
        ],
    }
}

/// The .NET exception inside it: `GetType().FullName` and `Message` (its first line is measured, :321-322).
pub(super) fn inner_type() -> RegistryType {
    RegistryType {
        clr_name: DOTNET_INVOKE_INNER_EXCEPTION.into(),
        members: vec![
            super::registry::RegistryMember {
                pin: pin(),
                ..member("Message", MemberKind::Get, message)
            },
            super::registry::RegistryMember {
                pin: pin(),
                ..member("GetType", MemberKind::Call, inner_get_type)
            },
        ],
    }
}
