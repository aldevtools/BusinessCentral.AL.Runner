//! al-host: capability traits the host provides to the core crates (ADR-006).
//!
//! The core crates do no OS access. Whatever is environmental (the time, the user, the session)
//! comes from here, through a [`Host`] the session owns. This crate does no OS access either: the
//! real clock is a [`Clock`] the front end (`al-test-cli`) supplies.
//!
//! The defaults mirror AL Runner v2 (ADR-013, decision 9): one fixed test user, tenant and
//! session, and a fixed-start clock for a run that wants to be deterministic (the session steps it, ADR-045).
//!
//! # Time model
//!
//! A [`Clock`] answers one instant per call, in milliseconds since the Unix epoch, UTC. The
//! session time zone of the runtime is UTC (docs/probes-needed.md, row "Dates"): `Today`, `Time`
//! and `CurrentDateTime` are three views of one reading each, and `DT2Date`, `DT2Time`,
//! `CreateDateTime` and `Format` read a `DateTime` in UTC too. There is no zone offset to
//! configure, so the date part of `CurrentDateTime` always equals `Today`.

pub mod dotnet;
pub mod inproc;
pub mod random;
pub mod seed;
pub mod testdata;
pub mod tz;
pub mod windows_zones;

/// The wall clock.
pub trait Clock {
    /// The current instant: milliseconds since 1970-01-01T00:00:00Z.
    fn now_utc_millis(&self) -> i64;
}

/// A clock that always answers the same instant. The default for a run that must be reproducible;
/// the session adds the time `Sleep` has waited and keeps system stamps strictly increasing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedClock(pub i64);

/// 2024-01-15T12:00:00.000Z, midday so a zone shift of up to 12 hours cannot move the date.
pub const DEFAULT_INSTANT_MILLIS: i64 = 1_705_320_000_000;

impl FixedClock {
    pub const fn new(millis_since_unix_epoch: i64) -> FixedClock {
        FixedClock(millis_since_unix_epoch)
    }
}

impl Default for FixedClock {
    fn default() -> FixedClock {
        FixedClock(DEFAULT_INSTANT_MILLIS)
    }
}

impl Clock for FixedClock {
    fn now_utc_millis(&self) -> i64 {
        self.0
    }
}

/// The members of the platform's `ClientType` option that a host answers with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientType {
    Background,
    Web,
    Windows,
}

impl ClientType {
    /// The member name as AL writes it (`ClientType::Background`).
    pub fn name(self) -> &'static str {
        match self {
            ClientType::Background => "Background",
            ClientType::Web => "Web",
            ClientType::Windows => "Windows",
        }
    }
}

/// The server base address when nothing sets one (ADR-042).
pub const DEFAULT_SERVER_URL: &str = "http://localhost:8080/BC";

/// Checks a `--server-url` value: an absolute `http` or `https` URL with a host.
pub fn check_server_url(url: &str) -> Result<String, String> {
    let u = url.trim();
    let rest = u
        .strip_prefix("http://")
        .or_else(|| u.strip_prefix("https://"))
        .ok_or_else(|| format!("`{url}` is not an absolute http or https URL"))?;
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    if host.is_empty() || u.contains(char::is_whitespace) || u.contains(['?', '#']) {
        return Err(format!(
            "`{url}` is not an absolute http or https URL without query (scheme://host[:port]/path)"
        ));
    }
    Ok(u.to_string())
}

/// What kind of server the session runs on, as far as Environment Information can tell
/// (`--environment`, ADR-014 amendment 2026-10-15).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Environment {
    /// The probed oracle tier: a SaaS-like sandbox (`IsSaaS` and `IsSandbox` Yes, `IsProduction` No).
    #[default]
    Saas,
    /// An on-prem server, which Microsoft's own test suites run on: `IsSaaS`, `IsSandbox` No until
    /// a test library's `SetTestability*` changes it.
    OnPrem,
}

impl std::str::FromStr for Environment {
    type Err = String;
    fn from_str(s: &str) -> Result<Environment, String> {
        match s.to_ascii_lowercase().as_str() {
            "saas" => Ok(Environment::Saas),
            "onprem" => Ok(Environment::OnPrem),
            other => Err(format!("`{other}` is not an environment (saas or onprem)")),
        }
    }
}

