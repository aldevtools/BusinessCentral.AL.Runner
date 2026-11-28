//! `ErrorInfo`: a property bag describing an error (corpus errors/errorinfo/TestErrorInfoType.al,
//! TestErrorInfoErrorType.al). Raising one (`Error(ErrorInfo)`), collecting errors and the
//! callback of an action are not here: they need the error pipeline.
//!
//! ErrorInfo: the property bag, Error(ErrorInfo), collected errors. Entry points: `raise`, `copy_of`,
//! `native_for`.

use crate::describe::Describe;
use crate::native::{Ctx, NativeFn, Signal, unsupported};
use crate::signature::Signature;
use crate::value::{CollectedInternal, ErrorInfoBuilt, ErrorInfoState, LibraryValue, Value};
use std::cell::RefCell;
use std::rc::Rc;

type Out = Result<Value, Signal>;

/// An entry of `CALLSTACKS`: the cell of an `ErrorInfo` value and its `Callstack`.
type CallstackEntry = (std::rc::Weak<RefCell<LibraryValue>>, Rc<str>);

thread_local! {
    /// The `Callstack` of a collected error, by the cell of the `ErrorInfo` value that carries it (`set_callstack_of`; `copy_of`
    /// hands it on to the copy). A side table: the state struct of an `ErrorInfo` has no field for it. A `Weak` keeps its cell's
    /// address from being reused while the entry lives; dead entries go at the next insert.
    static CALLSTACKS: RefCell<Vec<CallstackEntry>> = const { RefCell::new(Vec::new()) };
}

