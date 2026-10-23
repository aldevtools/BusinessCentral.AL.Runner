//! Native builtin implementations, registered by catalog overload id (ADR-008).

use crate::describe::Describe;
use crate::format::{FormatCtx, FormatFail, format_value, try_format};
use crate::record_op::RecordOp;
use crate::value::{LibraryValue, Value};
use std::rc::Rc;

/// The error state AL code can read back (`GetLastErrorText`, ...).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AlError {
    pub message: String,
    pub code: String,
    pub call_stack: String,
    /// How the error was raised, which `GetLastErrorText(true)` needs.
    pub origin: ErrorOrigin,
    /// The .NET exception BC holds for the error, which `GetLastErrorObject` needs.
    pub exception: ExceptionKind,
}

/// The .NET exception type behind an error, for `GetLastErrorObject`. Only the kinds the corpus pins are named
/// (tests/al-language-onprem-dotnet/lt-dotnet-interop/GetLastErrorObject.al); an error raised any other way is
/// `Unknown` and `GetLastErrorObject` refuses it, naming the code.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ExceptionKind {
    #[default]
    Unknown,
    /// `Error('literal', args)`: NavNCLDialogException, code `Dialog` (GetLastErrorObject.al:138-230).
    Dialog,
    /// `Error(Text, ...)` with a text that is not a literal (a Label, a variable): assumed the same exception.
    DialogComputed,
    /// `Get` without a row: NavCSideRecordNotFoundException, code `DB:RecordNotFound` (GetLastErrorObject.al:232-255).
    RecordNotFound,
    /// A division by zero: NavALException with an inner exception, code `AL:DivideByZero` (GetLastErrorObject.al:257-280).
    DivideByZero,
    /// Set by `ClearLastError`: no error, as opposed to the default of a session where no error has happened yet.
    Cleared,
}

/// How an error was raised, for the text `GetLastErrorText(true)` answers (corpus
/// types/lt-evaluate-format-lasterror-setview/P86LastError.al, errors/lt-last-error-and-events/P70_02LastErrFlag.al,
/// misc/lt-probes-needed-6/PnF01LastErrorFlag.al). An origin the corpus does not pin is `Unknown` and refused.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ErrorOrigin {
    #[default]
    Unknown,
    /// The flag leaves the text as it is (a runtime error without customer content: a division by zero, an empty table).
    SameText,
    /// The text the flag answers instead (the personal data removed from it).
    Redacted(String),
}

/// The text of `Error(ErrorInfo)` for the flag (P86LastError.al:54, :88-104, :121, :145, :169).
pub const PERSONAL_DATA_REMOVED: &str = "This message had personal data removed. The original may still be in transient telemetry. Find it using the transientTelemetryId.";

/// The text of `Error('literal', args)` for the flag (P86LastError.al:189, PnF01LastErrorFlag.al:85, :103).
pub const USE_ERROR_WITH_A_TEXT_CONSTANT: &str = "Use ERROR with a text constant to improve telemetry details";

/// Raises an AL error with a pinned text and code.
pub fn raise<T>(message: impl Into<String>, code: &str) -> Result<T, Signal> {
    // A division by zero carries no customer content: the flag leaves its text (P86LastError.al:RuntimeError_Other `div_excl`).
    let (origin, exception) = if code == "AL:DivideByZero" {
        (ErrorOrigin::SameText, ExceptionKind::DivideByZero)
    } else {
        (ErrorOrigin::Unknown, ExceptionKind::Unknown)
    };
    Err(Signal::Error(Box::new(AlError {
        message: message.into(),
        code: code.into(),
        origin,
        exception,
        ..Default::default()
    })))
}

/// Non-local control flow (ADR-002, ADR-011).
#[derive(Debug, Clone)]
pub enum Signal {
    Error(Box<AlError>),
    Exit(Option<Value>),
    Break,
    /// `continue`: go on with the next round of the innermost loop.
    Continue,
    /// Behaviour the runtime does not implement yet, met at run time (an edge
    /// case no corpus test pins). The runner reports the test as not run.
    Unsupported(String),
}

/// An app's name and its packaged resources (name relative to the resource folder, content).
pub type AppResources = (String, Vec<(String, Vec<u8>)>);

/// What `NavApp.GetModuleInfo` finds.
pub enum ModuleLookup {
    Found(crate::value::ModuleInfoState),
    /// No app of the workspace has this id and none of its manifests names it.
    NotInstalled,
    /// An app a manifest names as a dependency, which is not part of the workspace.
    OutsideWorkspace,
}

/// What builtins need from the running session.
/// A request an `HttpClient` call sends, as a `[HttpClientHandler]` sees it (ADR-027 section 5).
#[derive(Debug, Clone)]
pub struct HttpOutgoing {
    /// `GET`, `POST`, `PUT`, `DELETE`, `PATCH`, `HEAD` or the request's own method, upper case.
    pub method: String,
    /// The absolute uri in its normal form; for a relative path with no base address (or a request with no uri) the path as
    /// written, which only a request that no handler sees can carry (network/lt-http-relative-request/HttpRelativeRequestPath.al).
    pub url: String,
    pub has_secret_uri: bool,
    /// The uri came from joining a path to the client's base address in a form the corpus does not pin (registry entry
    /// http-relative-request-path).
    pub unpinned_join: bool,
}

/// The field an `Evaluate` into a `FieldRef` writes (see [`Ctx::evaluate_field_target`]).
pub struct FieldTarget {
    pub rec: std::rc::Rc<std::cell::RefCell<crate::value::RecordState>>,
    pub pos: usize,
    pub ty: crate::value::RtType,
    pub option_caption: bool,
}

pub trait Ctx: FormatCtx {
    /// Gives `request` to the running test's `[HttpClientHandler]` and answers the response it filled; raises the
    /// policy error without one. No request ever reaches a network.
    fn http_send(&mut self, _request: HttpOutgoing) -> Result<crate::value::TestHttpResponseState, Signal> {
        Err(Signal::Unsupported("an HttpClient call needs a session".into()))
    }

    fn last_error(&self) -> AlError;
    fn set_last_error(&mut self, e: AlError);
    /// Whether the run has the newer behaviour of `gate` (ADR-015 3.4); a context with no target has every gate.
    fn gate(&self, _gate: al_version::Gate) -> bool {
        true
    }
    /// Record methods (`Record.Insert`, ...) need the store, triggers and
    /// table metadata: the VM implements them. `method` is the catalog name.
    fn record(
        &mut self,
        op: crate::record_op::RecordOp,
        rec: Option<&mut Value>,
        args: &mut [Value],
    ) -> Result<Value, Signal>;
    /// `TestPage` / `TestField` / `TestAction` methods: the VM's test client
    /// implements them (ADR-012). `method` is `Receiver.Name`.
    fn page(
        &mut self,
        method: crate::page_op::PageOp,
        _recv: Option<&mut Value>,
        _args: &mut [Value],
    ) -> Result<Value, Signal> {
        Err(Signal::Unsupported(format!("{method} needs a page runtime")))
    }

    /// Set around a `TestField.SetValue` whose argument is written as a Variant (lowering picks
    /// `TestField.SetValueVariant#1`): BC reads a Text inside a Variant differently on an Enum or Option control.
    fn set_value_from_variant(&mut self, _on: bool) {}

    /// `Query` variable methods: the VM executes the query (`al-vm/src/queries.rs`).
    /// `method` is the catalog name (`Open`, `SetRange`, ...).
    fn query(&mut self, method: &str, _recv: Option<&mut Value>, _args: &mut [Value]) -> Result<Value, Signal> {
        Err(Signal::Unsupported(format!("Query.{method} needs a query runtime")))
    }

    /// `RecordRef`, `FieldRef` and `KeyRef` methods need the table schema and
    /// the store, like record methods. `id` is the catalog overload id
    /// (`RecordRef.Open#2`); the VM resolves it to the method.
    fn ref_op(&mut self, id: &str, _receiver: Option<&mut Value>, _args: &mut [Value]) -> Result<Value, Signal> {
        Err(Signal::Unsupported(format!("builtin {id}")))
    }
    /// Today's day number (1 = 0001-01-01) on the host's clock; `None` where there is no host.
    fn today_day(&self) -> Option<i32> {
        None
    }
    /// `CodeCoverageRefresh()`: writes what was recorded into the Code Coverage tables.
    fn code_coverage_refresh(&mut self) -> Result<(), Signal> {
        Ok(())
    }
    /// The session's work date as a day number; `None` where there is no host.
    fn work_date_day(&self) -> Option<i32> {
        None
    }
    /// The host clock in milliseconds since the Unix epoch; `None` where there is no host.
    fn now_unix_millis(&self) -> Option<i64> {
        None
    }
    /// `Format` of a record variable, a `RecordRef` or a `FieldRef` (standard format 0): needs the
    /// table's fields. `None` where the context has no program.
    fn format_composite(&mut self, _v: &Value) -> Option<Result<String, Signal>> {
        None
    }
    /// `Format` of a `DotNet` object or enum: its `ToString()` (ADR-024). `None` where the context
    /// has no .NET host.
    fn format_dotnet(&mut self, _v: &Value) -> Option<Result<String, Signal>> {
        None
    }
    /// The places `min:max` the AutoFormat of the field numbered `field_no` of `record` gives (its AutoFormatExpression evaluated with the
    /// record as Rec), for `Format` in code. Refused where the context has no session.
    fn auto_format_field_places(&mut self, _record: &Value, _field_no: usize) -> Result<String, Signal> {
        Err(Signal::Unsupported(
            "Format of a field with an AutoFormatExpression outside a session".into(),
        ))
    }
    /// `CopyStream` where one side is a `DotNet` stream (ADR-024). `None` where the context has no
    /// .NET host.
    fn copy_stream_dotnet(&mut self, _out: &Value, _src: &Value, _count: Option<i64>) -> Option<Result<(), Signal>> {
        None
    }
    /// `Evaluate(RecordId, Text)`: the RecordId the text names (a table name, then `:` and the key), `Ok(None)` where no table
    /// is named or the key is not a value of its type. `None` where the context has no program.
    fn evaluate_record_id(
        &mut self,
        _text: &str,
        _number: i64,
    ) -> Option<Result<Option<crate::value::LibraryValue>, Signal>> {
        None
    }
    /// The field a `FieldRef` stands on, for `Evaluate` into it: its record buffer, position, declared type and whether the field
    /// declares an OptionCaption. `None` where the context has no program or `v` is no assigned `FieldRef`.
    fn evaluate_field_target(&mut self, _v: &Value) -> Option<Result<FieldTarget, Signal>> {
        None
    }
    /// The declared values of enum object `id` as (ordinal, name), in declaration order with
    /// extension values after them. `None` when the context has no program.
    fn enum_members(&self, _id: u32) -> Option<Vec<(i32, String)>> {
        None
    }
    /// The captions the values of enum object `id` declare, as (ordinal, caption).
    fn enum_captions(&self, _id: u32) -> Vec<(i32, String)> {
        Vec::new()
    }
    /// Builtins that read or change session state (number sequences, the transaction state): the
    /// VM implements them. `id` is the catalog overload id (`NumberSequence.Next#1`); `args` holds
    /// the call's arguments, with `var` parameters written back in place.
    fn session_op(&mut self, id: crate::builtin_id::BuiltinId, _args: &mut [Value]) -> Result<Value, Signal> {
        Err(Signal::Unsupported(format!("builtin {id} needs a session")))
    }
    /// The language id, locale id and Windows time zone name of the running session, which
    /// `SessionSettings.Init` reads. `None` where the context has no host.
    fn session_locale(&self) -> Option<(i32, i32, String)> {
        None
    }
    /// The profile id, the profile's app id and the company of the session user's User Personalization row, which
    /// `SessionSettings.Init` reads (session/W6UserPersonalization.al). `None` where the context has none.
    fn session_profile(&self) -> Option<(String, [u8; 16], String)> {
        None
    }
    /// The app of the routine `depth` frames out on the call stack: 0 is the routine that runs the
    /// builtin, 1 its caller. `None` when there is no such frame (the caller of a test is the test
    /// runner) or the app's manifest has no readable id.
    fn module_at_frame(&self, _depth: usize) -> Option<crate::value::ModuleInfoState> {
        None
    }
    /// The distinct apps of the frames on the call stack, innermost first, leaving out the
    /// `skip` innermost frames.
    fn module_callstack(&self, _skip: usize) -> Vec<crate::value::ModuleInfoState> {
        Vec::new()
    }
    /// The publisher of the app whose code called into the innermost frame's app: the first frame, from the
    /// innermost, that belongs to another app. `None` when every frame belongs to that app.
    fn calling_app_publisher(&self) -> Option<String> {
        None
    }
    /// The frames of the test runner below the test, in `Callstack` form (backslash after each
    /// frame); empty where the context has no runner (al-vm/callstack.rs).
    fn runner_call_stack(&self) -> String {
        String::new()
    }
    /// The name and packaged resources of the app that owns the running code.
    fn app_resources(&self) -> Option<AppResources> {
        None
    }
    /// The module BC answers `GetModuleInfo` with for the null app id: the installed application.
    fn application_module(&self) -> crate::value::ModuleInfoState {
        crate::value::ModuleInfoState {
            id: [0; 16],
            name: String::new(),
            publisher: String::new(),
            version: None,
            dependencies: Vec::new(),
            synthesized: true,
        }
    }
    /// The installed app with this id.
    fn module_by_id(&self, _id: &[u8; 16]) -> ModuleLookup {
        ModuleLookup::NotInstalled
    }
    /// Whether table `number` (`Database::"Name"`) is declared in the workspace.
    fn workspace_table(&self, _number: i32) -> bool {
        false
    }
    /// Whether an install codeunit is running (DataTransfer works there and only there).
    fn install_scope(&self) -> bool {
        false
    }
    /// Whether a test codeunit is being run by the test runner (under TestIsolation = Codeunit).
    fn in_test_run(&self) -> bool {
        false
    }
    /// Whether the test codeunit runs under its RequiredTestIsolation = Function or Disabled, as BC's TestRunnerHub runs
    /// it, instead of under a test runner codeunit's TestIsolation.
    fn isolation_from_hub(&self) -> bool {
        false
    }
    /// `Error(ErrorInfo)` of a collectible error: inside a `[ErrorBehavior(Collect)]` scope the
    /// session keeps a copy and the call returns normally (`true`); outside one it is raised like
    /// any other error (`false`).
    fn collect_error(&mut self, _info: &Value) -> bool {
        false
    }
    /// The session's media objects and media sets (`Media` and `MediaSet` fields).
    fn media_store(&mut self) -> Option<&mut crate::media::MediaStore> {
        None
    }
    /// Whether the host file system has a file with this path (`File` data type).
    fn fs_exists(&mut self, _path: &str) -> Result<bool, Signal> {
        Err(Signal::Unsupported("File: this context has no file system".into()))
    }
    /// The bytes of the file at `path`, `None` if there is none.
    fn fs_read(&mut self, _path: &str) -> Result<Option<Vec<u8>>, Signal> {
        Err(Signal::Unsupported("File: this context has no file system".into()))
    }
    /// Creates the file at `path` or replaces its content; the host's reason if it cannot.
    fn fs_write(&mut self, _path: &str, _bytes: &[u8]) -> Result<(), String> {
        Err("this context has no file system".into())
    }
    /// Deletes the file at `path`; `false` if there was none.
    fn fs_remove(&mut self, _path: &str) -> Result<bool, Signal> {
        Err(Signal::Unsupported("File: this context has no file system".into()))
    }
    /// Creates the file at `path` with content the runtime does not know (`mark` names why, as for a stream: ADR-040): the
    /// file exists, and any look at it is refused until AL code writes it again.
    fn fs_write_unproduced(&mut self, _path: &str, _mark: &str) -> Result<(), String> {
        Err("this context has no file system".into())
    }
    /// Whether `path` lies in the company build's file system, which also answers folder questions (al-host
    /// `FileSystem::serves_path`; owner rule 2026-10-22: folder access only inside the company build).
    fn fs_serves_path(&self, _path: &str) -> bool {
        false
    }
    /// Whether the folder exists, for a path [`Ctx::fs_serves_path`] accepts.
    fn fs_dir_exists(&self, _path: &str) -> bool {
        false
    }
    /// A new name for a temporary file, under [`crate::file::TEMP_DIR`].
    fn fs_temp_name(&mut self) -> String {
        format!("{}0.tmp", crate::file::TEMP_DIR)
    }
    /// `Commit()`.
    fn commit(&mut self) -> Result<(), Signal>;
    /// A new, unique Guid (from the host or a deterministic sequence, ADR-006).
    fn new_guid(&mut self) -> [u8; 16];
    /// Whether the result of the builtin being called is used (`[Ok :=]` forms
    /// return false instead of raising when it is).
    fn ret_used(&self) -> bool {
        true
    }
    /// `Message(text)` after argument substitution. The runner decides what a
    /// message does (a test handler, an "Unhandled UI" refusal); a context
    /// with no UI refuses explicitly.
    fn ui_message(&mut self, _text: String) -> Result<(), Signal> {
        Err(Signal::Unsupported("Message: this context has no UI".into()))
    }
    /// `Confirm(question, default)`: the question is NOT substituted
    /// (corpus handlers/TestConfirmFormatArgs.al).
    fn ui_confirm(&mut self, _question: String, _default: bool) -> Result<bool, Signal> {
        Err(Signal::Unsupported("Confirm: this context has no UI".into()))
    }
    /// `Notification.Send()`: hands a copy of the notification (with its id) to the bound
    /// `[SendNotificationHandler]`, if there is one, and answers the notification as the handler
    /// left it. With no handler nothing is shown and `None` comes back.
    fn ui_send_notification(&mut self, _notification: Value, _id: [u8; 16]) -> Result<Option<Value>, Signal> {
        Err(Signal::Unsupported("Notification.Send: this context has no UI".into()))
    }
    /// `Notification.Recall()` of the notification with this id: false when it was never sent.
    /// A bound `[RecallNotificationHandler]` gets a copy of the notification.
    fn ui_recall_notification(&mut self, _notification: Value, _id: [u8; 16]) -> Result<bool, Signal> {
        Err(Signal::Unsupported(
            "Notification.Recall: this context has no UI".into(),
        ))
    }
    /// `SessionSettings.RequestSessionUpdate(false)`: hands a copy of the settings to the bound
    /// `[SessionSettingsHandler]`.
    /// Answers the settings as the handler left them.
    fn ui_session_settings(&mut self, _settings: Value) -> Result<Option<Value>, Signal> {
        Err(Signal::Unsupported(
            "SessionSettings.RequestSessionUpdate: this context has no UI".into(),
        ))
    }
    /// `StrMenu(options, default, instruction)`: the chosen option, 1-based, 0 for none.
    fn ui_str_menu(&mut self, _options: String, _default: i32, _instruction: String) -> Result<i32, Signal> {
        Err(Signal::Unsupported("StrMenu: this context has no UI".into()))
    }
}

