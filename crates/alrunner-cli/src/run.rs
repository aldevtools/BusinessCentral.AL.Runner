//! `al-runner test`: load the APPs and their dependencies, compile, run, report.

use crate::apps::{self, AppInput};
use crate::args::{DEFAULT_TOP, TestArgs};
use crate::capture;
use crate::coverage;
use crate::output::{self, Counts};
use crate::style;
use crate::testdata;
use al_bcdb_std::BackupProvider;
use al_deps::load::WorkspaceApp;
use al_host::dotnet::DotNetHost;
use al_host::testdata::TestData;
use al_testrun::deps::{self, Provision};
use al_testrun::discover;
use al_testrun::dotnet::{SidecarConfig, SidecarHost};
use al_testrun::refusals;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun, TestResult};
use al_testrun::{CaseId, Outcome, target};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// With `--output-json` and no file, stdout holds the JSON document and nothing else: the report goes to stderr.
static REPORT_TO_STDERR: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// `println!` for the human report.
macro_rules! say {
    ($($t:tt)*) => {
        if REPORT_TO_STDERR.load(std::sync::atomic::Ordering::Relaxed) {
            eprintln!($($t)*)
        } else {
            println!($($t)*)
        }
    };
}

/// A test codeunit found in an APP's source, before anything is compiled.
struct Found {
    app: usize,
    id: u32,
    tests: Vec<String>,
}

/// The newest BC major this runtime is verified against (the fallback when nothing names one).
pub(crate) const NEWEST_VERIFIED: u16 = 29;

/// Which tests a run is about.
#[derive(Default, Clone)]
struct Selection {
    codeunits: Vec<u32>,
    filters: Vec<String>,
    exact: Vec<String>,
    excludes: Vec<String>,
    /// `--parity`: the tests Microsoft turns off are not picked.
    parity: Option<Arc<al_testrun::disabled::DisabledTests>>,
}

/// `Codeunit<ID>.<Method>`, the name AL Runner v2 reports and accepts.
fn qualified(codeunit: u32, test: &str) -> String {
    format!("Codeunit{codeunit}.{test}")
}

impl Selection {
    /// Whether the run is narrowed by anything.
    fn active(&self) -> bool {
        !self.includes_none() || !self.excludes.is_empty() || self.parity.is_some()
    }

    fn includes_none(&self) -> bool {
        self.codeunits.is_empty() && self.filters.is_empty() && self.exact.is_empty()
    }

    /// A pattern with a dot is matched against the qualified name, one without against the method name; any case,
    /// as a substring.
    fn matches(pattern: &str, codeunit: u32, test: &str) -> bool {
        let p = pattern.to_lowercase();
        if p.contains('.') {
            qualified(codeunit, test).to_lowercase().contains(&p)
        } else {
            test.to_lowercase().contains(&p)
        }
    }

    fn picks(&self, codeunit: u32, test: &str) -> bool {
        let by_filter = self.filters.iter().any(|f| Self::matches(f, codeunit, test));
        let by_exact = self
            .exact
            .iter()
            .any(|e| e.eq_ignore_ascii_case(test) || e.eq_ignore_ascii_case(&qualified(codeunit, test)));
        // `--test` with `--test-exact` narrows to the tests both match, as AL Runner v2 did, so that a tool that
        // sends both selects no more than either alone (issue 5612). Each alone selects what it matches.
        let by_name = match (self.filters.is_empty(), self.exact.is_empty()) {
            (false, false) => by_filter && by_exact,
            _ => by_filter || by_exact,
        };
        let included = self.includes_none() || self.codeunits.contains(&codeunit) || by_name;
        included
            && !self.excludes.iter().any(|x| Self::matches(x, codeunit, test))
            && !self.parity.as_ref().is_some_and(|d| d.is_disabled(codeunit, test))
    }
}

/// `--disabled-tests DIR`: every `*.json` below DIR, as Microsoft's list of disabled tests.
fn load_disabled(a: &TestArgs) -> Result<Option<Arc<al_testrun::disabled::DisabledTests>>, String> {
    let Some(dir) = &a.disabled_tests_dir else {
        return Ok(None);
    };
    let mut d = al_testrun::disabled::DisabledTests::default();
    let mut stack = vec![dir.clone()];
    let mut seen_any = false;
    while let Some(p) = stack.pop() {
        let meta = std::fs::metadata(&p).map_err(|e| format!("--disabled-tests {}: {e}", p.display()))?;
        if meta.is_dir() {
            let mut kids: Vec<PathBuf> = std::fs::read_dir(&p)
                .map_err(|e| format!("--disabled-tests {}: {e}", p.display()))?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .collect();
            kids.sort();
            stack.extend(kids.into_iter().rev());
        } else if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("json")) {
            seen_any = true;
            let text = std::fs::read_to_string(&p).map_err(|e| format!("--disabled-tests {}: {e}", p.display()))?;
            d.add_file(&text)
                .map_err(|e| format!("--disabled-tests {}: {e}", p.display()))?;
        }
    }
    if !seen_any {
        return Err(format!("--disabled-tests {}: no *.json file below it", dir.display()));
    }
    Ok(Some(Arc::new(d)))
}

/// The line about the tests Microsoft turns off: how they came out, or how many `--parity` left out.
fn disabled_line(
    d: &al_testrun::disabled::DisabledTests,
    results: &[&TestResult],
    found: &[Found],
    selection: &Selection,
    parity: bool,
) -> String {
    let list = format!("{} entries of {} files", d.entries(), d.files());
    if parity {
        let skipped: usize = found
            .iter()
            .map(|f| {
                f.tests
                    .iter()
                    .filter(|t| selection.picks(f.id, t) && d.is_disabled(f.id, t))
                    .count()
            })
            .sum();
        return format!("Disabled by Microsoft: {skipped} tests not run (--parity; {list})");
    }
    let (mut n, mut pass, mut fail, mut not_run) = (0usize, 0usize, 0usize, 0usize);
    for r in results.iter().filter(|r| d.is_disabled(r.codeunit, &r.test)) {
        n += 1;
        match r.outcome {
            Outcome::Pass => pass += 1,
            Outcome::Fail(_) => fail += 1,
            Outcome::NotRun(_) => not_run += 1,
        }
    }
    format!("Disabled by Microsoft: {n} of the tests run ({pass} passed, {fail} failed, {not_run} not run; {list})")
}

/// How many worker threads a phase uses: the variable's value, else one per core.
fn threads(var: &str) -> String {
    match std::env::var(var).ok().and_then(|v| v.parse::<usize>().ok()) {
        Some(n) => format!("{n} ({var})"),
        None => format!(
            "{} (cores)",
            std::thread::available_parallelism().map_or(1, |n| n.get())
        ),
    }
}

/// The peak resident memory of this process so far, in MB (Linux; `None` elsewhere).
fn peak_memory_mb() -> Option<u64> {
    let s = std::fs::read_to_string("/proc/self/status").ok()?;
    let kb: u64 = s
        .lines()
        .find_map(|l| l.strip_prefix("VmHWM:"))?
        .split_whitespace()
        .next()?
        .parse()
        .ok()?;
    Some(kb / 1024)
}

/// A test's run time for the per-test lines: milliseconds down to microseconds, right-aligned.
fn ms(d: Duration) -> String {
    format!("{:>10.3} ms", d.as_secs_f64() * 1000.0)
}

/// [`secs`] right-aligned, so the numbers of the `Time:` line stack up from run to run.
fn secs_w(d: Duration) -> String {
    format!("{:>6.2} s", d.as_secs_f64())
}

fn secs(d: Duration) -> String {
    format!("{:.2} s", d.as_secs_f64())
}

/// The repository root when this binary was built inside one (for the .NET helper's source).
fn source_root() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    exe.ancestors()
        .find(|p| p.join("tools/dotnet-sidecar").is_dir())
        .map(Path::to_path_buf)
}

/// Writes `text` to `path`, creating the parent folders (OUT-11).
pub fn write_file(path: &Path, text: &str) -> Result<(), String> {
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// With `al-runner serve`: the backup opened last, kept with the tables read from it, for the next request.
type KeptBackup = ((PathBuf, Option<String>), TestData);
static KEPT_BACKUP: std::sync::Mutex<Option<KeptBackup>> = std::sync::Mutex::new(None);
static KEEP_BACKUPS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// `al-runner serve` keeps the opened backup (and so the tables read from it) between requests.
pub fn keep_backups() {
    KEEP_BACKUPS.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Opens a backup file.
fn open_backup(a: &TestArgs, path: PathBuf) -> Result<(TestData, PathBuf), String> {
    if KEEP_BACKUPS.load(std::sync::atomic::Ordering::Relaxed) {
        let key = (path.clone(), a.test_data_company.clone());
        if let Some((k, d)) = KEPT_BACKUP.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            if *k == key {
                return Ok((d.clone(), path));
            }
        }
        let opened = open_backup_fresh(a, path)?;
        *KEPT_BACKUP.lock().unwrap_or_else(|e| e.into_inner()) = Some((key, opened.0.clone()));
        return Ok(opened);
    }
    open_backup_fresh(a, path)
}

fn open_backup_fresh(a: &TestArgs, path: PathBuf) -> Result<(TestData, PathBuf), String> {
    // The page map of the backup is kept next to the compile cache, so a cold or a warm open of a
    // 900 MB backup reads the map instead of verifying every extent.
    let index_dir = (!a.no_compile_cache
        && !std::env::var("ALRUNNER_NO_COMPILE_CACHE").is_ok_and(|v| !v.is_empty() && v != "0"))
    .then(|| al_deps::cache::cache_root_from_env(a.cache_dir.as_deref()).join("backup-index"));
    let provider = BackupProvider::open_with_index_dir(&path, a.test_data_company.as_deref(), index_dir.as_deref())
        .map_err(|e| format!("--test-data {}: {e}", path.display()))?;
    Ok((TestData::new(Arc::new(provider)), path))
}

/// `--company-data package`: the artifact's RapidStart package as the company data (the Legacy lane's company: a fresh
/// company, Company-Initialize, then the package). `bak` is the located backup, whose artifact folder holds the package.
fn open_package(a: &TestArgs, bak: Option<&Path>) -> Result<(TestData, PathBuf), String> {
    let path = match &a.package_path {
        Some(p) => p.clone(),
        None => {
            let dir = bak
                .and_then(|b| b.parent())
                .and_then(|b| b.parent())
                .map(|root| root.join("platform").join("ConfigurationPackages"))
                .ok_or("no artifact folder to find the package in; name it with --package PATH")?;
            let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
                .map_err(|e| format!("{}: {e}; name the package with --package PATH", dir.display()))?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("rapidstart")))
                .collect();
            found.sort();
            // The Extended English package is the DemoTool's Legacy data; any other is a fallback.
            found
                .iter()
                .find(|p| p.to_string_lossy().to_ascii_uppercase().contains("ENU.EXTENDED"))
                .or(found.first())
                .cloned()
                .ok_or_else(|| {
                    format!(
                        "no *.rapidstart file in {}; name one with --package PATH",
                        dir.display()
                    )
                })?
        }
    };
    let bytes = std::fs::read(&path).map_err(|e| format!("--package {}: {e}", path.display()))?;
    let name = path
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let company = a.test_data_company.as_deref().unwrap_or("CRONUS International Ltd.");
    let provider = al_rapidstart::PackageProvider::from_bytes(&bytes, &name, company)
        .map_err(|e| format!("--package {}: {e}", path.display()))?;
    Ok((TestData::new(Arc::new(provider)), path))
}

