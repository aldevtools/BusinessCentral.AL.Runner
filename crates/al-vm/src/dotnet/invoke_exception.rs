//! A .NET exception thrown by a DotNet call the host made becomes an AL error (al-runner issue 7924), only for the calls the corpus
//! pins. tests/al-language-onprem-dotnet/dotnet/dotnet/DotNetExceptionOutcome.al measures four, every major: an instance method
//! (`Uri.IsBaseOf` of an unassigned Uri, :66, :194), a static method (`Convert.ToInt32('abc')`, :71, :205), a property get on an
//! object (`AbsolutePath` of a relative Uri, :76, :216) and a constructor (`StringBuilder(-1)`, :79, :224). For each: Codeunit.Run
//! answers No (:229-256), asserterror catches it (:258-282), a TryFunction answers No (:285-302, :388-426), the text is
//! `A call to <Type>.<Member> failed with this message: <.NET message>` (a constructor names the type only, :251) with code
//! `DotNetInvoke:<class without Exception>`, and GetLastErrorObject is a NavNCLDotNetInvokeException whose Message is the text and
//! whose InnerException is the .NET exception (:305-386). streams/DdStreamLeads.al:538 pins the same text for
//! `Convert.FromBase64String` (BC 27.5; from 28.5 the System Application decodes without that call, :536). Y58AzureKeyVault.al:
//! 317-325 pins that ArgumentException of the X509Certificate2 constructor is an error a TryFunction catches, not its text.
//!
//! The exceptions whose text an older pin covers are raised by `pinned_exceptions::signal` first; this module answers what is
//! left, only for the calls in `MEASURED_CALLS` and `CERTIFICATE_CONSTRUCTOR`, sent with no stream. Everything else keeps the
//! refusal of the caller.
//!
//! A .NET exception of a DotNet method, property get or constructor the host ran becomes an AL error
//! (al-runner issue 7924): the pinned text and code only for the pinned calls (`MEASURED_CALLS`), an
//! error with an unpinned text for ArgumentException of the X509Certificate2 constructor
//! (`CERTIFICATE_CONSTRUCTOR`), only for a call that sent no stream, and the
//! NavNCLDotNetInvokeException `GetLastErrorObject` answers. Entry points: `signal`, `outer_type`,
//! `inner_type`.

use super::Target;
use super::last_error::{get_type, member, message};
use super::registry::{MemberKind, Pin, RegistryType};
use crate::Session;
use al_builtins::dotnet_sigs::{DOTNET_INVOKE_INNER_EXCEPTION, NCL_DOTNET_INVOKE_EXCEPTION, RUNTIME_TYPE};
use al_builtins::native::{AlError, EXC_CODE, EXC_MESSAGE, ExceptionKind, Signal, unpinned_error_for, unsupported};
use al_builtins::value::{DotNetKind, DotNetObject, Value};
use al_host::dotnet::DotNetException;
use std::cell::RefCell;
use std::rc::Rc;

/// The exception classes of the measured calls, with their code (DotNetExceptionOutcome.al:234-235, :237-238, :248-249,
/// :251-252).
const MEASURED: [(&str, &str); 4] = [
    ("System.ArgumentNullException", "DotNetInvoke:ArgumentNull"),
    ("System.FormatException", "DotNetInvoke:Format"),
    ("System.InvalidOperationException", "DotNetInvoke:InvalidOperation"),
    ("System.ArgumentOutOfRangeException", "DotNetInvoke:ArgumentOutOfRange"),
];

/// The bindings of the pinned calls, each by the request kind, what it was made on and what the sidecar says threw. `None` for
/// any other (a setter, an indexer, a property reached through a call, a call on a boxed value, a static property).
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

/// The calls whose error text and code are pinned: (type, member as .NET spells it, binding, exception class). A constructor has
/// no member. The member must be written in that letter case and the exception must have no inner one, as in the pinned calls.
const MEASURED_CALLS: [(&str, &str, Shape, &str); 5] = [
    // DotNetExceptionOutcome.al:66, :234-235.
    (
        "System.Uri",
        "IsBaseOf",
        Shape::InstanceMethod,
        "System.ArgumentNullException",
    ),
    // :71, :237-238.
    (
        "System.Convert",
        "ToInt32",
        Shape::StaticMethod,
        "System.FormatException",
    ),
    // :76, :248-249.
    (
        "System.Uri",
        "AbsolutePath",
        Shape::InstanceGet,
        "System.InvalidOperationException",
    ),
    // :79, :251-252.
    (
        "System.Text.StringBuilder",
        "",
        Shape::Constructor,
        "System.ArgumentOutOfRangeException",
    ),
    // streams/DdStreamLeads.al:538 (BC 27.5, through the System Application's Base64 Convert).
    (
        "System.Convert",
        "FromBase64String",
        Shape::StaticMethod,
        "System.FormatException",
    ),
];