/// Who runs the code, and on what kind of server.
pub trait Identity {
    /// The kind of server (default: the probed SaaS-like sandbox).
    fn environment(&self) -> Environment {
        Environment::Saas
    }
    /// `UserId()`.
    fn user_id(&self) -> String;
    /// `UserSecurityId()`, as the 16 bytes of the Guid in text order.
    fn user_security_id(&self) -> [u8; 16];
    /// `CompanyName()`: the company the session runs in (`BcRuntime.cs:1309`).
    fn company_name(&self) -> String;
    /// `SessionId()`; positive for a live session (session/TestSessionIdentityValues.al:43-59).
    fn session_id(&self) -> i32;
    /// `ServiceInstanceId()`; positive (session/TestSessionIdentityValues.al:24-40).
    fn service_instance_id(&self) -> i32;
    /// `TenantId()`; not empty (session/TestDatabaseExtended.al:63-70).
    fn tenant_id(&self) -> String;
    /// `GuiAllowed()` (session/TestBCPlatformContracts.al:27-32).
    fn gui_allowed(&self) -> bool;
    /// `IsServiceTier()` (session/TestBCPlatformContracts.al:37-42).
    fn is_service_tier(&self) -> bool;
    /// `WindowsLanguage()`: the language id of the Windows user (a positive LCID).
    fn windows_language(&self) -> i32;
    /// The language id `GlobalLanguage()` answers until AL sets it.
    fn global_language(&self) -> i32;
    /// The locale (regional settings) id of the session, which `SessionSettings.Init` reads.
    fn locale_id(&self) -> i32;
    /// The Windows name of the session's time zone, which `SessionSettings.Init` reads. The
    /// runtime's time model is UTC (see the crate docs), so a host answers `UTC`.
    fn time_zone(&self) -> String;
    /// `Session.CurrentClientType()`.
    fn current_client_type(&self) -> ClientType;
    /// `Session.DefaultClientType()`.
    fn default_client_type(&self) -> ClientType;
    /// The id of the company the session runs in: the `Id` of its row in the Company table.
    fn company_id(&self) -> [u8; 16] {
        COMPANY_ID
    }
    /// The base address of the web client that `GetUrl` builds on: scheme, host, port and instance path, no query
    /// (ADR-042). The default is a stated host value, not a claim about BC.
    fn server_base_url(&self) -> String {
        DEFAULT_SERVER_URL.into()
    }
    /// The name of the machine that hosts the session (the `Host Name` of the Session table).
    fn host_name(&self) -> String {
        "localhost".into()
    }
    /// The time zones the host knows, as the Time Zone table lists them (`No.` counts from 1 in
    /// this order). The corpus pins only that there is more than one, that every one has an id and
    /// that `No.` runs 1 to N (record/TestTimeZoneVirtualTable.al); the list is the host's.
    fn time_zones(&self) -> Vec<TimeZoneInfo> {
        default_time_zones()
    }
    /// The companies of the database (ADR-046 decision 2, the `--companies` opt-in). The default is one company, the
    /// session company.
    fn companies(&self) -> Companies {
        Companies::One
    }
}

/// The companies a run declares for its database (ADR-046 decision 2).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub enum Companies {
    /// One company, the session company (the default): the install code of the apps under test runs there only.
    #[default]
    One,
    /// The complete company list of the database, in run order; the first is the session company.
    List(Vec<String>),
    /// Every company of the database the run starts from: the session company first, then the others in the key order
    /// of the Company table. A run without test data starts from an empty database, so this is the session company.
    All,
}

impl Companies {
    /// Parses the value of `--companies`: `all`, or names separated by commas. A name given twice, or an empty name, is
    /// an error.
    pub fn parse(text: &str) -> Result<Companies, String> {
        if text.trim().eq_ignore_ascii_case("all") {
            return Ok(Companies::All);
        }
        let mut names: Vec<String> = Vec::new();
        for n in text.split(',') {
            let n = n.trim();
            if n.is_empty() {
                return Err(format!("--companies `{text}`: an empty company name"));
            }
            if names.iter().any(|m| m == n) {
                return Err(format!("--companies `{text}`: the company `{n}` is named twice"));
            }
            names.push(n.to_string());
        }
        Ok(Companies::List(names))
    }

    /// Checks that a list starts with the session company `session` (ADR-046 decision 2: the first name is the session
    /// company, or the run stops before anything runs).
    pub fn check_session_first(&self, session: &str) -> Result<(), String> {
        match self {
            Companies::List(names) if names.first().map(String::as_str) != Some(session) => Err(format!(
                "--companies: the first company must be the session company `{session}`, not `{}`",
                names.first().map(String::as_str).unwrap_or("")
            )),
            _ => Ok(()),
        }
    }

    /// The text of the setting, for keys and messages: empty for one company.
    pub fn key_text(&self) -> String {
        match self {
            Companies::One => String::new(),
            Companies::All => "all".into(),
            Companies::List(n) => n.join(","),
        }
    }
}

/// One entry of the Time Zone table: a Windows time zone id and its display name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeZoneInfo {
    pub id: String,
    pub display_name: String,
}

/// The time zones of the default host: a few Windows zones with the names Windows gives them.
pub fn default_time_zones() -> Vec<TimeZoneInfo> {
    [
        ("UTC", "(UTC) Coordinated Universal Time"),
        ("GMT Standard Time", "(UTC+00:00) Dublin, Edinburgh, Lisbon, London"),
        (
            "W. Europe Standard Time",
            "(UTC+01:00) Amsterdam, Berlin, Bern, Rome, Stockholm, Vienna",
        ),
        ("Eastern Standard Time", "(UTC-05:00) Eastern Time (US & Canada)"),
        ("Pacific Standard Time", "(UTC-08:00) Pacific Time (US & Canada)"),
    ]
    .into_iter()
    .map(|(id, display_name)| TimeZoneInfo {
        id: id.into(),
        display_name: display_name.into(),
    })
    .collect()
}