pub type NativeFn = fn(&mut dyn Ctx, Option<&mut Value>, &mut [Value]) -> Result<Value, Signal>;

const REGISTRY: &[(&str, NativeFn)] = &[
    ("System.Format#1", format_1),
    ("System.Format#2", format_2),
    ("System.FormatVariant#1", format_variant),
    ("TextBuilder.AppendVariant#1", crate::textfns::tb_append_variant),
    // `RecordRef.GetTable(V)` with a Variant argument (lowering picks it): the VM drops the record's company.
    ("RecordRef.GetTableVariant#1", |c, r, a| {
        c.ref_op("RecordRef.GetTableVariant#1", r, a)
    }),
    ("TextBuilder.AppendFieldValue#1", crate::textfns::tb_append_field_value),
    // `TestField.SetValue(V)` with a Variant argument (lowering picks it): an Enum or Option control reads a Text in it by
    // ordinal digits (pages/lt-be-enum-setvalue-blank/BeEnumSetValueBlank.al:532-597).
    ("TestField.SetValueVariant#1", |c, r, a| {
        c.set_value_from_variant(true);
        let out = c.page(crate::page_op::PageOp::TestFieldSetValue, r, a);
        c.set_value_from_variant(false);
        out
    }),
    ("System.FormatDecimalField#1", format_decimal_field),
    ("System.FormatAutoFormatField#1", format_auto_format_field),
    ("System.OptionCaptionML#1", option_caption_ml),
    ("System.Evaluate#1", crate::evaluate::evaluate),
    (
        "System.EvaluateVariableOption#1",
        crate::evaluate::evaluate_variable_option,
    ),
    ("System.EvaluateBounded#1", crate::evaluate::evaluate_bounded),
    ("System.EvaluateOptionField#1", crate::evaluate::evaluate_option_field),
    ("System.Abs#1", abs),
    ("System.GetLastErrorText#1", get_last_error_text),
    (
        "System.GetLastErrorText#2",
        get_last_error_text_without_customer_content,
    ),
    ("System.GetLastErrorCode#1", get_last_error_code),
    ("System.GetLastErrorCallStack#1", get_last_error_call_stack),
    ("System.GetLastErrorObject#1", get_last_error_object),
    ("System.ClearLastError#1", clear_last_error),
    ("System.CodeCoverageRefresh#1", code_coverage_refresh),
    ("Dialog.Error#2", error_text),
    ("Dialog.ErrorNotLiteral#1", error_text_not_literal),
    ("Dialog.Error#1", crate::errorinfo::raise),
    ("Session.GetExecutionContext#1", execution_context),
    ("Session.GetCurrentModuleExecutionContext#1", execution_context),
    ("Session.GetModuleExecutionContext#1", module_execution_context),
    ("Dialog.Message#1", message),
    ("Dialog.Confirm#1", confirm),
    ("Dialog.StrMenu#1", str_menu),
    ("Text.StrLen#1", strlen),
    ("Text.StrPos#1", strpos),
    ("Text.StrCheckSum#1", strchecksum),
    ("Session.LogMessage#1", log_message),
    ("Session.LogMessage#2", log_message),
    ("Session.LogSecurityAudit#1", log_message),
    ("Session.LogAuditMessage#1", log_audit_message),
    ("System.Hyperlink#1", hyperlink),
    ("System.Variant2Date#1", variant_2_date),
    ("System.Variant2Time#1", variant_2_time),
    // Whether the host can create tasks is not pinned (corpus session/TestTaskSchedulerCloudBehavior.al
    // pins only that CreateTask returns a task id): a run that reaches the question is refused.
    // The URL of the web client depends on the server's address, which a host has not been asked
    // for: a run that builds one is refused (session/TestFinalCoverage.al:105-125 asks only that
    // GetUrl does not throw).
    // Measured on real BC: session/TestEnvironmentUserTenantContracts.al:237-241.
    ("Session.CurrentExecutionMode#1", |_, _, _| {
        let members: std::sync::Arc<[String]> = crate::sysenum::option_members("ExecutionMode")
            .expect("a system option")
            .into();
        Ok(Value::Option(0, members))
    }),
    ("Session.ApplicationIdentifier#1", |_, _, _| {
        Ok(Value::Text("NAV".into()))
    }),
    ("ProductName.Full#1", |_, _, _| {
        Ok(Value::Text("Dynamics 365 Business Central".into()))
    }),
    ("ProductName.Short#1", |_, _, _| {
        Ok(Value::Text("Business Central".into()))
    }),
    ("ProductName.Marketing#1", |_, _, _| {
        Ok(Value::Text("Microsoft Dynamics 365 Business Central".into()))
    }),
    // False in a test: measured on real BC by session/TestEnvironmentUserTenantContracts.al:238
    // (Assert.IsFalse(TaskScheduler.CanCreateTask())).
    ("TaskScheduler.CanCreateTask#1", |_, _, _| Ok(Value::Boolean(false))),
    // CreateTask hands back a new task id (session/TestTaskSchedulerCloudBehavior.al:9-20 pins that
    // and nothing more). A task starts in a session of its own after the creating transaction
    // commits, which a test does not wait for, so the task is not run (docs/probes-needed.md).
    ("TaskScheduler.CreateTask#1", |c, _, _| Ok(Value::Guid(c.new_guid()))),
    ("TaskScheduler.CreateTask#2", |c, _, _| Ok(Value::Guid(c.new_guid()))),
    ("TaskScheduler.SetTaskReady#1", |_, _, _| {
        unsupported("TaskScheduler.SetTaskReady (not pinned)")
    }),
    ("TaskScheduler.TaskExists#1", |_, _, _| {
        unsupported("TaskScheduler.TaskExists (not pinned)")
    }),
    ("TaskScheduler.CancelTask#1", |_, _, _| {
        unsupported("TaskScheduler.CancelTask (not pinned)")
    }),
    ("Session.StartSession#1", start_session),
    ("Session.StartSession#2", start_session),
    ("Session.StartSession#3", start_session),
    ("Text.CopyStr#1", copystr),
    ("Text.StrSubstNo#1", strsubstno),
    ("Text.UpperCase#1", uppercase),
    ("Text.LowerCase#1", lowercase),
    ("Database.Commit#1", |c, _, _| c.commit().map(|_| Value::Null)),
    ("Database.SID#1", database_sid),
    ("System.Clear#1", clear),
    ("System.ArrayLen#1", array_len),
    // A progress dialog is a headless no-op in a test (handlers/TestHandlersDialogProgressNoOp.al:24-110).
    ("Dialog.Open#1", dialog_open),
    ("Dialog.Update#1", dialog_update),
    ("Dialog.Close#1", dialog_close),
    // Reading the latest committed data is what this runtime always does (session/TestBCDatabaseContracts.al:170-187).
    // Telemetry goes to the server's log, which AL cannot read (ADR-009).
    ("Session.LogMessage#1", |_, _, _| Ok(Value::Null)),
    ("Session.LogMessage#2", |_, _, _| Ok(Value::Null)),
    // Unique and not null is all the corpus asks (session/TestSystemExtended.al:306-328).
    ("Guid.CreateSequentialGuid#1", |c, _, _| Ok(Value::Guid(c.new_guid()))),
    // A TextBuilder is bounded by the Integer range; the corpus needs at least a million
    // (session/TestSystemExtended.al:450-473).
    ("TextBuilder.MaxCapacity#1", |_, _, _| Ok(Value::Integer(i32::MAX))),
    ("Enum.AsInteger#1", enum_as_integer),
    ("Enum.Names#1", enum_names),
    ("Enum.Ordinals#1", enum_ordinals),
    ("Enum.FromInteger#1", enum_from_integer),
    ("Record.Init#1", |c, r, a| c.record(RecordOp::Init, r, a)),
    ("Record.Insert#1", |c, r, a| c.record(RecordOp::Insert, r, a)),
    ("Record.Insert#2", |c, r, a| c.record(RecordOp::Insert, r, a)),
    ("Record.Modify#1", |c, r, a| c.record(RecordOp::Modify, r, a)),
    ("Record.Delete#1", |c, r, a| c.record(RecordOp::Delete, r, a)),
    ("Record.DeleteAll#1", |c, r, a| c.record(RecordOp::DeleteAll, r, a)),
    ("Record.Get#1", |c, r, a| c.record(RecordOp::Get, r, a)),
    ("Record.Find#1", |c, r, a| c.record(RecordOp::Find, r, a)),
    ("Record.FindFirst#1", |c, r, a| c.record(RecordOp::FindFirst, r, a)),
    ("Record.FindLast#1", |c, r, a| c.record(RecordOp::FindLast, r, a)),
    ("Record.FindSet#1", |c, r, a| c.record(RecordOp::FindSet, r, a)),
    ("Record.FindSet#2", |c, r, a| c.record(RecordOp::FindSet, r, a)),
    ("Record.Next#1", |c, r, a| c.record(RecordOp::Next, r, a)),
    ("Record.Count#1", |c, r, a| c.record(RecordOp::Count, r, a)),
    ("Record.IsEmpty#1", |c, r, a| c.record(RecordOp::IsEmpty, r, a)),
    ("Record.SetRange#1", |c, r, a| c.record(RecordOp::SetRange, r, a)),
    ("Record.SetFilter#1", |c, r, a| c.record(RecordOp::SetFilter, r, a)),
    // Event subscriptions are session state; the VM intercepts these ids
    // (al-vm dispatch.rs). Registered so lowering accepts them.
    ("Session.ApplicationArea#1", vm_only),
    // ClearAll resets the running object's globals: the VM runs it (al-vm lib.rs).
    ("System.ClearAll#1", vm_only),
    ("Session.BindSubscription#1", vm_only),
    ("Session.UnbindSubscription#1", vm_only),
    // Codeunit.Run runs a codeunit and `IsInWriteTransaction` reads the transaction: both are
    // session state, run by the VM (al-vm transactions.rs).
    ("Codeunit.Run#1", vm_only),
    ("Codeunit.Run#2", vm_only),
    ("Codeunit.Run#3", vm_only),
    ("Database.IsInWriteTransaction#1", vm_only),
    // XmlPort import and export run triggers and write records: the VM runs them
    // (al-vm xmlports.rs).
    ("Xmlport.Export#1", vm_only),
    ("Xmlport.Export#2", vm_only),
    ("Xmlport.Import#1", vm_only),
    ("Xmlport.Import#2", vm_only),
    ("Xmlport.Run#2", vm_only),
    ("Xmlport.SetSource#1", vm_only),
    ("Xmlport.SetDestination#1", vm_only),
    ("Xmlport.SetTableView#1", vm_only),
    ("Xmlport.FieldDelimiter#1", vm_only),
    ("Xmlport.FieldSeparator#1", vm_only),
    ("Xmlport.RecordSeparator#1", vm_only),
    ("Xmlport.TableSeparator#1", vm_only),
    ("Xmlport.TextEncoding#1", vm_only),
    ("Xmlport.Filename#1", vm_only),
    ("Xmlport.CurrentPath#1", vm_only),
    ("Xmlport.Break#1", vm_only),
    ("Xmlport.Skip#1", vm_only),
    ("Xmlport.BreakUnbound#1", vm_only),
    ("Xmlport.Quit#1", vm_only),
    // Report runs execute triggers and write records: the VM runs them (al-vm reports.rs). The
    // ids not listed (SaveAsPdf, SaveAsXml and the other file forms, Preview, the layout accessors, Skip, Break, ...) need
    // output or behaviour the corpus does not pin and are refused in lowering.
    // A FilterPageBuilder keeps record buffers and renders views: the VM runs it (al-vm filterpage.rs).
    ("FilterPageBuilder.AddTable#1", vm_only),
    ("FilterPageBuilder.AddRecord#1", vm_only),
    ("FilterPageBuilder.AddRecordRef#1", vm_only),
    ("FilterPageBuilder.AddField#1", vm_only),
    ("FilterPageBuilder.AddField#2", vm_only),
    ("FilterPageBuilder.AddFieldNo#1", vm_only),
    ("FilterPageBuilder.Count#1", vm_only),
    ("FilterPageBuilder.GetView#1", vm_only),
    ("FilterPageBuilder.SetView#1", vm_only),
    ("FilterPageBuilder.Name#1", vm_only),
    ("FilterPageBuilder.PageCaption#1", vm_only),
    ("FilterPageBuilder.RunModal#1", vm_only),
    ("Report.Run#1", vm_only),
    ("Report.Run#3", vm_only),
    ("Report.RunModal#1", vm_only),
    ("Report.RunModal#3", vm_only),
    ("Report.Execute#1", vm_only),
    ("Report.Execute#3", vm_only),
    ("Report.WordXmlPart#1", vm_only),
    ("Report.WordXmlPart#2", vm_only),
    ("Report.Print#1", vm_only),
    ("Report.Print#2", vm_only),
    ("Report.RunRequestPage#1", vm_only),
    ("Report.RunRequestPage#2", vm_only),
    ("Report.SetTableView#1", vm_only),
    ("Report.UseRequestPage#1", vm_only),
    ("Report.ObjectId#1", vm_only),
    ("Report.Preview#1", vm_only),
    ("RequestPage.Update#1", vm_only),
    ("RequestPage.Caption#1", vm_only),
    ("Report.DefaultLayout#2", vm_only),
    ("Report.Language#1", vm_only),
    ("Report.FormatRegion#1", vm_only),
    // SaveAs with a stream runs the report and writes its dataset (al-vm reports.rs); SaveAs to a file
    // name (SaveAsXml, SaveAsPdf, ...) writes a file no corpus test pins and stays refused.
    ("Report.SaveAs#1", vm_only),
    ("Report.SaveAs#2", vm_only),
    // The rendering saves with a file name run only for the reports on the allow-list of ADR-040 and are refused for the
    // others when they run. SaveAsXml with a file name stays refused in lowering.
    ("Report.SaveAsPdf#1", vm_only),
    ("Report.SaveAsPdf#2", vm_only),
    ("Report.SaveAsExcel#1", vm_only),
    ("Report.SaveAsExcel#2", vm_only),
    ("Report.SaveAsWord#1", vm_only),
    ("Report.SaveAsWord#2", vm_only),
    ("Report.SaveAsHtml#1", vm_only),
    ("Report.SaveAsHtml#2", vm_only),
    ("Report.Quit#1", vm_only),
    ("Report.Skip#1", vm_only),
    ("Report.Break#1", vm_only),
    ("System.CreateGuid#1", |c, _, _| Ok(Value::Guid(c.new_guid()))),
    // `IsNull(DotNet)`: an unassigned DotNet variable, or a .NET call that returned null, holds
    // `Null` (ADR-014). Microsoft's Azure AD Tenant Impl. and Json Impl. ask it
    // (session/TestEnvironmentUserTenantContracts.al:204-232 reaches them).
    ("System.IsNull#1", |_, _, a| {
        Ok(Value::Boolean(match a.first() {
            Some(Value::Null) => true,
            // GetLastErrorObject without a last error (GetLastErrorObject.al:290-296).
            Some(Value::DotNet(o)) => matches!(
                &o.kind,
                crate::value::DotNetKind::Internal { type_name, .. } if type_name == crate::dotnet_sigs::NULL_EXCEPTION
            ),
            _ => false,
        }))
    }),
    ("System.IsNullGuid#1", |_, _, a| {
        Ok(Value::Boolean(matches!(a[0], Value::Guid(g) if g == [0; 16])))
    }),
    ("Record.Reset#1", |c, r, a| c.record(RecordOp::Reset, r, a)),
    ("Record.CalcFields#1", |c, r, a| c.record(RecordOp::CalcFields, r, a)),
    ("Record.SetCurrentKey#1", |c, r, a| {
        c.record(RecordOp::SetCurrentKey, r, a)
    }),
    ("Record.Validate#1", |c, r, a| c.record(RecordOp::Validate, r, a)),
    ("Record.Rename#1", |c, r, a| c.record(RecordOp::Rename, r, a)),
    ("Record.Truncate#1", |c, r, a| c.record(RecordOp::Truncate, r, a)),
    ("Record.CalcSums#1", |c, r, a| c.record(RecordOp::CalcSums, r, a)),
    ("Record.SetLoadFields#1", |c, r, a| {
        c.record(RecordOp::SetLoadFields, r, a)
    }),
    ("Record.AddLoadFields#1", |c, r, a| {
        c.record(RecordOp::AddLoadFields, r, a)
    }),
    ("Record.LoadFields#1", |c, r, a| c.record(RecordOp::LoadFields, r, a)),
    ("Record.SetBaseLoadFields#1", |c, r, a| {
        c.record(RecordOp::SetBaseLoadFields, r, a)
    }),
    ("Record.AreFieldsLoaded#1", |c, r, a| {
        c.record(RecordOp::AreFieldsLoaded, r, a)
    }),
    ("Record.SetAutoCalcFields#1", |c, r, a| {
        c.record(RecordOp::SetAutoCalcFields, r, a)
    }),
    // record/PZFindDirections.al:106 pins `ToText` of 5, 20, 25 and 35. Whether a larger value gets a thousands separator is
    // not pinned, so it is refused (docs/probes-needed.md "Integer.ToText").
    ("Integer.ToText#1", |_, r, _| match r.as_deref() {
        Some(Value::Integer(n)) if n.unsigned_abs() < 1000 => Ok(Value::Text(n.to_string().into())),
        Some(Value::Integer(n)) => Err(Signal::Unsupported(format!(
            "Integer.ToText of {n}: whether the text has a thousands separator is not pinned (docs/probes-needed.md)"
        ))),
        other => Err(Signal::Unsupported(format!("Integer.ToText on {}", other.describe()))),
    }),
    // `Date.ToText()` and `Boolean.ToText()` (Base Application's Customer Mgt. CalcLastPaymentInfo puts them in a Dictionary
    // of [Text, Text]): Format(value) with the default format, pinned by types/lt-conversions-hidden-parts-doctype/
    // P142ToTextLang.al:P142_Date_Language and P142_Boolean_Language.
    ("Date.ToText#1", |c, r, a| primitive_to_text(c, r, "Date", a)),
    ("Boolean.ToText#1", |c, r, a| primitive_to_text(c, r, "Boolean", a)),
    ("Decimal.ToText#1", |c, r, a| primitive_to_text(c, r, "Decimal", a)),
    ("Decimal.ToText#2", |c, r, a| primitive_to_text(c, r, "Decimal", a)),
    ("Boolean.ToText#2", |c, r, a| primitive_to_text(c, r, "Boolean", a)),
    ("Time.ToText#1", |c, r, a| primitive_to_text(c, r, "Time", a)),
    ("DateTime.ToText#1", |c, r, a| primitive_to_text(c, r, "DateTime", a)),
    ("Duration.ToText#1", |c, r, a| primitive_to_text(c, r, "Duration", a)),
    ("Variant.IsDefaultLayout#1", |_, r, _| is(r, "defaultlayout")),
    ("Variant.IsTestPermissions#1", |_, r, _| is(r, "testpermissions")),
    ("Variant.IsDotNet#1", |_, r, _| is(r, "DotNet")),
    ("Variant.IsAction#1", |_, r, _| is(r, "action")),
    ("Variant.IsClientType#1", |_, r, _| is(r, "clienttype")),
    ("Variant.IsDataClassification#1", |_, r, _| is(r, "dataclassification")),
    ("Variant.IsExecutionMode#1", |_, r, _| is(r, "executionmode")),
    ("Variant.IsObjectType#1", |_, r, _| is(r, "objecttype")),
    ("Variant.IsPromptMode#1", |_, r, _| is(r, "promptmode")),
    ("Variant.IsReportFormat#1", |_, r, _| is(r, "reportformat")),
    ("Variant.IsTableConnectionType#1", |_, r, _| {
        is(r, "tableconnectiontype")
    }),
    ("Variant.IsTextEncoding#1", |_, r, _| is(r, "textencoding")),
    // A Char is also a wide char; nothing else is (types/lt-variant-is-text-streams/LTP89IsMisc.al:WideChar_Variants: Char values
    // `WideChar,Char,`, a Byte, an Integer, a BigInteger, a Text and a two-character literal not).
    ("Variant.IsWideChar#1", |_, r, _| is(r, "Char")),
    ("Variant.IsFilterPageBuilder#1", |_, r, _| is(r, "FilterPageBuilder")),
    ("Variant.IsInStream#1", |_, r, _| is(r, "InStream")),
    ("Variant.IsOutStream#1", |_, r, _| is(r, "OutStream")),
    // `IsBinary` is No for every value the corpus puts in a Variant: numbers, text, a record, a Blob field's value, an InStream,
    // an OutStream, a TextBuilder, a FieldRef and an empty Variant (types/lt-variant-is-text-streams/LTP89IsMisc.al:Binary_*).
    ("Variant.IsBinary#1", |_, _, _| Ok(Value::Boolean(false))),
    ("Variant.IsBoolean#1", |_, r, _| is(r, "Boolean")),
    ("Variant.IsInteger#1", |_, r, _| is(r, "Integer")),
    ("Variant.IsBigInteger#1", |_, r, _| is(r, "BigInteger")),
    ("Variant.IsDecimal#1", |_, r, _| is(r, "Decimal")),
    ("Variant.IsByte#1", |_, r, _| is(r, "Byte")),
    ("Variant.IsChar#1", |_, r, _| is(r, "Char")),
    ("Variant.IsText#1", |_, r, _| is(r, "Text")),
    ("Variant.IsCode#1", |_, r, _| is(r, "Code")),
    ("Variant.IsDate#1", |_, r, _| is(r, "Date")),
    ("Variant.IsTime#1", |_, r, _| is(r, "Time")),
    ("Variant.IsDateTime#1", |_, r, _| is(r, "DateTime")),
    ("Variant.IsDuration#1", |_, r, _| is(r, "Duration")),
    ("Variant.IsGuid#1", |_, r, _| is(r, "Guid")),
    // A Media or MediaSet value put into a Variant: the media id, as a value no Is... test answers Yes for.
    ("Media.ToVariant#1", |_, _, a| {
        let Some(Value::Guid(g)) = a.first() else {
            return unsupported("a Media value that is not a media id");
        };
        let text = crate::format::format_guid(g);
        Ok(Value::Library(
            crate::library_kind::LibraryKind::Media,
            std::rc::Rc::new(std::cell::RefCell::new(crate::value::LibraryValue::Text(text))),
        ))
    }),
    // The routine that created this InStream on its own local hands it back and ends: it reads as empty
    // (streams/lt-dc-stream-leads/DcInStreamScope.al:107-166).
    ("InStream.OwnerOutOfScope#1", crate::stream::owner_out_of_scope),
    // An interface assigned from another interface variable: one that holds no implementation raises
    // (codeunit/lt-enums-interfaces/OjT71047.al:250-251, Uninitialized_AssignedFromUninitialized `af224`).
    ("Interface.AssertAssigned#1", |_, _, a| match a.first() {
        Some(Value::Null) | None => raise("Interface not initialized", "InvalidOperation"),
        Some(v) => Ok(v.clone()),
    }),
    // A Variant assigned to an Integer: a BigInteger or a Duration beyond the Integer range raises TypeConversion:Overflow
    // (types/lt-variant/VaTConvert.al:183-184; types/lt-filter-enum-relation-duration-guid/D_Duration.al:226). Everything
    // else converts as any value does.
    ("Variant.IntoInteger#1", |_, _, a| {
        let v = a.first().cloned().unwrap_or(Value::Null);
        if let Value::BigInteger(n) | Value::Duration(n) = v {
            if i32::try_from(n).is_err() {
                return raise(
                    "The expression Variant cannot be type-converted to a Integer value.",
                    "TypeConversion:Overflow",
                );
            }
        }
        crate::ops::convert(v, &crate::value::RtType::Integer)
    }),
    // A Variant assigned to an Option variable or field: a BigInteger beyond the Integer range raises TypeConversion:Overflow
    // (types/lt-bigint-enum-option/BigIntToEnumOption.al:353-383 QO_Variant_ToOptionVariable, QO_Variant_ToOptionField_Assign).
    // The value is handed back unchanged; the assignment converts it.
    ("Variant.IntoOption#1", |_, _, a| {
        let v = a.first().cloned().unwrap_or(Value::Null);
        if let Value::BigInteger(n) = v {
            if i32::try_from(n).is_err() {
                return raise(
                    "The expression Variant cannot be type-converted to a Integer value.",
                    "TypeConversion:Overflow",
                );
            }
        }
        Ok(v)
    }),
    ("Variant.IsDateFormula#1", |_, r, _| is(r, "DateFormula")),
    ("Variant.IsOption#1", |_, r, _| is(r, "Option")),
    ("Variant.IsList#1", |_, r, _| is(r, "List")),
    ("Variant.IsDictionary#1", |_, r, _| is(r, "Dictionary")),
    ("Variant.IsCodeunit#1", |_, r, _| is(r, "Codeunit")),
    // RecordId, RecordRef and FieldRef variables are library handles named after their type.
    ("Variant.IsNotification#1", |_, r, _| is(r, "Notification")),
    ("Variant.IsTextBuilder#1", |_, r, _| is(r, "TextBuilder")),
    ("Variant.IsRecordId#1", |_, r, _| is(r, "RecordId")),
    ("Variant.IsRecord#1", |_, r, _| is(r, "Record")),
    ("Variant.IsRecordRef#1", |_, r, _| is(r, "RecordRef")),
    ("Variant.IsFieldRef#1", |_, r, _| is(r, "FieldRef")),
    // Permission enforcement is not modelled (`Permissions Mock`, docs/probes-needed.md): the session
    // user reads every row, so the permission filter filters nothing. Record: record/TestRecordFilter.al:
    // 406-413 pins that it does not throw. RecordRef is the same call on the same model, not pinned itself.
    ("Record.SetPermissionFilter#1", |_, _, _| Ok(Value::Null)),
    ("RecordRef.SetPermissionFilter#1", |_, _, _| Ok(Value::Null)),
    // Kinds the value model has no variant for yet are never true.
    ("Variant.IsTextConstant#1", |_, _, _| Ok(Value::Boolean(false))),
    ("Variant.IsAutomation#1", |_, _, _| Ok(Value::Boolean(false))),
    ("Variant.IsFile#1", |_, _, _| Ok(Value::Boolean(false))),
];

