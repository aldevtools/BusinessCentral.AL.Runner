//! `ErrorInfo`: a property bag describing an error (corpus errorinfo/TestErrorInfoType.al,
//! TestErrorInfoErrorType.al). Raising one (`Error(ErrorInfo)`), collecting errors and the
//! callback of an action are not here: they need the error pipeline.

use crate::describe::Describe;
use crate::native::{Ctx, NativeFn, Signal};
use crate::signature::Signature;
use crate::value::{ErrorInfoState, LibraryValue, Value};
use std::cell::RefCell;
use std::rc::Rc;

type Out = Result<Value, Signal>;

thread_local! {
    /// The `Callstack` of a collected error, by the cell of the `ErrorInfo` value that carries it (`set_callstack_of`; `copy_of`
    /// hands it on to the copy). A side table: the state struct of an `ErrorInfo` has no field for it. A `Weak` keeps its cell's
    /// address from being reused while the entry lives; dead entries go at the next insert.
    static CALLSTACKS: RefCell<Vec<(std::rc::Weak<RefCell<LibraryValue>>, Rc<str>)>> = const { RefCell::new(Vec::new()) };
}

fn callstack_of(cell: &Rc<RefCell<LibraryValue>>) -> Option<Rc<str>> {
    CALLSTACKS.with_borrow(|t| {
        t.iter()
            .find(|(w, _)| std::ptr::eq(w.as_ptr(), Rc::as_ptr(cell)))
            .map(|(_, s)| s.clone())
    })
}

fn put_callstack(cell: &Rc<RefCell<LibraryValue>>, stack: Rc<str>) {
    CALLSTACKS.with_borrow_mut(|t| {
        t.retain(|(w, _)| w.strong_count() > 0);
        t.retain(|(w, _)| !std::ptr::eq(w.as_ptr(), Rc::as_ptr(cell)));
        t.push((Rc::downgrade(cell), stack));
    });
}

/// Sets what `Callstack` of an `ErrorInfo` value reads (a collected error: the frames of its `Error` call).
pub fn set_callstack_of(v: &Value, stack: &str) {
    if let Value::Library(_, cell) = v {
        put_callstack(cell, stack.into());
    }
}

/// `Callstack`: empty until the error is raised (session/TestProbeSessionMiscContracts.al:ErrorInfo_Callstack_IsAvailable); a
/// collected error has the frames of its `Error` call (misc/lt-probes-needed-2/PnBErrorsCollections.al:329-358).
fn callstack(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    let cell = match &r {
        Some(Value::Library(_, cell)) => Some(cell.clone()),
        _ => None,
    };
    with_state(r, |_| ())?;
    let stack = cell.and_then(|c| callstack_of(&c));
    Ok(Value::Text(stack.as_deref().unwrap_or_default().to_string().into()))
}

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

fn option(type_name: &str, ordinal: i32) -> Value {
    let members: std::sync::Arc<[String]> = crate::sysenum::option_members(type_name)
        .expect("a system option")
        .into();
    Value::Option(ordinal, members)
}

fn with_state<R>(r: Option<&mut Value>, f: impl FnOnce(&mut ErrorInfoState) -> R) -> Result<R, Signal> {
    let Some(Value::Library(_, cell)) = r else {
        return unsupported("ErrorInfo method without a receiver");
    };
    let mut held = cell.borrow_mut();
    // A declared but never created ErrorInfo takes a property and gives it back
    // (types/TestHttpRequestFileUploadFieldErrorContracts.al:FieldError_WithErrorInfoAndWithText sets
    // `Message` on one and raises it). It is not collectible (session/TestProbeSessionMiscContracts.al:
    // ErrorInfo_UnsetAndOrdinals).
    if matches!(&*held, LibraryValue::Empty) {
        *held = LibraryValue::ErrorInfo(fresh(String::new(), false));
    }
    match &mut *held {
        LibraryValue::ErrorInfo(s) => Ok(f(s)),
        other => unsupported(format!(
            "ErrorInfo method on an ErrorInfo nobody created ({})",
            other.describe()
        )),
    }
}

fn text(a: &[Value], i: usize) -> Result<String, Signal> {
    match a.get(i).and_then(crate::native::text_of) {
        Some(t) => Ok(t.to_string()),
        None => unsupported(format!("a text argument, got {}", a.get(i).describe())),
    }
}