/// `{C0A1BDFA-0000-0000-0000-43524F4E5553}`, the id of the company (ADR-013 decision 9).
pub const COMPANY_ID: [u8; 16] = [
    0xC0, 0xA1, 0xBD, 0xFA, 0, 0, 0, 0, 0, 0, 0x43, 0x52, 0x4F, 0x4E, 0x55, 0x53,
];

/// The identity AL Runner v2 gives the code it runs (ADR-013, decision 9; `BcRuntime.cs`).
#[derive(Debug, Clone, Copy, Default)]
pub struct AlRunnerIdentity;

/// `{C0A1BDFA-0000-0000-0000-545553545553}`, the user's security id (`BcRuntime.cs:1441`).
pub const USER_SECURITY_ID: [u8; 16] = [
    0xC0, 0xA1, 0xBD, 0xFA, 0, 0, 0, 0, 0, 0, 0x54, 0x55, 0x53, 0x54, 0x55, 0x53,
];

/// The LCID of en-US, the culture of the runtime (ADR-013, decision 3).
pub const LCID_EN_US: i32 = 1033;

impl Identity for AlRunnerIdentity {
    fn user_id(&self) -> String {
        "TESTUSER".into()
    }
    fn user_security_id(&self) -> [u8; 16] {
        USER_SECURITY_ID
    }
    /// The company of the oracle's test database (session/TestEnvironmentUserTenantContracts.al:238);
    /// AL Runner's `My Company` (ADR-013 decision 9) is superseded.
    fn company_name(&self) -> String {
        "CRONUS International Ltd.".into()
    }
    /// AL Runner draws the session id from a counter that starts at 1 (`SessionPatches.cs`), and
    /// the corpus asks for a positive id. ADR-013 listed 0 before codeunit 60023 pinned this.
    fn session_id(&self) -> i32 {
        1
    }
    /// `EnvironmentPatches.SkeletonServiceInstanceId`.
    fn service_instance_id(&self) -> i32 {
        1
    }
    /// The tenant id of a standalone run (ADR-013, decision 3).
    fn tenant_id(&self) -> String {
        "default".into()
    }
    fn gui_allowed(&self) -> bool {
        true
    }
    fn is_service_tier(&self) -> bool {
        true
    }
    fn windows_language(&self) -> i32 {
        LCID_EN_US
    }
    fn global_language(&self) -> i32 {
        LCID_EN_US
    }
    fn locale_id(&self) -> i32 {
        LCID_EN_US
    }
    fn time_zone(&self) -> String {
        "UTC".into()
    }
    /// The oracle's tests run in a web client session (session/W5SessionFacts.al:135, :137); AL Runner's
    /// `Background` is superseded.
    fn current_client_type(&self) -> ClientType {
        ClientType::Web
    }
    /// The oracle's tier names Windows (session/W5SessionFacts.al:136, :138).
    fn default_client_type(&self) -> ClientType {
        ClientType::Windows
    }
}

/// An identity on another kind of server than the one it would name.
struct InEnvironment {
    inner: Box<dyn Identity>,
    environment: Environment,
}

impl Identity for InEnvironment {
    fn environment(&self) -> Environment {
        self.environment
    }
    fn user_id(&self) -> String {
        self.inner.user_id()
    }
    fn user_security_id(&self) -> [u8; 16] {
        self.inner.user_security_id()
    }
    fn company_name(&self) -> String {
        self.inner.company_name()
    }
    fn session_id(&self) -> i32 {
        self.inner.session_id()
    }
    fn service_instance_id(&self) -> i32 {
        self.inner.service_instance_id()
    }
    fn tenant_id(&self) -> String {
        self.inner.tenant_id()
    }
    fn gui_allowed(&self) -> bool {
        self.inner.gui_allowed()
    }
    fn is_service_tier(&self) -> bool {
        self.inner.is_service_tier()
    }
    fn windows_language(&self) -> i32 {
        self.inner.windows_language()
    }
    fn global_language(&self) -> i32 {
        self.inner.global_language()
    }
    fn locale_id(&self) -> i32 {
        self.inner.locale_id()
    }
    fn time_zone(&self) -> String {
        self.inner.time_zone()
    }
    fn current_client_type(&self) -> ClientType {
        self.inner.current_client_type()
    }
    fn default_client_type(&self) -> ClientType {
        self.inner.default_client_type()
    }
    fn company_id(&self) -> [u8; 16] {
        self.inner.company_id()
    }
    fn host_name(&self) -> String {
        self.inner.host_name()
    }
    fn server_base_url(&self) -> String {
        self.inner.server_base_url()
    }
    fn time_zones(&self) -> Vec<TimeZoneInfo> {
        self.inner.time_zones()
    }
    fn companies(&self) -> Companies {
        self.inner.companies()
    }
}

/// An identity whose server answers at another base address.
struct InServerUrl {
    inner: Box<dyn Identity>,
    url: String,
}

