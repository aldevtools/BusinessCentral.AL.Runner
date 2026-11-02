use super::*;

/// An app's name and its packaged resources (name relative to the resource folder, content).
pub type AppResources = (String, Vec<(String, Vec<u8>)>);

/// What `NavApp.GetModuleInfo` finds.
pub enum ModuleLookup {
    Found(crate::value::ModuleInfoState),
    /// No app of the workspace has this id and none of its manifests names it.
    NotInstalled,
    /// An app a manifest names as a dependency, which is not part of the workspace.
    OutsideWorkspace,
    /// Whether the app is installed depends on an install order nobody pinned (an app with no dependency path to the app
    /// whose install code runs): why.
    OrderUnknown(String),
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
    /// The client had `UseServerCertificateValidation` called on it: what the setting does to a request no
    /// `[HttpClientHandler]` answers is not pinned.
    pub cert_validation_set: bool,
    /// The last `UseServerCertificateValidation` of the client was `false`.
    pub cert_validation_off: bool,
    /// The client had Windows authentication set: a request no `[HttpClientHandler]` answers is not pinned.
    pub windows_auth_set: bool,
}

/// A table as `Evaluate` into a TableFilter reads it (see [`Ctx::table_filter_table`]).
pub struct TableFilterTable {
    pub name: String,
    /// The caption the FieldNotFound error names the table by.
    pub caption: String,
    pub fields: Vec<TableFilterField>,
}

/// A field of a [`TableFilterTable`].
pub struct TableFilterField {
    pub number: u32,
    pub name: String,
    /// Its caption, or why reading it is refused.
    pub caption: Result<String, String>,
    pub ty: crate::value::RtType,
    /// An Option field's members as (name, caption), or why reading them is refused; `Ok(None)` for other fields.
    pub options: Result<Option<Vec<(String, String)>>, String>,
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

    /// Refuses a read of a text of the app with id `app` (`what`, for the reason) that a translation file of the app could
    /// change in the language in force (ADR-053 step 1, `System.TranslatedText#1`). A context without a program has none.
    fn translated_app_read(&self, app: &str, what: &str) -> Result<(), Signal> {
        let _ = (app, what);
        Ok(())
    }

    /// The text of the Label of the app with id `app` whose XLIFF unit is `xliff`, in the language in force (ADR-053 step 2,
    /// `System.TranslatedText#1` with an id): `None` keeps the source text. A context without a program has none.
    fn translated_app_text(&self, app: &str, what: &str, xliff: &str) -> Result<Option<String>, Signal> {
        let _ = (app, what, xliff);
        Ok(None)
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
    /// `Record.FieldError(Field, Text)` with the form its text is written in (the flag's text depends on it).
    fn field_error_with_form(
        &mut self,
        rec: Option<&mut Value>,
        args: &mut [Value],
        form: ErrorTextForm,
    ) -> Result<Value, Signal> {
        let _ = form;
        self.record(crate::record_op::RecordOp::FieldError, rec, args)
    }
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
    /// record as Rec), for `Format` in code; for AutoFormatType 11 the format string. Refused where the context has no session.
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
    /// `Evaluate` into a TableFilter: the table `table` names (its name in any casing, or its number), with its fields.
    /// `Some(Ok(None))` where no single table has that name, `None` where the context has no program.
    fn table_filter_table(&mut self, _table: &str) -> Option<Result<Option<TableFilterTable>, Signal>> {
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
    /// The declared (source) captions of enum `id` when [`Self::enum_captions`] gives translated ones in the language in
    /// force (ADR-053 step 2); `None` when they are the source captions.
    fn enum_source_captions(&self, _id: u32) -> Option<Vec<(i32, String)>> {
        None
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
    /// The app of the outermost frame on the call stack (the root routine the session called). While an install
    /// routine runs, that is the app being installed: each install root belongs to its own app (al-testrun
    /// `install::plan`). `None` when there is no frame or the app's manifest has no readable id.
    fn module_at_root(&self) -> Option<crate::value::ModuleInfoState> {
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
    /// Whether an install routine runs in BC's install session, which has no client (al-vm `set_install_without_client`).
    fn install_without_client(&self) -> bool {
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
    /// The test codeunit runs under Disabled isolation: the TestRunnerHub's RequiredTestIsolation = Disabled, or a test
    /// runner's TestIsolation = Disabled.
    fn hub_disabled(&self) -> bool {
        false
    }
    /// `StartSession(var SessionId, CodeunitId [, Company [, var Record]])` under Disabled test isolation: runs the
    /// codeunit's OnRun in a new session to its end and commits what it wrote; the new session's id.
    fn start_background_session(
        &mut self,
        _codeunit: i32,
        _company: Option<String>,
        _record: Option<Value>,
    ) -> Result<i32, Signal> {
        Err(Signal::Unsupported(
            "StartSession: a second session is not modelled in this context".into(),
        ))
    }
    /// `StartSession` with a Timeout of `timeout_ms` milliseconds under Disabled test isolation: as
    /// [`Ctx::start_background_session`] for a session that ends before the Timeout.
    fn start_background_session_timeout(
        &mut self,
        _codeunit: i32,
        _company: Option<String>,
        _record: Option<Value>,
        _timeout_ms: i64,
    ) -> Result<i32, Signal> {
        Err(Signal::Unsupported(
            "StartSession: a second session is not modelled in this context".into(),
        ))
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
    /// The media and media sets the stored rows hold in Media and MediaSet fields (`FindOrphans`).
    fn media_holders(&mut self) -> Result<crate::media_orphans::MediaHolders, Signal> {
        Err(Signal::Unsupported("FindOrphans: this context has no database".into()))
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
    /// Whether `Clear` of a DotNet variable holding the host object `handle` of type `clr_type` changes nothing AL can see, so
    /// the variable just becomes unassigned (an OpenXml workbook writer after its `Close` and the worksheet writers it handed
    /// out, a workbook reader and its worksheet readers). The context marks the assumption it relies on.
    fn dotnet_clear_is_inert(&mut self, _handle: u64, _clr_type: &str) -> bool {
        false
    }
    /// Whether the company data codeunit (Microsoft's DemoTool) is building the company: only there content behind a carried
    /// mark (`stream::is_carried_mark`) moves unread.
    fn company_build(&self) -> bool {
        false
    }
    /// In the company build, the carried mark (`stream::is_carried_mark`) of a file written with content the runtime does not
    /// know (`fs_write_unproduced`): File.Open gives an opaque buffer instead of refusing.
    fn fs_carried_mark(&self, _path: &str) -> Option<std::sync::Arc<str>> {
        None
    }
    /// The app name and object name of the AL routine that made the current call.
    fn calling_object(&self) -> Option<(String, String)> {
        None
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
    /// `Hyperlink(url)` in a test with a bound `[HyperlinkHandler]`: runs the handler with the URL and answers true. False
    /// when no such handler is bound (the caller then raises `Unhandled UI: Hyperlink <url>`).
    fn ui_hyperlink(&mut self, _url: String) -> Result<bool, Signal> {
        Ok(false)
    }
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
