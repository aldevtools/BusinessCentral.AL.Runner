//! `WebServiceActionContext`: the object an API page action tells the OData layer to answer with.
//! A mutable in-memory value shared by assignment (corpus session-system/TestWebServiceActionContext.al).

use crate::describe::Describe;
use crate::native::{Ctx, NativeFn, Signal, unsupported};
use crate::signature::Signature;
use crate::value::{LibraryValue, Value, WebServiceActionState};

type Out = Result<Value, Signal>;

/// The state behind the receiver; an untouched context becomes an explicit default on first use.
fn with_state<R>(r: Option<&mut Value>, f: impl FnOnce(&mut WebServiceActionState) -> R) -> Result<R, Signal> {
    let Some(Value::Library(_, cell)) = r else {
        return unsupported("WebServiceActionContext method without a receiver");
    };
    let mut lib = cell.borrow_mut();
    if matches!(&*lib, LibraryValue::Empty) {
        *lib = LibraryValue::WebServiceAction(WebServiceActionState::default());
    }
    match &mut *lib {
        LibraryValue::WebServiceAction(s) => Ok(f(s)),
        other => unsupported(format!("WebServiceActionContext method on {}", other.describe())),
    }
}

fn option(type_name: &str, ordinal: i32) -> Value {
    let members: std::sync::Arc<[String]> = crate::sysenum::option_members(type_name)
        .expect("a system option")
        .into();
    Value::Option(ordinal, members)
}

fn get_object_id(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_state(r, |s| Value::Integer(s.object_id))
}

fn get_object_type(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_state(r, |s| option("ObjectType", s.object_type))
}

fn get_result_code(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_state(r, |s| option("WebServiceActionResultCode", s.result_code))
}

/// A plain assignment, with no check that the id names an object
/// (session-system/TestWebServiceActionContext.al:279-290, :294-300 stores -7).
fn set_object_id(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let Value::Integer(id) = a[0] else {
        return unsupported(format!("SetObjectId with {}", a[0].describe()));
    };
    with_state(r, |s| s.object_id = id).map(|()| Value::Null)
}

fn set_object_type(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let Value::Option(o, _) = a[0] else {
        return unsupported(format!("SetObjectType with {}", a[0].describe()));
    };
    with_state(r, |s| s.object_type = o).map(|()| Value::Null)
}

/// `Updated` and `Get` are both 200, and the platform stores the code by value: `Updated` reads
/// back as `Get` (session-system/TestWebServiceActionContext.al:352-395).
fn set_result_code(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let Value::Option(o, members) = &a[0] else {
        return unsupported(format!("SetResultCode with {}", a[0].describe()));
    };
    let stored = if members.get(*o as usize).is_some_and(|m| m == "Updated") {
        members.iter().position(|m| m == "Get").unwrap_or(*o as usize) as i32
    } else {
        *o
    };
    with_state(r, |s| s.result_code = stored).map(|()| Value::Null)
}

/// The keys are a map by field id: a second add of the same id fails whatever the value is. The
/// result used, it is `false`; not used, an error that names the type
/// (session-system/TestWebServiceActionContext.al:437-505). The wording around the type name is not pinned.
fn add_entity_key(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let Value::Integer(field) = a[0] else {
        return unsupported(format!("AddEntityKey with field id {}", a[0].describe()));
    };
    let added = with_state(r, |s| {
        if s.keys.contains(&field) {
            false
        } else {
            s.keys.push(field);
            true
        }
    })?;
    if added {
        return Ok(Value::Boolean(true));
    }
    // A key for a field that already has one (also with the same value) is an invalid argument. With the result used
    // the call answers false and the text stays in GetLastErrorText; unused, it raises
    // (session/W5WsacDuplicateKey.al, BC 27.5 and 29.0).
    let error = crate::native::AlError {
        message: "An invalid argument was passed to a 'WebServiceActionContext' data type method.".into(),
        code: "Argument".into(),
        call_stack: String::new(),
        ..Default::default()
    };
    if c.ret_used() {
        c.set_last_error(error);
        return Ok(Value::Boolean(false));
    }
    Err(Signal::Error(Box::new(error)))
}

pub fn native_for(sig: &Signature) -> Option<NativeFn> {
    if sig.receiver != "WebServiceActionContext" {
        return None;
    }
    Some(match sig.name.as_str() {
        "GetObjectId" => get_object_id,
        "GetObjectType" => get_object_type,
        "GetResultCode" => get_result_code,
        "SetObjectId" => set_object_id,
        "SetObjectType" => set_object_type,
        "SetResultCode" => set_result_code,
        "AddEntityKey" => add_entity_key,
        _ => return None,
    })
}