/// What the test data of a run came to.
struct DataChoice {
    data: Option<(TestData, PathBuf)>,
    /// Lines for stderr: where the data is from when that is not the build of the apps, why it is
    /// off, what failed on the way.
    notes: Vec<String>,
}

/// Finds and opens the demo backup (ADR-020): the build of the Microsoft apps first, on disk, then
/// in the cache, then downloaded; see [`testdata::locate`]. A default run whose backup cannot be
/// found or opened has test data off and says why; a run that asked for test data fails instead.
fn choose_test_data(
    a: &TestArgs,
    builds: &[String],
    downloadable: &[al_deps::build::BuildId],
    major: u16,
    minor: Option<u16>,
) -> Result<DataChoice, String> {
    // A named package needs no backup at all.
    if a.company_package && a.package_path.is_some() {
        return match open_package(a, None) {
            Ok(d) => Ok(DataChoice {
                data: Some(d),
                notes: Vec::new(),
            }),
            Err(e) => Err(format!("--company-data package: {e}")),
        };
    }
    let asked = a.test_data_asked_for() || a.company_package;
    let fail = |why: String| -> Result<DataChoice, String> {
        if asked {
            Err(format!("--test-data: {why}"))
        } else {
            Ok(DataChoice {
                data: None,
                notes: vec![format!(
                    "test data is off: {why}. Microsoft's tests expect a CRONUS company; failures of such a run are mostly missing setup data. Name a backup with --test-data PATH.bak"
                )],
            })
        }
    };
    let (path, mut notes) = match &a.test_data_path {
        Some(p) => (p.clone(), Vec::new()),
        None => {
            let cache_root = al_deps::cache::cache_root_from_env(a.cache_dir.as_deref());
            let local = al_deps::cache::bcartifacts_sandbox_from_env();
            let download = (!a.no_download).then_some(|build: &al_deps::build::BuildId| -> Result<PathBuf, String> {
                let cdn = al_deps::artifact::HttpCdn::from_env();
                let mut printed = 0u64;
                let t = Instant::now();
                eprintln!(
                    "test data: downloading the {} demo data of build {build} (about 0.5 GB, once) ...",
                    a.country()
                );
                let got =
                    al_deps::demo::ensure_demo_backup(&cdn, &cache_root, build, a.country(), &mut |read, total| {
                        let pct = (read * 100).checked_div(total).unwrap_or(100);
                        if pct >= printed + 10 || (pct == 100 && printed < 100) {
                            printed = pct - pct % 10;
                            eprintln!(
                                "test data: {pct}% ({} of {} MB, {:.0} s)",
                                read >> 20,
                                total >> 20,
                                t.elapsed().as_secs_f64()
                            );
                        }
                    })?;
                Ok(got.path)
            });
            let found = testdata::locate(
                builds,
                downloadable,
                major,
                minor,
                testdata::Where {
                    local_root: local.as_deref(),
                    cache_root: &cache_root,
                    download,
                    explicit_country: a.country.is_some(),
                },
                a.country(),
            );
            match found.path {
                Some(p) => (p, found.notes),
                None => return fail(found.off.unwrap_or_default()),
            }
        }
    };
    if a.company_package {
        return match open_package(a, Some(&path)) {
            Ok(d) => Ok(DataChoice {
                data: Some(d),
                notes: std::mem::take(&mut notes),
            }),
            Err(e) => Err(format!("--company-data package: {e}")),
        };
    }
    match open_backup(a, path) {
        Ok(d) => Ok(DataChoice {
            data: Some(d),
            notes: std::mem::take(&mut notes),
        }),
        Err(e) if asked => Err(e),
        Err(e) => fail(e),
    }
}

/// Whether the run looks for a demo backup (and so may download one): never with `--no-test-data`, whatever the
/// country (owner 2026-10-17: partners who test across countries write tests that need no data); not when a backup
/// is open already; a default run with no Microsoft app has no use for the CRONUS company.
fn should_choose_test_data(a: &TestArgs, already_open: bool, microsoft: bool) -> bool {
    a.wants_test_data() && !already_open && (microsoft || a.test_data_asked_for())
}

/// `al-runner warm`: finds the backup, opens it, and so builds the index kept next to the compile cache.
pub fn warm(a: &TestArgs) -> Result<(), String> {
    if a.country.is_some() && !a.no_download {
        al_deps::artifact::check_country(&al_deps::artifact::HttpCdn::from_env(), a.country())?;
    }
    let version = a.bc_version.as_deref();
    let numbers: Vec<u16> = version
        .map(|v| v.split('.').map_while(|p| p.parse().ok()).collect())
        .unwrap_or_default();
    let (major, minor) = (numbers.first().copied().unwrap_or(0), numbers.get(1).copied());
    let builds: Vec<String> = version.map(str::to_string).into_iter().collect();
    let downloadable: Vec<al_deps::build::BuildId> = version
        .and_then(|v| al_deps::build::BuildId::parse(v).ok())
        .into_iter()
        .collect();
    let choice = choose_test_data(a, &builds, &downloadable, major, minor)?;
    for n in &choice.notes {
        eprintln!("{n}");
    }
    let (data, path) = choice.data.ok_or("no backup could be opened")?;
    println!(
        "{}: {}",
        path.display(),
        data.provider.cost().unwrap_or_else(|| "opened".to_string())
    );
    Ok(())
}

/// The environment variables that set the test time limit when `--test-timeout-secs` is not given, in order.
const TEST_TIMEOUT_VARS: [&str; 2] = ["ALRUNNER_TEST_TIMEOUT", "AL_RUNNER_TEST_TIMEOUT_SEC"];

/// The test time limit of a run that names none: 20 s (owner decision 2026-10-22: the implicit default stays).
pub const DEFAULT_TEST_TIMEOUT_SECS: u64 = 20;

/// The test time limit a run of Microsoft's recipe sets for itself (`--recipe microsoft`, `test-microsoft`,
/// `--affected-from microsoft`) when neither `--test-timeout-secs` nor an environment variable names one: Microsoft's
/// test codeunits are larger than a partner's (owner decision 2026-10-22).
pub const RECIPE_TEST_TIMEOUT_SECS: u64 = 60;

/// Whether an environment variable sets the test time limit.
pub fn test_timeout_env_set() -> bool {
    TEST_TIMEOUT_VARS
        .iter()
        .any(|var| std::env::var(var).is_ok_and(|v| !v.trim().is_empty()))
}

/// The test time limit in seconds: `--test-timeout-secs` (a recipe lane sets it, see
/// [`RECIPE_TEST_TIMEOUT_SECS`]), else `ALRUNNER_TEST_TIMEOUT`, else AL Runner v2's `AL_RUNNER_TEST_TIMEOUT_SEC`,
/// else [`DEFAULT_TEST_TIMEOUT_SECS`].
fn test_timeout_secs(a: &TestArgs) -> Result<u64, String> {
    if let Some(n) = a.test_timeout_secs {
        return Ok(n);
    }
    for var in TEST_TIMEOUT_VARS {
        if let Some(v) = std::env::var(var).ok().filter(|v| !v.trim().is_empty()) {
            return v
                .trim()
                .parse()
                .map_err(|_| format!("{var} needs a number of seconds, got `{v}`"));
        }
    }
    Ok(DEFAULT_TEST_TIMEOUT_SECS)
}

pub fn test(a: &TestArgs) -> Result<u8, String> {
    if !a.impact_of.is_empty() {
        return crate::impact::run(a);
    }
    test_with(a, None)
}

/// What one side of an impact run (ADR-043), or the run that builds its index, asks for and keeps of its run.
#[derive(Default)]
pub struct Collected {
    /// Asked for: note what each test raised and touched (`al-runner impact-index build`).
    pub impact_trace: bool,
    /// Asked for: narrow the test codeunits once the compile is done (dynamic selection).
    pub narrow: Option<al_testrun::runtime::Narrow>,
    /// The selected tests, in the order of the report.
    pub results: Vec<TestResult>,
    /// With coverage: every statement of the workspace's apps.
    pub statements: Vec<al_testrun::coverage::CoverageStatement>,
    /// With `impact_trace`: the keys `TestResult::impact` numbers, and what the install baseline touched.
    pub impact_keys: Vec<String>,
    pub impact_baseline: Option<Vec<u32>>,
    /// The apps of the compiled program: (id, name, version).
    pub program_apps: Vec<(String, String, String)>,
    /// For the JUnit of an impact run: the Microsoft build, the assumptions of the install baseline, and per result
    /// whether Microsoft turns the test off.
    pub bc_build: Option<String>,
    pub baseline_assumptions: Vec<String>,
    pub disabled: Vec<bool>,
}

/// A run whose selected tests and coverage statements are handed back in `into` (one side of an impact run).
/// Prints the summary line of the run but no verdict.
pub fn test_collect(a: &TestArgs, into: &mut Collected) -> Result<u8, String> {
    test_inner(a, None, Some(into), None)
}

/// What the server hears for each test as it finishes: the event, and its capture as JSON.
pub type OnTest = Arc<dyn Fn(al_testrun::events::TestEvent, Option<serde_json::Value>) + Send + Sync>;