fn int(a: &[Value], i: usize) -> Result<i32, Signal> {
    match a.get(i) {
        Some(Value::Integer(n)) => Ok(*n),
        other => unsupported(format!("an Integer argument, got {}", other.describe())),
    }
}

fn option_arg(a: &[Value], i: usize) -> Result<i32, Signal> {
    match a.get(i) {
        Some(Value::Option(o, _)) => Ok(*o),
        other => unsupported(format!("an option argument, got {}", other.describe())),
    }
}

fn fresh(message: String, collectible: bool) -> ErrorInfoState {
    ErrorInfoState {
        message,
        title: String::new(),
        detailed_message: String::new(),
        control_name: String::new(),
        // Verbosity::Error and DataClassification::CustomerContent (errorinfo/TestErrorInfoType.al:30-34).
        verbosity: 1,
        data_classification: 0,
        error_type: None,
        collectible,
        table_id: 0,
        record_id: None,
        system_id: None,
        field_no: 0,
        page_no: 0,
        custom_dimensions: None,
    }
}

fn wrap(s: ErrorInfoState) -> Value {
    Value::Library("ErrorInfo".into(), Rc::new(RefCell::new(LibraryValue::ErrorInfo(s))))
}

/// `ErrorInfo.Create()` is collectible; `Create(Message[, Collectible, ...])` is not unless asked
/// (errorinfo/TestErrorInfoType.al:21-60,:225-245).
fn create(ctx: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    if a.is_empty() {
        return Ok(wrap(fresh(String::new(), true)));
    }
    let collectible = match a.get(1) {
        None => false,
        Some(Value::Boolean(b)) => *b,
        Some(other) => return unsupported(format!("ErrorInfo.Create with Collectible {}", other.describe())),
    };
    let mut s = fresh(text(a, 0)?, collectible);
    if let Some(rec @ Value::Record(_)) = a.get_mut(2) {
        let id = ctx.record(crate::record_op::RecordOp::RecordId, Some(rec), &mut [])?;
        if let Value::Library(_, l) = &id {
            if let LibraryValue::RecordId { table_no, .. } = &*l.borrow() {
                s.table_id = *table_no as i32;
            }
        }
        s.record_id = Some(id);
    } else if a.get(2).is_some() {
        return unsupported("ErrorInfo.Create with a record argument that is not a record");
    }
    if a.len() > 3 {
        s.field_no = int(a, 3)?;
    }
    if a.len() > 4 {
        s.page_no = int(a, 4)?;
    }
    if a.len() > 5 {
        s.control_name = text(a, 5)?;
    }
    if a.len() > 6 {
        s.verbosity = option_arg(a, 6)?;
    }
    if a.len() > 7 {
        s.data_classification = option_arg(a, 7)?;
    }
    if a.len() > 8 {
        return unsupported("ErrorInfo.Create with custom dimensions (not pinned)");
    }
    Ok(wrap(s))
}

macro_rules! text_prop {
    ($name:ident, $field:ident) => {
        fn $name(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
            if a.is_empty() {
                return with_state(r, |s| Value::Text(s.$field.clone().into()));
            }
            let t = text(a, 0)?;
            with_state(r, |s| s.$field = t).map(|()| Value::Null)
        }
    };
}
text_prop!(message, message);
text_prop!(title, title);
text_prop!(detailed_message, detailed_message);
text_prop!(control_name, control_name);

macro_rules! int_prop {
    ($name:ident, $field:ident) => {
        fn $name(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
            if a.is_empty() {
                return with_state(r, |s| Value::Integer(s.$field));
            }
            let n = int(a, 0)?;
            with_state(r, |s| s.$field = n).map(|()| Value::Null)
        }
    };
}
int_prop!(field_no, field_no);
int_prop!(page_no, page_no);
int_prop!(table_id, table_id);

macro_rules! option_prop {
    ($name:ident, $field:ident, $ty:literal) => {
        fn $name(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
            if a.is_empty() {
                return with_state(r, |s| option($ty, s.$field));
            }
            let o = option_arg(a, 0)?;
            with_state(r, |s| s.$field = o).map(|()| Value::Null)
        }
    };
}
option_prop!(verbosity, verbosity, "Verbosity");
option_prop!(data_classification, data_classification, "DataClassification");

