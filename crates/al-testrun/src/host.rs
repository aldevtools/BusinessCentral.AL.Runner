//! The host the command line gives a session (ADR-006): the real wall clock, and AL Runner's
//! identity. The core crates read both through `al-host`; only this crate touches the OS.

use al_host::{AlRunnerIdentity, Clock, FileSystem, Host};
use std::path::{Component, PathBuf};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// The system clock, read once: 12:00:00 UTC of the day the run first asked for the time ([`run_clock_start`]).
/// Every later answer is that instant (the session adds 1 ms per read and the time `Sleep` has waited, ADR-045, and
/// keeps system stamps strictly increasing, see `al-host`). A clock that ran with the wall would make an outcome depend
/// on the load of the machine: `Item.OnModify` sets "Last Time Modified" to `Time`, and
/// `Format(xRec) <> Format(Rec)` is then true only when a second passed between the insert and the
/// modify, which decides whether Base Application's workflow subscriber runs. A clock that started at the wall time
/// of the first read would make it depend on when the run began (ADR-045 amendment, 2026-10-19). The wall clock is
/// back with `ALRUNNER_REAL_CLOCK=1`.
pub struct SystemClock;

/// The instant the run began: set by the first read of [`SystemClock`], or by [`SystemClock::begin_at`] before it.
static RUN_START: OnceLock<i64> = OnceLock::new();

/// Milliseconds in a day.
const MS_PER_DAY: i64 = 86_400_000;

/// Where the run clock starts when the wall clock reads `wall` (Unix milliseconds): 12:00:00.000 UTC of that UTC day.
/// The time of day is the same in every run, so an outcome that depends on `Time` (a generator seeded with
/// `Randomize(Time - 000000T)`, as `Any.SetDefaultSeed` and `LibraryRandom.Init` of Microsoft's test libraries do, or
/// a second boundary that `Format(Time)` crosses between two reads) does not depend on when the run began, and so not
/// on which other test codeunits a run selected (ADR-043 condition 1). The date stays the wall clock's, so `Today` is
/// today. Noon is the deterministic host's time of day too (`al-host`, 2024-01-15T12:00:00Z).
pub fn run_clock_start(wall: i64) -> i64 {
    wall.div_euclid(MS_PER_DAY) * MS_PER_DAY + MS_PER_DAY / 2
}

impl SystemClock {
    /// The wall clock, read now. It does not start the run clock.
    pub fn wall() -> i64 {
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(d) => d.as_millis() as i64,
            // A clock set before 1970: the same distance on the other side.
            Err(e) => -(e.duration().as_millis() as i64),
        }
    }
}

impl Clock for SystemClock {
    fn now_utc_millis(&self) -> i64 {
        if std::env::var_os("ALRUNNER_REAL_CLOCK").is_some() {
            return SystemClock::wall();
        }
        *RUN_START.get_or_init(|| run_clock_start(SystemClock::wall()))
    }
}

impl SystemClock {
    /// Starts the run clock at `ms` (Unix milliseconds), as if the run had begun then. A run that reads an install
    /// baseline an earlier run kept does this with the instant that run began: the rows of the baseline carry that
    /// instant, and a run that began later must not see them stamped with a time it never had. False, and nothing
    /// changes, when the clock has started already (something read it) or the real clock is on.
    pub fn begin_at(ms: i64) -> bool {
        !real_clock() && RUN_START.set(ms).is_ok()
    }

    /// The instant the run began, which starts the run clock if nothing did.
    pub fn run_start() -> i64 {
        SystemClock.now_utc_millis()
    }

    /// Whether the run clock has started.
    pub fn started() -> bool {
        RUN_START.get().is_some()
    }
}

/// A clock that stands still at one instant.
pub struct FixedClock(pub i64);

impl Clock for FixedClock {
    fn now_utc_millis(&self) -> i64 {
        self.0
    }
}

/// The directory `--files-dir` named, if any. Set once at start-up.
static FILES_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Makes the `File` data type use a real directory instead of memory (`--files-dir DIR`).
pub fn set_files_dir(dir: PathBuf) {
    let _ = FILES_DIR.set(dir);
}

/// The environment `--environment` named. Set once at start-up; the default is the probed SaaS-like sandbox.
static ENVIRONMENT: OnceLock<al_host::Environment> = OnceLock::new();