/// What `al-runner serve` keeps between requests, and how it hears about a request's run (ADR-028).
#[derive(Default)]
pub struct WarmCtx {
    pub runtime: al_testrun::runtime::Warm,
    /// Called for each selected test as it finishes.
    /// With a capture asked for, the second argument is its JSON for that test (`capture::json`).
    pub on_test: Option<OnTest>,
    /// Called just before each selected test starts to run.
    pub on_start: Option<al_testrun::events::OnStart>,
    /// The entries of the request (`runTests` with `entries`, issue 5635): each runs its own tests with its own
    /// resource files, over one compile and one install baseline. Empty: one run, as the arguments say.
    pub entries: Vec<Entry>,
    /// Set from outside: the run stops after the current test.
    pub cancel: Option<Arc<std::sync::atomic::AtomicBool>>,
    /// What the last run came to.
    pub report: Option<Report>,
    /// Whether the last run used the compile of an earlier request.
    pub used: Option<al_testrun::runtime::WarmUse>,
}

/// One entry of a server request (`WarmCtx::entries`).
#[derive(Debug, Clone, Default)]
pub struct Entry {
    /// As `--test` and `--exclude`, for this entry.
    pub filters: Vec<String>,
    pub excludes: Vec<String>,
    /// Resource name (relative to the resource folder) -> the text the entry's run reads in place of the file's.
    pub resources: std::collections::BTreeMap<String, String>,
}

/// The result of a run for a caller that is not the console.
pub struct Report {
    /// The `--output-json` document.
    pub doc: serde_json::Value,
    /// With coverage: the `--coverage-json` document.
    pub coverage: Option<serde_json::Value>,
    /// Per test of `doc["tests"]`, in its order: the text of the `Message` calls of a scratch run.
    pub messages: Vec<Vec<String>>,
    /// Per test of `doc["tests"]`: the name of its codeunit.
    pub object_names: Vec<String>,
    /// Per test of `doc["tests"]`: its capture as JSON (`capture::json`), when one was asked for.
    pub captured: Vec<Option<serde_json::Value>>,
    /// Why the run came to its exit code when no test could tell (a selection that matches no test).
    pub warnings: Vec<String>,
}

impl Report {
    /// The report of a run that ran no test: exit code `code`, all counts 0, and why.
    fn empty(code: u8, wall_s: f64, why: &str) -> Report {
        Report {
            doc: serde_json::from_str(&output::json(&[], code, wall_s)).unwrap_or_default(),
            coverage: None,
            messages: Vec::new(),
            object_names: Vec::new(),
            captured: Vec::new(),
            warnings: vec![why.to_string()],
        }
    }
}

/// Everything the compile depends on but the selection and the text of the workspace files: the workspace file names,
/// the manifests (resources, layout files, `app.json`), the dependency apps, the symbols, the target.
fn warm_key(
    inputs: &[AppInput],
    dependencies: &al_testrun::deps::Dependencies,
    symbols: &BTreeSet<String>,
    target: &str,
) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for i in inputs {
        i.source.id.hash(&mut h);
        i.source.dependencies.hash(&mut h);
        // The names of the files, not their text: a change of text is the warm compile's to take in.
        for (path, _) in &i.source.files {
            path.hash(&mut h);
        }
        // What the app's manifest gives the program beside its code: a change to a layout file or `app.json` is a new
        // key, as the warm compile takes in only source text (issue 5604). The resource files are not: they are
        // run-time data, which each request hands to the program as they are then (issue 5635).
        let m = &i.manifest;
        (
            &m.publisher,
            &m.version,
            m.include_source,
            &m.target,
            &m.symbols,
            &m.layout_files,
        )
            .hash(&mut h);
        i.symbols.hash(&mut h);
        let j = &i.json;
        (&j.id, &j.name, &j.publisher, j.version.to_string()).hash(&mut h);
        (
            j.platform.as_ref().map(ToString::to_string),
            j.application.as_ref().map(ToString::to_string),
        )
            .hash(&mut h);
    }
    for (app, lazy) in dependencies.apps.iter().zip(&dependencies.lazy) {
        app.id.hash(&mut h);
        app.files.len().hash(&mut h);
        match lazy {
            Some(l) => l.identity.hash(&mut h),
            None => {
                for (path, text) in &app.files {
                    path.hash(&mut h);
                    text.len().hash(&mut h);
                }
            }
        }
    }
    dependencies.summary.hash(&mut h);
    symbols.hash(&mut h);
    target.hash(&mut h);
    format!("{:016x}", h.finish())
}

pub fn test_with(a: &TestArgs, warm: Option<&mut WarmCtx>) -> Result<u8, String> {
    test_inner(a, warm, None, None)
}

/// What `al-runner debug` adds to a run (ADR-048).
pub struct DebugSetup {
    /// Called once the compile is done; returns when the tests may start.
    pub ready: al_testrun::runtime::DebugReadyHook,
    /// The debugger of each test codeunit's session.
    pub debugger: al_testrun::events::MakeDebugger,
    /// The time the debugger kept the tests stopped (the time limit does not count it).
    pub paused: Arc<al_testrun::events::PauseClock>,
    /// Called for each selected test as it finishes.
    pub on_test: Arc<dyn Fn(al_testrun::events::TestEvent) + Send + Sync>,
    /// Filled with the sources of the run's apps before the compile (where breakpoints bind, ADR-048 decision 6).
    pub sources: Arc<std::sync::Mutex<Option<crate::debug::breakpoints::Sources>>>,
}

/// `al-runner debug`: the run of `al-runner test` with a debugger. The report goes to stderr.
pub fn test_debug(a: &TestArgs, debug: DebugSetup) -> Result<u8, String> {
    test_inner(a, None, None, Some(debug))
}