/// Builtins that are refused on purpose, with the reason. No corpus test pins what they answer, so
/// the lowering says so instead of "is not implemented" (hard rule 3; each has a line in
/// `docs/probes-needed.md`, "Builtins refused until a probe pins them").
pub fn refusal(id: &str) -> Option<&'static str> {
    Some(match id {
        "Xmlport.Run#1" => {
            "Xmlport.Run is in no corpus test, so where a run without a stream writes or reads (a file on the service tier a test cannot read) and what the request window does are not pinned"
        }
        _ => return None,
    })
}

fn all() -> impl Iterator<Item = &'static (&'static str, NativeFn)> {
    REGISTRY
        .iter()
        .chain(crate::collections::REGISTRY)
        .chain(crate::pagebuiltins::REGISTRY)
        .chain(crate::querybuiltins::REGISTRY)
        .chain(crate::refs::REGISTRY)
        .chain(crate::refs::RECORD_REGISTRY)
        .chain(crate::datefns::REGISTRY)
        .chain(crate::mathfns::REGISTRY)
        .chain(crate::sessionfns::REGISTRY)
        .chain(crate::media::REGISTRY)
        .chain(crate::moduleinfo::REGISTRY)
}

/// Implementations chosen per catalog signature instead of being listed by id
/// (families with many near-identical overloads: JSON, XML, streams). Built once
/// from the catalog, so a lookup is a hash probe.
fn derived() -> &'static std::collections::HashMap<String, NativeFn> {
    static TABLE: std::sync::OnceLock<std::collections::HashMap<String, NativeFn>> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        let cat = crate::catalog::Catalog::shared();
        cat.signatures
            .iter()
            .filter_map(|sig| {
                crate::json::native_for(sig)
                    .or_else(|| crate::xml::native_for(sig))
                    .or_else(|| crate::stream::native_for(sig))
                    .or_else(|| crate::file::native_for(sig))
                    .or_else(|| crate::textfns::native_for(sig))
                    .or_else(|| crate::wsac::native_for(sig))
                    .or_else(|| crate::errorinfo::native_for(sig))
                    .or_else(|| crate::version::native_for(sig))
                    .or_else(|| crate::datatransfer::native_for(sig))
                    .or_else(|| crate::moduleinfo::native_for(sig))
                    .or_else(|| crate::sessiontypes::native_for(sig))
                    .or_else(|| crate::http::native_for(sig))
                    .map(|f| (cat.id(sig), f))
            })
            .collect()
    })
}

/// The implementations by name: the registry lists in order (the first entry for an id wins), then the
/// implementations chosen per catalog signature, then the platform's. Built once.
fn by_name() -> &'static std::collections::HashMap<String, NativeFn> {
    static TABLE: std::sync::OnceLock<std::collections::HashMap<String, NativeFn>> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        let mut t = std::collections::HashMap::new();
        for (i, f) in all() {
            t.entry((*i).to_string()).or_insert(*f);
        }
        for (i, f) in derived() {
            t.entry(i.clone()).or_insert(*f);
        }
        for (i, f) in crate::platform_native::entries() {
            t.entry(i.to_string()).or_insert(f);
        }
        t
    })
}

/// Ids the lowering writes that are no catalog overload and have no native of their own (the VM runs them).
pub const INVENTED_IDS: &[&str] = &[
    "Record.SetCurrFieldNo#1",
    "Record.CurrFieldNo#1",
    "Media.ToVariant#1",
    "TextBuilder.AppendFieldValue#1",
    "Interface.AssertAssigned#1",
    "Variant.IntoInteger#1",
    "Variant.IntoOption#1",
    "Text.MaxStrLenOfVarParameter#1",
    "InStream.OwnerOutOfScope#1",
];

/// Every name that gets a [`BuiltinId`]: the catalog's overloads of every version, the registered natives
/// and the invented ids. Sorted, unique. This is what `builtin_ids.rs` is generated from.
pub fn all_builtin_names() -> Vec<String> {
    let cat = crate::catalog::Catalog::shared();
    let mut names: std::collections::BTreeSet<String> = cat.signatures.iter().map(|s| cat.id(s)).collect();
    names.extend(by_name().keys().cloned());
    names.extend(INVENTED_IDS.iter().map(|s| (*s).to_string()));
    names.into_iter().collect()
}

/// The implementation of builtin `id`, if any: one array index into a table built once (the VM's own
/// builtins have no entry).
pub fn by_id(id: crate::builtin_id::BuiltinId) -> Option<NativeFn> {
    static TABLE: std::sync::OnceLock<Vec<Option<NativeFn>>> = std::sync::OnceLock::new();
    TABLE
        .get_or_init(|| {
            let names = by_name();
            crate::builtin_id::BuiltinId::all()
                .map(|i| names.get(i.name()).copied())
                .collect()
        })
        .get(id.index())
        .copied()
        .flatten()
}

/// The implementation registered for overload `id`, if any.
pub fn lookup(id: &str) -> Option<NativeFn> {
    crate::builtin_id::BuiltinId::of(id).and_then(by_id)
}

/// [`lookup`] the way it was before the table: scans in the same order. The test compares the two.
#[cfg(test)]
fn lookup_by_scan(id: &str) -> Option<NativeFn> {
    all()
        .find(|(i, _)| *i == id)
        .map(|(_, f)| *f)
        .or_else(|| derived().get(id).copied())
        .or_else(|| crate::platform_native::lookup_by_scan(id))
}