/// `--environment saas|onprem` (owner decision 2026-10-15, ADR-014 amendment).
pub fn set_environment(environment: al_host::Environment) {
    let _ = ENVIRONMENT.set(environment);
}

/// The server base address of the run (`--server-url`, ADR-042). A server sets it per request, so it is not set-once.
static SERVER_URL: std::sync::RwLock<Option<String>> = std::sync::RwLock::new(None);

/// Sets the server base address; `None` is the default of `al_host::DEFAULT_SERVER_URL`.
pub fn set_server_url(url: Option<String>) {
    if let Ok(mut w) = SERVER_URL.write() {
        *w = url;
    }
}

/// The server base address the system host answers `GetUrl` with, and whether it is the default.
pub fn server_url() -> (String, bool) {
    match SERVER_URL.read().ok().and_then(|r| r.clone()) {
        Some(u) => (u, false),
        None => (al_host::DEFAULT_SERVER_URL.to_string(), true),
    }
}

/// The environment of the system host, as text (a key of the install baseline on disk holds it).
pub fn environment_text() -> &'static str {
    match ENVIRONMENT.get().copied().unwrap_or_default() {
        al_host::Environment::Saas => "saas",
        al_host::Environment::OnPrem => "onprem",
    }
}

/// Whether `--files-dir` gave the `File` type a real directory: what a run reads from there is not part of any key.
pub fn uses_real_files() -> bool {
    FILES_DIR.get().is_some()
}

/// Whether the system host reads the wall clock as it goes (`ALRUNNER_REAL_CLOCK`) instead of standing at the instant
/// the run began.
pub fn real_clock() -> bool {
    std::env::var_os("ALRUNNER_REAL_CLOCK").is_some()
}

/// The real clock and AL Runner's identity, on the file system the command line chose: memory,
/// unless `--files-dir` named a directory, on the environment `--environment` named.
pub fn system_host() -> Host {
    host_at(None)
}

/// [`system_host`] whose clock stands still at `start` (Unix milliseconds), or reads the system clock when `None`.
pub fn host_at(start: Option<i64>) -> Host {
    host_in_company(start, company_name().as_deref())
}

/// The company name the run answers `CompanyName()` with when the run has no restored company (a lane of Microsoft's
/// recipe names one, for example `Empty Company`). `None` keeps AL Runner's identity.
static COMPANY_NAME: std::sync::RwLock<Option<String>> = std::sync::RwLock::new(None);

/// Sets the company name of the run; `None` is the default of [`AlRunnerIdentity`].
pub fn set_company_name(name: Option<String>) {
    if let Ok(mut w) = COMPANY_NAME.write() {
        *w = name;
    }
}

/// The company name set by [`set_company_name`].
pub fn company_name() -> Option<String> {
    COMPANY_NAME.read().ok().and_then(|r| r.clone())
}

/// The companies of the run's database (`--companies`, ADR-046 decision 2). The default is one company.
static COMPANIES: std::sync::RwLock<al_host::Companies> = std::sync::RwLock::new(al_host::Companies::One);

/// Sets the companies of the run.
pub fn set_companies(companies: al_host::Companies) {
    if let Ok(mut w) = COMPANIES.write() {
        *w = companies;
    }
}

/// The companies set by [`set_companies`].
pub fn companies() -> al_host::Companies {
    COMPANIES.read().map(|r| r.clone()).unwrap_or_default()
}

/// [`host_at`] with the company name given instead of read from the run setting.
pub fn host_in_company(start: Option<i64>, company: Option<&str>) -> Host {
    let host = match start {
        Some(ms) => Host::new(FixedClock(ms), AlRunnerIdentity),
        None => Host::new(SystemClock, AlRunnerIdentity),
    };
    let host = match company {
        Some(c) => host.in_company(c),
        None => host,
    };
    let host = host.with_companies(companies());
    let host = host.in_environment(ENVIRONMENT.get().copied().unwrap_or_default());
    let host = host.with_server_url(&server_url().0);
    match FILES_DIR.get() {
        Some(dir) => host.with_files(DirFileSystem::new(dir.clone())),
        None => host,
    }
}

/// Files in one directory. AL paths are taken as relative to it, with `/` or `\` as separators;
/// a path that is absolute or climbs out with `..` names no file (the run cannot touch anything
/// outside the directory the user opted into).
pub struct DirFileSystem {
    root: PathBuf,
}