impl Identity for InServerUrl {
    fn server_base_url(&self) -> String {
        self.url.clone()
    }
    fn environment(&self) -> Environment {
        self.inner.environment()
    }
    fn user_id(&self) -> String {
        self.inner.user_id()
    }
    fn user_security_id(&self) -> [u8; 16] {
        self.inner.user_security_id()
    }
    fn company_name(&self) -> String {
        self.inner.company_name()
    }
    fn session_id(&self) -> i32 {
        self.inner.session_id()
    }
    fn service_instance_id(&self) -> i32 {
        self.inner.service_instance_id()
    }
    fn tenant_id(&self) -> String {
        self.inner.tenant_id()
    }
    fn gui_allowed(&self) -> bool {
        self.inner.gui_allowed()
    }
    fn is_service_tier(&self) -> bool {
        self.inner.is_service_tier()
    }
    fn windows_language(&self) -> i32 {
        self.inner.windows_language()
    }
    fn global_language(&self) -> i32 {
        self.inner.global_language()
    }
    fn locale_id(&self) -> i32 {
        self.inner.locale_id()
    }
    fn time_zone(&self) -> String {
        self.inner.time_zone()
    }
    fn current_client_type(&self) -> ClientType {
        self.inner.current_client_type()
    }
    fn default_client_type(&self) -> ClientType {
        self.inner.default_client_type()
    }
    fn company_id(&self) -> [u8; 16] {
        self.inner.company_id()
    }
    fn host_name(&self) -> String {
        self.inner.host_name()
    }
    fn time_zones(&self) -> Vec<TimeZoneInfo> {
        self.inner.time_zones()
    }
    fn companies(&self) -> Companies {
        self.inner.companies()
    }
}

/// Identity values a stability run sets (ADR-058 `identity.*`); `None` keeps the inner identity's value.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct IdentityOverrides {
    pub user_id: Option<String>,
    pub user_security_id: Option<[u8; 16]>,
    pub session_id: Option<i32>,
    pub service_instance_id: Option<i32>,
    pub tenant_id: Option<String>,
}

/// An identity with some of its values replaced ([`IdentityOverrides`]).
struct WithOverrides {
    inner: Box<dyn Identity>,
    o: IdentityOverrides,
}

impl Identity for WithOverrides {
    fn server_base_url(&self) -> String {
        self.inner.server_base_url()
    }
    fn environment(&self) -> Environment {
        self.inner.environment()
    }
    fn user_id(&self) -> String {
        self.o.user_id.clone().unwrap_or_else(|| self.inner.user_id())
    }
    fn user_security_id(&self) -> [u8; 16] {
        self.o.user_security_id.unwrap_or_else(|| self.inner.user_security_id())
    }
    fn company_name(&self) -> String {
        self.inner.company_name()
    }
    fn session_id(&self) -> i32 {
        self.o.session_id.unwrap_or_else(|| self.inner.session_id())
    }
    fn service_instance_id(&self) -> i32 {
        self.o
            .service_instance_id
            .unwrap_or_else(|| self.inner.service_instance_id())
    }
    fn tenant_id(&self) -> String {
        self.o.tenant_id.clone().unwrap_or_else(|| self.inner.tenant_id())
    }
    fn gui_allowed(&self) -> bool {
        self.inner.gui_allowed()
    }
    fn is_service_tier(&self) -> bool {
        self.inner.is_service_tier()
    }
    fn windows_language(&self) -> i32 {
        self.inner.windows_language()
    }
    fn global_language(&self) -> i32 {
        self.inner.global_language()
    }
    fn locale_id(&self) -> i32 {
        self.inner.locale_id()
    }
    fn time_zone(&self) -> String {
        self.inner.time_zone()
    }
    fn current_client_type(&self) -> ClientType {
        self.inner.current_client_type()
    }
    fn default_client_type(&self) -> ClientType {
        self.inner.default_client_type()
    }
    fn company_id(&self) -> [u8; 16] {
        self.inner.company_id()
    }
    fn host_name(&self) -> String {
        self.inner.host_name()
    }
    fn time_zones(&self) -> Vec<TimeZoneInfo> {
        self.inner.time_zones()
    }
    fn companies(&self) -> Companies {
        self.inner.companies()
    }
}

/// An identity whose database holds the companies given (`--companies`).
struct WithCompanies {
    inner: Box<dyn Identity>,
    companies: Companies,
}

impl Identity for WithCompanies {
    fn environment(&self) -> Environment {
        self.inner.environment()
    }
    fn user_id(&self) -> String {
        self.inner.user_id()
    }
    fn user_security_id(&self) -> [u8; 16] {
        self.inner.user_security_id()
    }
    fn company_name(&self) -> String {
        self.inner.company_name()
    }
    fn session_id(&self) -> i32 {
        self.inner.session_id()
    }
    fn service_instance_id(&self) -> i32 {
        self.inner.service_instance_id()
    }
    fn tenant_id(&self) -> String {
        self.inner.tenant_id()
    }
    fn gui_allowed(&self) -> bool {
        self.inner.gui_allowed()
    }
    fn is_service_tier(&self) -> bool {
        self.inner.is_service_tier()
    }
    fn windows_language(&self) -> i32 {
        self.inner.windows_language()
    }
    fn global_language(&self) -> i32 {
        self.inner.global_language()
    }
    fn locale_id(&self) -> i32 {
        self.inner.locale_id()
    }
    fn time_zone(&self) -> String {
        self.inner.time_zone()
    }
    fn current_client_type(&self) -> ClientType {
        self.inner.current_client_type()
    }
    fn default_client_type(&self) -> ClientType {
        self.inner.default_client_type()
    }
    fn company_id(&self) -> [u8; 16] {
        self.inner.company_id()
    }
    fn host_name(&self) -> String {
        self.inner.host_name()
    }
    fn server_base_url(&self) -> String {
        self.inner.server_base_url()
    }
    fn time_zones(&self) -> Vec<TimeZoneInfo> {
        self.inner.time_zones()
    }
    fn companies(&self) -> Companies {
        self.companies.clone()
    }
}

