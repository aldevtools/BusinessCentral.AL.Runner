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
    /// Whether the folder exists (for a path [`FileSystem::serves_path`] accepts).
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
}

/// Files kept in memory, by path. Matching is exact and case sensitive (the directory-backed
/// system follows the host OS).
#[derive(Debug, Clone, Default)]
pub struct MemFileSystem {
    files: std::collections::BTreeMap<String, Vec<u8>>,
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
}

/// Everything environmental a session needs.
pub struct Host {
    pub clock: Box<dyn Clock>,
    pub identity: Box<dyn Identity>,
    pub files: Box<dyn FileSystem>,
    /// The seed of the session's `Random` before AL calls `Randomize(seed)`. BC seeds it from the
    /// clock; AL sees only the range of the numbers, so the host fixes it (ADR-009).
    pub random_seed: i32,
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
    fn host_is_deterministic_by_default() {
        let h = Host::default();
        assert_eq!(h.clock.now_utc_millis(), DEFAULT_INSTANT_MILLIS);
        assert_eq!(h.identity.current_client_type(), ClientType::Web);
    }
}