fn test_inner(
    a: &TestArgs,
    mut warm: Option<&mut WarmCtx>,
    collect: Option<&mut Collected>,
    debug: Option<DebugSetup>,
) -> Result<u8, String> {
    let started = Instant::now();
    REPORT_TO_STDERR.store(
        matches!(a.output_json, Some(None)) || warm.is_some() || debug.is_some(),
        std::sync::atomic::Ordering::Relaxed,
    );
    {
        use std::io::IsTerminal;
        let to_stderr = REPORT_TO_STDERR.load(std::sync::atomic::Ordering::Relaxed);
        let terminal = if to_stderr {
            std::io::stderr().is_terminal()
        } else {
            std::io::stdout().is_terminal()
        };
        let no_color = std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty());
        // A server request is never colored.
        style::set(warm.is_none() && debug.is_none() && a.color.enabled(terminal, no_color));
    }
    let test_timeout = test_timeout_secs(a)?;
    // The country of the run (ADR-020 amendment): only `--country` selects it. An unknown one stops here, before
    // anything is loaded.
    let country = a.country().to_string();
    if a.country.is_some() && !a.no_download {
        al_deps::artifact::check_country(&al_deps::artifact::HttpCdn::from_env(), &country)?;
    }
    // The key of the compile cache starts with a hash of this executable (10 ms): make it beside the loading.
    if !a.no_compile_cache {
        std::thread::spawn(|| {
            al_testrun::compile_cache::runtime_build();
        });
    }
    if let Some(env) = a.environment {
        al_testrun::host::set_environment(env);
    }
    // ADR-042: --server-url, else $ALRUNNER_SERVER_URL, else the default. A request of a server sets it for that run.
    let server_url = match &a.server_url {
        Some(u) => Some(u.clone()),
        None => match std::env::var("ALRUNNER_SERVER_URL") {
            Ok(v) if !v.trim().is_empty() => {
                Some(al_host::check_server_url(&v).map_err(|e| format!("ALRUNNER_SERVER_URL: {e}"))?)
            }
            _ => None,
        },
    };
    let server_url_from = if a.server_url.is_some() {
        "--server-url"
    } else {
        "ALRUNNER_SERVER_URL"
    };
    let server_url_set = server_url.is_some();
    al_testrun::host::set_server_url(server_url);
    al_testrun::host::set_company_name(a.company_name.clone());
    if let Some(dir) = &a.files_dir {
        al_testrun::host::set_files_dir(dir.clone());
    }
    // Fail on an unwritable output path before the long part, not after it.
    if let Some(p) = &a.junit {
        write_file(p, "")?;
    }
    if let Some(Some(p)) = &a.output_json {
        write_file(p, "")?;
    }
    for p in [&a.coverage_out, &a.coverage_json].into_iter().flatten() {
        write_file(p, "")?;
    }
    // A backup named on the command line is opened before the long part: a wrong path or company
    // fails here. The cached artifact is chosen once the Microsoft apps are known.
    let mut test_data = match (a.wants_test_data(), &a.test_data_path) {
        (true, Some(p)) => Some(open_backup(a, p.clone())?),
        _ => None,
    };
    let mut data_notes: Vec<String> = Vec::new();
    let inputs = a
        .apps
        .iter()
        .map(|p| apps::load(p, a.bc_version.as_deref()))
        .collect::<Result<Vec<AppInput>, String>>()?;

    // Which test codeunits there are, and which of them this run is about.
    let mut found: Vec<Found> = Vec::new();
    for (i, app) in inputs.iter().enumerate() {
        // An impact run's "with" side: the extensions' own test codeunits are not part of the run.
        if a.test_apps.is_some_and(|n| i >= n) {
            continue;
        }
        for (_, text) in &app.source.files {
            for (id, tests) in discover::test_codeunits(text) {
                found.push(Found { app: i, id, tests });
            }
        }
    }
    let disabled = load_disabled(a)?;
    let selection = Selection {
        codeunits: a.codeunits.clone(),
        exact: a.exact.clone(),
        excludes: a.excludes.clone(),
        parity: a.parity.then(|| disabled.clone()).flatten(),
        // `--explain NAME --codeunit ID` explains the test of that one codeunit; the name alone
        // selects every codeunit that has a test of that name.
        filters: a
            .filters
            .iter()
            .chain(a.explain.iter().filter(|_| a.codeunits.is_empty()))
            .cloned()
            .collect(),
    };
    // A request with entries (issue 5635): each entry selects its own tests; the run compiles for all of them.
    let entries: Vec<Entry> = warm.as_deref().map(|w| w.entries.clone()).unwrap_or_default();
    for name in entries.iter().flat_map(|e| e.resources.keys()) {
        if !inputs
            .iter()
            .any(|i| i.manifest.resources.iter().any(|(n, _)| n == name))
        {
            return Err(format!(
                "`entries`: no app under test has a resource file `{name}` (an entry replaces the text of one; it adds none)"
            ));
        }
    }
    let entry_selections: Vec<Selection> = entries
        .iter()
        .map(|e| Selection {
            filters: e.filters.clone(),
            excludes: e.excludes.clone(),
            ..selection.clone()
        })
        .collect();
    let picks = |id: u32, t: &str| {
        if entry_selections.is_empty() {
            selection.picks(id, t)
        } else {
            entry_selections.iter().any(|s| s.picks(id, t))
        }
    };
    let chosen_units: Vec<&Found> = found
        .iter()
        .filter(|f| f.tests.iter().any(|t| picks(f.id, t)))
        .collect();
    if chosen_units.is_empty() && !a.execute {
        let what = if selection.active() || !entries.is_empty() {
            "the selection matches no test"
        } else {
            "no test codeunit (Subtype = Test) in the given apps"
        };
        eprintln!("error: {what}");
        let empty = Report::empty(6, started.elapsed().as_secs_f64(), what);
        // `--output-json` writes the document of a run with no test, with the reason in `warnings`, as `serve`
        // answers its summary (issue 5613).
        if let Some(target) = &a.output_json
            && warm.is_none()
        {
            let mut doc = empty.doc.clone();
            doc["warnings"] = serde_json::json!(empty.warnings);
            let doc = serde_json::to_string_pretty(&doc).expect("a JSON value serializes");
            match target {
                Some(path) => write_file(path, &doc)?,
                None => println!("{doc}"),
            }
        }
        // A server request answers a summary with exit code 6, as `al-runner test` exits 6 (ADR-028 item 1).
        if let Some(w) = warm.as_deref_mut() {
            w.report = Some(empty);
        }
        return Ok(6);
    }

    // The target: --bc-version, else the floor of the first app (ADR-015 3.2).
    let requested = a
        .bc_version
        .as_deref()
        .map(al_version::BcVersion::parse)
        .transpose()
        .map_err(|e| format!("--bc-version: {e}"))?;
    let mut chosen = al_version::choose_target(
        requested.as_ref(),
        &inputs[0].versions,
        &al_version::BcVersion::new(NEWEST_VERIFIED, 0),
    )
    .map_err(|e| e.to_string())?;
    // Each app under test sees the builtins of the runtime its own app.json declares (al-runner issue 5573): the AL
    // compiler rejects a call newer than that runtime with AL0666.
    chosen.target.app_runtimes = inputs
        .iter()
        .filter_map(|i| Some((i.source.id.clone(), i.versions.runtime?)))
        .collect();
    for note in &chosen.notes {
        eprintln!("note: {note}");
    }

    // Dependencies: --dep, --package-cache, each folder APP's .alpackages, then the feed.
    let project = apps::combined_project(&inputs);
    let workspace: Vec<WorkspaceApp> = inputs
        .iter()
        .map(|i| WorkspaceApp {
            json: i.json.clone(),
            origin: format!("source {}", i.path.display()),
        })
        .collect();
    let mut search: Vec<PathBuf> = a.deps.clone();
    search.extend(a.package_caches.iter().cloned());
    search.extend(
        inputs
            .iter()
            .filter(|i| i.path.is_dir())
            .map(|i| i.path.join(".alpackages")),
    );
    let floor_versions = &inputs[0].versions;
    // Which Microsoft build the project targets: see `apps::choose_build` (ADR-013, "Amendment 2026-10-15").
    let own = apps::own_build(&inputs);
    let floor = floor_versions.floor().map(|f| (u32::from(f.major), u32::from(f.minor)));
    let packages = if a.bc_version.is_none() && own.is_none() {
        al_deps::load::microsoft_packages_build(&search, None)
    } else {
        None
    };
    let choice = apps::choose_build(
        a.bc_version.as_deref(),
        own,
        packages,
        floor,
        u32::from(chosen.target.bc.major),
    );
    if let Some(note) = &choice.note {
        eprintln!("note: {note}");
    }
    if let Some(minor) = choice.minor {
        chosen.target.bc.minor = minor as u16;
    }
    let from_packages = choice.from_packages;
    let mut build_rule = choice.rule;
    let preferred = choice.preferred;
    let preferred_build = preferred.clone();
    let provision = Provision {
        cache_dir: a.cache_dir.clone(),
        download: !a.no_download,
        query: target::build_query(a.bc_version.as_deref(), &chosen, floor_versions).with_preferred(preferred),
        local_artifacts: al_deps::cache::bcartifacts_sandbox_from_env(),
        country: country.clone(),
    };
    // The text of a package's files is left unread: a run whose tests reach nothing in it never needs it.
    // ALRUNNER_EAGER_SOURCES=1 reads it at once, as before (the other side of the differential check).
    let lazy_sources = std::env::var_os("ALRUNNER_EAGER_SOURCES").is_none();
    let dependencies = deps::load_project_with(
        &project,
        &workspace,
        &search,
        Some(&provision),
        a.bc_version.as_deref(),
        true,
        lazy_sources,
    )?;
    if let Some(d) = &debug {
        let written = al_deps::cache::cache_root_from_env(a.cache_dir.as_deref()).join("debug-sources");
        *d.sources.lock().unwrap_or_else(|e| e.into_inner()) =
            Some(crate::debug::sources_of(&inputs, &dependencies, written));
    }
    let microsoft = inputs.iter().any(|i| apps::is_microsoft(&i.manifest.publisher))
        || dependencies.manifests.iter().any(|m| apps::is_microsoft(&m.publisher));
    if should_choose_test_data(a, test_data.is_some(), microsoft) {
        // The build of the Microsoft apps is the build of the data: what the feed gave, what
        // --bc-version named, then the versions of the loaded packages and of the APPs
        // (Microsoft's test archives carry the build as their version).
        let mut builds: Vec<String> = Vec::new();
        builds.extend(dependencies.build.as_ref().map(|u| u.build.to_string()));
        builds.extend(a.bc_version.clone());
        builds.extend(apps::own_build(&inputs).map(|b| b.to_string()));
        builds.extend(dependencies.manifests.iter().map(|m| m.version.clone()));
        builds.extend(inputs.iter().map(|i| i.manifest.version.clone()));
        let mut downloadable: Vec<al_deps::build::BuildId> = Vec::new();
        let named = dependencies
            .build
            .as_ref()
            .map(|u| u.build.clone())
            .into_iter()
            .chain(
                a.bc_version
                    .as_deref()
                    .and_then(|v| al_deps::build::BuildId::parse(v).ok()),
            )
            .chain(apps::own_build(&inputs));
        for b in named {
            if !downloadable.contains(&b) {
                downloadable.push(b);
            }
        }
        {
            let chosen_data = choose_test_data(
                a,
                &builds,
                &downloadable,
                chosen.target.bc.major,
                Some(chosen.target.bc.minor),
            )?;
            test_data = chosen_data.data;
            data_notes = chosen_data.notes;
        }
    }
    let mut bc_build = None;
    if let Some(u) = &dependencies.build {
        chosen.target.bc.build = Some(u.build.to_string());
        bc_build = Some(u.build.to_string());
        if from_packages && Some(&u.build) != preferred_build.as_ref() {
            // The build the packages come from is neither cached nor published: the usual rules chose
            // another one of the same major, and the target follows what the apps really are.
            build_rule.push_str(&format!(
                "; that build is not cached, on disk or published, so BC {} was used",
                u.build
            ));
            if u.build.major() == u32::from(chosen.target.bc.major) {
                chosen.target.bc.minor = u.build.minor() as u16;
            }
        }
    }
    // The resolver repeats a note on each round of the artifact feed; say each once.
    let mut seen = BTreeSet::new();
    let mut hidden = 0;
    for line in &dependencies.log {
        let is_problem = output::dependency_line_is_problem(line);
        if (a.verbose || is_problem) && seen.insert(line.as_str()) {
            if a.verbose || seen.len() <= 12 {
                // A missing-dependency line repeats the whole search list; the start says it all.
                eprintln!(
                    "{}",
                    if a.verbose {
                        line.clone()
                    } else {
                        al_testrun::refusals::clip(line, 400)
                    }
                );
            } else {
                hidden += 1;
            }
        }
    }
    if hidden > 0 {
        eprintln!("... {hidden} more dependency notes (--verbose lists them)");
    }
    for note in &data_notes {
        eprintln!("note: {note}");
    }

    if !a.quiet {
        say!("{}", style::bold(&crate::version::line()));
    }
    // Setup details: --verbose only (see HELP, "Output").
    if a.verbose {
        if a.country.is_some() {
            say!("country: {country}");
        }
        say!(
            "target: BC {}, runtime {}{}",
            chosen.target.bc,
            chosen.target.runtime,
            match &bc_build {
                Some(b) => format!(", Microsoft apps from build {b}"),
                None => String::new(),
            }
        );
        for (i, app) in inputs.iter().enumerate() {
            let units = found.iter().filter(|f| f.app == i).count();
            let tests: usize = found.iter().filter(|f| f.app == i).map(|f| f.tests.len()).sum();
            say!(
                "app: {} {} ({} files, {units} test codeunits, {tests} tests) from {}",
                app.json.name,
                app.manifest.version,
                app.source.files.len(),
                app.path.display()
            );
        }
        say!("build chosen by: {build_rule}");
        say!("dependencies: {} apps", dependencies.apps.len());
        say!(
            "threads: check {}, lower {}, test {}",
            threads("AL_CHECK_THREADS"),
            threads("AL_LOWER_THREADS"),
            threads("AL_TEST_THREADS")
        );
        match a.isolation {
            None | Some(al_testrun::isolation::TestIsolation::Codeunit) => {
                say!("isolation: TestIsolation = Codeunit (the default of Microsoft's test runner)")
            }
            Some(i) => say!("isolation: TestIsolation = {} (--isolation)", i.name()),
        }
        {
            let (url, _) = al_testrun::host::server_url();
            let from = if server_url_set { server_url_from } else { "default" };
            say!("server address: {url} ({from})");
        }
        if let Some((data, path)) = &test_data {
            say!(
                "test data: {} (company '{}', rows read when a test first touches a table)",
                path.display(),
                data.provider.company()
            );
        }
        if selection.active() {
            say!("selection: {} test codeunits of {}", chosen_units.len(), found.len());
        }
    }

    // Compile and run.
    let build_apps = || {
        let mut all_apps: Vec<al_sema::workspace::AppSource> = inputs.iter().map(|i| i.source.clone()).collect();
        // An app with an `application` or `platform` floor sees `Application`, what it propagates and
        // `System` without declaring them.
        for (app, input) in all_apps.iter_mut().zip(&inputs) {
            let implied =
                dependencies.implied_dependencies(input.json.platform.is_some(), input.json.application.is_some());
            for id in implied {
                if !id.eq_ignore_ascii_case(&app.id) && !app.dependencies.contains(&id) {
                    app.dependencies.push(id);
                }
            }
        }
        let mut kinds = vec![al_sema::AppKind::Workspace; all_apps.len()];
        let mut manifests: Vec<al_testrun::runtime::Manifest> = inputs.iter().map(|i| i.manifest.clone()).collect();
        kinds.extend(dependencies.apps.iter().map(|_| al_sema::AppKind::Dependency));
        manifests.extend(dependencies.manifests.iter().cloned());
        all_apps.extend(dependencies.apps.iter().cloned());
        (all_apps, kinds, manifests)
    };
    let symbols: BTreeSet<String> = inputs
        .iter()
        .flat_map(|i| i.symbols.iter().cloned())
        .chain(a.defines.iter().cloned())
        .collect();

    let host = Arc::new(SidecarHost::new(SidecarConfig {
        sidecar: a.dotnet_sidecar.clone(),
        source_root: source_root(),
        // ADR-014 section 3, step 2: after the user's folders, the service tier of the platform artifact in use.
        assembly_dirs: al_testrun::dotnet::probe_dirs(
            &a.dotnet_assembly_dirs,
            al_deps::cache::bcartifacts_sandbox_from_env().as_deref(),
            bc_build.as_deref(),
            u32::from(chosen.target.bc.major),
        ),
        allow_roll_forward: a.dotnet_allow_roll_forward,
        timeout: Duration::from_secs(a.dotnet_timeout_secs.unwrap_or(60)),
        ..Default::default()
    }));
    if a.require_dotnet {
        if let Err(why) = host.availability() {
            return Err(format!("--require-dotnet: DotNet {why}"));
        }
    }
    let sidecar_host = host.clone();
    // ADR-038: the closed set of pure framework members is answered in process; ALRUNNER_NO_INPROC=1 sends all to the sidecar.
    let host = al_host::inproc::wrap(host);
    // An impact run names the test codeunits on both sides, so that the extensions' own are not compiled as tests.
    let opts_only = ((selection.active() || !entries.is_empty() || a.test_apps.is_some()) && !a.execute).then(|| {
        let mut v: Vec<(usize, u32)> = chosen_units.iter().map(|f| (f.app, f.id)).collect();
        v.sort_unstable();
        v.dedup();
        v
    });
    // Tests that look free of data are compiled first without the baseline and may need no .NET helper
    // at all: start the helpers only when the run turns out to need them (a session starts one then).
    let looks_pure = warm.is_some() || {
        // A warm server starts no helper ahead: a session starts one when a test needs it.
        let (all_apps, kinds, _) = build_apps();
        opts_only.is_some()
            && std::env::var_os("AL_FORCE_BASELINE").is_none()
            && al_testrun::observe::text_looks_pure(&all_apps, &kinds, &symbols, opts_only.as_deref())
    };
    let compile_cache_on =
        !a.no_compile_cache && !std::env::var("ALRUNNER_NO_COMPILE_CACHE").is_ok_and(|v| !v.is_empty() && v != "0");
    if !dependencies.apps.is_empty() && !looks_pure {
        // The Microsoft apps call .NET (Media, images). Without the compile cache the compile takes
        // seconds: start the helper processes now so the starts overlap it. With the cache the
        // compile is short and many runs never call .NET: the first session that calls starts its
        // own helper and the standbys then (ADR-014 section 6). A run without them starts nothing.
        if compile_cache_on {
            sidecar_host.prewarm_on_first_use(al_testrun::dotnet::STANDBY_SIDECARS);
        } else {
            sidecar_host.prewarm(al_testrun::dotnet::STANDBY_SIDECARS);
        }
    }
    let dotnet = al_vm::dotnet::DotNetConfig {
        host,
        ..Default::default()
    };
    // `--affected` (ADR-050): the stored result of the last run, and the callback that narrows this one.
    let affected = if a.affected && collect.is_none() {
        Some(crate::affected::Affected::prepare(
            a,
            &inputs,
            &dependencies,
            bc_build.as_deref(),
        )?)
    } else {
        None
    };
    let hooks = match warm.as_deref() {
        Some(w) => {
            let pick = selection.clone();
            let forward = w.on_test.clone();
            let cfg = a.capture;
            let paths: Vec<PathBuf> = inputs.iter().map(|i| i.path.clone()).collect();
            let cwd = std::env::current_dir().unwrap_or_default();
            // Loop ids are not repeated within a request, whatever test they come from.
            let next_loop = Arc::new(std::sync::atomic::AtomicU32::new(0));
            al_testrun::events::Hooks {
                on_test: forward.map(|f| {
                    Arc::new(move |e: al_testrun::events::TestEvent| {
                        if pick.picks(e.codeunit, &e.test) {
                            let json = e.captured.as_ref().map(|c| {
                                let base = next_loop
                                    .fetch_add(c.captured.loops.len() as u32, std::sync::atomic::Ordering::SeqCst);
                                capture::json(c, cfg, base, &|app: usize, file: &str| {
                                    coverage::label(&paths[app], file, &cwd)
                                })
                            });
                            f(e, json)
                        }
                    }) as Arc<dyn Fn(al_testrun::events::TestEvent) + Send + Sync>
                }),
                on_start: w.on_start.clone(),
                cancel: w.cancel.clone(),
                debugger: None,
                paused: None,
            }
        }
        None => match &debug {
            // `al-runner debug` (ADR-048): each selected test is told as it finishes, and the sessions get a debugger.
            Some(d) => {
                let pick = selection.clone();
                let on_test = d.on_test.clone();
                al_testrun::events::Hooks {
                    on_test: Some(Arc::new(move |e: al_testrun::events::TestEvent| {
                        if pick.picks(e.codeunit, &e.test) {
                            on_test(e)
                        }
                    })),
                    on_start: None,
                    cancel: None,
                    debugger: Some(d.debugger.clone()),
                    paused: Some(d.paused.clone()),
                }
            }
            None => Default::default(),
        },
    };
    let opts = RunOptions {
        only: opts_only,
        lazy: std::iter::repeat_n(None, inputs.len())
            .chain(dependencies.lazy.iter().cloned())
            .collect(),
        trace: a.explain.is_some() || std::env::var_os("AL_STACK_OF").is_some(),
        test_time_limit: match test_timeout {
            0 => None,
            n => Some(std::time::Duration::from_secs(n)),
        },
        isolation: a.isolation,
        // AL Runner runs every test codeunit as Microsoft's runner 130450 does (TestIsolation = Codeunit unless
        // --isolation says otherwise); RequiredTestIsolation on a codeunit has no effect there, as on BC.
        required_isolation: false,
        coverage: a.coverage,
        execute: a.execute,
        capture: a.capture,
        hooks,
        clock_starts: None,
        quiet_baseline: !a.verbose,
        impact_trace: collect.as_ref().is_some_and(|c| c.impact_trace) || affected.is_some(),
        narrow: collect
            .as_ref()
            .and_then(|c| c.narrow.clone())
            .or_else(|| affected.as_ref().map(crate::affected::Affected::narrow)),
        object_hashes: affected.is_some(),
        tdd: a.tdd,
        debug_ready: debug.as_ref().map(|d| d.ready.clone()),
        // Code the AL compiler rejects is not run (al-runner issue 5602): exit 3 with the errors.
        stop_on_compile_errors: true,
        entries: Vec::new(),
        // `--run-selected-only` (issue 5611): the selected tests of the selected codeunits; the rest do not run.
        run_only: (a.run_selected_only && selection.active() && !a.execute).then(|| {
            Arc::new(
                found
                    .iter()
                    .flat_map(|f| f.tests.iter().map(move |t| (f.id, t)))
                    .filter(|(id, t)| selection.picks(*id, t))
                    .map(|(id, t)| (id, t.to_lowercase()))
                    .collect(),
            )
        }),
        test_data: test_data.as_ref().map(|(d, _)| d.clone()),
        compile_cache: compile_cache_on.then(|| al_testrun::compile_cache::CompileCache {
            dir: al_testrun::compile_cache::CompileCache::under(&al_deps::cache::cache_root_from_env(
                a.cache_dir.as_deref(),
            )),
            require: a.require_compile_cache,
            rebuild: a.rebuild_compile_cache,
            country: country.clone(),
        }),
    };
    let mut opts = opts;
    if let Some(w) = warm.as_deref() {
        let cfg = a.capture;
        let paths: Vec<PathBuf> = inputs.iter().map(|i| i.path.clone()).collect();
        let cwd = std::env::current_dir().unwrap_or_default();
        let next_loop = Arc::new(std::sync::atomic::AtomicU32::new(0));
        opts.entries = entries
            .iter()
            .zip(&entry_selections)
            .enumerate()
            .map(|(k, (e, sel))| {
                let units: Vec<(usize, u32)> = {
                    let mut v: Vec<(usize, u32)> = found
                        .iter()
                        .filter(|f| f.tests.iter().any(|t| sel.picks(f.id, t)))
                        .map(|f| (f.app, f.id))
                        .collect();
                    v.sort_unstable();
                    v.dedup();
                    v
                };
                let run_only = (a.run_selected_only && sel.active()).then(|| {
                    Arc::new(
                        found
                            .iter()
                            .flat_map(|f| f.tests.iter().map(move |t| (f.id, t)))
                            .filter(|(id, t)| sel.picks(*id, t))
                            .map(|(id, t)| (id, t.to_lowercase()))
                            .collect(),
                    )
                });
                // The entry's resource texts, in each app under test that has a resource of that name (the request
                // checked that one has).
                let resources: Vec<(usize, al_testrun::runtime::ResourceFiles)> = inputs
                    .iter()
                    .enumerate()
                    .filter(|(_, i)| i.manifest.resources.iter().any(|(n, _)| e.resources.contains_key(n)))
                    .map(|(app, i)| {
                        let r = i
                            .manifest
                            .resources
                            .iter()
                            .map(|(n, c)| {
                                (
                                    n.clone(),
                                    e.resources.get(n).map_or_else(|| c.clone(), |t| t.as_bytes().to_vec()),
                                )
                            })
                            .collect();
                        (app, r)
                    })
                    .collect();
                let pick = sel.clone();
                let forward = w.on_test.clone();
                let (paths, cwd, next_loop) = (paths.clone(), cwd.clone(), next_loop.clone());
                al_testrun::runtime::EntryRun {
                    only: Some(units),
                    run_only,
                    hooks: al_testrun::events::Hooks {
                        on_test: forward.map(|f| {
                            Arc::new(move |mut e: al_testrun::events::TestEvent| {
                                if pick.picks(e.codeunit, &e.test) {
                                    e.entry = Some(k);
                                    let json = e.captured.as_ref().map(|c| {
                                        let base = next_loop.fetch_add(
                                            c.captured.loops.len() as u32,
                                            std::sync::atomic::Ordering::SeqCst,
                                        );
                                        capture::json(c, cfg, base, &|app: usize, file: &str| {
                                            coverage::label(&paths[app], file, &cwd)
                                        })
                                    });
                                    f(e, json)
                                }
                            }) as Arc<dyn Fn(al_testrun::events::TestEvent) + Send + Sync>
                        }),
                        ..opts.hooks.clone()
                    },
                    resources,
                }
            })
            .collect();
    }
    let load_time = started.elapsed();
    // A scratch run has a run of its own for every app; a test run has one for all of them.
    let suites: Vec<SuiteRun> = if a.execute {
        inputs
            .iter()
            .enumerate()
            .map(|(i, input)| SuiteRun {
                name: input.source.name.clone(),
                app: Some(i),
                runner: None,
            })
            .collect()
    } else {
        vec![SuiteRun::everything()]
    };
    let rt = match warm.as_deref_mut() {
        None => {
            let (all_apps, kinds, manifests) = build_apps();
            CorpusRuntime::try_run_suites_with(
                all_apps,
                kinds,
                &manifests,
                &symbols,
                Some(&chosen.target),
                dotnet,
                &suites,
                &opts,
            )?
        }
        Some(w) => {
            // `--tdd` changes what compiles (ADR-051 decision 9), so it is part of the key.
            let target_key = format!("{:?} {}", chosen.target, a.execute);
            let target_key = format!("{target_key} tdd={}", a.tdd);
            // `affectedOnly` (ADR-050) needs a compile that hashed each object.
            let target_key = format!("{target_key} affected={}", a.affected);
            let key = warm_key(&inputs, &dependencies, &symbols, &target_key);
            let files: Vec<al_testrun::runtime::FileChange> = inputs
                .iter()
                .enumerate()
                .flat_map(|(app, i)| {
                    i.source
                        .files
                        .iter()
                        .map(move |(path, text)| al_testrun::runtime::FileChange { app, path, text })
                })
                .collect();
            // Resources are read per request, so a changed resource file reuses the compile (issue 5635).
            let resources: Vec<&[(String, Vec<u8>)]> = inputs.iter().map(|i| i.manifest.resources.as_slice()).collect();
            let (rt, used) = w.runtime.run_with_resources(
                &key,
                &files,
                Some(&resources),
                build_apps,
                &symbols,
                Some(&chosen.target),
                dotnet,
                &suites,
                &opts,
            )?;
            w.used = Some(used);
            rt
        }
    };
    let p = &rt.phases;
    let d = sidecar_host.stats();
    sidecar_host.shutdown();

    // An app under test that does not compile runs no test (al-runner issue 5602), as the AL compiler would not
    // publish it: the errors, exit 3, and `compilationErrors` in the JSON document. Source our parser does not accept
    // is not such an error, as the parser can reject valid AL (al-runner issue 5576): it is said, and the tests that
    // need the file are not run (orchestrator decision 2026-10-21).
    let cwd = std::env::current_dir().unwrap_or_default();
    let located = |e: &al_testrun::runtime::CompileError| {
        let file = inputs
            .get(e.app)
            .map_or_else(|| e.file.clone(), |i| coverage::label(&i.path, &e.file, &cwd).relative);
        (file, output::compile_error_line(e))
    };
    let parse_errors: Vec<(String, String)> = rt
        .compile_errors()
        .iter()
        .filter(|e| e.code.is_empty())
        .map(located)
        .collect();
    if !parse_errors.is_empty() {
        eprintln!(
            "parse: {} place(s) in the APPs our parser did not accept; the tests that need those files are not run",
            parse_errors.len()
        );
        for (file, line) in parse_errors.iter().take(100) {
            eprintln!("parse: {file}{line}");
        }
    }
    if rt.compile_errors().iter().any(|e| !e.code.is_empty()) {
        let errors: Vec<(String, String)> = rt
            .compile_errors()
            .iter()
            .filter(|e| !e.code.is_empty())
            .map(located)
            .collect();
        for (file, line) in errors.iter().take(100) {
            eprintln!("compile: {file}{line}");
        }
        if errors.len() > 100 {
            eprintln!("compile: ... {} more", errors.len() - 100);
        }
        let code = output::COMPILE_ERRORS;
        let why = format!(
            "the APPs do not compile: {} error(s); no test ran (exit {code})",
            errors.len()
        );
        let total_time = started.elapsed();
        if let Some(into) = collect {
            into.bc_build = bc_build.clone();
            return Ok(code);
        }
        say!("{why}");
        let result = format!("Result: COMPILE ERRORS (exit {code})");
        say!("{}", style::bold(&style::yellow(&result)));
        let doc = output::json_compile_errors(code, total_time.as_secs_f64(), &errors);
        if let Some(w) = warm {
            w.report = Some(Report {
                doc: serde_json::from_str(&doc).unwrap_or_default(),
                coverage: None,
                messages: Vec::new(),
                object_names: Vec::new(),
                captured: Vec::new(),
                warnings: vec![why.clone()],
            });
        }
        if let Some(target) = &a.output_json {
            match target {
                Some(path) => write_file(path, &doc)?,
                None => println!("{doc}"),
            }
        }
        return Ok(code);
    }

    // The selected tests, in a stable order: app, codeunit, then declaration order.
    let mut results: Vec<&TestResult> = rt
        .results()
        .iter()
        .filter(|r| a.execute || selection.picks(r.codeunit, &r.test))
        .filter(|r| a.test_apps.is_none_or(|n| r.app < n))
        .collect();
    results.sort_by_key(|r| (r.app, r.codeunit));
    // A request with entries: the results of each entry, picked by its own selection, in the order of the entries.
    let mut entry_counts: Vec<Counts> = Vec::new();
    if !rt.entries.is_empty() {
        results.clear();
        for (range, sel) in rt.entries.iter().zip(&entry_selections) {
            let mut own: Vec<&TestResult> = rt.results()[range.clone()]
                .iter()
                .filter(|r| sel.picks(r.codeunit, &r.test))
                .filter(|r| a.test_apps.is_none_or(|n| r.app < n))
                .collect();
            own.sort_by_key(|r| (r.app, r.codeunit));
            entry_counts.push(Counts::of(own.iter().copied()));
            results.extend(own);
        }
    }
    // `--affected`: the stored result for the next run is written; the skipped codeunits keep their stored outcomes
    // and are reported apart (ADR-050 decisions 9 to 11).
    // A cancelled serve request leaves the stored result as it was (decision 10).
    let cancelled = warm
        .as_ref()
        .and_then(|w| w.cancel.as_ref())
        .is_some_and(|c| c.load(std::sync::atomic::Ordering::SeqCst));
    let affected = affected.and_then(|af| af.finish(rt.results(), rt.impact_keys(), !cancelled));
    let mut counts = Counts::of(results.iter().copied());
    if let Some(r) = &affected {
        counts.still_failing = r.still_failing().len();
    }
    let skipped_results: Vec<TestResult> = affected
        .as_ref()
        .map(crate::affected::Report::junit_results)
        .unwrap_or_default();
    // A missing .NET helper stops tests for a reason that is the machine's, not the tests': say so
    // before the results and again with the summary.
    let dotnet_warning = results
        .iter()
        .find_map(|r| match &r.outcome {
            al_testrun::Outcome::NotRun(why) if why.starts_with(al_vm::dotnet::UNAVAILABLE) => Some(why.as_str()),
            _ => None,
        })
        .and_then(|why| output::dotnet_warning(&counts, why));
    if let Some(w) = &dotnet_warning {
        eprintln!("{w}");
    }

    // Debug aid: `AL_STACK_OF=TEXT` prints, for up to five not-run tests whose reason contains TEXT
    // (and, with `AL_STACK_NAME`, whose name contains it),
    // the routines they were refused in (as `--explain` does for one test).
    if let Some(text) = std::env::var_os("AL_STACK_OF").map(|t| t.to_string_lossy().to_string()) {
        for r in results
            .iter()
            .filter(|r| matches!(&r.outcome, Outcome::NotRun(w) if w.contains(&text)))
            .filter(|r| std::env::var("AL_STACK_NAME").map_or(true, |n| r.test.contains(&n)))
            .take(5)
        {
            let id = CaseId {
                codeunit: r.codeunit,
                name: r.test.clone(),
            };
            let head = format!("{} {}.{}", r.codeunit, r.codeunit_name, r.test);
            eprintln!(
                "{}",
                al_testrun::explain::render_without_oracle(&head, &r.outcome, rt.detail_in("", &id))
            );
        }
    }
    if let Some(pattern) = &a.explain {
        return explain(&rt, &results, pattern, &a.codeunits, &chosen.target.bc.to_string());
    }

    if a.timings {
        let c = &p.compile_parts;
        eprintln!(
            "compile: load {}, members {}, check workspace {}, baseline roots {}, demand {}, check rest {}, lower {}, engine {}",
            secs(c.load),
            secs(c.members),
            secs(c.check.workspace),
            secs(c.check.baseline),
            secs(c.check.demand),
            secs(c.check.rest),
            secs(c.lower),
            secs(p.engine)
        );
        if let Some(line) = p.baseline_line() {
            eprintln!("{line}");
        }
        if let Some(line) = p.compile_line() {
            eprintln!("{line}");
        }
        if let Some(cost) = test_data.as_ref().and_then(|(d, _)| d.provider.cost()) {
            eprintln!("test data: {cost}");
        }
        if let Some(c) = &p.cache {
            eprintln!(
                "compile cache: {}; {} {}",
                if c.hit {
                    format!("hit, opened in {}", secs(c.open))
                } else if c.built {
                    format!("miss, built in {}", secs(c.build))
                } else {
                    "not used".to_string()
                },
                c.file.display(),
                if c.notes.is_empty() {
                    String::new()
                } else {
                    format!("({})", c.notes.join("; "))
                }
            );
            for (i, l) in c.layers.iter().enumerate() {
                eprintln!(
                    "compile cache: layer {} of {} ({}): {}, {:.1} MB",
                    i + 1,
                    c.layers.len(),
                    l.apps.join(", "),
                    if l.hit {
                        "hit".to_string()
                    } else {
                        format!("built in {}", secs(l.build))
                    },
                    l.bytes as f64 / 1e6
                );
            }
            eprintln!(
                "compile cache: {:.1} MB, {} dependency objects, {} compiled again because the workspace extends or implements what they embed",
                c.bytes as f64 / 1e6,
                c.objects,
                c.relowered
            );
            eprintln!(
                "compile cache: reading the blocks {}",
                secs(p.compile_parts.cache_blocks)
            );
        }
        let slowest: Vec<String> = p
            .slowest
            .iter()
            .map(|c| {
                let start = if c.sidecar_start.is_zero() {
                    String::new()
                } else {
                    format!(" (sidecar start {})", secs(c.sidecar_start))
                };
                format!("{} {} {}{start}", c.id, c.name, secs(c.took))
            })
            .collect();
        eprintln!("slowest test codeunits: {}", slowest.join("; "));
        if d.started > 0 {
            eprintln!(
                "dotnet: {} .NET sidecar process(es) started: starting {} in total, {} at most; finding dotnet {}; sessions waited {} for theirs; {} request(s) {}",
                d.started,
                secs(d.start_total),
                secs(d.start_max),
                secs(d.find_total),
                secs(d.waited_total),
                d.calls,
                secs(d.call_total)
            );
        }
    }
    if rt.panics > 0 {
        eprintln!(
            "warning: {} test codeunit(s) hit a runtime panic (reported as not run)",
            rt.panics
        );
    }

    // One line per test, or per failure.
    if !a.quiet {
        let mut shown_failures = 0;
        let mut hidden_failures = 0;
        for r in &results {
            let line = match &r.outcome {
                Outcome::Pass if a.verbose => format!(
                    "{} {} {} {}.{}{}",
                    style::green("PASS   "),
                    ms(r.time),
                    r.codeunit,
                    r.codeunit_name,
                    r.test,
                    if r.assumptions.is_empty() {
                        String::new()
                    } else {
                        style::cyan(" (flagged)")
                    }
                ),
                Outcome::NotRun(why) if a.verbose => format!(
                    "{} {} {} {}.{}: {}",
                    style::yellow("NOT RUN"),
                    ms(r.time),
                    r.codeunit,
                    r.codeunit_name,
                    r.test,
                    output::first_line(why, 300)
                ),
                Outcome::Fail(why) => {
                    if !a.verbose && shown_failures >= 30 {
                        hidden_failures += 1;
                        continue;
                    }
                    shown_failures += 1;
                    let mut line = format!(
                        "{} {}{} {}.{}: {}",
                        style::red("FAIL   "),
                        if a.verbose {
                            format!("{} ", ms(r.time))
                        } else {
                            String::new()
                        },
                        r.codeunit,
                        r.codeunit_name,
                        r.test,
                        output::first_line(why, 300)
                    );
                    for frame in output::stack_lines(&r.call_stack) {
                        line.push_str(&format!("\n          {frame}"));
                    }
                    line
                }
                _ => continue,
            };
            // `--tdd` (ADR-051): the stubs the test ran, as v2 named them.
            let line = if r.stubs.is_empty() {
                line
            } else {
                format!("{line}\n          reaches generated stub(s): {}", r.stubs.join(", "))
            };
            say!("{line}");
        }
        if hidden_failures > 0 {
            say!("... {hidden_failures} more failures (--verbose lists them, --junit has every message)");
        }
    }

    // The ranking of refusal reasons: every run, unless --quiet; --top-refusals N sets N.
    let top = a.top_refusals.or((!a.quiet).then_some(DEFAULT_TOP));
    if let Some(n) = top.filter(|_| counts.not_run > 0) {
        let list = refusals::rank(results.iter().filter_map(|r| match &r.outcome {
            Outcome::NotRun(why) => Some((
                CaseId {
                    codeunit: r.codeunit,
                    name: r.test.clone(),
                },
                why.clone(),
            )),
            _ => None,
        }));
        let text = refusals::render_top("not-run tests", &list, n);
        let (head, rest) = text.split_once('\n').unwrap_or((&text, ""));
        say!("\n{}\n{rest}", style::bold(head));
    }

    // The assumptions the run went through: the counts are kept for `--list-assumptions`; the ranking is printed
    // with --top-assumptions, or as a pointer when passes were flagged.
    {
        let rows = al_testrun::flagged::rank(
            results
                .iter()
                .map(|r| (matches!(r.outcome, Outcome::Pass), r.assumptions.as_slice())),
        );
        al_testrun::flagged::save_hits(&al_deps::cache::cache_root_from_env(a.cache_dir.as_deref()), &rows);
        if let Some(n) = a.top_assumptions {
            println!("\n{}", al_testrun::flagged::render_top(&rows, n));
        } else if counts.flagged > 0 && !a.quiet {
            println!(
                "\n{}",
                style::cyan(&format!(
                    "{} passes ran through an unprobed assumption (flagged); --top-assumptions ranks the assumptions, --list-assumptions describes them.",
                    counts.flagged
                ))
            );
        }
    }

    if let Some(n) = a.top_failures.filter(|_| counts.fail > 0) {
        let list = refusals::rank(results.iter().filter_map(|r| match &r.outcome {
            Outcome::Fail(why) => Some((
                CaseId {
                    codeunit: r.codeunit,
                    name: r.test.clone(),
                },
                why.clone(),
            )),
            _ => None,
        }));
        say!("\n{}", refusals::render_top_failures("failed tests", &list, n));
    }

    if let Some(needle) = &a.refusal_detail {
        let list = refusals::detail(
            results.iter().filter_map(|r| match &r.outcome {
                Outcome::NotRun(why) => Some((
                    CaseId {
                        codeunit: r.codeunit,
                        name: r.test.clone(),
                    },
                    why.clone(),
                )),
                _ => None,
            }),
            needle,
        );
        say!(
            "\n{}",
            refusals::render_top(&format!("not-run tests containing `{needle}`"), &list, 30)
        );
    }

    let mut coverage_doc: Option<String> = None;
    // An impact run counts statements for its own report and writes no coverage file.
    if a.coverage && collect.is_none() {
        let cwd = std::env::current_dir().unwrap_or_default();
        let labels = |app: usize, file: &str| coverage::label(&inputs[app].path, file, &cwd);
        let names: Vec<String> = inputs.iter().map(|i| i.json.name.clone()).collect();
        let statements = rt.coverage_statements();
        let cwd_text = cwd.to_string_lossy().replace('\\', "/");
        // A server request gets its coverage in the summary and writes a file only when it names one.
        if warm.is_none() || a.coverage_out.is_some() {
            let cobertura_path = a
                .coverage_out
                .clone()
                .unwrap_or_else(|| PathBuf::from("coverage.cobertura.xml"));
            write_file(
                &cobertura_path,
                &coverage::cobertura(statements, &results, &names, &labels, &cwd_text),
            )?;
            say!(
                "coverage: {} statements of {} app(s); Cobertura in {}",
                statements.len(),
                inputs.len(),
                cobertura_path.display()
            );
        }
        if a.coverage_json.is_some() || warm.is_some() {
            coverage_doc = Some(coverage::json(statements, &results, &labels));
        }
        if let (Some(path), Some(doc)) = (&a.coverage_json, &coverage_doc) {
            write_file(path, doc)?;
        }
    }
    let total_time = started.elapsed();
    if let Some(path) = &a.junit {
        let mut with_skipped: Vec<&TestResult> = results.iter().copied().chain(&skipped_results).collect();
        with_skipped.sort_by_key(|r| (r.app, r.codeunit));
        write_file(
            path,
            &output::junit(
                &with_skipped,
                bc_build.as_deref(),
                total_time.as_secs_f64(),
                &|cu, t| disabled.as_ref().is_some_and(|d| d.is_disabled(cu, t)),
                &rt.phases.baseline_assumptions,
            ),
        )?;
    }

    let rate = if p.tests.as_secs_f64() > 0.0 {
        counts.total as f64 / p.tests.as_secs_f64()
    } else {
        0.0
    };
    if let Some((data, _)) = test_data.as_ref().filter(|_| a.verbose) {
        let summary = data.stats.summary();
        say!("{}", summary.line(&data.provider.source(), data.provider.company()));
        let refusals = summary.refusals();
        for line in refusals.iter().take(if a.verbose { usize::MAX } else { 10 }) {
            say!("{}", al_testrun::refusals::clip(line, 400));
        }
        if !a.verbose && refusals.len() > 10 {
            say!("... {} more refused tables (--verbose lists them)", refusals.len() - 10);
        }
    }
    // A warning that changes how to read the result stays in the default output, as one line.
    if let Some(w) = p
        .baseline_roots
        .and_then(|(ran, total)| output::baseline_warning(ran, total))
    {
        if !a.quiet {
            say!("{w}");
        }
    }
    if let Some(r) = &affected {
        for line in r.lines() {
            say!("{line}");
        }
    }
    say!("{}", style::bold(&output::summary_line(&counts, test_data.is_some())));
    if !a.quiet && counts.total > 0 {
        for line in output::summary_table(&counts) {
            say!("{line}");
        }
    }
    if let Some(d) = &disabled {
        say!(
            "{}",
            disabled_line(
                d,
                &results,
                &found,
                &Selection {
                    parity: None,
                    ..selection.clone()
                },
                a.parity
            )
        );
    }
    if let Some(w) = &dotnet_warning {
        say!("{w}");
    }
    if !a.verbose && !a.quiet && counts.total > 0 {
        say!("{}", output::time_line(total_time, p.compile, p.tests, rate));
    }
    if a.verbose {
        say!(
            "Time: load {}, compile {}, install baseline {}, run {} ({:.0} tests/s), total {}{}",
            secs_w(load_time),
            secs_w(p.compile),
            secs_w(p.baseline),
            secs_w(p.tests),
            rate,
            secs_w(total_time),
            peak_memory_mb().map_or(String::new(), |m| format!(", peak memory {m} MB"))
        );
    }
    // `--tdd` (ADR-051 decision 8): v2's closing block on stderr, the stubs and the tests that ran one.
    let tdd_stubs: Option<&[String]> = a.tdd.then(|| rt.tdd_stubs());
    if let Some(all) = tdd_stubs {
        for line in output::tdd_block(&results, all) {
            eprintln!("{line}");
        }
    }
    let mut code = output::exit_code(&counts, a.allow_not_run);
    if let Some(r) = &affected {
        code = r.exit_code(code);
    }
    let verdict = match code {
        0 => "PASSED",
        1 => "FAILED",
        6 => "NOTHING SELECTED",
        7 => "TESTS NOT RUN",
        8 => "DOTNET UNAVAILABLE",
        _ => "ERROR",
    };
    if let Some(into) = collect {
        // One side of an impact run: its caller prints the verdict.
        into.results = results.iter().map(|r| (*r).clone()).collect();
        into.statements = rt.coverage_statements().to_vec();
        into.impact_keys = rt.impact_keys().to_vec();
        into.impact_baseline = rt.impact_baseline().map(<[u32]>::to_vec);
        into.program_apps = rt.program_apps().to_vec();
        into.bc_build = bc_build.clone();
        into.baseline_assumptions = rt.phases.baseline_assumptions.clone();
        into.disabled = results
            .iter()
            .map(|r| disabled.as_ref().is_some_and(|d| d.is_disabled(r.codeunit, &r.test)))
            .collect();
        return Ok(code);
    }
    let result = format!("Result: {verdict} (exit {code})");
    say!(
        "{}",
        style::bold(&match code {
            0 => style::green(&result),
            1 => style::red(&result),
            _ => style::yellow(&result),
        })
    );
    if let Some(w) = warm {
        w.report = Some(Report {
            doc: serde_json::from_str(&crate::affected::with_selection(
                output::json_tdd(&results, code, total_time.as_secs_f64(), tdd_stubs),
                affected.as_ref(),
            ))
            .map(|mut doc: serde_json::Value| {
                if !entry_counts.is_empty() {
                    doc["entries"] = serde_json::Value::Array(entry_counts.iter().map(output::json_counts).collect());
                }
                doc
            })
            .unwrap_or_default(),
            coverage: coverage_doc.as_deref().and_then(|d| serde_json::from_str(d).ok()),
            messages: results.iter().map(|r| r.messages.clone()).collect(),
            object_names: results.iter().map(|r| r.codeunit_name.clone()).collect(),
            captured: {
                let cwd = std::env::current_dir().unwrap_or_default();
                let mut base = 0u32;
                results
                    .iter()
                    .map(|r| {
                        r.captured.as_ref().map(|c| {
                            let json = capture::json(c, a.capture, base, &|app: usize, file: &str| {
                                coverage::label(&inputs[app].path, file, &cwd)
                            });
                            base += c.captured.loops.len() as u32;
                            json
                        })
                    })
                    .collect()
            },
            warnings: Vec::new(),
        });
    }
    if let Some(target) = &a.output_json {
        let doc = crate::affected::with_selection(
            output::json_tdd(&results, code, total_time.as_secs_f64(), tdd_stubs),
            affected.as_ref(),
        );
        match target {
            Some(path) => write_file(path, &doc)?,
            None => println!("{doc}"),
        }
    }
    // Last, so that it never delays the tests: the automatic cleanup of the compile cache folder (at most once a
    // day, 200 ms at most; ADR-022 "Cleanup of old builds").
    if !a.no_compile_cache && std::env::var_os("ALRUNNER_NO_COMPILE_CACHE").is_none() {
        let folder = al_testrun::compile_cache::CompileCache::under(&al_deps::cache::cache_root_from_env(
            a.cache_dir.as_deref(),
        ));
        if let Some(r) = al_testrun::cache_gc::auto_prune(&folder, &al_testrun::cache_gc::this_build())
            && r.count() > 0
        {
            say!(
                "compile cache: removed {} old file(s), {:.1} MB",
                r.count(),
                r.bytes() as f64 / 1e6
            );
        }
    }
    Ok(code)
}