/// An identity that runs in another company than the one it would name.
struct InCompany {
    inner: Box<dyn Identity>,
    company: String,
}

impl Identity for InCompany {
    fn environment(&self) -> Environment {
        self.inner.environment()
    }
    fn user_id(&self) -> String {
        self.inner.user_id()
    }
    fn user_security_id(&self) -> [u8; 16] {
        self.inner.user_security_id()
    }
    fn company_name(&self) -> String {
        self.company.clone()
    }
    fn session_id(&self) -> i32 {
        self.inner.session_id()
    }
    fn service_instance_id(&self) -> i32 {
        self.inner.service_instance_id()
    }
    fn tenant_id(&self) -> String {
        self.inner.tenant_id()
    }
    fn gui_allowed(&self) -> bool {
        self.inner.gui_allowed()
    }
    fn is_service_tier(&self) -> bool {
        self.inner.is_service_tier()
    }
    fn windows_language(&self) -> i32 {
        self.inner.windows_language()
    }
    fn global_language(&self) -> i32 {
        self.inner.global_language()
    }
    fn locale_id(&self) -> i32 {
        self.inner.locale_id()
    }
    fn time_zone(&self) -> String {
        self.inner.time_zone()
    }
    fn current_client_type(&self) -> ClientType {
        self.inner.current_client_type()
    }
    fn default_client_type(&self) -> ClientType {
        self.inner.default_client_type()
    }
    fn company_id(&self) -> [u8; 16] {
        self.inner.company_id()
    }
    fn host_name(&self) -> String {
        self.inner.host_name()
    }
    fn server_base_url(&self) -> String {
        self.inner.server_base_url()
    }
    fn time_zones(&self) -> Vec<TimeZoneInfo> {
        self.inner.time_zones()
    }
    fn companies(&self) -> Companies {
        self.inner.companies()
    }
}

/// The file system AL code sees through the `File` type (ADR-006). Paths are the text AL passes;
/// the capability does not interpret them. A file is a byte string.
///
/// The default is [`MemFileSystem`]: nothing leaves the process, and every session starts empty.
/// The command line can supply a directory-backed one (`al-test-cli/src/host.rs`).
pub trait FileSystem {
    /// The bytes of the file, `None` if there is none.
    fn read(&self, path: &str) -> Option<Vec<u8>>;
    /// Creates the file or replaces its content.
    fn write(&mut self, path: &str, bytes: &[u8]) -> Result<(), String>;
    /// Whether a file with this path exists.
    fn exists(&self, path: &str) -> bool;
    /// Deletes the file; `false` if there was none.
    fn remove(&mut self, path: &str) -> bool;
    /// The paths of the files in the folder `dir`, only for a folder AL Runner staged itself for the company build (the
    /// DemoTool's resources, al-testrun company_data.rs). `None` for every other folder: a folder listing a test could
    /// observe is not served (Windows and Linux BC differ on it, bc-linux issue 175).
    fn staged_folder_files(&self, _dir: &str) -> Option<Vec<String>> {
        None
    }
    /// The paths of the files directly in `dir`, for a folder AL code created in the company build's temporary folder (al-testrun
    /// company_data.rs), in name order without case: `None` for every other folder.
    fn temp_folder_files(&self, _dir: &str) -> Option<Vec<String>> {
        None
    }
    /// Whether .NET's static `System.IO.Directory` and `System.IO.File` calls on `path` are answered by this file system
    /// instead of being refused: only the company build's (al-testrun company_data.rs, owner rule 2026-10-22: folder
    /// access only inside the company build) for its temporary folder and its staged folders.
    fn serves_path(&self, _path: &str) -> bool {
        false
    }
    /// Whether the folder exists (for a path [`FileSystem::serves_path`] accepts, and for the folder of a `File.Create` or
    /// `File.Open` name under `TemporaryPath()`).
    fn dir_exists(&self, _path: &str) -> bool {
        false
    }
    /// Creates the folder and the folders above it.
    fn create_dir(&mut self, _path: &str) -> Result<(), String> {
        Err("this file system has no folders".into())
    }
    /// Removes the folder; with `recursive` also what it holds.
    fn remove_dir(&mut self, _path: &str, _recursive: bool) -> Result<(), String> {
        Err("this file system has no folders".into())
    }
    /// Whether the whole file system lives in the session's memory, so .NET's static `System.IO.File` and
    /// `System.IO.Directory` calls are answered from it (ADR-056). A file system on a real directory answers false: .NET
    /// file calls there stay refused, and nothing reaches the disk through .NET.
    fn in_memory(&self) -> bool {
        false
    }
    /// A file or folder whose path equals `path` only when ASCII case and `/` against `\` are ignored, while `path`
    /// itself names neither (ADR-056: Windows would find it, and whether BC does is not pinned).
    fn near_match(&self, _path: &str) -> Option<String> {
        None
    }
}