/// Every registered id, for the conformance test.
pub fn registered_ids() -> Vec<&'static str> {
    all()
        .map(|(i, _)| *i)
        .chain(derived().keys().map(String::as_str))
        .collect()
}

fn vm_only(_: &mut dyn Ctx, _: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    unsupported("a builtin the VM runs itself")
}

fn unsupported<T>(what: &str) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.to_string()))
}

fn int_arg(args: &[Value], i: usize) -> Option<i64> {
    match args.get(i)? {
        Value::Integer(n) => Some(i64::from(*n)),
        Value::BigInteger(n) => Some(*n),
        Value::Byte(n) => Some(i64::from(*n)),
        Value::Char(n) => Some(i64::from(*n)),
        // An Option takes the place of an Integer (record/lt-record-conversion-assumptions/OptCopyStr_risky.al: CopyStr('abcdef', Opt)).
        Value::Option(n, _) => Some(i64::from(*n)),
        _ => None,
    }
}

/// The character inside a text that marks a call stack with an untracked frame. A noncharacter (Unicode permanently
/// reserves U+FDD0 to U+FDEF for internal use): real code puts private-use characters in texts (a barcode font's glyphs
/// are U+E000 and up), so the marks may not be private-use characters.
pub const CALL_STACK_MARK: char = '\u{FDD0}';

/// Stands for a call stack that exists in BC but is not tracked here.
pub const UNTRACKED_CALL_STACK: &str = "\u{FDD0}";

/// Stands for the text and code of an error BC raises but whose wording the corpus does not pin.
/// `GetLastErrorText` and `GetLastErrorCode` refuse to answer while it is the last error, so a
/// test that reads it is not run, and one that only needs the call to fail still passes.
pub const UNPINNED_ERROR: &str = "\u{FDD1}";

/// The last error to set when a call fails with an error whose text is not pinned.
pub fn unpinned_error() -> AlError {
    AlError {
        message: UNPINNED_ERROR.into(),
        code: UNPINNED_ERROR.into(),
        call_stack: String::new(),
        ..Default::default()
    }
}

/// An error BC raises whose text and code the corpus does not pin, naming the probe that would pin them (its row in
/// docs/probes-needed.md). It is the same marker as [`unpinned_error`], with the probe after the mark in the text.
pub fn unpinned_error_for(probe: &str) -> AlError {
    AlError {
        message: format!("{UNPINNED_ERROR}{probe}"),
        ..unpinned_error()
    }
}

/// Follows [`UNPINNED_ERROR`] in the text of an error whose code is pinned and whose text is not ([`unpinned_text_error`]):
/// `GetLastErrorCode` answers the code, `GetLastErrorText` refuses.
const PINNED_CODE: char = '\u{FDD2}';

/// An error with a pinned `code` whose text the corpus does not pin, naming the probe that would pin the text.
pub fn unpinned_text_error(code: &str, probe: &str) -> AlError {
    AlError {
        message: format!("{UNPINNED_ERROR}{PINNED_CODE}{probe}"),
        code: code.into(),
        ..unpinned_error()
    }
}

/// Whether the text of `e` is not pinned ([`unpinned_error`], [`unpinned_error_for`]). Some errors have a pinned text and an
/// unpinned code (the code alone is the mark).
pub fn text_unpinned(e: &AlError) -> bool {
    // Anywhere in the text: a text built around a value whose text is not pinned (a refused table caption) holds the mark inside.
    e.message.contains(UNPINNED_ERROR)
}

fn unpinned_refusal(e: &AlError, what: &str) -> Signal {
    let probe = e
        .message
        .strip_prefix(UNPINNED_ERROR)
        .map(|p| p.trim_start_matches(PINNED_CODE));
    match probe.filter(|p| !p.is_empty()) {
        Some(probe) => Signal::Unsupported(format!(
            "{what} of an error whose text is not pinned (probe wanted: {probe}, docs/probes-needed.md)"
        )),
        None => Signal::Unsupported(format!("{what} of an error whose text is not pinned")),
    }
}

/// Refuses a read of the text of an error whose text is not pinned: `what` is the read; the reason names the probe when the
/// error carries one. Every place that hands an error's text to AL code or to the test result calls it.
pub fn refuse_unpinned_text(e: &AlError, what: &str) -> Result<(), Signal> {
    if text_unpinned(e) {
        Err(unpinned_refusal(e, what))
    } else {
        Ok(())
    }
}

/// Refuses a read of the text or the code of an error when either is not pinned.
pub fn refuse_unpinned(e: &AlError, what: &str) -> Result<(), Signal> {
    let code_pinned = e
        .message
        .strip_prefix(UNPINNED_ERROR)
        .is_some_and(|p| p.starts_with(PINNED_CODE));
    if (text_unpinned(e) && !code_pinned) || e.code == UNPINNED_ERROR {
        Err(unpinned_refusal(e, what))
    } else {
        Ok(())
    }
}

/// Text of a Text/Code/Char argument (Text parameters accept all three). Text that holds the
/// untracked call stack marker is refused, so its content is never silently wrong.
/// `Database.SID(UserName)`: a Windows-style security identifier as Text. Probed on BC 27.5 (corpus
/// branch al-runtime-rs-staging, ae3fe88, tests/al-language-onprem/session/PXDatabaseSid.al:31-46):
/// `S-1-5-21-<a>-<b>-<c>-<rid>`, eight dash-separated parts, all but the first numeric (each read by
/// `Evaluate` into an Integer), the same for the same name, not the text of `UserSecurityId()`, and
/// a SID of that shape for a name no user has. The numbers are the machine's and are not pinned:
/// the domain part is fixed and the rid is a stable hash of the name (docs/probes-needed.md).
/// Without a name, which user is meant is not pinned: refused.
fn database_sid(_: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    let Some(name) = args.first().and_then(text_of) else {
        return unsupported("Database.SID without a user name (whose SID it answers is not pinned)");
    };
    crate::assumptions::assume(crate::assumptions::Assumption::DatabaseSidNumbers);
    // FNV-1a over the name's UTF-8 bytes: the same on every platform and run.
    let hash = name.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    });
    let rid = 1000 + hash % (i32::MAX as u64 - 1000);
    Ok(Value::Text(
        format!("S-1-5-21-1004336348-1177238915-682003330-{rid}").into(),
    ))
}

pub fn text_of(v: &Value) -> Option<crate::text::Txt> {
    match v {
        Value::Text(s) | Value::Code(s) if s.has_marker() => None,
        Value::Text(s) | Value::Code(s) => Some(s.clone()),
        Value::Char(c) => char::from_u32(u32::from(*c)).map(|c| c.to_string().into()),
        _ => None,
    }
}

fn text_arg(args: &[Value], i: usize) -> Result<crate::text::Txt, Signal> {
    // A Guid where a Text is expected is its text form, as in an assignment (`CopyStr(CreateGuid(), 1, 50)` in
    // Microsoft's Match General Jnl Lines UT, 61 tests; unprobed, docs/probes-needed.md "A Guid assigned to a Text
    // or Code"; hard rule 3 exception, owner 2026-10-15).
    if let Some(Value::Guid(g)) = args.get(i) {
        crate::assumptions::assume(crate::assumptions::Assumption::GuidAsTextArgument);
        return Ok(crate::format::format_guid(g).into());
    }
    args.get(i).and_then(text_of).map_or_else(
        || unsupported(&format!("text argument {i} of type {}", args.get(i).describe())),
        Ok,
    )
}

/// `Format(V, Length, Number)` with `V` declared as a Variant: the standard format numbers 3 to 8 do not exist for a Variant
/// (types/lt-scalars/ScT70050.al:FormatNumbers_*), though a Decimal variable has 3 and 4.
fn format_variant(ctx: &mut dyn Ctx, r: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    let std = int_arg(args, 2).unwrap_or(0) as i32;
    if !matches!(std, 0..=2 | 9) && !matches!(&args[0], Value::Null) {
        return Err(Signal::Error(Box::new(AlError {
            message: format!("Standard format number {std} does not exist for the type 'Variant'."),
            code: "InvalidStandardFormat".into(),
            call_stack: String::new(),
            ..Default::default()
        })));
    }
    format_1(ctx, r, args)
}

/// An Option value with the OptionCaptionML captions of the session language (the lowering passes the property text):
/// `Format` then shows the caption (record/lt-x-table-props/XTablePropsTests.al:219-228). See
/// [`crate::format::ml_text_in_language`] for what is pinned.
fn option_caption_ml(ctx: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let (Some(Value::Option(o, names)), Some(ml)) = (a.first(), a.get(1).and_then(text_of)) else {
        return unsupported("OptionCaptionML without an Option value and its property text");
    };
    let list = crate::format::ml_text_in_language(ml.as_str(), ctx.language())
        .map_err(|e| Signal::Unsupported(format!("OptionCaptionML {e}")))?;
    let captions: Vec<String> = list.split(',').map(str::to_string).collect();
    if captions.len() != names.len() {
        return unsupported("OptionCaptionML with another number of entries than the members (not pinned)");
    }
    Ok(Value::Option(*o, captions.into()))
}

/// `Format(Rec.Field)` of a Decimal field that declares its places (`DecimalPlaces` or an `AutoFormatExpression` precision;
/// lowering appends them as a fourth argument `min:max`). The standard formats 0 and 1 and the XML format 9 show those places; 0 groups
/// thousands (pages/lt-page-autoformat-lookups/P113AutoFormat.al:P113_AF_TableFieldFormatIsPlain, `1,234.500` and, for 1 and 9,
/// `1234.500`; pages/lt-page-presentation-properties/P119Table.al:P119_DecimalPlaces `1.500`).
fn format_decimal_field(ctx: &mut dyn Ctx, r: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    let len = int_arg(args, 1).unwrap_or(0) as i32;
    let places = match args.get(3) {
        Some(Value::Text(t)) => crate::pagefmt::parse_decimal_places(t),
        _ => None,
    };
    // An amount field without AutoFormatExpression rounds from BC 28 only; BC 27 writes every digit
    // (types/lt-at-partner-gaps/AtPrecisionStandardFormat2.al:684-723, `#else` side).
    let gated = matches!(args.get(4), Some(Value::Boolean(true)));
    let plain = |ctx: &mut dyn Ctx, r: Option<&mut Value>, args: &mut [Value]| {
        if matches!(args.get(2), Some(Value::Text(_) | Value::Code(_))) {
            format_2(ctx, r, &mut args[..3])
        } else {
            let n = args.len().min(3);
            format_1(ctx, r, &mut args[..n])
        }
    };
    if gated && !ctx.format_gate(al_version::Gate::AutoFormatAmountFieldRoundsFrom28) {
        return plain(ctx, r, args);
    }
    // A format string: one with a `<Precision,..>` code formats the value by itself, the field's places do not matter
    // (AtPrecisionStandardFormat2.al:684-723, `P22S2` and `P2S2` of every field); `<Standard Format,2>` alone is format number 2
    // with the field's places (`S2`: `1234.6` for the `<Precision,1:1>` field, `1234.57` for the amount from BC 28). Any other
    // string is not pinned.
    let std = match args.get(2) {
        Some(Value::Text(t) | Value::Code(t)) => {
            let t = t.as_str();
            if t.contains("<Precision,") {
                return plain(ctx, r, args);
            }
            if t != "<Standard Format,2>" {
                return Err(Signal::Unsupported(format!(
                    "Format of a Decimal field with DecimalPlaces or an AutoFormat with the format string `{t}` (not pinned)"
                )));
            }
            2
        }
        _ => int_arg(args, 2).unwrap_or(0) as i32,
    };
    if let (Value::Decimal(d), Some(places), 0 | 1 | 2 | 9) = (&args[0], places, std) {
        // The amount field is pinned for formats 0, 2 and 9 on values of two decimals or more (AtPrecisionStandardFormat2.al:
        // 663-723: 1234.5678, -0.005).
        if gated && (std == 1 || d.normalize().scale() < 2) {
            return Err(Signal::Unsupported(format!(
                "Format of an amount field (AutoFormatType 1 without AutoFormatExpression) of {d} in format {std} (not pinned)"
            )));
        }
        let mut text = crate::pagefmt::decimal_text(d, places);
        // A negative value that rounds to zero keeps its minus: -0.005 with `<Precision,1:1>` is `-0.0` in formats 0, 2 and 9
        // (AtPrecisionStandardFormat2.al:684-723, `b default`, `b S2`, `b number 2`, `b number 9`); format 1 is not pinned.
        if d.is_sign_negative() && !d.is_zero() && !text.starts_with('-') {
            if std == 1 {
                return Err(Signal::Unsupported(format!(
                    "Format 1 of a Decimal field with places of {d}, a negative value that rounds to zero (not pinned)"
                )));
            }
            text.insert(0, '-');
        }
        if std != 0 {
            text.retain(|c| c != ',');
        }
        return Ok(Value::Text(crate::format::pad(text, len).into()));
    }
    plain(ctx, r, args)
}

/// `Format(Rec.Field[, Length, FormatNumber])` of a Decimal table field whose AutoFormatExpression is an expression: the
/// places come from the field's AutoFormat evaluated on the record (`Ctx::auto_format_field_places`), then as
/// `format_decimal_field`. Arguments: the value, length, format number, the record, the field's number.
fn format_auto_format_field(ctx: &mut dyn Ctx, r: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    let pos = match args.get(4) {
        Some(Value::Integer(p)) => *p as usize,
        _ => {
            return Err(Signal::Unsupported(
                "Format of an AutoFormat field without its number".into(),
            ));
        }
    };
    let record = args.get(3).cloned().unwrap_or(Value::Null);
    let places = ctx.auto_format_field_places(&record, pos)?;
    let mut rest = vec![
        args[0].clone(),
        args[1].clone(),
        args[2].clone(),
        Value::Text(places.into()),
    ];
    format_decimal_field(ctx, r, &mut rest)
}

fn format_1(ctx: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    let len = int_arg(args, 1).unwrap_or(0) as i32;
    let std = int_arg(args, 2).unwrap_or(0) as i32;
    // The text of a DotNet object is its ToString(); the length and the format number change
    // nothing for the numbers 0 and 2 (tests/al-language-onprem-dotnet/TestDotNetValueKindsContracts.al,
    // codeunit 61400, FormatOfDotNetObjects_IsToString). Other format numbers are not pinned.
    // The number 9 is pinned for an enum member only (XmlNodeTypeFormat.al: `Format(NodeType, 0, 9)` is `Element`).
    let enum_member = matches!(&args[0], Value::DotNet(o) if matches!(o.kind, crate::value::DotNetKind::Enum { .. }));
    if matches!(&args[0], Value::DotNet(_)) && (matches!(std, 0 | 2) || (std == 9 && enum_member)) {
        if let Some(r) = ctx.format_dotnet(&args[0]) {
            return r.map(|s| Value::Text(s.into()));
        }
    }
    // The first day of the calendar (00010101D) is not a valid date to format with the numbers 0, 1, 4 and 9 (types/lt-datetime/
    // Format.al:Date_Format_0D_And_Bounds); the keys and messages the runtime builds itself format it all the same.
    if matches!(&args[0], Value::Date(1, _)) && matches!(std, 0 | 1 | 4 | 9) {
        return Err(Signal::Error(Box::new(AlError {
            message: "The date is not valid.".into(),
            code: "DateInvalid".into(),
            call_stack: String::new(),
            ..Default::default()
        })));
    }
    // A record as a whole, a RecordRef and a FieldRef need the table's schema.
    let composite = matches!(&args[0], Value::Record(_))
        || matches!(
            &args[0],
            Value::Library(
                crate::library_kind::LibraryKind::RecordRef | crate::library_kind::LibraryKind::FieldRef,
                _
            )
        );
    if composite && std == 0 {
        if let Some(r) = ctx.format_composite(&args[0]) {
            return r.map(|t| Value::Text(crate::format::pad(t, len).into()));
        }
    }
    format_value(&args[0], len, std, ctx)
        .map(|s| Value::Text(s.into()))
        .map_err(|f| match f {
            FormatFail::Unsupported(s) => Signal::Unsupported(s),
            FormatFail::Error { message, code } => Signal::Error(Box::new(AlError {
                message,
                code,
                call_stack: String::new(),
                ..Default::default()
            })),
        })
}