/// `--explain PATTERN`: the one test the pattern means (a test named exactly so, else the only one
/// whose name contains it), with where it was refused or failed.
fn explain(
    rt: &CorpusRuntime,
    results: &[&TestResult],
    pattern: &str,
    codeunits: &[u32],
    target: &str,
) -> Result<u8, String> {
    // `--codeunit` narrows the candidates here; elsewhere it selects the whole codeunit.
    let named = |r: &&&TestResult| r.test.to_lowercase().contains(&pattern.to_lowercase());
    let narrowed: Vec<&&TestResult> = results
        .iter()
        .filter(|r| codeunits.is_empty() || codeunits.contains(&r.codeunit))
        .filter(named)
        .collect();
    let exact: Vec<&&TestResult> = narrowed
        .iter()
        .copied()
        .filter(|r| r.test.eq_ignore_ascii_case(pattern))
        .collect();
    let candidates = if exact.len() == 1 { exact } else { narrowed };
    let [r] = candidates.as_slice() else {
        if candidates.is_empty() {
            say!("--explain: no test matches `{pattern}`");
            return Ok(6);
        }
        let list: Vec<String> = candidates
            .iter()
            .take(10)
            .map(|r| format!("  {} {}.{}", r.codeunit, r.codeunit_name, r.test))
            .collect();
        return Err(format!(
            "--explain `{pattern}` matches {} tests; give a longer name or add --codeunit ID:\n{}",
            candidates.len(),
            list.join("\n")
        ));
    };
    let head = format!(
        "BC {target}: test {} in codeunit {} ({})",
        r.test, r.codeunit, r.codeunit_name
    );
    let id = CaseId {
        codeunit: r.codeunit,
        name: r.test.clone(),
    };
    say!(
        "{}",
        al_testrun::explain::render_without_oracle(&head, &r.outcome, rt.detail_in("", &id))
    );
    if !r.assumptions.is_empty() {
        println!(
            "{}",
            al_testrun::flagged::render_explain(&r.assumptions, matches!(r.outcome, Outcome::Pass))
        );
    }
    Ok(match r.outcome {
        Outcome::Pass => 0,
        Outcome::Fail(_) => 1,
        Outcome::NotRun(_) => 7,
    })
}