/// The temporary folders of a run (`TemporaryPath()` is `\TEMP\`, `File.CreateTempFile` names files under `temp/`), as
/// folder keys: they always exist.
const MEM_TEMP_ROOTS: [&str; 2] = ["\\TEMP", "temp"];

/// The key of a folder: `/` read as `\`, closing separators dropped.
fn mem_dir_key(path: &str) -> String {
    path.replace('/', "\\").trim_end_matches('\\').to_string()
}

fn mem_fold(path: &str) -> String {
    mem_dir_key(path).to_ascii_lowercase()
}

/// Files kept in memory, by path, and the folders AL code created (ADR-056). Files match exactly and case sensitive (the
/// directory-backed system follows the host OS); folders match with `/` read as `\` and closing separators dropped. The
/// temporary folders always exist.
#[derive(Debug, Clone, Default)]
pub struct MemFileSystem {
    files: std::collections::BTreeMap<String, Vec<u8>>,
    dirs: std::collections::BTreeSet<String>,
}

impl MemFileSystem {
    /// The paths of all files, in order.
    pub fn paths(&self) -> Vec<String> {
        self.files.keys().cloned().collect()
    }
}

impl FileSystem for MemFileSystem {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        self.files.get(path).cloned()
    }
    fn write(&mut self, path: &str, bytes: &[u8]) -> Result<(), String> {
        self.files.insert(path.to_string(), bytes.to_vec());
        Ok(())
    }
    fn exists(&self, path: &str) -> bool {
        self.files.contains_key(path)
    }
    fn remove(&mut self, path: &str) -> bool {
        self.files.remove(path).is_some()
    }
    fn in_memory(&self) -> bool {
        true
    }
    /// The temporary folders and a folder AL code created, until it is removed; the empty text names none
    /// (QxDotNetDemoShapes.al:279-302: missing No, created Yes, with a closing separator Yes, a file No, '' No,
    /// TemporaryPath Yes, deleted No).
    fn dir_exists(&self, path: &str) -> bool {
        let k = mem_dir_key(path);
        !path.is_empty() && (MEM_TEMP_ROOTS.contains(&k.as_str()) || self.dirs.contains(&k))
    }
    /// The folder and the folders above it (a nested folder at once: QxDotNetDemoShapes.al:313-314). A file of that name
    /// raises a .NET exception no test pins: refused.
    fn create_dir(&mut self, path: &str) -> Result<(), String> {
        let k = mem_dir_key(path);
        if k.is_empty() || self.files.keys().any(|f| mem_dir_key(f) == k) {
            return Err("a file of that name exists, or the path is empty (the .NET exception is not pinned)".into());
        }
        let mut at = k.as_str();
        while !at.is_empty() && !MEM_TEMP_ROOTS.contains(&at) {
            self.dirs.insert(at.to_string());
            at = &at[..at.rfind('\\').unwrap_or(0)];
        }
        Ok(())
    }
    /// A folder AL code created, with what it holds when `recursive` (QxDotNetDemoShapes.al:317-327); a missing folder,
    /// a temporary folder, and one that holds something without `recursive` raise .NET exceptions no test pins: refused.
    fn remove_dir(&mut self, path: &str, recursive: bool) -> Result<(), String> {
        let k = mem_dir_key(path);
        if !self.dirs.contains(&k) {
            return Err("the folder does not exist (the .NET exception is not pinned)".into());
        }
        let prefix = format!("{k}\\");
        let files: Vec<String> = self
            .files
            .keys()
            .filter(|f| mem_dir_key(f).starts_with(&prefix))
            .cloned()
            .collect();
        let below: Vec<String> = self.dirs.iter().filter(|d| d.starts_with(&prefix)).cloned().collect();
        if !recursive && !(files.is_empty() && below.is_empty()) {
            return Err("the folder is not empty (the .NET exception is not pinned)".into());
        }
        for f in files {
            self.files.remove(&f);
        }
        for d in below {
            self.dirs.remove(&d);
        }
        self.dirs.remove(&k);
        Ok(())
    }
    fn near_match(&self, path: &str) -> Option<String> {
        if self.files.contains_key(path) || self.dir_exists(path) {
            return None;
        }
        let want = mem_fold(path);
        if want.is_empty() {
            return None;
        }
        self.files
            .keys()
            .chain(self.dirs.iter())
            .map(String::as_str)
            .chain(MEM_TEMP_ROOTS)
            .find(|p| mem_fold(p) == want)
            .map(str::to_string)
    }
}