/// The `Callstack` an `ErrorInfo` cell carries on this thread (`set_callstack_of`); `None`: none (an error not collected).
pub fn callstack_of(cell: &Rc<RefCell<LibraryValue>>) -> Option<Rc<str>> {
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

/// `Callstack`: empty until the error is raised (session-system/TestProbeSessionMiscContracts.al:ErrorInfo_Callstack_IsAvailable); a
/// collected error has the frames of its `Error` call, a CR LF after each, and on BC 27.5 the runner's frames after them
/// (session-system/callstack/CallstackFullText.al; al-vm `collected_call_stack`). A frame whose format is not pinned stands as
/// the untracked mark, so no text function reads past it.
fn callstack(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    let cell = match &r {
        Some(Value::Library(_, cell)) => Some(cell.clone()),
        _ => None,
    };
    with_state(r, |_| ())?;
    let stack = cell.and_then(|c| callstack_of(&c));
    Ok(Value::Text(stack.as_deref().unwrap_or_default().to_string().into()))
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
    // `Message` on one and raises it). It is not collectible (session-system/TestProbeSessionMiscContracts.al:
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

/// A write to the ErrorInfo: it is no longer built by `Create` alone (`ErrorInfoBuilt`).
fn set(r: Option<&mut Value>, f: impl FnOnce(&mut ErrorInfoState)) -> Out {
    with_state(r, |s| {
        f(s);
        s.built = ErrorInfoBuilt::Other;
    })
    .map(|()| Value::Null)
}

fn text(a: &[Value], i: usize) -> Result<String, Signal> {
    match a.get(i).and_then(crate::native::text_of) {
        Some(t) => Ok(t.to_string()),
        None => unsupported(format!("a text argument, got {}", a.get(i).describe())),
    }
}

/// A text that is only stored (a message, a title): a call stack text is carried (`text_of_carrying_call_stack`).
fn text_stored(a: &[Value], i: usize) -> Result<String, Signal> {
    match a.get(i).and_then(crate::native::text_of_carrying_call_stack) {
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
        // Verbosity::Error and DataClassification::CustomerContent (errors/errorinfo/TestErrorInfoType.al:30-34).
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
        actions: 0,
        built: ErrorInfoBuilt::Other,
        collected_internal: None,
    }
}

fn wrap(s: ErrorInfoState) -> Value {
    Value::Library("ErrorInfo".into(), Rc::new(RefCell::new(LibraryValue::ErrorInfo(s))))
}

/// `ErrorInfo.Create()` is collectible; `Create(Message[, Collectible, ...])` is not unless asked
/// (errors/errorinfo/TestErrorInfoType.al:21-60,:225-245).
fn create(ctx: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    if a.is_empty() {
        let mut s = fresh(String::new(), true);
        s.built = ErrorInfoBuilt::CreateNoMessage;
        return Ok(wrap(s));
    }
    let collectible = match a.get(1) {
        None => false,
        Some(Value::Boolean(b)) => *b,
        Some(other) => return unsupported(format!("ErrorInfo.Create with Collectible {}", other.describe())),
    };
    let mut s = fresh(text_stored(a, 0)?, collectible);
    if let Some(rec @ Value::Record(_)) = a.get_mut(2) {
        let id = ctx.record(crate::record_op::RecordOp::RecordId, Some(rec), &mut [])?;
        if let Value::Library(_, l) = &id {
            if let LibraryValue::RecordId { table_no, .. } = &*l.borrow() {
                s.table_id = *table_no as i32;
            }
        }
        s.record_id = Some(id);
        // The SystemId is the record's (errors/errorinfo/BcErrorInfoSystemId.al:69-99), which is not read here:
        // a read of it is refused.
        s.system_id = Some(Value::Unknown(std::sync::Arc::from(
            "the SystemId of an ErrorInfo created with a record (the record's SystemId is not carried over yet)",
        )));
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

/// `ErrorInfo.Create('literal')`: one string literal and no other argument (the lowering picks this id). It creates what
/// `Create(Message)` creates and records that it was built so.
pub(crate) fn create_literal(ctx: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let v = create(ctx, r, a)?;
    if let Value::Library(_, cell) = &v {
        if let LibraryValue::ErrorInfo(s) = &mut *cell.borrow_mut() {
            s.built = ErrorInfoBuilt::CreateLiteral;
        }
    }
    Ok(v)
}

macro_rules! text_prop {
    ($name:ident, $field:ident) => {
        fn $name(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
            if a.is_empty() {
                return with_state(r, |s| Value::Text(s.$field.clone().into()));
            }
            let t = text_stored(a, 0)?;
            set(r, |s| s.$field = t)
        }
    };
}
text_prop!(message_text, message);
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
            set(r, |s| s.$field = n)
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
            set(r, |s| s.$field = o)
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
    set(r, |s| s.collectible = b)
}

/// Both members round-trip (errors/errorinfo/TestErrorInfoErrorType.al:25-45); the default is Client
/// (record/ProbePropertyAssignmentForms.al:302-329, BC 27.5).
fn error_type(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    if a.is_empty() {
        return with_state(r, |s| option("ErrorType", s.error_type.unwrap_or(0)));
    }
    let o = option_arg(a, 0)?;
    set(r, |s| s.error_type = Some(o))
}

/// `Err.RecordId := Rid` reads back and leaves the TableId as it was (record/ProbePropertyAssignmentForms.al:352-373).
fn record_id(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    if let Some(id) = a.first() {
        let id = id.clone();
        return set(r, |s| s.record_id = Some(id));
    }
    Ok(with_state(r, |s| s.record_id.clone())?.unwrap_or_else(|| Value::Library("RecordId".into(), Rc::default())))
}

/// `Err.SystemId := G` reads back, a null Guid reads as null, and a fresh ErrorInfo holds a null Guid
/// (errors/errorinfo/EnTErrorInfoProps.al:42, :106-115, :259-262).
fn system_id(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    if let Some(g) = a.first() {
        let g = match g {
            Value::Guid(_) => g.clone(),
            other => return unsupported(format!("SystemId with {}", other.describe())),
        };
        return set(r, |s| s.system_id = Some(g));
    }
    Ok(with_state(r, |s| s.system_id.clone())?.unwrap_or(Value::Guid([0; 16])))
}

/// A fresh ErrorInfo carries no custom dimensions (errors/errorinfo/TestErrorInfoType.al:423-442). An assigned
/// dictionary is held, not copied: an entry added to it later shows through `CustomDimensions()`
/// (record/ProbePropertyAssignmentForms.al:332-349, BC 27.5). So is the empty one a read gives: `CustomDimensions.Add` on a
/// fresh ErrorInfo keeps the entry (errors/error-texts/ErrorMessage.al:225-227 `customdims after add`,
/// errors/PnBErrorsCollections.al:270 and :329-358 `dims=1:Dim1=Val1`).
fn custom_dimensions(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    if a.is_empty() {
        // The dictionary is handed out by reference: a later `Add` on it is a write to the ErrorInfo.
        return with_state(r, |s| {
            s.built = ErrorInfoBuilt::Other;
            s.custom_dimensions
                .get_or_insert_with(|| Value::Dictionary(Rc::new(RefCell::new(Vec::new()))))
                .clone()
        });
    }
    let Some(d @ Value::Dictionary(_)) = a.first() else {
        return unsupported(format!("CustomDimensions with {}", a.first().describe()));
    };
    let held = d.clone();
    set(r, |s| s.custom_dimensions = Some(held))
}

/// Actions are stored for the client; there is none in a test, so they only have to bind and not
/// disturb the object (errors/errorinfo/TestErrorInfoType.al:523-557).
fn add_action(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    text(a, 0)?;
    let codeunit = int(a, 1)?;
    let method = text(a, 2)?;
    if method.eq_ignore_ascii_case("Run") {
        add_run_action(c, codeunit, &method)?;
    }
    set(r, |s| s.actions += 1)
}

/// `AddAction` whose method is `Run` (assumption `errorinfo-add-action-run`, corpus PR 1601
/// EnTErrorInfoRunAction.al): on a codeunit of no Subtype whose OnRun trigger and TableNo are both there or both absent
/// it fails with the unknown-type error; codeunit 0 and an id no codeunit has take the action.
fn add_run_action(c: &dyn Ctx, codeunit: i32, method: &str) -> Result<(), Signal> {
    use crate::native::CodeunitShape;
    let spelled = method == "Run" || method == "run";
    let shape = u32::try_from(codeunit)
        .ok()
        .filter(|&id| id > 0)
        .map(|id| c.codeunit_shape(id));
    match (spelled, codeunit, shape) {
        (true, 0, _) | (true, _, Some(CodeunitShape::Missing)) => {}
        (true, _, Some(CodeunitShape::Normal)) => {
            crate::assumptions::assume(crate::assumptions::Assumption::ErrorInfoAddActionRun);
            return crate::native::raise(
                "The type Microsoft.Dynamics.Nav.Types.DataError is unknown.",
                "NotSupportedType",
            );
        }
        (true, _, Some(CodeunitShape::Other(why))) => {
            return unsupported(format!(
                "ErrorInfo.AddAction with the method '{method}' on codeunit {codeunit}, {why} (not measured)"
            ));
        }
        _ => {
            return unsupported(format!(
                "ErrorInfo.AddAction with the method '{method}' on codeunit {codeunit} (not measured)"
            ));
        }
    }
    crate::assumptions::assume(crate::assumptions::Assumption::ErrorInfoAddActionRun);
    Ok(())
}

fn add_navigation_action(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    set(r, |s| s.actions += 1)
}

/// `Error(ErrorInfo)`: raises the message as it is for a client error
/// (errors/errorinfo/TestErrorInfoErrorType.al:50-65). A collectible one is collected inside a collecting scope.
pub fn raise(c: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    let Some(info @ Value::Library(_, cell)) = a.first() else {
        return unsupported(format!("Error with {} as an ErrorInfo", a.first().describe()));
    };
    let (message, internal, collectible, shape_measured) = match &*cell.borrow() {
        LibraryValue::ErrorInfo(s) => (
            s.message.clone(),
            s.error_type == Some(1),
            s.collectible,
            internal_text_measured(s),
        ),
        _ => return unsupported("Error with an ErrorInfo nobody created"),
    };
    let measured = measured_contexts(c, cell, shape_measured);
    // An internal error hides its message behind a fixed text and a correlation id (session/
    // TestProbeSessionMiscContracts.al:ErrorInfo_ErrorClientAndInternal; the id changes from run to run). Collected, its Message
    // is that text with a line break before the id (errors/PnBErrorsCollections.al:329-358 `internal message`).
    let braced = internal.then(|| crate::pagefmt::guid_text(&c.new_guid()));
    // A collectible error inside a `[ErrorBehavior(Collect)]` scope is kept and execution goes on
    // (errors/errorinfo/TestErrorInfoType.al:448-459); a non-collectible one unwinds even there (:474-490).
    if collectible {
        let collected = match &braced {
            Some(braced) => {
                let copy = copy_of(info).expect("an ErrorInfo");
                let message = format!(
                    "{INTERNAL_ERROR_TEXT}{COLLECTED_ID_LABEL}{}",
                    shown_id(braced, measured.collected)
                );
                state_mut_of(&copy, |s| {
                    s.collected_internal = measured.collected.then(|| CollectedInternal {
                        message: message.clone(),
                        exception_message: raised_text(braced),
                    });
                    s.message = message;
                });
                copy
            }
            None => info.clone(),
        };
        if c.collect_error(&collected) {
            assume_measured(measured.collected);
            return Ok(Value::Null);
        }
    }
    // Raised, a collectible one is measured only inside a collecting scope (the return above).
    let measured = matches!((measured.raised, collectible), (true, false));
    assume_measured(measured);
    // GetLastErrorObject was not measured (assumption `errorinfo-internal-text`): its Message keeps main's braced id.
    let (message, exception_message) = match &braced {
        Some(braced) => (
            raised_text(shown_id(braced, measured)),
            measured.then(|| raised_text(braced)),
        ),
        None => (message, None),
    };
    // The flag's text for any classification, the default one, custom dimensions included (P86LastError.al:54, :88-104,
    // :121, :145, :169).
    Err(Signal::Error(Box::new(crate::native::AlError {
        message,
        code: "Dialog".into(),
        origin: crate::native::ErrorOrigin::Redacted(crate::native::PERSONAL_DATA_REMOVED.into()),
        exception_message,
        ..Default::default()
    })))
}

/// Where an Internal `ErrorInfo` of a measured shape shows the measured id (assumption `errorinfo-internal-text`, corpus PR
/// 1666 ErrorInfoInternalText.al:144-160, :339-389): never collected before (no collected call stack), and collected with
/// exactly one collecting scope open that holds no error now, or raised while no collecting scope holds an error.
struct Measured {
    collected: bool,
    raised: bool,
}

fn measured_contexts(c: &dyn Ctx, cell: &Rc<RefCell<LibraryValue>>, shape_measured: bool) -> Measured {
    let (open, held) = c.error_scopes_open_and_held();
    let never_collected = callstack_of(cell).is_none();
    Measured {
        collected: matches!((shape_measured, never_collected, open, held), (true, true, 1, 0)),
        raised: matches!((shape_measured, never_collected, held), (true, true, 0)),
    }
}

/// Whether an `ErrorInfo` is an Internal one of a shape the probe measured (assumption `errorinfo-internal-text`, corpus PR
/// 1666 errors/errorinfo/ErrorInfoInternalText.al): any Message and DetailedMessage, every other property as a fresh ErrorInfo
/// has it, and no action. Any other Internal shape keeps the braced id.
fn internal_text_measured(s: &ErrorInfoState) -> bool {
    s.error_type == Some(1)
        && s.title.is_empty()
        && s.control_name.is_empty()
        && s.verbosity == 1
        && s.data_classification == 0
        && s.table_id == 0
        && s.record_id.is_none()
        && s.system_id.is_none()
        && s.field_no == 0
        && s.page_no == 0
        && s.custom_dimensions.is_none()
        && s.actions == 0
}

/// The correlation id of an Internal error. A measured shape (assumption `errorinfo-internal-text`, corpus PR 1666
/// ErrorInfoInternalText.al:177-187, :352-362) shows 36 characters, lower case hex with dashes and no braces; any other
/// shape keeps the braced form.
fn shown_id(braced: &str, measured: bool) -> &str {
    if measured {
        braced.trim_matches(['{', '}'])
    } else {
        braced
    }
}

/// What comes between the generic text and the correlation id in the Message of a collected Internal error.
const COLLECTED_ID_LABEL: &str = "\r\nCorrelation ID: ";

/// The text an Internal error raises: the generic text, a backslash and its correlation id.
fn raised_text(id: &str) -> String {
    format!("{INTERNAL_ERROR_TEXT}\\Correlation ID: {id}")
}

/// Marks a measured Internal error as running through the assumption, where it is collected or raised.
fn assume_measured(measured: bool) {
    if measured {
        crate::assumptions::assume(crate::assumptions::Assumption::ErrorInfoInternalText);
    }
}

/// The text an Internal error shows instead of its message (session-system/TestProbeSessionMiscContracts.al:
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

/// The texts of the error a collecting scope raises when it ends holding one Internal error `held`, whose Message is
/// `collected`: the raised text has a backslash where the collected Message has a line break (errors/PnBErrorsCollections.al:
/// 360-375 RO_CollectedErrorInfo_Internal_NotCleared_ScopeExit; corpus PR 1666 ErrorInfoInternalText.al:365-389). The second
/// text is the Message of the exception `GetLastErrorObject` answers where it is not the raised text. GetLastErrorObject was
/// not measured (assumption `errorinfo-internal-text`): where `raise` collected `held` with the measured id and its Message is
/// still that one, it keeps main's braced id; any other held Message (one the test wrote, also through TestField) gets
/// main's answer.
pub fn internal_scope_end_texts(held: &Value, collected: &str) -> (String, Option<String>) {
    let stored = state_of(held, |s| s.collected_internal.clone()).flatten();
    let exception_message = stored.filter(|m| m.message == collected).map(|m| {
        crate::assumptions::assume(crate::assumptions::Assumption::ErrorInfoInternalText);
        m.exception_message
    });
    (collected.replace("\r\n", "\\"), exception_message)
}

/// `f` on the state of an `ErrorInfo` value; `None` for any other value.
fn state_of<R>(v: &Value, f: impl FnOnce(&ErrorInfoState) -> R) -> Option<R> {
    let Value::Library(_, cell) = v else { return None };
    match &*cell.borrow() {
        LibraryValue::ErrorInfo(s) => Some(f(s)),
        _ => None,
    }
}

/// `f` on the state of an `ErrorInfo` value, to change it; `None` for any other value.
fn state_mut_of<R>(v: &Value, f: impl FnOnce(&mut ErrorInfoState) -> R) -> Option<R> {
    let Value::Library(_, cell) = v else { return None };
    match &mut *cell.borrow_mut() {
        LibraryValue::ErrorInfo(s) => Some(f(s)),
        _ => None,
    }
}

/// Whether an `ErrorInfo` value has ErrorType Internal.
pub fn is_internal(v: &Value) -> bool {
    state_of(v, |s| s.error_type == Some(1)).unwrap_or(false)
}

/// Sets the message of an `ErrorInfo` value and answers whether it is collectible.
pub fn set_message_of(v: &Value, message: &str) -> bool {
    state_mut_of(v, |s| {
        s.message = message.to_string();
        s.collectible
    })
    .unwrap_or(false)
}

/// Sets the table and field an `ErrorInfo` value names (what `TestField` records on a collected error) and
/// answers whether it is collectible.
pub fn set_origin_of(v: &Value, table_id: i32, field_no: i32) -> bool {
    state_mut_of(v, |s| {
        s.table_id = table_id;
        s.field_no = field_no;
        s.collectible
    })
    .unwrap_or(false)
}

/// Sets the RecordId an `ErrorInfo` value names (what a collected `TestField` records).
pub fn set_record_id_of(v: &Value, record_id: Value) {
    state_mut_of(v, |s| s.record_id = Some(record_id));
}

/// The message of an `ErrorInfo` value.
pub fn message_of(v: &Value) -> Option<String> {
    state_of(v, |s| s.message.clone())
}

/// How an `ErrorInfo` value was built.
pub fn built_of(v: &Value) -> Option<ErrorInfoBuilt> {
    state_of(v, |s| s.built)
}

pub fn native_for(sig: &Signature) -> Option<NativeFn> {
    if sig.receiver != "ErrorInfo" {
        return None;
    }
    Some(match sig.name.as_str() {
        "Create" => create,
        "Message" => message_text,
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

#[cfg(test)]
mod tests;