#[cfg(test)]
mod country_tests {
    use super::*;

    fn args(country: Option<&str>, no_test_data: bool) -> TestArgs {
        TestArgs {
            country: country.map(str::to_string),
            no_test_data,
            ..TestArgs::default()
        }
    }

    #[test]
    fn no_test_data_never_looks_for_a_backup_in_any_country() {
        for country in [None, Some("us"), Some("gb")] {
            // Even for a run that uses Microsoft's apps: nothing is located, nothing is downloaded.
            assert!(
                !should_choose_test_data(&args(country, true), false, true),
                "{country:?}"
            );
            assert!(
                !should_choose_test_data(&args(country, true), false, false),
                "{country:?}"
            );
        }
    }

    #[test]
    fn test_data_on_looks_for_the_backup_of_the_country_when_microsoft_apps_are_used() {
        for country in [None, Some("us")] {
            assert!(should_choose_test_data(&args(country, false), false, true));
            assert!(!should_choose_test_data(&args(country, false), false, false));
            assert!(!should_choose_test_data(&args(country, false), true, true));
        }
    }
}

#[cfg(test)]
mod disabled_line_tests {
    use super::*;

    #[test]
    fn the_parity_count_covers_only_the_selected_codeunits() {
        let mut d = al_testrun::disabled::DisabledTests::default();
        d.add_file(r#"[{"codeunitId":1,"method":"*"},{"codeunitId":2,"method":"*"}]"#)
            .unwrap();
        let found = vec![
            Found {
                app: 0,
                id: 1,
                tests: vec!["A".into(), "B".into()],
            },
            Found {
                app: 0,
                id: 2,
                tests: vec!["C".into()],
            },
        ];
        let sel = Selection {
            codeunits: vec![1],
            ..Selection::default()
        };
        let line = disabled_line(&d, &[], &found, &sel, true);
        assert!(line.starts_with("Disabled by Microsoft: 2 tests not run"), "{line}");
    }
}

#[cfg(test)]
mod selection_tests {
    use super::*;