fn collectible(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    if a.is_empty() {
        return with_state(r, |s| Value::Boolean(s.collectible));
    }
    let Some(Value::Boolean(b)) = a.first() else {
        return unsupported(format!("Collectible with {}", a.first().describe()));
    };
    let b = *b;
    with_state(r, |s| s.collectible = b).map(|()| Value::Null)
}

/// Both members round-trip (errorinfo/TestErrorInfoErrorType.al:25-45); the default is Client
/// (record/ProbePropertyAssignmentForms.al:302-329, BC 27.5).
fn error_type(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    if a.is_empty() {
        return with_state(r, |s| option("ErrorType", s.error_type.unwrap_or(0)));
    }
    let o = option_arg(a, 0)?;
    with_state(r, |s| s.error_type = Some(o)).map(|()| Value::Null)
}

/// `Err.RecordId := Rid` reads back and leaves the TableId as it was (record/ProbePropertyAssignmentForms.al:352-373).
fn record_id(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    if let Some(id) = a.first() {
        let id = id.clone();
        return with_state(r, |s| s.record_id = Some(id)).map(|()| Value::Null);
    }
    Ok(with_state(r, |s| s.record_id.clone())?.unwrap_or_else(|| Value::Library("RecordId".into(), Rc::default())))
}

/// `Err.SystemId := G` reads back, a null Guid reads as null, and a fresh ErrorInfo holds a null Guid
/// (errorinfo/lt-errnotif/EnTErrorInfoProps.al:42, :106-115, :259-262).
fn system_id(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    if let Some(g) = a.first() {
        let g = match g {
            Value::Guid(_) => g.clone(),
            other => return unsupported(format!("SystemId with {}", other.describe())),
        };
        return with_state(r, |s| s.system_id = Some(g)).map(|()| Value::Null);
    }
    Ok(with_state(r, |s| s.system_id.clone())?.unwrap_or(Value::Guid([0; 16])))
}

/// A fresh ErrorInfo carries no custom dimensions (errorinfo/TestErrorInfoType.al:423-442). An assigned
/// dictionary is held, not copied: an entry added to it later shows through `CustomDimensions()`
/// (record/ProbePropertyAssignmentForms.al:332-349, BC 27.5). So is the empty one a read gives: `CustomDimensions.Add` on a
/// fresh ErrorInfo keeps the entry (misc/lt-system-functions/ErrorMessage.al:225-227 `customdims after add`,
/// misc/lt-probes-needed-2/PnBErrorsCollections.al:270 and :329-358 `dims=1:Dim1=Val1`).
fn custom_dimensions(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    if a.is_empty() {
        return with_state(r, |s| {
            s.custom_dimensions
                .get_or_insert_with(|| Value::Dictionary(Rc::new(RefCell::new(Vec::new()))))
                .clone()
        });
    }
    let Some(d @ Value::Dictionary(_)) = a.first() else {
        return unsupported(format!("CustomDimensions with {}", a.first().describe()));
    };
    let held = d.clone();
    with_state(r, |s| s.custom_dimensions = Some(held)).map(|()| Value::Null)
}

/// Actions are stored for the client; there is none in a test, so they only have to bind and not
/// disturb the object (errorinfo/TestErrorInfoType.al:523-557).
fn add_action(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    text(a, 0)?;
    int(a, 1)?;
    text(a, 2)?;
    with_state(r, |_| ()).map(|()| Value::Null)
}

fn add_navigation_action(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_state(r, |_| ()).map(|()| Value::Null)
}