/// `Format(Value, Length, FormatString)`: see [`crate::formatstr`].
fn format_2(ctx: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    let len = int_arg(args, 1).unwrap_or(0) as i32;
    let template = text_arg(args, 2)?;
    crate::formatstr::format_with_string(&args[0], len, &template, ctx)
        .map(|s| Value::Text(s.into()))
        .map_err(|e| match e.strip_prefix(crate::formatstr::INVALID_FORMAT_MARK) {
            Some(template) => Signal::Error(Box::new(AlError {
                message: format!(
                    "An invalid field or attribute has been specified for the 'Format' property.\\\\{template}"
                ),
                code: "InvalidFormatString".into(),
                call_stack: String::new(),
                ..Default::default()
            })),
            None => match e
                .strip_prefix(crate::formatstr::RAISE_MARK)
                .and_then(|r| r.split_once(crate::formatstr::RAISE_MARK))
            {
                Some((code, message)) => Signal::Error(Box::new(AlError {
                    message: message.to_string(),
                    code: code.to_string(),
                    call_stack: String::new(),
                    ..Default::default()
                })),
                None => Signal::Unsupported(e),
            },
        })
}

fn abs(_: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    Ok(match &args[0] {
        Value::Decimal(d) => Value::Decimal(d.abs()),
        // Abs keeps the type of its argument: `V := Abs(I)` holds an Integer and `Abs(Bi)` a BigInteger
        // (types/lt-variant-is-text-streams/LTP89IsMisc.al:NearTypes_TextVsCode_IntegerVsBigInteger `abs_int`, `abs_bigint`).
        Value::Integer(i) => match i.checked_abs() {
            Some(n) => Value::Integer(n),
            // types/lt-scalars/ScT70056.al:Integer_Overflow `abs_min`.
            None => {
                return raise(
                    "Negating the minimum value of a twos complement number is invalid.",
                    "AL:Overflow",
                );
            }
        },
        Value::BigInteger(i) => match i.checked_abs() {
            Some(n) => Value::BigInteger(n),
            None => return unsupported("Abs of the smallest BigInteger (the overflow error is not pinned)"),
        },
        // A Duration takes the BigInteger overload: its milliseconds (misc/lt-protected-var-abs-query-views/P146AbsDuration.al,
        // P146AbsDurationBigInt.al).
        other => return abs_other(other),
    })
}

/// `Abs` of a Duration is the BigInteger of its milliseconds: `Format(Abs(D))` is `1500`, assigned to a Decimal it is `1,500`
/// (misc/lt-protected-var-abs-query-views/P146AbsDuration.al:P146_Abs_Duration_ResultFormats, P146AbsDurationBigInt.al).
#[cold]
#[inline(never)]
fn abs_other(v: &Value) -> Result<Value, Signal> {
    match v {
        Value::Duration(ms) => Ok(Value::BigInteger(ms.abs())),
        other => unsupported(&format!("Abs of {}", other.describe())),
    }
}

fn get_last_error_text(ctx: &mut dyn Ctx, _: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    let e = ctx.last_error();
    refuse_unpinned_text(&e, "GetLastErrorText")?;
    Ok(Value::Text(e.message.into()))
}

/// `GetLastErrorText(true)`: the text with the personal data removed. Which text depends on how the error was raised; the
/// pinned origins are in [`ErrorOrigin`]. No error pending, or an empty text, answers the empty text (P86LastError.al:60
/// `cleared_excl`, :192 `empty_excl`, :410 `text_excl`); an origin the corpus does not pin is refused.
fn get_last_error_text_without_customer_content(
    ctx: &mut dyn Ctx,
    r: Option<&mut Value>,
    a: &mut [Value],
) -> Result<Value, Signal> {
    if matches!(a.first(), Some(Value::Boolean(false))) {
        return get_last_error_text(ctx, r, &mut []);
    }
    let e = ctx.last_error();
    refuse_unpinned_text(&e, "GetLastErrorText(true)")?;
    if e.message.is_empty() {
        return Ok(Value::Text(String::new().into()));
    }
    match e.origin {
        ErrorOrigin::SameText => Ok(Value::Text(e.message.into())),
        ErrorOrigin::Redacted(t) => Ok(Value::Text(t.into())),
        ErrorOrigin::Unknown => unsupported(&format!(
            "GetLastErrorText(true) of an error raised in a way the corpus does not pin for the flag (code {})",
            e.code
        )),
    }
}

fn get_last_error_code(ctx: &mut dyn Ctx, _: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    let e = ctx.last_error();
    refuse_unpinned(&e, "GetLastErrorCode")?;
    Ok(Value::Text(e.code.into()))
}

/// Slots of the state of a last-error exception object (`DotNetKind::Internal`): the message, the AL call stack
/// text (`GetLastErrorCallStack` form without the trailing backslashes, `\n` between frames; may be empty).
pub const EXC_MESSAGE: usize = 0;
pub const EXC_CALL_STACK: usize = 1;

/// `GetLastErrorObject()`: the last error as the .NET exception BC holds, or null when there is none.
/// Pinned by tests/al-language-onprem-dotnet/lt-dotnet-interop/GetLastErrorObject.al for `Error('literal', args)`
/// (:138-230), `Get` without a row (:232-255) and a division by zero (:257-280); null after `ClearLastError`
/// (:283-301). Any other error is refused, naming its code.
fn get_last_error_object(ctx: &mut dyn Ctx, _: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    use crate::dotnet_sigs::{
        AL_EXCEPTION, NCL_DIALOG_EXCEPTION, NCL_EVALUATE_EXCEPTION, RECORD_NOT_FOUND_EXCEPTION, TEST_FIELD_EXCEPTION,
    };
    let e = ctx.last_error();
    refuse_unpinned(&e, "GetLastErrorObject")?;
    if e.message.is_empty() && e.code.is_empty() {
        if e.exception != ExceptionKind::Cleared {
            return unsupported(
                "GetLastErrorObject before any error happened in the session (what BC answers is not pinned; only the answer after ClearLastError is)",
            );
        }
        return Ok(Value::DotNet(Rc::new(crate::value::DotNetObject::new(
            crate::value::DotNetKind::Internal {
                type_name: crate::dotnet_sigs::NULL_EXCEPTION.into(),
                state: std::cell::RefCell::new(Vec::new()),
            },
        ))));
    }
    let type_name = match e.exception {
        ExceptionKind::Dialog => NCL_DIALOG_EXCEPTION,
        // Error(Label, ...) and Error(Variable) raise the same exception as a literal (GetLastErrorObjectKinds.al:Kind_ErrorWithLabel,
        // _ErrorWithTextVariable, _ErrorWithErrorInfo).
        ExceptionKind::DialogComputed => NCL_DIALOG_EXCEPTION,
        ExceptionKind::RecordNotFound => RECORD_NOT_FOUND_EXCEPTION,
        ExceptionKind::DivideByZero => AL_EXCEPTION,
        // The class follows the error code for these (GetLastErrorObjectKinds.al:Kind_FailingTestFieldWithValue, _WithoutValue,
        // _ModifyOfMissingRow, _DeleteOfMissingRow, _EvaluateWithoutReturnValue).
        ExceptionKind::Unknown if e.code == "Dialog" => NCL_DIALOG_EXCEPTION,
        ExceptionKind::Unknown if e.code == "TestField" => TEST_FIELD_EXCEPTION,
        ExceptionKind::Unknown if e.code == "Evaluate" => NCL_EVALUATE_EXCEPTION,
        ExceptionKind::Unknown if e.code == "DB:RecordNotFound" => RECORD_NOT_FOUND_EXCEPTION,
        ExceptionKind::Unknown | ExceptionKind::Cleared => {
            return unsupported(&format!(
                "GetLastErrorObject after an error of code '{}' (the .NET exception BC holds for it is not pinned; only Error() with a text, Get without a row and a division by zero are)",
                if e.code == UNPINNED_ERROR {
                    "unpinned"
                } else {
                    e.code.as_str()
                }
            ));
        }
    };
    Ok(Value::DotNet(Rc::new(crate::value::DotNetObject::new(
        crate::value::DotNetKind::Internal {
            type_name: type_name.into(),
            state: std::cell::RefCell::new(vec![Value::Text(e.message.into()), Value::Text(e.call_stack.into())]),
        },
    ))))
}

fn get_last_error_call_stack(ctx: &mut dyn Ctx, _: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    let e = ctx.last_error();
    if e.call_stack.is_empty() && !(e.message.is_empty() && e.code.is_empty()) {
        // BC returns the AL frames of the error (codeunit/TestCodeunitAlCallStack.al). The VM adds
        // them as the error unwinds (al-vm/src/callstack.rs), so an error with no frame at all
        // (raised outside any AL routine) is the only one that gets here. The answer is non-empty,
        // as in BC, but its content is a marker that every text function refuses to read
        // ([`text_of`]), so a test that inspects the frames is not run while one that only calls
        // the function (error-handling/TestGetLastError.al) still passes.
        return Ok(Value::Text(UNTRACKED_CALL_STACK.to_string().into()));
    }
    if e.call_stack.is_empty() {
        return Ok(Value::Text(String::new().into()));
    }
    // The frames are separated by a backslash and the text ends with one, the runner's frames
    // below the test's (session/TestSessionInformationCallstackContracts.al:200-218).
    let mut out = String::with_capacity(e.call_stack.len() + 64);
    for frame in e.call_stack.split('\n') {
        out.push_str(frame);
        out.push('\\');
    }
    out.push_str(&ctx.runner_call_stack());
    Ok(Value::Text(out.into()))
}

/// `CodeCoverageRefresh()`: loads what was recorded into the Code Coverage tables. While logging is on a read of the table holds
/// nothing until this call or until logging stops; after logging stopped the call changes nothing (platform/lt-code-coverage-refresh/
/// CodeCoverageRefreshLogged.al: CodeCoverageRefresh_WhileLogging_Result, _AfterLoggingStopped_Result, _NeverLogging_Result).
fn code_coverage_refresh(ctx: &mut dyn Ctx, _: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    ctx.code_coverage_refresh()?;
    Ok(Value::Null)
}

fn clear_last_error(ctx: &mut dyn Ctx, _: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    ctx.set_last_error(AlError {
        exception: ExceptionKind::Cleared,
        ..Default::default()
    });
    Ok(Value::Null)
}