    fn sel(filters: &[&str], exact: &[&str]) -> Selection {
        Selection {
            filters: filters.iter().map(|s| s.to_string()).collect(),
            exact: exact.iter().map(|s| s.to_string()).collect(),
            ..Selection::default()
        }
    }

    /// Issue 5612: `--test` with `--test-exact` narrows to the tests both match, as in AL Runner v2
    /// (`TestExactFlagTests.TestAndTestExact_Together_AreTheIntersection` on the `v2` branch).
    #[test]
    fn test_and_test_exact_together_select_the_tests_both_match() {
        let s = sel(&["Codeunit50650.Passes"], &["Codeunit50650.Passes"]);
        assert!(s.picks(50650, "Passes"));
        assert!(!s.picks(50650, "PassesToo"));
        let s = sel(&["PassesToo"], &["Codeunit50650.Passes"]);
        assert!(!s.picks(50650, "Passes") && !s.picks(50650, "PassesToo"));
        // Each kind alone, and several of one kind, still select every test one of them matches.
        let s = sel(&["Passes", "Other"], &[]);
        assert!(s.picks(50650, "PassesToo") && s.picks(1, "Other"));
        let s = sel(&[], &["Codeunit50650.Passes", "Codeunit1.Other"]);
        assert!(s.picks(50650, "Passes") && s.picks(1, "Other") && !s.picks(50650, "PassesToo"));
    }
}
