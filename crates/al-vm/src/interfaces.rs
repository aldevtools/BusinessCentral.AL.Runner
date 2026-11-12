//! Interface variables: casting a value to an interface and calling through one.
//!
//! An interface variable holds the codeunit instance it was assigned, or `Null` while
//! unassigned. The lowering resolved which codeunit answers for an enum value (corpus
//! types/TestEnumDefaultImplTests.Codeunit.al) and lists every implementing routine of an
//! interface procedure; the VM only picks among them by the instance's object.

use crate::{Session, Signal};
use al_builtins::describe::Describe;
use al_builtins::native::AlError;
use al_builtins::value::{Instance, Value};
use std::cell::RefCell;
use std::rc::Rc;

/// A call on an interface variable that holds no implementation
/// (codeunits/TestInterfaceRuntimeContracts.al:76,
/// Interface_CallOnUnassignedVariable_Throws_InterfaceNotInitialized). GetLastErrorText(true) leaves the text
/// (errors/last-error/LastErrorFlagRuntimeCodes.al InvalidOperationSites_ExcludeFlag `interface_true`; corpus pull request 1327).
fn not_initialized() -> Signal {
    Signal::Error(Box::new(AlError {
        message: "Interface not initialized".into(),
        code: "InvalidOperation".into(),
        call_stack: String::new(),
        origin: al_builtins::native::ErrorOrigin::SameText,
        ..Default::default()
    }))
}

impl Session {
    /// `value is I` / `value as I`: the instance the interface variable holds is a codeunit
    /// from `objects`. `is` is false, not an error, for an instance that does not implement `I` and
    /// for an unassigned variable; `as` to such an interface or on an unassigned variable raises
    /// `The expression cannot be the target interface.` / `UnableToCast`
    /// (codeunits/TestInterfaceIsAsContracts.al:61-102; the implemented case:
    /// codeunits/TestInterfaceRuntimeContracts.al:Interface_ExtendedInterface_IsAndAs).
    pub(crate) fn interface_test(&mut self, value: Value, objects: &[u32], cast: bool) -> Result<Value, Signal> {
        let unable = || {
            Signal::Error(Box::new(AlError {
                message: "The expression cannot be the target interface.".into(),
                code: "UnableToCast".into(),
                call_stack: String::new(),
                ..Default::default()
            }))
        };
        match value {
            Value::Codeunit(i) => {
                let implements = objects.contains(&i.borrow().object);
                match (cast, implements) {
                    (false, hit) => Ok(Value::Boolean(hit)),
                    (true, true) => Ok(Value::Codeunit(i)),
                    (true, false) => Err(unable()),
                }
            }
            Value::Null if cast => Err(unable()),
            Value::Null => Ok(Value::Boolean(false)),
            other => Err(Signal::unsupported(format!("`is` or `as` on {}", other.describe()))),
        }
    }

    /// The routine an interface call runs: the target belonging to the object of the instance
    /// the receiver holds, and that instance.
    pub(crate) fn interface_target_value(
        &mut self,
        receiver: Value,
        targets: &[(usize, usize)],
    ) -> Result<(usize, usize, Rc<RefCell<Instance>>), Signal> {
        match receiver {
            Value::Codeunit(i) => {
                let object = i.borrow().object as usize;
                match targets.iter().find(|(o, _)| *o == object) {
                    // The codeunit does not declare the member: the interface's default body runs on its
                    // instance (codeunits/CwDefaultInterfaceMethods.al:114-138, CW_Default_InheritAndOverride).
                    Some((_, al_ir::walk::DEFAULT_BODY)) => {
                        let kinds = &self.engine.program.objects;
                        match targets
                            .iter()
                            .find(|(o, _)| kinds[*o].kind == al_ir::ObjKind::Interface)
                        {
                            Some((o, r)) => Ok((*o, *r, self.shared_instance(i))),
                            None => Err(Signal::Unsupported(
                                "interface call: the default body of the interface is not compiled".into(),
                            )),
                        }
                    }
                    Some((o, r)) => Ok((*o, *r, self.shared_instance(i))),
                    None => Err(Signal::unsupported(format!(
                        "interface call on an instance of `{}`, which the interface does not list",
                        self.engine.program.objects[object].name
                    ))),
                }
            }
            // codeunits/TestInterfaceRuntimeContracts.al:76
            // (Interface_CallOnUnassignedVariable_Throws_InterfaceNotInitialized).
            Value::Null => Err(not_initialized()),
            other => Err(Signal::unsupported(format!("interface call on {}", other.describe()))),
        }
    }

    /// An enum value cast to an interface: a fresh, lazily initialised instance of the
    /// implementing codeunit (corpus types/TestInterfaceCodeunitStateField.al: the
    /// implementation's own state must be alive after the cast).
    pub(crate) fn enum_implementation(
        &mut self,
        value: Value,
        by_ordinal: &[(i32, usize)],
        unknown: Option<usize>,
    ) -> Result<Value, Signal> {
        let Value::Enum(id, ordinal) = value else {
            return Err(Signal::unsupported(format!("enum cast of {}", value.describe())));
        };
        self.note_enum(id);
        let object = match by_ordinal.iter().find(|(o, _)| *o == ordinal) {
            Some((_, c)) => *c,
            // An ordinal the enum does not declare and no UnknownValueImplementation to answer for
            // it: codeunits/TestInterfaceRuntimeContracts.al:90
            // (Interface_CastUndeclaredEnumOrdinalWithoutUnknownImplementation_Throws).
            None => unknown.ok_or_else(|| {
                Signal::Error(Box::new(AlError {
                    message: format!("Enum does not contain value {ordinal}."),
                    code: "ArgumentOutOfRange".into(),
                    call_stack: String::new(),
                    ..Default::default()
                }))
            })?,
        };
        Ok(Value::Codeunit(self.instance(object)))
    }
}