/// The numbers of the `#n####` and `@n@@@@` fields of a dialog template.
fn dialog_fields(template: &str) -> Vec<i32> {
    let c: Vec<char> = template.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        if matches!(c[i], '#' | '@') {
            let mut j = i + 1;
            while j < c.len() && c[j].is_ascii_digit() {
                j += 1;
            }
            if j > i + 1 {
                if let Ok(n) = c[i + 1..j].iter().collect::<String>().parse::<i32>() {
                    out.push(n);
                }
            }
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

fn dialog_not_open() -> Signal {
    Signal::Error(Box::new(AlError {
        message: "The operation failed because the dialog box is not open.".into(),
        code: "Dialog".into(),
        call_stack: String::new(),
        ..Default::default()
    }))
}

/// `Dialog.Open(Template)`: the dialog is open until `Close`; opening an open one is not an error
/// (pages/lt-page-runtime/P82Dialog.al:Dialog_EdgeCases `open-reopen`, `open-empty-template`).
fn dialog_open(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    if let (Some(Value::Library(_, h)), Some(t)) = (r, a.first().and_then(text_of)) {
        *h.borrow_mut() = LibraryValue::Text(t.to_string());
    }
    Ok(Value::Null)
}

/// `Dialog.Update(Number, Value, ...)`. An unopened or closed dialog raises, and so does a number that is
/// no field of the template, naming the template with each field as `%s` (errorinfo/lt-errnotif/EnTDialogSession.al:
/// Dialog_Errors, Dialog_UpdateWithoutOpen; pages/lt-page-runtime/P82Dialog.al:Dialog_EdgeCases). Only `#` fields have their
/// template text pinned: a failing number against a template with `@` fields is refused. `Update()` is a no-op
/// on an open dialog (handlers/TestHandlersDialogProgressNoOp.al).
fn dialog_update(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let template = match r {
        Some(Value::Library(_, h)) => match &*h.borrow() {
            LibraryValue::Text(t) => t.clone(),
            _ => return Err(dialog_not_open()),
        },
        _ => return unsupported("Dialog.Update on something that is no Dialog"),
    };
    let fields = dialog_fields(&template);
    for n in a.iter().step_by(2) {
        let Value::Integer(n) = n else {
            return unsupported("Dialog.Update with a field number that is not an Integer");
        };
        if !fields.contains(n) {
            if template.contains('@') {
                return unsupported(
                    "Dialog.Update of a field number the template lacks, with an @ field (message not pinned)",
                );
            }
            let mut shown = String::new();
            let chars: Vec<char> = template.chars().collect();
            let mut i = 0;
            while i < chars.len() {
                if chars[i] == '#' && chars.get(i + 1).is_some_and(char::is_ascii_digit) {
                    i += 1;
                    while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '#') {
                        i += 1;
                    }
                    shown.push_str("%s");
                } else {
                    shown.push(chars[i]);
                    i += 1;
                }
            }
            return Err(Signal::Error(Box::new(AlError {
                message: format!("No parameter with index '{n}' was found in the format string '{shown}'."),
                code: "FormatStringParameterNotFound".into(),
                call_stack: String::new(),
                ..Default::default()
            })));
        }
    }
    Ok(Value::Null)
}

/// `Dialog.Close()`: closing a dialog that is not open raises.
fn dialog_close(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    match r {
        Some(Value::Library(_, h)) => {
            let open = matches!(&*h.borrow(), LibraryValue::Text(_));
            if !open {
                return Err(dialog_not_open());
            }
            *h.borrow_mut() = LibraryValue::Empty;
            Ok(Value::Null)
        }
        _ => unsupported("Dialog.Close on something that is no Dialog"),
    }
}

/// `Error(Text, Args...)`. The code is `Dialog` (probes-needed: Microsoft's
/// test libraries and the corpus Assert's DialogTxt use it; no corpus test
/// pins it directly).
fn error_text(ctx: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    error_text_with(ctx, args, true)
}

/// `Error(Label, ...)` and any `Error(Text, ...)` that is not a string literal: the flag's text for it is not pinned.
fn error_text_not_literal(ctx: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    error_text_with(ctx, args, false)
}

fn error_text_with(ctx: &mut dyn Ctx, args: &mut [Value], literal: bool) -> Result<Value, Signal> {
    let template = text_arg(args, 0)?;
    let mut message = substitute(&template, &args[1..], ctx)?;
    // The error text of `Error(Text)` without arguments shows a CR LF as one backslash (misc/lt-system-functions/ErrorMessage.al:
    // 606-618, LT_BU_Message_BackslashAndAppendLine: 17 characters). With arguments no corpus test pins it.
    if message.contains("\r\n") {
        if args.len() > 1 {
            return unsupported(
                "Error with arguments whose text holds a CR LF: the error text is not pinned (docs/probes-needed.md)",
            );
        }
        message = message.replace("\r\n", "\\");
    }
    let (origin, exception) = if literal {
        (
            ErrorOrigin::Redacted(USE_ERROR_WITH_A_TEXT_CONSTANT.into()),
            ExceptionKind::Dialog,
        )
    } else {
        (ErrorOrigin::Unknown, ExceptionKind::DialogComputed)
    };
    Err(Signal::Error(Box::new(AlError {
        message,
        code: "Dialog".into(),
        origin,
        exception,
        ..Default::default()
    })))
}

/// `Message(String, Values...)`: the text is substituted like `StrSubstNo`
/// before it reaches the UI (corpus handlers/TestMessageHandler.al
/// Message_FormattedString_HandlerGetsFormatted).
fn message(ctx: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    let mut text = substitute(&text_arg(args, 0)?, &args[1..], ctx)?;
    // A CR LF in the text or in an argument reaches the handler as one backslash (misc/lt-system-functions/ErrorMessage.al:598-616,
    // LT_BU_Message_BackslashAndAppendLine).
    if text.contains("\r\n") {
        text = text.replace("\r\n", "\\");
    }
    ctx.ui_message(text)?;
    Ok(Value::Null)
}

/// `Confirm(String, Default, Values...)`. Values are not substituted into the
/// question a handler sees (corpus handlers/TestConfirmFormatArgs.al). An
/// omitted Default is false (docs/probes-needed.md).
fn confirm(ctx: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    let question = text_arg(args, 0)?;
    let default = match args.get(1) {
        None => false,
        Some(Value::Boolean(b)) => *b,
        Some(other) => return unsupported(&format!("Confirm default {}", other.describe())),
    };
    Ok(Value::Boolean(ctx.ui_confirm(question.to_string(), default)?))
}

/// `StrMenu(Options, Default, Instruction)`. An omitted Default is 1 and an
/// omitted Instruction is empty (docs/probes-needed.md).
fn str_menu(ctx: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    let options = text_arg(args, 0)?;
    let default = match args.get(1) {
        None => 1,
        Some(_) => int_arg(args, 1).map_or_else(|| unsupported("StrMenu default"), |n| Ok(n as i32))?,
    };
    let instruction = match args.get(2) {
        None => String::new(),
        Some(_) => text_arg(args, 2)?.to_string(),
    };
    Ok(Value::Integer(ctx.ui_str_menu(
        options.to_string(),
        default,
        instruction,
    )?))
}

/// `%n` replaced by `Format(arg n)`. A placeholder without an argument stays as written, `%0`, `%10` of one argument and a
/// `%` at the end of the text too; an argument is not scanned again (types/lt-variant/VaTTextConst.al:TextConst_StrSubstNo
/// `one_arg_only`, `no_args`, `pct10`, `pct0`, `pct5_not_given`, `pct_end`, `arg_has_percent`;
/// text/TestTextProcessingContracts.al:StrSubstNo_FewerArgsThanPlaceholders_NoError). `#n` fields are pinned for
/// `StrSubstNo` only ([`substitute_fields`]); here they are refused.
pub fn substitute(template: &str, args: &[Value], ctx: &dyn FormatCtx) -> Result<String, Signal> {
    substitute_with(template, args, ctx, false)
}

/// [`substitute`] that also fills `#n` fields, as `StrSubstNo` does (types/lt-evaluate-format-lasterror-setview/
/// P86StrSubstNo.al:Sub_FieldWidth, text/lt-text-functions/StrSubstNo.Codeunit.al:SubstNoHash): a run of `#`, one digit
/// n and a run of `#` is a field as wide as that run (`#1` 2, `##1##` 5, `#1#1` is `#1#` then `1`). `Format(arg n)` fills
/// it from the left, padded with spaces; a value longer than the field gives `*` over the whole width. A field whose
/// digit is 0 or names no argument, a `#` run without a digit and `# 1` stay as written. Two or more digits are not
/// pinned and are refused.
pub fn substitute_fields(template: &str, args: &[Value], ctx: &dyn FormatCtx) -> Result<String, Signal> {
    substitute_with(template, args, ctx, true)
}

fn substitute_with(template: &str, args: &[Value], ctx: &dyn FormatCtx, fields: bool) -> Result<String, Signal> {
    let b = template.as_bytes();
    if !fields && b.windows(2).any(|w| w[0] == b'#' && w[1].is_ascii_digit()) {
        return unsupported("a #n field in an Error or Message text (pinned for StrSubstNo only)");
    }
    let mut out = String::new();
    // `lit` is the start of the text not yet copied; `i` scans. The markers are ASCII, so every cut is a char boundary.
    let (mut lit, mut i) = (0, 0);
    while i < b.len() {
        match b[i] {
            b'%' => {
                out.push_str(&template[lit..i]);
                let rest = &template[i + 1..];
                // A number that starts with 0 is no placeholder (`%0`, `%01`, `%001` stay as written:
                // types/lt-evaluate-format-lasterror-setview/P86StrSubstNo.al:Sub_Percent t11, t19, t20).
                let digits = if rest.starts_with('0') {
                    0
                } else {
                    rest.bytes().take_while(u8::is_ascii_digit).count()
                };
                let n: usize = rest[..digits].parse().unwrap_or(0);
                match args.get(n.wrapping_sub(1)).filter(|_| digits > 0 && n >= 1) {
                    Some(v) => out.push_str(&try_format(v, 0, 0, ctx).map_err(Signal::Unsupported)?),
                    None => {
                        out.push('%');
                        out.push_str(&rest[..digits]);
                    }
                }
                i += 1 + digits;
                lit = i;
            }
            b'#' if fields => {
                let start = i;
                while i < b.len() && b[i] == b'#' {
                    i += 1;
                }
                let ds = i;
                while i < b.len() && b[i].is_ascii_digit() {
                    i += 1;
                }
                if i == ds {
                    continue;
                }
                if i - ds > 1 {
                    return unsupported("StrSubstNo with a #n field of more than one digit (not pinned)");
                }
                let n = usize::from(b[ds] - b'0');
                while i < b.len() && b[i] == b'#' {
                    i += 1;
                }
                let Some(v) = args.get(n.wrapping_sub(1)).filter(|_| n >= 1) else {
                    continue;
                };
                out.push_str(&template[lit..start]);
                let width = i - start;
                let text = try_format(v, 0, 0, ctx).map_err(Signal::Unsupported)?;
                let len = text.encode_utf16().count();
                if len > width {
                    out.extend(std::iter::repeat_n('*', width));
                } else {
                    out.push_str(&text);
                    out.extend(std::iter::repeat_n(' ', width - len));
                }
                lit = i;
            }
            _ => i += 1,
        }
    }
    out.push_str(&template[lit..]);
    Ok(out)
}

fn strlen(_: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    Ok(Value::Integer(text_arg(args, 0)?.utf16_len() as i32))
}

fn strpos(_: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    let (s, sub) = (text_arg(args, 0)?, text_arg(args, 1)?);
    // text/TestTextProcessingContracts.al StrPos_EmptySubstring_ReturnsZero.
    if sub.is_empty() {
        return Ok(Value::Integer(0));
    }
    // A match at a char boundary is the same match unit by unit; the position counts UTF-16 units. No copy of the
    // text: a loop that calls StrPos on a long text (Pageworks' asset parsing) would copy it each time.
    // One character is a byte search (no searcher to build); longer needles are searched as a string.
    let mut cs = sub.chars();
    let found = match (cs.next(), cs.next()) {
        (Some(c), None) => s.find(c),
        _ => s.find(sub.as_str()),
    };
    let pos = found.map_or(0, |b| {
        if s.is_ascii() {
            b + 1
        } else {
            s[..b].encode_utf16().count() + 1
        }
    });
    Ok(Value::Integer(pos as i32))
}

/// `Session.LogMessage`, `Session.LogSecurityAudit` and `Session.LogAuditMessage`: telemetry and the security audit log have
/// no effect AL code can see in a test, so the call returns (session/TestSessionExtended.al:76-89
/// only needs LogMessage callable; session/TestEnvironmentUserTenantContracts.al:315-374 runs
/// Change Log Management, which logs a security audit entry, and reads only the change log).
fn log_message(_: &mut dyn Ctx, _: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    Ok(Value::Null)
}

/// `Session.LogAuditMessage(Description, Result, Category, Operation, OperationResult [, Dimensions])`: the audit log has no effect AL
/// code can see, but an Operation outside the members of AuditMessageOperation (0 to 7, as the error text lists them) raises
/// (al-language-onprem/lt-system-database-onprem/PLogAuditMessage.al:P109_LogAuditMessage_Overload1 `negative`, `large`).
fn log_audit_message(c: &mut dyn Ctx, r: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    if let Some(Value::Integer(op)) = args.get(3) {
        if !(0..=7).contains(op) {
            return raise(
                format!(
                    "The AuditMessageOperation value '{op}' is invalid. Please use one the following values: 0 = AdministeredEnvironment, 1 = ConfiguredExtension, 2 = AdministeredUser, 3 = AdministeredCompany, 4 = ConfiguredIntegration, 5 = ConfiguredCopilot, 6 = ConfiguredCloudMigration, 7 = AdministeredReport."
                ),
                "InvalidValue",
            );
        }
    }
    log_message(c, r, args)
}

/// `Hyperlink(Url)` asks the client to open a page. A test has no handler for it, so the platform raises
/// `Unhandled UI: Hyperlink <url>` (record/ProbeClearAllSetKeyHyperlink.al:322-338, BC 27.5); a routine
/// that only mentions the call still runs.
fn hyperlink(_: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    let url = text_arg(args, 0)?;
    Err(Signal::Error(Box::new(AlError {
        message: format!("Unhandled UI: Hyperlink {url}"),
        code: "MissingUIHandler".into(),
        call_stack: String::new(),
        ..Default::default()
    })))
}

/// `Variant2Date(Variant)`: the Date a Variant holds (session/TestFinalCoverage.al:79-88). Another
/// content raises a conversion error whose text is not pinned: refused.
fn variant_2_date(_: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    match args.first() {
        Some(d @ Value::Date(..)) => Ok(d.clone()),
        other => unsupported(&format!(
            "Variant2Date of {other} (the conversion error is not pinned)",
            other = other.describe()
        )),
    }
}

/// `Variant2Time(Variant)`: the Time a Variant holds (session/TestFinalCoverage.al:91-100).
fn variant_2_time(_: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    match args.first() {
        Some(t @ Value::Time(_)) => Ok(t.clone()),
        other => unsupported(&format!(
            "Variant2Time of {other} (the conversion error is not pinned)",
            other = other.describe()
        )),
    }
}

/// `StartSession`, every overload. While an app installs it starts nothing, leaves `SessionId` alone
/// and answers false (or raises nothing when the result is ignored): install/TestInstallStartSession_Tests.al,
/// ALSession.ALStartSessionAsyncImpl. In a test run under TestIsolation = Codeunit it raises the
/// refusal below, also for the overload with a record, and `SessionId` stays 0
/// (session/TestStartSessionRecord.al:StartSession_FromATestCodeunit_IsRefusedUnderCodeunitIsolation).
/// Anywhere else a session would start, which this runtime does not do: refused.
fn start_session(c: &mut dyn Ctx, _: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    if c.install_scope() {
        return Ok(if c.ret_used() {
            Value::Boolean(false)
        } else {
            Value::Null
        });
    }
    // Whether BC starts a session in a test codeunit the TestRunnerHub runs under RequiredTestIsolation = Function or
    // Disabled is not pinned (al-runner issue 5609; the corpus pins only Codeunit isolation).
    if c.in_test_run() && c.isolation_from_hub() {
        return unsupported(
            "StartSession in a test codeunit run under its RequiredTestIsolation = Function or Disabled (--isolation \
             required): whether BC starts the session there is not pinned",
        );
    }
    if c.in_test_run() {
        return Err(Signal::Error(Box::new(AlError {
            message: "Sessions can only be started in tests that are run by a TestRunner that has TestIsolation set to Disabled."
                .into(),
            code: "TestStartSessionNotAllowed".into(),
            call_stack: String::new(), ..Default::default() })));
    }
    unsupported("StartSession outside an install trigger and a test run (a second session is not modelled)")
}

/// `StrCheckSum(String [, WeightString] [, Modulus])`: `(M - (sum of digit * weight) mod M) mod M`, the
/// formula of the documentation (methods-auto/text/text-strchecksum-method.md, its example gives 0
/// for '4378' with weights '1234' and modulus 7). The corpus pins only that the result is not
/// negative, is below the modulus and is not 0 for '12345' (session/TestSystemExtended.al:218-256);
/// values and the four errors: types/TestProbeScalarContracts.al:25-123.
fn strchecksum(_: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    let digits = |s: &str| -> Option<Vec<i64>> { s.chars().map(|c| c.to_digit(10).map(i64::from)).collect() };
    let source = text_arg(args, 0)?;
    let Some(source) = digits(&source) else {
        return raise("The value string must be numeric.", "ALSystemString");
    };
    let mut weights = match args.get(1) {
        Some(_) => {
            let Some(w) = digits(&text_arg(args, 1)?) else {
                return raise("The weight string must be numeric.", "ALSystemString");
            };
            w
        }
        None => Vec::new(),
    };
    // An empty source is 0 whatever the weights (types/TestProbeScalarContracts.al:StrCheckSum_Values, `empty_w1`).
    if source.is_empty() {
        return Ok(Value::Integer(0));
    }
    if weights.len() > source.len() {
        return raise(
            "The weight parameter cannot be longer than the value string.",
            "ALSystemString",
        );
    }
    // A short weight string is padded with 1 up to the length of the source.
    weights.resize(source.len(), 1);
    let modulus = int_arg(args, 2).unwrap_or(10);
    if modulus < 1 {
        return raise("The modulus parameter cannot be less than 1.", "ALSystemString");
    }
    if source.is_empty() {
        return Ok(Value::Integer(0));
    }
    let sum: i64 = source.iter().zip(&weights).map(|(d, w)| d * w).sum();
    Ok(Value::Integer(((modulus - sum % modulus) % modulus) as i32))
}

fn copystr(_: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    let whole = text_arg(args, 0)?;
    copystr_of(&whole, int_arg(args, 1).unwrap_or(1), int_arg(args, 2))
}

/// `CopyStr` of a text the caller holds by reference (the VM's fast path reads a variable without copying it).
pub fn copystr_of(whole: &crate::text::Txt, pos: i64, len: Option<i64>) -> Result<Value, Signal> {
    // A text of ASCII only is indexed by byte, with no copy: a loop of CopyStr over a long text stays linear in
    // the loop (Pageworks' asset parsing).
    let ascii = whole.is_ascii();
    let s: Vec<u16> = if ascii {
        Vec::new()
    } else {
        whole.encode_utf16().collect()
    };
    let total = if ascii { whole.len() } else { s.len() };
    // text/TestProbeTextDateFormatContracts.al:Text_SubstringBounds, `copystr_start0`, `copystr_neg_count`.
    // The two-argument form allows a start up to 2048, the three-argument form up to the largest Integer (text/lt-text-functions/
    // CopyStr.Codeunit.al:CopyStrErrors `pos0` against `pos0len`).
    if pos < 1 {
        let top = if len.is_some() { "2,147,483,647" } else { "2048" };
        return raise(
            format!(
                "The value of COPYSTR parameter 2 is outside of the permitted range. The current value is: {pos}. The permitted range is: from 1 to {top}."
            ),
            "OutsidePermittedRange",
        );
    }
    let start = (pos as usize - 1).min(total);
    let end = match len {
        Some(len) if len < 0 => {
            return raise(
                format!(
                    "The value of COPYSTR parameter 3 is outside of the permitted range. The current value is: {len}. The permitted range is: from 0 to 2,147,483,647."
                ),
                "OutsidePermittedRange",
            );
        }
        // The count may reach to the largest Integer counted from the start (CopyStrErrors `af81`).
        Some(len) if len > 2_147_483_648 - pos => {
            return raise(
                format!(
                    "The value of COPYSTR parameter 3 is outside of the permitted range. The current value is: {}. The permitted range is: from 0 to {}.",
                    group_thousands(len),
                    group_thousands(2_147_483_648 - pos)
                ),
                "OutsidePermittedRange",
            );
        }
        Some(len) => (start + len as usize).min(total),
        None => total,
    };
    if ascii {
        return Ok(Value::Text(whole[start..end].to_string().into()));
    }
    Ok(Value::Text(String::from_utf16_lossy(&s[start..end]).into()))
}

fn group_thousands(n: i64) -> String {
    let digits = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

fn strsubstno(ctx: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    let template = text_arg(args, 0)?;
    Ok(Value::Text(substitute_fields(&template, &args[1..], ctx)?.into()))
}

/// `UpperCase` and `LowerCase` follow `GlobalLanguage` for Turkish (1055) and Azerbaijani (1068): `i` becomes the dotted `İ` and `I`
/// the dotless `ı`; every other letter maps as in the default language, and Greek (1032), German (1031) and Lithuanian (1063) map as
/// the default too (text/lt-text-functions/CaseConv.Codeunit.al:145-180, LT_BU_CaseConv_TurkishGreekLanguage; :218-276,
/// PYC_CaseConv_OtherLanguages). `ToUpper`/`ToLower` do not follow it.
pub fn is_turkic_language(language: i32) -> bool {
    matches!(language, 1055 | 1068)
}

fn uppercase(ctx: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    let t = text_arg(args, 0)?;
    if is_turkic_language(ctx.language()) {
        return Ok(Value::Text(simple_upper(&t.replace('i', "İ")).into()));
    }
    Ok(Value::Text(simple_upper(&t).into()))
}

fn lowercase(ctx: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    let t = text_arg(args, 0)?;
    if is_turkic_language(ctx.language()) {
        return Ok(Value::Text(simple_lower(&t.replace('I', "ı")).into()));
    }
    Ok(Value::Text(simple_lower(&t).into()))
}

/// `ArrayLen(Array)` of a one-dimensional array: the length of the array the variable holds. A `var` or value
/// parameter takes the caller's array whatever size it was declared with (types/W7VarArray.al: a parameter declared
/// `array[3]` called with `array[2]` and `array[5]` sees 2 and 5, BC 27.5 and 29.0).
fn array_len(_: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    match args {
        // The lowering passes the dimension asked for and the number of dimensions when the dimension is outside the array.
        [Value::Integer(dim), Value::Integer(n)] => raise(
            format!(
                "The value of ARRAYLEN parameter 2 is outside of the permitted range. The current value is: {dim}. The permitted range is: from 1 to {n}."
            ),
            "OutsidePermittedRange",
        ),
        _ => array_len_of(args),
    }
}

fn array_len_of(args: &[Value]) -> Result<Value, Signal> {
    match args.first() {
        Some(Value::Array(a)) => Ok(Value::Integer(a.borrow().len() as i32)),
        other => Err(Signal::Unsupported(format!("ArrayLen of {}", other.describe()))),
    }
}

/// `Clear(var X)`: the default of the value's own kind. Records go to the VM.
fn clear(ctx: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    let v = &mut args[0];
    *v = match v {
        Value::Record(_) => return ctx.record(RecordOp::Clear, Some(v), &mut []),
        // Clearing a RecordRef/FieldRef/KeyRef closes it: a fresh, unopened handle.
        Value::Library(name, _)
            if matches!(
                name.as_str(),
                "RecordRef"
                    | "FieldRef"
                    | "KeyRef"
                    | "RecordId"
                    | "WebServiceActionContext"
                    | "SessionSettings"
                    | "Notification"
                    // A cleared XML document, document type or declaration is an unassigned one (xml/lt-xml-doctype-declaration/
                    // DocTypeErrors.al:Uninitialised_*, DocTypeRead.al: `Clear(Doc)`).
                    | "XmlDocument"
                    | "XmlDocumentType"
                    | "XmlDeclaration"
                    // types/TestClearContracts.al:Clear_TextBuilder_ResetsToEmpty.
                    | "TextBuilder"
                    | "Cookie"
                    | "JsonObject"
                    | "JsonArray"
                    | "JsonValue"
                    | "JsonToken"
            ) =>
        {
            Value::Library(name.clone(), Default::default())
        }
        // Clear empties the controls and leaves an assigned page caption alone
        // (filterpage/TestFilterPageBuilder.al:FilterPageBuilder_Clear_KeepsTheAssignedCaption).
        Value::Library(crate::library_kind::LibraryKind::FilterPageBuilder, cell) => {
            let caption = match &*cell.borrow() {
                crate::value::LibraryValue::FilterPageBuilder(s) => s.caption.clone(),
                _ => None,
            };
            let state = crate::value::LibraryValue::FilterPageBuilder(crate::value::FilterPageState {
                controls: Vec::new(),
                caption,
            });
            Value::Library(
                crate::library_kind::LibraryKind::FilterPageBuilder,
                std::rc::Rc::new(std::cell::RefCell::new(state)),
            )
        }
        Value::Boolean(_) => Value::Boolean(false),
        Value::Integer(_) => Value::Integer(0),
        Value::BigInteger(_) => Value::BigInteger(0),
        Value::Decimal(_) => Value::Decimal(Default::default()),
        Value::Byte(_) => Value::Byte(0),
        Value::Char(_) => Value::Char(0),
        Value::Text(_) => Value::Text(String::new().into()),
        Value::Code(_) => Value::Code(String::new().into()),
        Value::Date(..) => Value::Date(0, false),
        Value::Time(_) => Value::Time(0),
        Value::DateTime(_) => Value::DateTime(0),
        Value::Duration(_) => Value::Duration(0),
        Value::Guid(_) => Value::Guid([0; 16]),
        // The empty formula (types/TestDateFormulaDurationFormatContracts.al:38-42).
        Value::DateFormula(_) => Value::DateFormula(String::new()),
        Value::Option(_, m) => Value::Option(0, m.clone()),
        // A cleared Enum is the enum's initial value, ordinal 0, as a variable that was never assigned (`default_for`;
        // types/TestBlankEnumInitTests.Codeunit.al for the blank member reading as ordinal 0).
        Value::Enum(id, _) => Value::Enum(*id, 0),
        // A cleared List or Dictionary is a new, empty one (types/TestClearContracts.al:130-150,
        // collections/TestListByRefContracts.al:155 through a `var` parameter).
        Value::List(_) => Value::List(Default::default()),
        Value::Dictionary(_) => Value::Dictionary(Default::default()),
        // A cleared Blob is a new, empty buffer (streams/TestBlobCalcFieldsUncommittedWrite.al).
        Value::Blob(_) => Value::Blob(crate::stream::new_blob()),
        // A cleared TestPage or Page variable is a fresh, closed one (corpus
        // handlers/TestPageEditable_Tests.al:93 EditableBoundToPageVariable_FollowsTheRow). A page
        // that is open, waits for Trap(), or is a part is not cleared: nothing pins what follows.
        Value::TestPage(st) => {
            let object = {
                let s = st.borrow();
                if s.open || s.trapping || s.host.is_some() {
                    return unsupported("Clear of a TestPage that is open, trapping or a part (not pinned)");
                }
                s.object
            };
            Value::TestPage(std::rc::Rc::new(std::cell::RefCell::new(
                crate::value::PageState::closed(object),
            )))
        }
        // A cleared Query is a fresh one: no filters, no row cap, not open
        // (query/TestQueryRuntimeContracts.al:Query_GetFilters_ThreeFilters_OrderIsNotColumnOrder).
        Value::Query(q) => {
            let object = q.borrow().object;
            Value::Query(Rc::new(std::cell::RefCell::new(crate::value::QueryState {
                object,
                filters: Vec::new(),
                filter_replaced: false,
                top: None,
                security_filtering: None,
                run: None,
                instance: None,
                in_before_open: false,
            })))
        }
        // A codeunit or report variable nothing has used yet has no instance to release, so
        // clearing it changes nothing (handlers/TestReportRunWithRequestPage.al: `Clear(Probe)`
        // before `Probe.Run()`). Clearing one in use gives a fresh instance: after `Clear(TempBlob)` the
        // blob is empty again (streams/TestStreamsRuntimeContracts.al:WriteText_ExplicitEncodings_Bytes).
        Value::Codeunit(rc) if !rc.borrow().initialized => return Ok(Value::Null),
        Value::Codeunit(rc) => {
            let object = rc.borrow().object;
            Value::Codeunit(std::rc::Rc::new(std::cell::RefCell::new(crate::value::Instance {
                object,
                ..Default::default()
            })))
        }
        // An array goes back to its initial value element by element, in place: a `var` parameter
        // that shares it sees the change (docs/probes-needed.md; the oracle runs
        // pageupdate/PageUpdateFacts.Page.al:58 `Clear(Derived)` on an array of Decimal).
        Value::Array(a) => {
            let a = a.clone();
            // An array of JsonObject keeps every element as it was (json/lt-db-json-leads/DbJsonLeads.al:150-164). An array
            // of another JSON type is not pinned.
            match a.borrow().first() {
                Some(Value::Library(name, _)) if name.as_str() == "JsonObject" => return Ok(Value::Null),
                Some(Value::Library(name, _)) if matches!(name.as_str(), "JsonArray" | "JsonValue" | "JsonToken") => {
                    return unsupported(&format!("Clear of an array of {} (not pinned)", name.as_str()));
                }
                _ => {}
            }
            for e in a.borrow_mut().iter_mut() {
                clear(ctx, None, std::slice::from_mut(e))?;
            }
            return Ok(Value::Null);
        }
        // An unassigned DotNet variable holds nothing, so clearing it changes nothing. One that holds
        // an object is refused: whether BC calls Dispose on it, and so whether a stream the object
        // wraps is flushed, is not pinned (docs/probes-needed.md, "Clear of a DotNet variable").
        Value::Null => return Ok(Value::Null),
        Value::DotNet(o) => {
            let ty = match &o.kind {
                crate::value::DotNetKind::Host { clr_type, .. } => clr_type.as_str(),
                crate::value::DotNetKind::Enum { clr_type, .. } => clr_type.as_str(),
                crate::value::DotNetKind::Internal { type_name, .. } => type_name.as_str(),
            };
            return unsupported(&format!(
                "Clear of a DotNet variable that holds a {ty}: whether BC disposes the object is not pinned"
            ));
        }
        // A cleared stream is one that was never created again (streams/TestProbeStreamContracts.al:RecordAssign_CopiesBlobBytes).
        // A cleared SecretText is an empty one and a copy taken before keeps its content
        // (fieldref/lt-fieldref-testfield-types/SystemClearArraySecret.al:69-90, errorinfo/lt-errnotif/EnTSecret.al:15-26).
        // A cleared HttpClient is a new one: no base address, the default timeout, no default headers; the next call
        // starts it fresh (network/lt-http-relative-request/HttpRelativeRequestPath.al:NewClient clears, then sets the base address).
        Value::Library(name, _)
            if matches!(
                name.as_str(),
                "OutStream" | "InStream" | "FileUpload" | "HttpClient" | "HttpHeaders" | "SecretText"
            ) =>
        {
            Value::Library(
                name.clone(),
                std::rc::Rc::new(std::cell::RefCell::new(crate::value::LibraryValue::Empty)),
            )
        }
        other => return unsupported(&format!("Clear of a {} value", value_kind(other))),
    };
    Ok(Value::Null)
}

fn context_value(c: &dyn Ctx) -> Value {
    let members: std::sync::Arc<[String]> = crate::sysenum::option_members("ExecutionContext")
        .expect("a system option")
        .into();
    // Install inside an install codeunit, Normal anywhere else (install/TestInstallEnvVisible_Tests.al:90-127).
    Value::Option(i32::from(c.install_scope()), members)
}

fn execution_context(c: &mut dyn Ctx, _: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    Ok(context_value(c))
}

/// Of this app (no argument) or of an app of the workspace; another app's context while an
/// installer runs is not pinned.
fn module_execution_context(c: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    if let Some(id) = a.first() {
        if c.install_scope() {
            return unsupported("GetModuleExecutionContext(AppId) inside install code (not pinned)");
        }
        let Value::Guid(g) = id else {
            return unsupported(&format!("GetModuleExecutionContext with {}", id.describe()));
        };
        if !matches!(c.module_by_id(g), ModuleLookup::Found(_)) {
            return unsupported("GetModuleExecutionContext of an app the workspace does not hold (not pinned)");
        }
    }
    Ok(context_value(c))
}

/// `Enum.AsInteger()`: the ordinal (types/TestEnum.al:27-40).
fn enum_as_integer(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    match r {
        Some(Value::Enum(_, o)) => Ok(Value::Integer(*o)),
        other => unsupported(&format!("AsInteger of {}", other.describe())),
    }
}

/// The enum a type-level call (`"Name".Names()`) or a call on a variable names: lowering passes
/// the enum object as the receiver value.
fn enum_receiver(ctx: &dyn Ctx, r: Option<&mut Value>, what: &str) -> Result<(u32, Vec<(i32, String)>), Signal> {
    let Some(Value::Enum(id, _)) = r else {
        return unsupported(&format!("{what} without an enum receiver"));
    };
    let id = *id;
    ctx.enum_members(id).map(|m| (id, m)).map_or_else(
        || unsupported(&format!("{what}: this context has no enum metadata")),
        Ok,
    )
}

/// `Enum.Names()`: every declared name, the blank ordinal included
/// (types/TestEnumNamesContracts.al:19-60). The order is the declaration order, which matches
/// `Ordinals()` position by position; BC's order is not pinned (docs/probes-needed.md).
fn enum_names(ctx: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    let (_, members) = enum_receiver(ctx, r, "Enum.Names")?;
    Ok(read_only_list(
        members.into_iter().map(|(_, n)| Value::Text(n.into())).collect(),
    ))
}

/// The lists of `Enum.Names()` and `Enum.Ordinals()` cannot be changed (collections/lt-cs-odd-leads/CsCollections.al:
/// EnumOrdinals_Mutations, EnumNames_Mutations).
fn read_only_list(items: Vec<Value>) -> Value {
    let mut l: crate::value::ListVec = items.into();
    l.read_only = true;
    Value::List(Rc::new(std::cell::RefCell::new(l)))
}

/// `Enum.Ordinals()` (session/TestSystemExtended.al:381-390, types/TestLanguageEdgeCases.al:91-105).
fn enum_ordinals(ctx: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    let (_, members) = enum_receiver(ctx, r, "Enum.Ordinals")?;
    Ok(read_only_list(
        members.into_iter().map(|(o, _)| Value::Integer(o)).collect(),
    ))
}

/// `Enum.FromInteger(n)`: any number is carried, declared or not (types/TestEnum.al:83-91,
/// types/TestEnumDefaultImplTests.Codeunit.al:70).
fn enum_from_integer(ctx: &mut dyn Ctx, r: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    let (id, _) = enum_receiver(ctx, r, "Enum.FromInteger")?;
    // An Option argument converts to its ordinal, as any Integer parameter takes it.
    let n = match args.first() {
        Some(Value::Option(o, _)) => Some(*o),
        // A Variant that holds an Enum, a whole Decimal or a Text of digits takes the ordinal or the number
        // (enum/lt-enum-fromint-variant/EnumFromIntegerVariant.al:FromInteger_VariantHolding asserts the holders Integer, Enum, Decimal and Text; no other holder is added).
        // A fraction or a text that is not a whole number is not pinned.
        Some(Value::Enum(_, o)) => Some(*o),
        Some(Value::Decimal(d)) if d.fract().is_zero() => i32::try_from(d.trunc()).ok(),
        Some(Value::Text(t)) if !t.is_empty() && t.trim_start_matches('-').chars().all(|c| c.is_ascii_digit()) => {
            t.parse::<i32>().ok()
        }
        _ => int_arg(args, 0).and_then(|n| i32::try_from(n).ok()),
    };
    match n {
        Some(n) => Ok(Value::Enum(id, n)),
        None => unsupported("Enum.FromInteger with a number outside the Integer range"),
    }
}

fn is(receiver: Option<&mut Value>, kind: &str) -> Result<Value, Signal> {
    Ok(Value::Boolean(receiver.is_some_and(|v| v.is(kind))))
}

/// The kind of a value for a refusal reason: the variant name, never its contents.
fn value_kind(v: &Value) -> String {
    let d = v.describe();
    d.split(|c: char| !c.is_alphanumeric()).next().unwrap_or("").to_string()
}

/// `<Date or Boolean>.ToText()` as `Format(value)` (assumption `primitive-totext-is-format`).
/// Case mapping one character to one (types/lt-scalars/ScT70509.al:LowerUpperBasic `a9`, LowerUpperLengthsAndTargets `lenu`): an
/// upper-case `ß` stays `ß` and the dotted capital I is a plain `i`.
pub(crate) fn simple_upper(s: &str) -> String {
    s.chars()
        .map(|c| {
            let mut up = c.to_uppercase();
            match (up.next(), up.next()) {
                (Some(u), None) => u,
                _ => c,
            }
        })
        .collect()
}

/// `Text.ToUpper()` / `ToLower()`: one character to one, and a character the mapping would turn into several stays (`ß`, `ﬁ`, the dotted
/// capital `İ`), the dotless `ı` stays too (text/lt-text-instance/CaseTrim1.Codeunit.al:ToLower_ToUpper_Unicode; unlike `UpperCase`/`LowerCase`).
pub(crate) fn method_upper(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c == 'ı' {
                return c;
            }
            let mut up = c.to_uppercase();
            match (up.next(), up.next()) {
                (Some(u), None) => u,
                _ => c,
            }
        })
        .collect()
}

