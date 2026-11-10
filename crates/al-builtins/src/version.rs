//! `Version`: four numbers read by `Evaluate` (corpus session-system/TestMiscTypes.al:44-80).
//! `Create` and `ToText` are not pinned and are not implemented.

use crate::describe::Describe;
use crate::native::{Ctx, NativeFn, Signal};
use crate::signature::Signature;
use crate::value::{LibraryValue, Value};

fn part(r: Option<&mut Value>, i: usize) -> Result<Value, Signal> {
    match r {
        Some(Value::Library(_, cell)) => match &*cell.borrow() {
            LibraryValue::Version(v) => Ok(Value::Integer(v[i])),
            other => Err(Signal::unsupported(format!(
                "a Version part of {other} (never evaluated)",
                other = other.describe()
            ))),
        },
        other => Err(Signal::unsupported(format!("a Version part of {}", other.describe()))),
    }
}

fn version_value(v: [i32; 4]) -> Value {
    Value::Library(
        "Version".into(),
        std::rc::Rc::new(std::cell::RefCell::new(LibraryValue::Version(v))),
    )
}

/// `Version.Create(Major, Minor [, Build [, Revision]])`: missing parts read 0 (types/
/// TestVersionCreateEncryptionContracts.al:VersionCreate_MissingPartsAreZero_AndTextForms). A negative part is not pinned.
fn create_parts(_: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let mut v = [0; 4];
    for (i, x) in a.iter().enumerate().take(4) {
        match x {
            Value::Integer(n) if *n >= 0 => v[i] = *n,
            other => return Err(Signal::unsupported(format!("Version.Create with {}", other.describe()))),
        }
    }
    Ok(version_value(v))
}

/// `Version.Create(Text)`: two to four numbers separated by dots; anything else raises "Could not parse
/// string '<text>' into a version." (code Argument; ibid. VersionCreate_BadText_RaisesAnArgumentError).
fn create_text(_: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let Some(text) = crate::native::text_of(&a[0]) else {
        return Err(Signal::Unsupported(
            "Version.Create with a text that is not text".into(),
        ));
    };
    let parts: Vec<Option<i32>> = text
        .split('.')
        .map(|p| {
            (!p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
                .then(|| p.parse().ok())
                .flatten()
        })
        .collect();
    if !(2..=4).contains(&parts.len()) || parts.iter().any(Option::is_none) {
        return crate::native::raise(format!("Could not parse string '{text}' into a version."), "Argument");
    }
    let mut v = [0; 4];
    for (i, p) in parts.into_iter().enumerate() {
        v[i] = p.unwrap_or(0);
    }
    Ok(version_value(v))
}

fn to_text(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    match r {
        Some(Value::Library(_, cell)) => match &*cell.borrow() {
            LibraryValue::Version(v) => Ok(Value::Text(format!("{}.{}.{}.{}", v[0], v[1], v[2], v[3]).into())),
            other => Err(Signal::unsupported(format!(
                "ToText of a Version nobody set ({})",
                other.describe()
            ))),
        },
        other => Err(Signal::unsupported(format!("Version.ToText on {}", other.describe()))),
    }
}

pub fn native_for(sig: &Signature) -> Option<NativeFn> {
    if sig.receiver == "Version" && !sig.instance && sig.name == "Create" {
        return Some(
            if sig.params.len() == 1 && sig.params[0].ty.eq_ignore_ascii_case("Text") {
                create_text
            } else {
                create_parts
            },
        );
    }
    if sig.receiver != "Version" || !sig.instance {
        return None;
    }
    let f: NativeFn = match sig.name.as_str() {
        "ToText" => to_text,
        "Major" => |_: &mut dyn Ctx, r, _| part(r, 0),
        "Minor" => |_: &mut dyn Ctx, r, _| part(r, 1),
        "Build" => |_: &mut dyn Ctx, r, _| part(r, 2),
        "Revision" => |_: &mut dyn Ctx, r, _| part(r, 3),
        _ => return None,
    };
    Some(f)
}