/// Everything environmental a session needs.
pub struct Host {
    pub clock: Box<dyn Clock>,
    pub identity: Box<dyn Identity>,
    pub files: Box<dyn FileSystem>,
    /// The seed of the session's `Random` before AL calls `Randomize(seed)`. BC seeds it from the
    /// clock; AL sees only the range of the numbers, so the host fixes it (ADR-009).
    pub random_seed: i32,
    /// The seed of `CreateGuid` and new SystemIds; `None` follows [`Host::random_seed`] (ADR-058 `guid.seed`).
    pub guid_seed: Option<i32>,
    /// The run seed (ADR-061): what [`seed::derive`] and [`seed::guid_key`] derive each test method's `Random` seed
    /// and Guid key from. The caller picks it (the CLI from the OS, `al-test-cli` [`seed::FIXED_RUN_SEED`]); install
    /// code does not use it.
    pub run_seed: i32,
    /// How far each read of the clock by AL code moves it, in milliseconds (ADR-045: 1; ADR-058 `clock.stepMs`).
    /// Always positive: two reads must differ.
    pub clock_step_ms: i64,
}

/// The seed a session's `Random` starts from on the default host.
pub const DEFAULT_RANDOM_SEED: i32 = 1;

impl Host {
    pub fn new(clock: impl Clock + 'static, identity: impl Identity + 'static) -> Host {
        Host {
            clock: Box::new(clock),
            identity: Box::new(identity),
            files: Box::new(MemFileSystem::default()),
            random_seed: DEFAULT_RANDOM_SEED,
            guid_seed: None,
            run_seed: seed::FIXED_RUN_SEED,
            clock_step_ms: 1,
        }
    }

    /// The same host, running in the company `company` (`CompanyName()` answers it). Test data
    /// from a restored company runs in that company.
    pub fn in_company(mut self, company: &str) -> Host {
        self.identity = Box::new(InCompany {
            inner: self.identity,
            company: company.to_string(),
        });
        self
    }

    /// The same host whose database holds the companies given (`--companies`, ADR-046 decision 2).
    pub fn with_companies(mut self, companies: Companies) -> Host {
        if companies == Companies::One {
            return self;
        }
        self.identity = Box::new(WithCompanies {
            inner: self.identity,
            companies,
        });
        self
    }

    /// The same host on another kind of server (`--environment`).
    pub fn in_environment(mut self, environment: Environment) -> Host {
        self.identity = Box::new(InEnvironment {
            inner: self.identity,
            environment,
        });
        self
    }

    /// The same host whose server answers at `url` (`--server-url`, ADR-042).
    pub fn with_server_url(mut self, url: &str) -> Host {
        self.identity = Box::new(InServerUrl {
            inner: self.identity,
            url: url.to_string(),
        });
        self
    }

    /// The same host with some identity values replaced (ADR-058 `identity.*`).
    pub fn with_identity_overrides(mut self, o: IdentityOverrides) -> Host {
        if o == IdentityOverrides::default() {
            return self;
        }
        self.identity = Box::new(WithOverrides {
            inner: self.identity,
            o,
        });
        self
    }

    /// The same host on another file system.
    pub fn with_files(mut self, files: impl FileSystem + 'static) -> Host {
        self.files = Box::new(files);
        self
    }

    /// The deterministic test host: a frozen clock and AL Runner's identity.
    pub fn deterministic() -> Host {
        Host::new(FixedClock::default(), AlRunnerIdentity)
    }

    /// AL Runner's identity on the clock given.
    pub fn with_clock(clock: impl Clock + 'static) -> Host {
        Host::new(clock, AlRunnerIdentity)
    }
}

impl Default for Host {
    fn default() -> Host {
        Host::deterministic()
    }
}

#[cfg(test)]
mod tests {
    // ADR-046 decision 2: `--companies NAME[,NAME...]` or `all`; a name given twice is a usage error, and the first name
    // must be the session company.
    #[test]
    fn companies_parse_a_list_or_all_and_check_the_session_company_comes_first() {
        use super::Companies;
        assert_eq!(Companies::parse("all").unwrap(), Companies::All);
        assert_eq!(
            Companies::parse("CRONUS International Ltd., My Company").unwrap(),
            Companies::List(vec!["CRONUS International Ltd.".into(), "My Company".into()])
        );
        assert!(Companies::parse("A,A").is_err());
        assert!(Companies::parse("A,,B").is_err());
        let l = Companies::parse("A,B").unwrap();
        assert!(l.check_session_first("A").is_ok());
        assert!(l.check_session_first("B").is_err());
        assert!(Companies::All.check_session_first("B").is_ok());
        assert_eq!(l.key_text(), "A,B");
        assert_eq!(Companies::One.key_text(), "");
        // Every wrapper of the identity hands the list on.
        let h = super::Host::default()
            .with_companies(l.clone())
            .in_company("A")
            .in_environment(super::Environment::OnPrem)
            .with_server_url("http://x:1/BC");
        assert_eq!(h.identity.companies(), l);
    }

