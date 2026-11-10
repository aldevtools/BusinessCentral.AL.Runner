//! `Microsoft.Dynamics.Nav.SystemEnvironment.SystemEnvironment`, the server class behind System Application's
//! "Type Helper".NewLine and "XML DOM Management" (al-runner issue 6924, item 2.4). Pinned by
//! tests/al-language-onprem-dotnet/dotnet/server-helpers/Y58ServerHelpers.al (real BC 27.5, 28.5 and 29.0):
//!
//! - the constructor (:408-439 SystemEnvironment_New_NewLine): `IsNull` is Yes before it and No after it.
//! - `NewLine` (:408-439): the corpus pins only that its last character is code 10 (the Linux CI containers measured LF;
//!   the case ignores the rest). The runner answers CR LF on every host, as a Windows service tier does: Microsoft's own
//!   TestNewLine (TypeHelperUnitTests.Codeunit.al:508-515) asserts NewLine = CRLFSeparator. That value is the registered
//!   assumption `type-helper-new-line-crlf`; the call marks it, so a test that passes through it is a flagged pass. The probe
//!   (corpus pull request 1281, TypeHelperNewLineValue.al) wins when it lands. The answer does not depend on the host.
//! - `NewLine` on a variable never constructed (:441-462 SystemEnvironment_NewLine_WithoutConstructor): BC 29.0 answers
//!   the same text, BC 27.5 and 28.5 raise `DotNetVariableNotCreated` (`not_created.rs`).
//! - `ToString()` and `Format` of the instance: the type name (:408-439).
//! - `Equals` of two constructed instances is No: reference equality (:465-480 SystemEnvironment_TwoInstances_SameNewLine).
//!
//! Every other member of the type is refused (a registry type serves only its listed members).

use super::registry::{MemberFn, MemberKind, Pin, RegistryMember, RegistryType};
use crate::Session;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::dotnet_sigs::SYSTEM_ENVIRONMENT;
use al_builtins::native::Signal;
use al_builtins::value::{DotNetKind, DotNetObject, Value};
use std::cell::RefCell;
use std::rc::Rc;

pub(super) const TYPE: &str = SYSTEM_ENVIRONMENT;
/// The member as the not-created error names it.
pub(super) const NEW_LINE: &str = "Microsoft.Dynamics.Nav.SystemEnvironment.SystemEnvironment.NewLine";

const FILE: &str = "tests/al-language-onprem-dotnet/dotnet/server-helpers/Y58ServerHelpers.al";

fn no_args(args: &[Value], what: &str) -> Result<(), Signal> {
    if args.is_empty() {
        Ok(())
    } else {
        Err(Signal::Unsupported(format!(
            "DotNet {TYPE}.{what} with arguments (not pinned)"
        )))
    }
}

fn construct(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    no_args(args, "SystemEnvironment")?;
    Ok(Value::DotNet(Rc::new(DotNetObject::new(DotNetKind::Internal {
        type_name: TYPE.into(),
        state: RefCell::new(Vec::new()),
    }))))
}

fn new_line(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    no_args(args, "NewLine")?;
    // The CR in front is unpinned; the last character LF is pinned (Y58ServerHelpers.al:408-439).
    assume(Assumption::TypeHelperNewLineCrlf);
    Ok(Value::Text("\r\n".into()))
}

fn to_string(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    no_args(args, "ToString")?;
    match this {
        Some(_) => Ok(Value::Text(TYPE.into())),
        None => Err(Signal::Unsupported(format!(
            "DotNet {TYPE}.ToString on a variable never constructed (not pinned)"
        ))),
    }
}

fn equals(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    match (this, args) {
        (Some(a), [Value::DotNet(b)]) => Ok(Value::Boolean(Rc::ptr_eq(a, b))),
        _ => Err(Signal::Unsupported(format!(
            "DotNet {TYPE}.Equals other than between two constructed instances (not pinned)"
        ))),
    }
}

fn member(name: &str, kind: MemberKind, line: &str, f: MemberFn) -> RegistryMember {
    RegistryMember {
        name: name.into(),
        kind,
        pin: Pin::Corpus(format!("{FILE}:{line}")),
        call: f,
        call_ref: None,
    }
}

pub fn types() -> Vec<RegistryType> {
    vec![RegistryType {
        clr_name: TYPE.into(),
        members: vec![
            member("SystemEnvironment", MemberKind::Construct, "408-439", construct),
            member("NewLine", MemberKind::Call, "408-462", new_line),
            member("ToString", MemberKind::Call, "408-439", to_string),
            member("Equals", MemberKind::Call, "465-480", equals),
        ],
    }]
}