pub(crate) fn method_lower(s: &str) -> String {
    s.chars()
        .map(|c| {
            let mut low = c.to_lowercase();
            match (low.next(), low.next()) {
                (Some(l), None) => l,
                _ => c,
            }
        })
        .collect()
}

pub(crate) fn simple_lower(s: &str) -> String {
    s.chars().map(|c| c.to_lowercase().next().unwrap_or(c)).collect()
}

fn primitive_to_text(c: &mut dyn Ctx, r: Option<&mut Value>, what: &str, a: &[Value]) -> Result<Value, Signal> {
    match r.as_deref() {
        // `Date.ToText(true)` is the round-trip form `yyyy-mm-dd`, `ToText(false)` the default one; the first of January 1 has
        // no text (types/lt-totext-bigtext-codeunit-run/P93Date.al:ToText_Normal, ToText_Closing).
        Some(v @ Value::Date(..)) => {
            let iso = matches!(a.first(), Some(Value::Boolean(true)));
            if matches!(v, Value::Date(1, _)) {
                return raise("The date is not valid.", "DateInvalid");
            }
            match crate::format::try_format(v, 0, if iso { 9 } else { 0 }, &*c) {
                Ok(t) => Ok(Value::Text(t.into())),
                Err(e) => Err(Signal::Unsupported(e)),
            }
        }
        // The text of a Time, DateTime, Duration, Boolean and Decimal is `Format` with the default format, and with `true` the
        // round-trip form (Format standard 9): `13:45:01`, `2024-03-15T13:45:01Z`, `true`, `1234567.891`, and the Duration words
        // `1 hour 2 minutes 3 seconds` (types/lt-conversions-hidden-parts-doctype/P142ToTextLang.al:131-135, 156-160, 175-190,
        // 237-253).
        Some(
            v @ (Value::Time(_) | Value::DateTime(_) | Value::Duration(_) | Value::Boolean(_) | Value::Decimal(_)),
        ) => {
            let inv = matches!(a.first(), Some(Value::Boolean(true)));
            match crate::format::try_format(v, 0, if inv { 9 } else { 0 }, &*c) {
                Ok(t) => Ok(Value::Text(t.into())),
                Err(e) => Err(Signal::Unsupported(e)),
            }
        }
        other => Err(Signal::Unsupported(format!("{what}.ToText on {}", other.describe()))),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    #[test]
    fn the_table_finds_what_the_scan_found_for_every_id_and_nothing_for_other_text() {
        let mut ids = registered_ids();
        ids.extend(crate::platform_native::entries().map(|(i, _)| i));
        assert!(ids.len() > 500, "{}", ids.len());
        for id in ids {
            let (a, b) = (lookup(id), lookup_by_scan(id));
            assert!(a.is_some() && b.is_some(), "{id}");
            assert_eq!(a.map(|f| f as usize), b.map(|f| f as usize), "{id}");
        }
        for id in ["", "Nope.Nothing#9", "System.StrLen", "system.strlen#1"] {
            assert!(lookup(id).is_none() && lookup_by_scan(id).is_none(), "{id}");
        }
    }

    // TestField with an ErrorInfo is implemented (issue 41): a check that holds raises nothing, a failing one
    // is refused at run time (al-vm recmeta.rs `test_field`, al-vm/tests/store/scm_service_41.rs). Issue 42 refused
    // it at lowering; that entry is gone because `lookup` now finds the ids.
    #[test]
    fn test_field_with_an_error_info_is_implemented_not_refused_at_lowering() {
        for id in ["Record.TestField#2", "Record.TestField#4"] {
            assert!(refusal(id).is_none(), "{id}");
            assert!(lookup(id).is_some(), "{id}");
        }
    }

    // GetDotNetType is pinned by the OnPrem DotNet corpus app (tests/al-language-onprem-dotnet/lt-dotnet-interop,
    // DotNetType*.al); the VM runs it through `Ctx::session_op`.
    #[test]
    fn get_dot_net_type_is_implemented_not_refused_at_lowering() {
        assert!(refusal("System.GetDotNetType#1").is_none());
        assert!(lookup("System.GetDotNetType#1").is_some());
    }

    #[derive(Default)]
    pub(crate) struct TestCtx {
        err: AlError,
        install: bool,
        in_test: bool,
        result_ignored: bool,
    }
    impl FormatCtx for TestCtx {
        fn enum_name(&self, _id: u32, o: i32) -> String {
            format!("E{o}")
        }
    }
    impl Ctx for TestCtx {
        fn last_error(&self) -> AlError {
            self.err.clone()
        }
        fn set_last_error(&mut self, e: AlError) {
            self.err = e;
        }
        fn record(&mut self, m: RecordOp, _: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
            Err(Signal::Unsupported(m.name().into()))
        }
        fn commit(&mut self) -> Result<(), Signal> {
            Ok(())
        }
        fn new_guid(&mut self) -> [u8; 16] {
            [1; 16]
        }
        fn install_scope(&self) -> bool {
            self.install
        }
        fn in_test_run(&self) -> bool {
            self.in_test
        }
        fn ret_used(&self) -> bool {
            !self.result_ignored
        }
    }

    #[test]
    fn variant_conversions_and_log_message() {
        assert!(matches!(
            call("System.Variant2Date#1", vec![Value::Date(5, false)]),
            Ok(Value::Date(5, false))
        ));
        assert!(matches!(
            call("System.Variant2Time#1", vec![Value::Time(7)]),
            Ok(Value::Time(7))
        ));
        assert!(matches!(
            call("System.Variant2Date#1", vec![Value::Integer(1)]),
            Err(Signal::Unsupported(_))
        ));
        assert!(matches!(
            call("Session.LogMessage#2", vec![t("E"), t("m")]),
            Ok(Value::Null)
        ));
        // The security audit log has no effect AL can see, like LogSecurityAudit.
        assert!(matches!(
            call(
                "Session.LogAuditMessage#1",
                vec![
                    t("D"),
                    Value::Integer(0),
                    Value::Integer(0),
                    Value::Integer(1),
                    Value::Integer(0)
                ]
            ),
            Ok(Value::Null)
        ));
    }

    #[test]
    fn start_session_answers_false_in_install_and_is_refused_in_a_test_run() {
        let run = |ctx: &mut TestCtx| {
            let mut args = vec![Value::Integer(777), Value::Integer(60448)];
            lookup("Session.StartSession#1").unwrap()(ctx, None, &mut args)
        };
        // install/TestInstallStartSession_Tests.al:51-65
        let mut install = TestCtx {
            install: true,
            ..Default::default()
        };
        assert!(matches!(run(&mut install), Ok(Value::Boolean(false))));
        let mut plain = TestCtx {
            install: true,
            result_ignored: true,
            ..Default::default()
        };
        assert!(matches!(run(&mut plain), Ok(Value::Null)));
        // session/TestStartSessionRecord.al:57-66
        let mut test = TestCtx {
            in_test: true,
            ..Default::default()
        };
        assert!(matches!(run(&mut test), Err(Signal::Error(e)) if e.message.contains(
            "can only be started in tests that are run by a TestRunner that has TestIsolation set to Disabled")));
        assert!(matches!(run(&mut TestCtx::default()), Err(Signal::Unsupported(_))));
    }

    pub(crate) fn call(id: &str, args: Vec<Value>) -> Result<Value, Signal> {
        let f = lookup(id).unwrap_or_else(|| panic!("{id} not registered"));
        let mut args = args;
        f(&mut TestCtx::default(), None, &mut args)
    }
    pub(crate) fn t(s: &str) -> Value {
        Value::Text(s.into())
    }
    fn text(r: Result<Value, Signal>) -> String {
        match r {
            Ok(Value::Text(s)) => s.to_string(),
            other => panic!("{other:?}"),
        }
    }
    fn int(r: Result<Value, Signal>) -> i32 {
        match r {
            Ok(Value::Integer(i)) => i,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn every_registered_id_is_in_the_catalog() {
        let cat = crate::catalog::Catalog::load();
        let ids: Vec<String> = cat.signatures.iter().map(|s| cat.id(s)).collect();
        // Ids lowering picks for a call of a catalog overload that needs another implementation (`Evaluate` into an Option variable).
        let derived = [
            "System.EvaluateVariableOption#1",
            "System.EvaluateBounded#1",
            "System.EvaluateOptionField#1",
            "System.FormatVariant#1",
            "TextBuilder.AppendVariant#1",
            "RecordRef.GetTableVariant#1",
            "TestField.SetValueVariant#1",
            "System.FormatDecimalField#1",
            "System.FormatAutoFormatField#1",
            "System.OptionCaptionML#1",
            "Dialog.ErrorNotLiteral#1",
            "TextBuilder.AppendFieldValue#1",
            "Media.ToVariant#1",
            "Interface.AssertAssigned#1",
            "Variant.IntoInteger#1",
            "Variant.IntoOption#1",
            "InStream.OwnerOutOfScope#1",
        ];
        for id in registered_ids().into_iter().filter(|id| !derived.contains(id)) {
            assert!(ids.iter().any(|i| i == id), "{id} is not a catalog overload id");
        }
    }

    #[test]
    fn format_with_length_and_standard_format() {
        assert_eq!(text(call("System.Format#1", vec![Value::Integer(42)])), "42");
        assert_eq!(
            text(call("System.Format#1", vec![Value::Integer(42), Value::Integer(5)])),
            "   42"
        );
        assert_eq!(
            text(call(
                "System.Format#1",
                vec![Value::Boolean(true), Value::Integer(0), Value::Integer(2)]
            )),
            "1"
        );
    }

    // A standard format number a type does not have raises the pinned error, not a panic that takes the whole codeunit down
    // (text/TestProbeTextDateFormatContracts.al:Dates_Format_Time, Dates_Format_DateTime, Dates_Format_Duration).
    #[test]
    fn format_number_a_type_does_not_have_raises_the_pinned_error() {
        for (v, ty) in [
            (Value::Duration(3_600_000), "Duration"),
            (Value::Time(43_200_001), "Time"),
            (Value::DateTime(5), "DateTime"),
        ] {
            let args = vec![v, Value::Integer(0), Value::Integer(7)];
            let Err(Signal::Error(e)) = call("System.Format#1", args) else {
                panic!("{ty}: format 7 must raise");
            };
            assert_eq!(
                (e.message.as_str(), e.code.as_str()),
                (
                    format!("Standard format number 7 does not exist for the type '{ty}'.").as_str(),
                    "InvalidStandardFormat"
                )
            );
        }
    }

    #[test]
    fn a_duration_format_number_that_does_not_exist_raises() {
        // text/TestProbeTextDateFormatContracts.al:Dates_Format_Duration, `duration_fmt_5`.
        let Err(Signal::Error(e)) = call(
            "System.Format#1",
            vec![Value::Duration(86_400_000), Value::Integer(0), Value::Integer(5)],
        ) else {
            panic!("format 5 of a Duration must raise");
        };
        assert_eq!(e.code, "InvalidStandardFormat");
    }

    #[test]
    fn the_call_stack_of_a_recorded_error_is_refused_not_invented() {
        // codeunit/TestCodeunitAlCallStack.al expects AL frames such as `(CodeUnit 60211)`. The VM
        // fills them in (al-vm/tests/language/call_stack.rs); an error that has no frame gets a marker no
        // text function reads.
        let mut ctx = TestCtx::default();
        let f = lookup("System.GetLastErrorCallStack#1").unwrap();
        assert!(matches!(f(&mut ctx, None, &mut []), Ok(Value::Text(ref s)) if s.is_empty()));
        ctx.set_last_error(AlError {
            message: "boom".into(),
            code: "Dialog".into(),
            call_stack: String::new(),
            ..Default::default()
        });
        let Ok(stack @ Value::Text(_)) = f(&mut ctx, None, &mut []) else {
            panic!("a call stack is returned after an error")
        };
        assert!(matches!(&stack, Value::Text(s) if !s.is_empty()));
        assert!(text_of(&stack).is_none(), "its content cannot be inspected");
    }

    // session/TestSystemExtended.al:218-256 pins the range; the values are the documentation's formula.
    #[test]
    fn strchecksum_follows_the_documented_formula() {
        let n = |v: Result<Value, Signal>| int(v);
        assert_eq!(n(call("Text.StrCheckSum#1", vec![t("12345")])), 5);
        assert_eq!(n(call("Text.StrCheckSum#1", vec![t("12345"), t("5432")])), 5); // weights 54321: 35 mod 10 = 5
        assert_eq!(
            n(call(
                "Text.StrCheckSum#1",
                vec![t("12345"), t("5432"), Value::Integer(10)]
            )),
            5
        );
        // The example of the documentation: (7 - (4*1 + 3*2 + 7*3 + 8*4) mod 7) mod 7 = 0.
        assert_eq!(
            n(call(
                "Text.StrCheckSum#1",
                vec![t("4378"), t("1234"), Value::Integer(7)]
            )),
            0
        );
        assert_eq!(n(call("Text.StrCheckSum#1", vec![t("")])), 0);
        // types/TestProbeScalarContracts.al:StrCheckSum_NonDigitSource and the StrCheckSum tests after it.
        for (args, message) in [
            (vec![t("12a")], "The value string must be numeric."),
            (
                vec![t("12"), t("123")],
                "The weight parameter cannot be longer than the value string.",
            ),
        ] {
            let Err(Signal::Error(e)) = call("Text.StrCheckSum#1", args) else {
                panic!("{message}");
            };
            assert_eq!((e.message.as_str(), e.code.as_str()), (message, "ALSystemString"));
        }
    }

    #[test]
    fn strlen_counts_utf16() {
        assert_eq!(int(call("Text.StrLen#1", vec![t("abc")])), 3);
        assert_eq!(int(call("Text.StrLen#1", vec![t("😀")])), 2);
    }

    #[test]
    fn strpos_is_one_based_and_zero_when_absent() {
        assert_eq!(int(call("Text.StrPos#1", vec![t("hello"), t("ll")])), 3);
        assert_eq!(int(call("Text.StrPos#1", vec![t("hello"), t("x")])), 0);
    }

    #[test]
    fn copystr() {
        assert_eq!(
            text(call(
                "Text.CopyStr#1",
                vec![t("Hello World"), Value::Integer(1), Value::Integer(5)]
            )),
            "Hello"
        );
        assert_eq!(text(call("Text.CopyStr#1", vec![t("Hello"), Value::Integer(3)])), "llo");
        assert_eq!(
            text(call(
                "Text.CopyStr#1",
                vec![t("Hello"), Value::Integer(4), Value::Integer(10)]
            )),
            "lo"
        );
        // text/TestProbeTextDateFormatContracts.al:Text_SubstringBounds, `copystr_start0`.
        let Err(Signal::Error(e)) = call("Text.CopyStr#1", vec![t("Hello"), Value::Integer(0)]) else {
            panic!("CopyStr from position 0 must raise");
        };
        assert_eq!(e.code, "OutsidePermittedRange");
    }

    #[test]
    fn strsubstno_formats_its_arguments() {
        assert_eq!(
            text(call(
                "Text.StrSubstNo#1",
                vec![t("%1 and %2"), Value::Integer(7), Value::Boolean(true)]
            )),
            "7 and Yes"
        );
        assert_eq!(text(call("Text.StrSubstNo#1", vec![t("no args")])), "no args");
    }

    #[test]
    fn case_conversion() {
        assert_eq!(text(call("Text.UpperCase#1", vec![t("abc")])), "ABC");
        assert_eq!(text(call("Text.LowerCase#1", vec![t("ABC")])), "abc");
    }

    // types/lt-evaluate-format-lasterror-setview/P86StrSubstNo.al:Sub_FieldWidth and
    // text/lt-text-functions/StrSubstNo.Codeunit.al:SubstNoHash.
    #[test]
    fn strsubstno_fills_hash_fields_to_their_width() {
        let sub = |f: &str, a: Vec<Value>| {
            let mut args = vec![t(f)];
            args.extend(a);
            text(call("Text.StrSubstNo#1", args))
        };
        let a = || vec![t("a")];
        assert_eq!(sub("#1####", a()), "a     ");
        assert_eq!(sub("#1#", a()), "a  ");
        assert_eq!(sub("#1", a()), "a ");
        assert_eq!(sub("#####", a()), "#####");
        assert_eq!(sub("#1####x#2####", vec![t("a"), t("b")]), "a     xb     ");
        assert_eq!(sub("#2####", a()), "#2####");
        assert_eq!(sub("#1####%1", a()), "a     a");
        assert_eq!(sub("%1#1####", a()), "aa     ");
        assert_eq!(sub("#1####", vec![]), "#1####");
        assert_eq!(sub("# 1###", a()), "# 1###");
        assert_eq!(sub("#0###", a()), "#0###");
        assert_eq!(sub("##1##", a()), "a    ");
        assert_eq!(sub("#####1", a()), "a     ");
        assert_eq!(sub("#1#1", a()), "a  1");
        assert_eq!(sub("#1", vec![t("abcdef")]), "**");
        assert_eq!(sub("#1###", vec![t("abcdef")]), "*****");
        assert_eq!(sub("#1####", vec![Value::Integer(12)]), "12    ");
        assert_eq!(sub("#1", vec![t("")]), "  ");
        assert_eq!(sub("#1###", vec![Value::Boolean(true)]), "Yes  ");
    }

    #[test]
    fn error_substitutes_and_raises() {
        let Err(Signal::Error(e)) = call("Dialog.Error#2", vec![t("bad %1"), Value::Integer(3)]) else {
            panic!()
        };
        assert_eq!(e.message, "bad 3");
    }

    #[test]
    fn last_error_round_trip() {
        let mut ctx = TestCtx::default();
        ctx.set_last_error(AlError {
            message: "boom".into(),
            code: "Dialog".into(),
            call_stack: String::new(),
            ..Default::default()
        });
        let get = lookup("System.GetLastErrorText#1").unwrap();
        assert!(matches!(get(&mut ctx, None, &mut []), Ok(Value::Text(ref s)) if s == "boom"));
        let code = lookup("System.GetLastErrorCode#1").unwrap();
        assert!(matches!(code(&mut ctx, None, &mut []), Ok(Value::Text(ref s)) if s == "Dialog"));
        lookup("System.ClearLastError#1").unwrap()(&mut ctx, None, &mut []).unwrap();
        assert_eq!(ctx.last_error().message, "");
        assert_eq!(ctx.last_error().exception, ExceptionKind::Cleared);
    }

    #[test]
    fn abs() {
        let r = call("System.Abs#1", vec![Value::Decimal((-2).into())]);
        assert!(matches!(r, Ok(Value::Decimal(d)) if d == 2.into()));
    }

    #[test]
    fn variant_type_tests_use_the_receiver() {
        let f = lookup("Variant.IsInteger#1").unwrap();
        let mut v = Value::Integer(1);
        assert!(matches!(
            f(&mut TestCtx::default(), Some(&mut v), &mut []),
            Ok(Value::Boolean(true))
        ));
        let mut v = Value::Option(0, std::sync::Arc::from(vec![]));
        let f = lookup("Variant.IsOption#1").unwrap();
        assert!(matches!(
            f(&mut TestCtx::default(), Some(&mut v), &mut []),
            Ok(Value::Boolean(true))
        ));
    }
}