    #[test]
    fn the_server_base_url_has_a_default_and_can_be_set_through_every_wrapper() {
        let h = Host::default();
        assert_eq!(h.identity.server_base_url(), "http://localhost:8080/BC");
        let h = h
            .with_server_url("https://bc.example/BC/")
            .in_company("X")
            .in_environment(Environment::OnPrem);
        assert_eq!(h.identity.server_base_url(), "https://bc.example/BC/");
        assert_eq!(h.in_company("Y").identity.server_base_url(), "https://bc.example/BC/");
    }

    #[test]
    fn a_server_url_is_an_absolute_http_url_without_query() {
        assert_eq!(check_server_url(" http://h:1/BC/ "), Ok("http://h:1/BC/".into()));
        assert!(check_server_url("https://h").is_ok());
        for bad in [
            "",
            "h:8080",
            "ftp://h/",
            "http://",
            "/BC/",
            "http://h/?a=1",
            "http://h /x",
        ] {
            assert!(check_server_url(bad).is_err(), "{bad}");
        }
    }

    use super::*;

    #[test]
    fn default_clock_is_fixed_and_at_midday() {
        let c = FixedClock::default();
        assert_eq!(c.now_utc_millis(), c.now_utc_millis());
        assert_eq!(c.now_utc_millis().rem_euclid(86_400_000), 43_200_000);
    }

    #[test]
    fn default_identity_mirrors_al_runner() {
        let i = AlRunnerIdentity;
        assert_eq!(i.user_id(), "TESTUSER");
        assert_eq!(i.company_name(), "CRONUS International Ltd.");
        // {C0A1BDFA-0000-0000-0000-545553545553}
        let text: String = USER_SECURITY_ID.iter().map(|b| format!("{b:02X}")).collect();
        assert_eq!(text, "C0A1BDFA000000000000545553545553");
        assert!(i.session_id() > 0 && i.service_instance_id() > 0);
        assert!(!i.tenant_id().is_empty());
        assert!(i.gui_allowed() && i.is_service_tier());
        assert_eq!(i.windows_language(), 1033);
        assert_eq!((i.locale_id(), i.time_zone().as_str()), (1033, "UTC"));
    }

    /// Owner decision 2026-10-15 (`--environment`): the default environment is the one probed (a
    /// SaaS-like sandbox); on-prem is chosen on purpose and survives wrapping in a company.
    #[test]
    fn the_environment_is_saas_unless_chosen() {
        assert_eq!(Host::default().identity.environment(), Environment::Saas);
        let h = Host::default().in_environment(Environment::OnPrem);
        assert_eq!(h.identity.environment(), Environment::OnPrem);
        assert_eq!(h.in_company("X").identity.environment(), Environment::OnPrem);
        assert_eq!("onprem".parse::<Environment>(), Ok(Environment::OnPrem));
        assert_eq!("SaaS".parse::<Environment>(), Ok(Environment::Saas));
        assert!("cloud".parse::<Environment>().is_err());
    }

    #[test]
    fn the_memory_file_system_keeps_bytes_by_exact_path() {
        let mut fs = MemFileSystem::default();
        assert!(!fs.exists("a"));
        fs.write("a", b"x").unwrap();
        fs.write("a", b"yz").unwrap();
        assert_eq!(fs.read("a").unwrap(), b"yz");
        assert!(!fs.exists("A"));
        assert!(fs.remove("a") && !fs.remove("a"));
        assert!(fs.read("a").is_none());
    }

    #[test]
    fn the_memory_file_system_keeps_folders_and_removes_them_with_their_files() {
        let mut fs = MemFileSystem::default();
        assert!(fs.in_memory());
        assert!(fs.dir_exists("\\TEMP\\") && fs.dir_exists("temp/") && !fs.dir_exists(""));
        assert!(!fs.dir_exists("\\TEMP\\a"));
        fs.create_dir("\\TEMP\\a/b").unwrap();
        assert!(fs.dir_exists("\\TEMP\\a") && fs.dir_exists("\\TEMP\\a\\b\\"));
        fs.write("\\TEMP\\a/b/f.txt", b"x").unwrap();
        assert!(!fs.dir_exists("\\TEMP\\a/b/f.txt"));
        assert!(fs.create_dir("\\TEMP\\a\\b\\f.txt").is_err());
        assert!(fs.remove_dir("\\TEMP\\a", false).is_err());
        assert_eq!(fs.near_match("\\temp\\A"), Some("\\TEMP\\a".to_string()));
        assert_eq!(
            fs.near_match("\\TEMP\\A\\B\\F.TXT"),
            Some("\\TEMP\\a/b/f.txt".to_string())
        );
        assert_eq!(fs.near_match("\\TEMP\\a"), None);
        fs.remove_dir("\\TEMP\\a", true).unwrap();
        assert!(!fs.dir_exists("\\TEMP\\a") && !fs.dir_exists("\\TEMP\\a\\b") && !fs.exists("\\TEMP\\a/b/f.txt"));
        assert!(fs.remove_dir("\\TEMP", true).is_err());
    }

    #[test]
    fn host_is_deterministic_by_default() {
        let h = Host::default();
        assert_eq!(h.clock.now_utc_millis(), DEFAULT_INSTANT_MILLIS);
        assert_eq!(h.identity.current_client_type(), ClientType::Web);
    }
}