/// The constructor whose ArgumentException is an AL error with a text that is not pinned (Y58AzureKeyVault.al:317-325).
const CERTIFICATE_CONSTRUCTOR: (&str, &str) = (
    "System.Security.Cryptography.X509Certificates.X509Certificate2",
    "System.ArgumentException",
);

/// The AL error of the exception `e` thrown by `clr_type.member`, or `None` when the call is not a pinned one (the caller then
/// refuses). The call must have sent no InStream or OutStream.
pub(super) fn signal(
    kind: MemberKind,
    target: &Target,
    clr_type: &str,
    member: &str,
    streams: usize,
    e: &DotNetException,
) -> Option<Signal> {
    // The pinned calls send no InStream or OutStream: an object, a Text, nothing, an Integer (DotNetExceptionOutcome.al:66,
    // :71, :76, :79). What .NET read from or wrote to a stream before it threw is not in the reply of a failed call. A member
    // with a ref or out parameter has no `bound` (the sidecar's `PlainParameters`), so `shape` answers `None` for it.
    let shape = (streams == 0).then(|| shape(kind, target, &e.bound)).flatten()?;
    let pinned_call = !e.folded
        && e.inner.is_empty()
        && MEASURED_CALLS.iter().any(|&(ty, m, s, class)| {
            ty == clr_type && s == shape && class == e.clr_type && (shape == Shape::Constructor || m == member)
        });
    if pinned_call {
        let code = MEASURED
            .iter()
            .find(|(class, _)| *class == e.clr_type)
            .map(|(_, code)| *code)?;
        let called = if shape == Shape::Constructor {
            clr_type.to_string()
        } else {
            format!("{clr_type}.{member}")
        };
        return Some(Signal::Error(Box::new(AlError {
            message: format!("A call to {called} failed with this message: {}", e.message),
            code: code.into(),
            exception: ExceptionKind::DotNetInvoke,
            ..Default::default()
        })));
    }
    if shape == Shape::Constructor && (clr_type, e.clr_type.as_str()) == CERTIFICATE_CONSTRUCTOR {
        return Some(Signal::Error(Box::new(unpinned_error_for(
            "Y58AzureKeyVault.al:317-325 pins only that the X509Certificate2 constructor fails",
        ))));
    }
    None
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
    if msg.contains('\n') && !multi_line_inner_measured(text.as_str(), code.as_str()) {
        return unsupported(format!(
            "Message of the inner exception of code '{code}' whose .NET message has more than one line (measured only for the \
             StringBuilder constructor, DotNetExceptionOutcome.al:369-385)"
        ));
    }
    Ok(Value::DotNet(Rc::new(DotNetObject::new(DotNetKind::Internal {
        type_name: DOTNET_INVOKE_INNER_EXCEPTION.into(),
        // The slots `message` and `get_type` read: the message, then the class `GetType().FullName` answers.
        state: RefCell::new(vec![Value::Text(msg.into()), Value::Text(class.into())]),
    }))))
}

/// The one call whose inner exception has a message of more than one line in the corpus: the StringBuilder constructor's
/// ArgumentOutOfRange, whose error text holds both lines (DotNetExceptionOutcome.al:373) and whose inner Message the probe reads
/// (its first line, :184, :382). The constructor names the type only, so its text starts with `A call to <type> failed`.
fn multi_line_inner_measured(text: &str, code: &str) -> bool {
    const CONSTRUCTOR: (&str, &str) = ("System.Text.StringBuilder", "DotNetInvoke:ArgumentOutOfRange");
    code == CONSTRUCTOR.1
        && text
            .strip_prefix("A call to ")
            .and_then(|t| t.strip_prefix(CONSTRUCTOR.0))
            .is_some_and(|t| t.starts_with(BEFORE_MESSAGE))
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
    Pin::Corpus("tests/al-language-onprem-dotnet/dotnet/dotnet/DotNetExceptionOutcome.al:305-386".into())
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

/// The .NET exception inside it: `GetType().FullName` and `Message` (:321-322; the probe reads the first line of Message, :382,
/// and the error text holds the whole .NET message, :373).
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