impl DirFileSystem {
    pub fn new(root: PathBuf) -> DirFileSystem {
        DirFileSystem { root }
    }

    fn resolve(&self, path: &str) -> Option<PathBuf> {
        // A leading backslash is the root of the Windows-style `\TEMP\` path AL code gets from TemporaryPath(); it names the files
        // directory's own root. A leading `/` still names no file.
        let rel = PathBuf::from(path.strip_prefix('\\').unwrap_or(path).replace('\\', "/"));
        let mut out = self.root.clone();
        for c in rel.components() {
            match c {
                Component::Normal(p) => out.push(p),
                Component::CurDir => {}
                _ => return None,
            }
        }
        (out != self.root).then_some(out)
    }
}

impl FileSystem for DirFileSystem {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        std::fs::read(self.resolve(path)?).ok()
    }
    fn write(&mut self, path: &str, bytes: &[u8]) -> Result<(), String> {
        let p = self
            .resolve(path)
            .ok_or_else(|| format!("`{path}` is outside the files directory"))?;
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(p, bytes).map_err(|e| e.to_string())
    }
    fn exists(&self, path: &str) -> bool {
        self.resolve(path).is_some_and(|p| p.is_file())
    }
    fn remove(&mut self, path: &str) -> bool {
        self.resolve(path).is_some_and(|p| std::fs::remove_file(p).is_ok())
    }
    /// A folder the user put into the files directory: File.Create and File.Open into a subfolder of TemporaryPath() that
    /// does not exist fail (al-builtins file.rs `missing_folder`).
    fn dir_exists(&self, path: &str) -> bool {
        self.resolve(path).is_some_and(|p| p.is_dir())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_run_clock_starts_at_noon_utc_of_the_wall_clocks_day() {
        // 2026-10-19T12:00:00.000Z, from 2026-10-19T23:20:55.239Z and from midnight of that day.
        let noon = 1_792_411_200_000;
        assert_eq!(run_clock_start(1_792_452_055_239), noon);
        assert_eq!(run_clock_start(noon - MS_PER_DAY / 2), noon);
        assert_eq!(run_clock_start(noon + MS_PER_DAY / 2 - 1), noon);
        // The next day starts its own noon.
        assert_eq!(run_clock_start(noon + MS_PER_DAY / 2), noon + MS_PER_DAY);
        // Before 1970: the day is still the one the instant lies in.
        assert_eq!(run_clock_start(-1), -MS_PER_DAY / 2);
    }

    #[test]
    fn a_named_company_is_what_company_name_answers() {
        assert_eq!(
            host_in_company(None, None).identity.company_name(),
            "CRONUS International Ltd."
        );
        assert_eq!(
            host_in_company(None, Some("Empty Company")).identity.company_name(),
            "Empty Company"
        );
    }

    #[test]
    fn the_directory_file_system_keeps_files_under_its_root_only() {
        let dir = tempfile::tempdir().unwrap();
        let mut fs = DirFileSystem::new(dir.path().to_path_buf());
        fs.write("a/b.txt", b"hi").unwrap();
        assert_eq!(std::fs::read(dir.path().join("a/b.txt")).unwrap(), b"hi");
        assert_eq!(fs.read("a\\b.txt").unwrap(), b"hi");
        assert!(fs.exists("a/b.txt") && !fs.exists("a"));
        assert!(fs.write("../escape.txt", b"x").is_err());
        assert!(fs.write("/abs.txt", b"x").is_err());
        assert!(!fs.exists("../escape.txt"));
        assert!(fs.remove("a/b.txt") && !fs.remove("a/b.txt"));
    }

    #[test]
    fn the_system_clock_is_after_2024_and_does_not_run_backwards() {
        let c = SystemClock;
        let (a, b) = (c.now_utc_millis(), c.now_utc_millis());
        assert!(a > 1_704_067_200_000, "{a}");
        assert!(b >= a);
    }

    /// Red before the fix: the clock followed the wall, so a run on a loaded machine saw other
    /// times than one on a quiet machine.
    #[test]
    fn the_clock_of_a_run_stands_still() {
        let c = SystemClock;
        let a = c.now_utc_millis();
        std::thread::sleep(std::time::Duration::from_millis(30));
        assert_eq!(c.now_utc_millis(), a);
    }
}