/// `Error(ErrorInfo)`: raises the message as it is for a client error
/// (errorinfo/TestErrorInfoErrorType.al:50-65). A collectible one is collected inside a collecting scope.
pub fn raise(c: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    let Some(info @ Value::Library(_, cell)) = a.first() else {
        return unsupported(format!("Error with {} as an ErrorInfo", a.first().describe()));
    };
    let (message, internal, collectible) = match &*cell.borrow() {
        LibraryValue::ErrorInfo(s) => (s.message.clone(), s.error_type == Some(1), s.collectible),
        _ => return unsupported("Error with an ErrorInfo nobody created"),
    };
    // An internal error hides its message behind a fixed text and a correlation id (session/
    // TestProbeSessionMiscContracts.al:ErrorInfo_ErrorClientAndInternal; the id changes from run to run). Collected, its Message
    // is that text with a line break before the id (misc/lt-probes-needed-2/PnBErrorsCollections.al:329-358 `internal message`).
    let (message, collected_message) = if internal {
        let g = c.new_guid();
        let id = crate::pagefmt::guid_text(&g);
        (
            format!("{INTERNAL_ERROR_TEXT}\\Correlation ID: {id}"),
            Some(format!("{INTERNAL_ERROR_TEXT}\r\nCorrelation ID: {id}")),
        )
    } else {
        (message, None)
    };
    // A collectible error inside a `[ErrorBehavior(Collect)]` scope is kept and execution goes on
    // (errorinfo/TestErrorInfoType.al:448-459); a non-collectible one unwinds even there (:474-490).
    if collectible {
        let collected = match &collected_message {
            Some(m) => {
                let copy = copy_of(info).expect("an ErrorInfo");
                set_message_of(&copy, m);
                copy
            }
            None => info.clone(),
        };
        if c.collect_error(&collected) {
            return Ok(Value::Null);
        }
    }
    // The flag's text for any classification, the default one, custom dimensions included (P86LastError.al:54, :88-104,
    // :121, :145, :169).
    Err(Signal::Error(Box::new(crate::native::AlError {
        message,
        code: "Dialog".into(),
        origin: crate::native::ErrorOrigin::Redacted(crate::native::PERSONAL_DATA_REMOVED.into()),
        ..Default::default()
    })))
}

/// The text an Internal error shows instead of its message (session/TestProbeSessionMiscContracts.al:
/// ErrorInfo_ErrorClientAndInternal).
pub const INTERNAL_ERROR_TEXT: &str =
    "An error has occurred. If the error persists, please contact your system administrator.";

/// A copy of an `ErrorInfo` value that shares nothing with the original, its `Callstack` included.
pub fn copy_of(v: &Value) -> Option<Value> {
    let Value::Library(name, cell) = v else { return None };
    let copy = match &*cell.borrow() {
        LibraryValue::ErrorInfo(s) => Rc::new(RefCell::new(LibraryValue::ErrorInfo(s.clone()))),
        _ => return None,
    };
    if let Some(stack) = callstack_of(cell) {
        put_callstack(&copy, stack);
    }
    Some(Value::Library(name.clone(), copy))
}

/// Whether an `ErrorInfo` value has ErrorType Internal.
pub fn is_internal(v: &Value) -> bool {
    let Value::Library(_, cell) = v else { return false };
    matches!(&*cell.borrow(), LibraryValue::ErrorInfo(s) if s.error_type == Some(1))
}

/// Sets the message of an `ErrorInfo` value and answers whether it is collectible.
pub fn set_message_of(v: &Value, message: &str) -> bool {
    let Value::Library(_, cell) = v else { return false };
    match &mut *cell.borrow_mut() {
        LibraryValue::ErrorInfo(s) => {
            s.message = message.to_string();
            s.collectible
        }
        _ => false,
    }
}

/// Sets the table and field an `ErrorInfo` value names (what `TestField` records on a collected error) and
/// answers whether it is collectible.
pub fn set_origin_of(v: &Value, table_id: i32, field_no: i32) -> bool {
    let Value::Library(_, cell) = v else { return false };
    match &mut *cell.borrow_mut() {
        LibraryValue::ErrorInfo(s) => {
            s.table_id = table_id;
            s.field_no = field_no;
            s.collectible
        }
        _ => false,
    }
}

/// The message of an `ErrorInfo` value.
pub fn message_of(v: &Value) -> Option<String> {
    let Value::Library(_, cell) = v else { return None };
    match &*cell.borrow() {
        LibraryValue::ErrorInfo(s) => Some(s.message.clone()),
        _ => None,
    }
}

pub fn native_for(sig: &Signature) -> Option<NativeFn> {
    if sig.receiver != "ErrorInfo" {
        return None;
    }
    Some(match sig.name.as_str() {
        "Create" => create,
        "Message" => message,
        "Title" => title,
        "DetailedMessage" => detailed_message,
        "ControlName" => control_name,
        "FieldNo" => field_no,
        "PageNo" => page_no,
        "TableId" => table_id,
        "Verbosity" => verbosity,
        "DataClassification" => data_classification,
        "Collectible" => collectible,
        "ErrorType" => error_type,
        "RecordId" => record_id,
        "SystemId" => system_id,
        "CustomDimensions" => custom_dimensions,
        "AddAction" => add_action,
        "AddNavigationAction" => add_navigation_action,
        "Callstack" => callstack,
        _ => return None,
    })
}
