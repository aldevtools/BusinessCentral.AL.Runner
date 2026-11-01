//! `TryGetValue`, `Remove` and `Values` of a `System.Collections.Generic.Dictionary`2` the runtime made (generic_dictionary.rs),
//! the members System Application's Edit in Excel filters call (EditInExcelFiltersImpl.Codeunit.al). Pinned by
//! tests/al-language-onprem-dotnet/lt-y51-dotnet-dictionary/Y51DotNetDictNoCap.al:DnDictNoCap_ObjectValues: `TryGetValue` of a
//! key gives true and the value into the `var` argument, of a missing key false and null there (`tryget_z_isnull`); `Remove`
//! gives true, then false; `Values` walks the values in insertion order (`node2;node1+;`).

use super::generic_dictionary::{FILL_PIN, filled, key_of, member, object, position};
use super::registry::{MemberKind, Pin, RegistryMember};
use crate::Session;
use al_builtins::dotnet_sigs::{GENERIC_DICTIONARY, GENERIC_DICTIONARY_VALUES};
use al_builtins::native::Signal;
use al_builtins::value::{DotNetObject, Value};
use std::rc::Rc;

pub(super) fn remove(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    let state = filled(this, "Remove")?;
    if args.len() != 1 {
        return Err(Signal::Unsupported(format!(
            "DotNet {GENERIC_DICTIONARY}.Remove: only the form with one key is pinned"
        )));
    }
    let Some(i) = position(state, &key_of(args, "Remove")?) else {
        return Ok(Value::Boolean(false));
    };
    state.borrow_mut().drain(1 + 2 * i..3 + 2 * i);
    Ok(Value::Boolean(true))
}

pub(super) fn values(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    let state = filled(this, "Values")?;
    if !args.is_empty() {
        return Err(Signal::Unsupported(format!(
            "DotNet {GENERIC_DICTIONARY}.Values takes no arguments"
        )));
    }
    let values: Vec<Value> = state.borrow()[1..].iter().skip(1).step_by(2).cloned().collect();
    Ok(object(GENERIC_DICTIONARY_VALUES, values))
}

fn try_get(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &mut [Value]) -> Result<Value, Signal> {
    let state = filled(this, "TryGetValue")?;
    if args.len() != 2 {
        return Err(Signal::Unsupported(format!(
            "DotNet {GENERIC_DICTIONARY}.TryGetValue takes a key and a var argument"
        )));
    }
    let found = position(state, &key_of(args, "TryGetValue")?).map(|i| state.borrow()[2 + 2 * i].clone());
    args[1] = found.clone().unwrap_or(Value::Null);
    Ok(Value::Boolean(found.is_some()))
}

pub(super) fn try_get_value() -> RegistryMember {
    fn plain(_: &mut Session, _: Option<&Rc<DotNetObject>>, _: &[Value]) -> Result<Value, Signal> {
        Err(Signal::Unsupported(
            "TryGetValue called without its var argument".into(),
        ))
    }
    RegistryMember {
        call_ref: Some(try_get),
        pin: Pin::Corpus(format!(
            "{FILL_PIN}; tests/al-language-onprem-dotnet/lt-y51-dotnet-dictionary/Y51DotNetDictNoCap.al"
        )),
        ..member("TryGetValue", MemberKind::Call, plain)
    }
}
