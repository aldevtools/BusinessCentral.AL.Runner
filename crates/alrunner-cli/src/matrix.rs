//! Matrix runs (ADR-052 section 3): one run per cell of BC versions times countries (`--bc-version 28.5,29.0
//! --country w1,us`) or of named cells (`--matrix "28.5:w1,29.0:us"`). Every cell is resolved before any runs (the
//! pre-flight, 3.2), then each cell is a child process of this binary with the cell's command line and a hidden
//! `--matrix-cell FILE` that makes it write its `--output-json` document. The parent prints a table of the cells, the
//! tests whose outcome differs between cells, and exits with the worst cell's code.
//!
//! AL Runner defines no preprocessor symbol of its own (3.4): a cell compiles with the app's `preprocessorSymbols` and
//! `--define`, as a single run does.

use crate::args::TestArgs;
use crate::{apps, sources_report};
use al_deps::artifact::{Fetch, HttpCdn};
use al_deps::build::BuildQuery;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// One cell as the command line names it: the version as given (none: the default rule) and the country.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub version: Option<String>,
    pub country: String,
}

impl Cell {
    /// `28.5 w1`; a cell without a version is named by its build once it is resolved.
    fn label(&self, build: &str) -> String {
        format!("{} {}", self.version.as_deref().unwrap_or(build), self.country)
    }
}

/// The cells of a run, or none for a single run. A list (a comma) in `--bc-version` or `--country` makes every
/// combination; `--matrix` names the cells and cannot be combined with either option (3.1, 3.7).
pub fn cells(bc: Option<&str>, country: Option<&str>, matrix: Option<&str>) -> Result<Option<Vec<Cell>>, String> {
    let items = |flag: &str, text: &str| -> Result<Vec<String>, String> {
        let v: Vec<String> = text.split(',').map(|s| s.trim().to_string()).collect();
        if v.iter().any(String::is_empty) {
            return Err(format!("{flag} {text}: an empty item in the list"));
        }
        Ok(v)
    };
    let country_of = |c: &str| al_deps::artifact::normalize_country(c).map_err(|e| format!("--country: {e}"));
    let mut out: Vec<Cell> = Vec::new();
    if let Some(m) = matrix {
        if bc.is_some() || country.is_some() {
            return Err("--matrix names the version and country of every cell: it cannot be combined with --bc-version or --country".into());
        }
        for item in items("--matrix", m)? {
            let (v, c) = match item.split_once(':') {
                Some((v, c)) => (v.trim(), c.trim()),
                None => (item.as_str(), "w1"),
            };
            if v.is_empty() || c.is_empty() {
                return Err(format!("--matrix {m}: `{item}` is not VERSION or VERSION:COUNTRY"));
            }
            out.push(Cell {
                version: Some(v.to_string()),
                country: country_of(c)?,
            });
        }
    } else {
        let listed = |v: Option<&str>| v.is_some_and(|v| v.contains(','));
        if !listed(bc) && !listed(country) {
            return Ok(None);
        }
        let versions: Vec<Option<String>> = match bc {
            Some(v) => items("--bc-version", v)?.into_iter().map(Some).collect(),
            None => vec![None],
        };
        let countries: Vec<String> = match country {
            Some(c) => items("--country", c)?
                .iter()
                .map(|c| country_of(c))
                .collect::<Result<_, _>>()?,
            None => vec!["w1".to_string()],
        };
        for v in &versions {
            for c in &countries {
                out.push(Cell {
                    version: v.clone(),
                    country: c.clone(),
                });
            }
        }
    }
    for (i, c) in out.iter().enumerate() {
        if out[..i].contains(c) {
            return Err(format!(
                "the cell {} {} is given twice",
                c.version.as_deref().unwrap_or("(default version)"),
                c.country
            ));
        }
    }
    Ok(Some(out))
}

/// A cell after the pre-flight: its exact build.
#[derive(Debug, Clone)]
struct Resolved {
    cell: Cell,
    build: String,
}

impl Resolved {
    fn label(&self) -> String {
        self.cell.label(&self.build)
    }

    /// `28.5-w1`, for file names.
    fn file_tag(&self) -> String {
        format!(
            "{}-{}",
            self.cell.version.as_deref().unwrap_or(&self.build),
            self.cell.country
        )
    }
}

/// The checks a single run makes before it starts, for one cell (3.2): its build (cache, artifacts on disk, the feed),
/// the country on the feed, the app.json floors and `--company-data package` only on w1.
fn preflight(a: &TestArgs, cell: &Cell, floor: Option<(u32, u32)>) -> Result<Resolved, String> {
    if a.company_package && !al_deps::artifact::is_w1(&cell.country) {
        return Err("--company-data package supports only the w1 country (its RapidStart package comes from the platform artifact)".into());
    }
    let cdn = HttpCdn::from_env();
    if !a.no_download {
        al_deps::artifact::check_country(&cdn, &cell.country)?;
    }
    let fetch = Fetch {
        cache_root: al_deps::cache::cache_root_from_env(a.cache_dir.as_deref()),
        download: !a.no_download,
        query: BuildQuery::new(cell.version.as_deref(), crate::run::NEWEST_VERIFIED, floor),
        local_artifacts: al_deps::cache::bcartifacts_sandbox_from_env(),
        country: cell.country.clone(),
    };
    let (sel, _, _) = al_deps::test_apps::choose(&fetch, &cdn)?;
    if let Some((major, minor)) = floor
        && (sel.build.major(), sel.build.minor()) < (major, minor)
    {
        return Err(format!(
            "build {} is below the app's floor {major}.{minor} (application/platform in app.json)",
            sel.build
        ));
    }
    Ok(Resolved {
        cell: cell.clone(),
        build: sel.build.to_string(),
    })
}

/// The floor of the APPs (the first APP's, as a single run reads it) and the symbols a cell compiles them with.
/// A BC major and minor.
type Floor = Option<(u32, u32)>;

fn app_floor_and_symbols(a: &TestArgs) -> Result<(Floor, Vec<String>), String> {
    let Some(first) = a.apps.first() else {
        return Ok((None, a.defines.clone()));
    };
    let input = apps::load(first, None)?;
    let floor = input.versions.floor().map(|f| (u32::from(f.major), u32::from(f.minor)));
    let mut symbols = input.symbols.clone();
    symbols.extend(a.defines.iter().cloned());
    Ok((floor, symbols))
}

/// Options that take a value and are set per cell by the parent: they are left out of a cell's command line.
const PER_CELL: [&str; 6] = [
    "--bc-version",
    "--country",
    "--matrix",
    "--matrix-parallel",
    "--matrix-memory",
    "--junit",
];

/// The command line of one cell: the run's own, without the options the parent sets per cell, then the cell's build
/// and country, its JUnit file, and the file of its document.
pub fn cell_args(
    args: &[String],
    tag: &str,
    build: &str,
    country: &str,
    junit: Option<&Path>,
    document: &Path,
) -> Vec<String> {
    let mut out = Vec::new();
    let mut it = args.iter().peekable();
    while let Some(a) = it.next() {
        if PER_CELL.contains(&a.as_str()) {
            it.next();
            continue;
        }
        // ADR-052 3.3: a cell's stored result of --affected is its own file.
        if a == "--affected-store" {
            if let Some(f) = it.next() {
                out.extend([a.clone(), format!("{f}.{tag}")]);
            }
            continue;
        }
        if a == "--output-json" {
            if it.peek().is_some_and(|v| v.to_ascii_lowercase().ends_with(".json")) {
                it.next();
            }
            continue;
        }
        if a.starts_with("--output-json=") || PER_CELL.iter().any(|f| a.starts_with(&format!("{f}="))) {
            continue;
        }
        out.push(a.clone());
    }
    out.extend(["--bc-version".into(), build.into(), "--country".into(), country.into()]);
    if let Some(j) = junit {
        out.extend(["--junit".into(), j.display().to_string()]);
    }
    out.extend(["--matrix-cell".into(), document.display().to_string()]);
    out
}

/// `results.xml` -> `results.28.5-w1.xml` (3.6).
fn cell_file(path: &Path, tag: &str) -> PathBuf {
    let stem = path
        .file_stem()
        .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
    path.with_file_name(match path.extension() {
        Some(e) => format!("{stem}.{tag}.{}", e.to_string_lossy()),
        None => format!("{stem}.{tag}"),
    })
}

/// The printable command line of a cell (without its hidden document option).
fn command_line(args: &[String]) -> String {
    let mut out = vec!["al-runner".to_string()];
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "--matrix-cell" {
            it.next();
            continue;
        }
        out.push(
            if a.is_empty() || a.contains(|c: char| c.is_whitespace() || c == '\'') {
                format!("'{}'", a.replace('\'', "'\\''"))
            } else {
                a.clone()
            },
        );
    }
    out.join(" ")
}

/// The free memory of the machine in MB (`MemAvailable`), where the system says it (Linux). `ALRUNNER_MATRIX_FREE_MB`
/// replaces it (for the tests of the scheduling rule).
fn free_mb() -> Option<u64> {
    if let Some(v) = std::env::var("ALRUNNER_MATRIX_FREE_MB")
        .ok()
        .and_then(|v| v.parse().ok())
    {
        return Some(v);
    }
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    let line = text.lines().find(|l| l.starts_with("MemAvailable:"))?;
    line.split_whitespace().nth(1)?.parse::<u64>().ok().map(|kb| kb / 1024)
}

/// The peak memory of a running process so far in MB (`VmHWM`), where the system says it (Linux).
fn peak_mb(pid: u32) -> Option<u64> {
    let text = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    let line = text.lines().find(|l| l.starts_with("VmHWM:"))?;
    line.split_whitespace().nth(1)?.parse::<u64>().ok().map(|kb| kb / 1024)
}

/// What is kept free besides the cells' expected peaks (3.5).
const RESERVE_MB: u64 = 2048;

/// Whether one more cell with `expected` MB may start (3.5): below the cap of cells at once, the free memory above
/// its expected peak plus the reserve, and with `--matrix-memory`, the expected peaks of the running cells plus this
/// one within the cap. With no cell running a cell always starts: waiting would never end, and one cell at a time is
/// the run without `--matrix-parallel`.
pub fn may_start(running: &[u64], expected: u64, parallel: usize, free: Option<u64>, cap_mb: Option<u64>) -> bool {
    if running.is_empty() {
        return true;
    }
    if running.len() >= parallel {
        return false;
    }
    if free.is_some_and(|f| f < expected + RESERVE_MB) {
        return false;
    }
    cap_mb.is_none_or(|cap| running.iter().sum::<u64>() + expected <= cap)
}

/// The expected peaks of the last matrix run, per cell kind (3.5): `<cache>/matrix/peaks.json`.
fn peaks_file(a: &TestArgs) -> PathBuf {
    al_deps::cache::cache_root_from_env(a.cache_dir.as_deref())
        .join("matrix")
        .join("peaks.json")
}

/// The key of a cell in the peaks file: the build, country and whether the `microsoft` source is part of the run.
fn peak_key(r: &Resolved, microsoft: bool) -> String {
    format!(
        "{}-{}-{}",
        r.build,
        r.cell.country,
        if microsoft { "microsoft" } else { "plain" }
    )
}

/// What one cell came to.
struct CellRun {
    resolved: Resolved,
    args: Vec<String>,
    code: u8,
    /// Why the cell ended without its own exit code (a signal).
    killed: Option<String>,
    peak_mb: Option<u64>,
    started_ms: u128,
    ended_ms: u128,
    /// stdout and stderr of the cell, in that order.
    output: String,
    /// The documents the cell wrote: (file name, document).
    documents: Vec<(String, serde_json::Value)>,
}

/// One test's outcome in one cell: `pass`, `fail` or `notRun` for a test of the own run, the change class for a test
/// of a source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seen {
    pub word: String,
    pub flagged: bool,
    pub message: Option<String>,
}

/// A test across cells: (part, codeunit, test). The part is empty for the run's own tests, else the source and the
/// Microsoft test app.
pub type Key = (String, u32, String);

/// The tests of a cell's documents.
pub fn tests_of(documents: &[(String, serde_json::Value)]) -> BTreeMap<Key, Seen> {
    let mut out = BTreeMap::new();
    for (_, doc) in documents {
        for t in doc["tests"].as_array().into_iter().flatten() {
            let (Some(cu), Some(test)) = (t["codeunit"].as_u64(), t["method"].as_str()) else {
                continue;
            };
            let word = match t["status"].as_str() {
                Some("pass") => "pass",
                Some("fail") => "fail",
                _ => "notRun",
            };
            out.insert(
                (String::new(), cu as u32, test.to_string()),
                Seen {
                    word: word.into(),
                    flagged: t["flagged"].as_bool().unwrap_or(false),
                    message: t["message"].as_str().map(str::to_string),
                },
            );
        }
        for s in doc["sources"].as_array().into_iter().flatten() {
            let source = s["source"].as_str().unwrap_or_default();
            for t in s["tests"].as_array().into_iter().flatten() {
                let (Some(cu), Some(test)) = (t["codeunit"].as_u64(), t["test"].as_str()) else {
                    continue;
                };
                let part = match t["testApp"].as_str() {
                    Some(app) => format!("{source} {app}"),
                    None => source.to_string(),
                };
                out.insert(
                    (part, cu as u32, test.to_string()),
                    Seen {
                        word: t["class"].as_str().unwrap_or_default().to_string(),
                        flagged: t["with"]["flagged"].as_bool().unwrap_or(false),
                        message: t["with"]["message"].as_str().map(str::to_string),
                    },
                );
            }
        }
    }
    out
}

/// The tests whose outcome differs between cells (3.6), and the tests that ran in some cells and were not run in
/// others. A test of the run's own: pass and fail (a flagged pass is a pass) and `absent` differ, not run is left
/// out of the comparison; failing everywhere with different texts differs too. A test of a source: its change class
/// differs, or it is absent from a cell.
pub fn differs(cells: &[BTreeMap<Key, Seen>]) -> (Vec<Key>, Vec<Key>) {
    let keys: BTreeSet<&Key> = cells.iter().flat_map(|c| c.keys()).collect();
    let mut differ = Vec::new();
    let mut gaps = Vec::new();
    for key in keys {
        let seen: Vec<Option<&Seen>> = cells.iter().map(|c| c.get(key)).collect();
        let words: BTreeSet<&str> = seen
            .iter()
            .map(|s| s.map_or("absent", |s| s.word.as_str()))
            .filter(|w| *w != "notRun")
            .collect();
        let own = key.0.is_empty();
        let all_fail_differently = own
            && words.len() == 1
            && words.contains("fail")
            && seen
                .iter()
                .flatten()
                .filter(|s| s.word == "fail")
                .map(|s| s.message.as_deref())
                .collect::<BTreeSet<_>>()
                .len()
                > 1;
        if words.len() > 1 || all_fail_differently {
            differ.push(key.clone());
        } else if own && seen.iter().flatten().any(|s| s.word == "notRun") && !words.is_empty() {
            gaps.push(key.clone());
        }
    }
    (differ, gaps)
}

/// A cell's value in the "differs" table.
fn shown(s: Option<&Seen>) -> String {
    match s {
        None => "absent".into(),
        Some(s) if s.word == "pass" && s.flagged => "PASS*".into(),
        Some(s) if matches!(s.word.as_str(), "pass" | "fail") => s.word.to_ascii_uppercase(),
        Some(s) if s.word == "notRun" => "not run".into(),
        Some(s) => s.word.clone(),
    }
}

fn table(rows: &[Vec<String>]) -> Vec<String> {
    let n = rows.iter().map(Vec::len).max().unwrap_or(0);
    let widths: Vec<usize> = (0..n)
        .map(|c| rows.iter().map(|r| r.get(c).map_or(0, String::len)).max().unwrap_or(0))
        .collect();
    rows.iter()
        .map(|r| {
            let cells: Vec<String> = r
                .iter()
                .enumerate()
                .map(|(c, v)| format!("{v:<w$}", w = widths[c]))
                .collect();
            format!("  {}", cells.join("  ").trim_end())
        })
        .collect()
}

/// Counts of a cell's own tests: (pass, flagged, fail, not run).
fn counts(tests: &BTreeMap<Key, Seen>) -> (usize, usize, usize, usize) {
    let own = || tests.iter().filter(|(k, _)| k.0.is_empty()).map(|(_, s)| s);
    (
        own().filter(|s| s.word == "pass").count(),
        own().filter(|s| s.word == "pass" && s.flagged).count(),
        own().filter(|s| s.word == "fail").count(),
        own().filter(|s| s.word == "notRun").count(),
    )
}

/// The `sources` entries of a cell's documents.
fn sources_of(documents: &[(String, serde_json::Value)]) -> Vec<&serde_json::Value> {
    documents
        .iter()
        .flat_map(|(_, d)| d["sources"].as_array().into_iter().flatten())
        .collect()
}

/// `al-runner test ... --bc-version A,B` and the other matrix forms (ADR-052 section 3). `microsoft`: the run has the
/// `microsoft` source (`test-microsoft`, `--affected-from microsoft`), for the expected peak of a cell.
pub fn run(a: &TestArgs, cells: &[Cell], microsoft: bool) -> Result<u8, String> {
    let (floor, symbols) = app_floor_and_symbols(a)?;
    // 3.2: every cell is resolved before any runs.
    let mut resolved = Vec::new();
    let mut bad = Vec::new();
    for c in cells {
        match preflight(a, c, floor) {
            Ok(r) => resolved.push(r),
            Err(e) => bad.push(format!(
                "  cell {} {}: {}",
                c.version.as_deref().unwrap_or("(default version)"),
                c.country,
                e.lines().next().unwrap_or_default()
            )),
        }
    }
    if !bad.is_empty() {
        return Err(format!(
            "the matrix cannot start: {} of {} cell(s) fail the checks a run makes before it starts; no cell was run\n{}",
            bad.len(),
            cells.len(),
            bad.join("\n")
        ));
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    // The cells' documents and output, removed when the report is written.
    let work = Work(
        al_deps::cache::cache_root_from_env(a.cache_dir.as_deref())
            .join("matrix")
            .join(format!("run-{}-{}", std::process::id(), now_ms())),
    );
    std::fs::create_dir_all(&work.0).map_err(|e| format!("{}: {e}", work.0.display()))?;
    let peaks_path = peaks_file(a);
    let mut peaks: BTreeMap<String, u64> = std::fs::read_to_string(&peaks_path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    let parallel = a.matrix_parallel.unwrap_or(1).max(1);
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let per_cell = (threads / parallel.min(resolved.len()).max(1)).max(1);
    let exe = std::env::current_exe().map_err(|e| format!("the al-runner binary: {e}"))?;
    let now = now_ms;

    let header = format!(
        "Matrix: {} cell(s): {}{}",
        resolved.len(),
        resolved
            .iter()
            .map(|r| format!("{} ({})", r.label(), r.build))
            .collect::<Vec<_>>()
            .join(", "),
        if parallel > 1 {
            format!("; up to {parallel} at once")
        } else {
            String::new()
        }
    );
    eprintln!("{header}");

    struct Running {
        index: usize,
        child: std::process::Child,
        expected: u64,
        peak: Option<u64>,
        started: u128,
        out_path: PathBuf,
        err_path: PathBuf,
        args: Vec<String>,
    }
    let mut runs: Vec<Option<CellRun>> = (0..resolved.len()).map(|_| None).collect();
    let mut running: Vec<Running> = Vec::new();
    let mut next = 0usize;
    while next < resolved.len() || !running.is_empty() {
        // Start what may start (3.5).
        while next < resolved.len() {
            let r = &resolved[next];
            let expected =
                peaks
                    .get(&peak_key(r, microsoft))
                    .copied()
                    .unwrap_or(if microsoft { 10 * 1024 } else { 3 * 1024 });
            let using: Vec<u64> = running.iter().map(|r| r.expected).collect();
            let cap = a.matrix_memory_gb.map(|g| (g * 1024.0) as u64);
            if !may_start(&using, expected, parallel, free_mb(), cap) {
                break;
            }
            let dir = work.0.as_path().join(format!("cell-{next}"));
            std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            let junit = a.junit.as_deref().map(|p| cell_file(p, &r.file_tag()));
            let cargs = cell_args(
                &args,
                &r.file_tag(),
                &r.build,
                &r.cell.country,
                junit.as_deref(),
                &dir.join("cell.json"),
            );
            let out_path = dir.join("stdout.txt");
            let err_path = dir.join("stderr.txt");
            let file = |p: &Path| std::fs::File::create(p).map_err(|e| format!("{}: {e}", p.display()));
            let mut cmd = std::process::Command::new(&exe);
            cmd.args(&cargs)
                .env("ALRUNNER_NO_PATH_NOTE", "1")
                .stdout(file(&out_path)?)
                .stderr(file(&err_path)?);
            if parallel > 1 {
                for v in ["AL_CHECK_THREADS", "AL_LOWER_THREADS", "AL_TEST_THREADS"] {
                    let given = std::env::var(v).ok().and_then(|t| t.parse::<usize>().ok());
                    let n = given.map_or(per_cell, |g| (g / parallel).max(1));
                    cmd.env(v, n.to_string());
                }
            }
            eprintln!("cell {} started: {}", r.label(), command_line(&cargs));
            let child = cmd.spawn().map_err(|e| format!("cell {}: {e}", r.label()))?;
            running.push(Running {
                index: next,
                child,
                expected,
                peak: None,
                started: now(),
                out_path,
                err_path,
                args: cargs,
            });
            next += 1;
        }
        // Reap what ended; note each running cell's peak so far.
        let mut i = 0;
        while i < running.len() {
            let pid = running[i].child.id();
            if let Some(p) = peak_mb(pid) {
                running[i].peak = Some(running[i].peak.map_or(p, |q| q.max(p)));
            }
            match running[i].child.try_wait() {
                Ok(Some(status)) => {
                    let done = running.swap_remove(i);
                    let r = resolved[done.index].clone();
                    let (code, killed) = match status.code() {
                        Some(c) => (u8::try_from(c).unwrap_or(2), None),
                        None => (2, Some(signal_reason(&status))),
                    };
                    let output = format!(
                        "{}{}",
                        std::fs::read_to_string(&done.out_path).unwrap_or_default(),
                        std::fs::read_to_string(&done.err_path).unwrap_or_default()
                    );
                    let documents = documents_in(done.out_path.parent().unwrap_or(work.0.as_path()));
                    if let Some(p) = done.peak {
                        peaks.insert(peak_key(&r, microsoft), p);
                    }
                    eprintln!(
                        "cell {} ended: exit {code}{}",
                        r.label(),
                        killed.as_deref().map(|k| format!(" ({k})")).unwrap_or_default()
                    );
                    runs[done.index] = Some(CellRun {
                        resolved: r,
                        args: done.args,
                        code,
                        killed,
                        peak_mb: done.peak,
                        started_ms: done.started,
                        ended_ms: now(),
                        output,
                        documents,
                    });
                }
                Ok(None) => i += 1,
                Err(e) => return Err(format!("a matrix cell: {e}")),
            }
        }
        if !running.is_empty() {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }
    if let Some(dir) = peaks_path.parent()
        && std::fs::create_dir_all(dir).is_ok()
    {
        let _ = std::fs::write(&peaks_path, serde_json::to_string_pretty(&peaks).unwrap_or_default());
    }
    let runs: Vec<CellRun> = runs.into_iter().flatten().collect();
    report(a, &runs, &symbols)
}

fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis())
}

/// A folder that is removed when it goes out of scope.
struct Work(PathBuf);

impl Drop for Work {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Why a child ended without an exit code.
fn signal_reason(status: &std::process::ExitStatus) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(s) = status.signal() {
            return format!(
                "killed by signal {s}{}",
                if s == 9 {
                    " (the system may have run out of memory)"
                } else {
                    ""
                }
            );
        }
    }
    format!("ended without an exit code ({status})")
}

/// The JSON documents a cell wrote in its folder, by file name.
fn documents_in(dir: &Path) -> Vec<(String, serde_json::Value)> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = rd
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    files
        .into_iter()
        .filter_map(|p| {
            let doc = serde_json::from_str(&std::fs::read_to_string(&p).ok()?).ok()?;
            Some((p.file_name()?.to_string_lossy().into_owned(), doc))
        })
        .collect()
}

/// The lines of a cell's own report shown without `--verbose`: failures, errors and the result lines.
fn short_lines(output: &str) -> Vec<&str> {
    output
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            t.starts_with("FAIL") || t.starts_with("error") || t.starts_with("Result:") || t.starts_with("Tests:")
        })
        .collect()
}

fn report(a: &TestArgs, runs: &[CellRun], symbols: &[String]) -> Result<u8, String> {
    for r in runs {
        println!(
            "== cell {} ({}): exit {}{} ==",
            r.resolved.label(),
            r.resolved.build,
            r.code,
            r.killed.as_deref().map(|k| format!(", {k}")).unwrap_or_default()
        );
        println!("   {}", command_line(&r.args));
        if a.verbose {
            print!("{}", r.output);
        } else {
            for l in short_lines(&r.output) {
                println!("{l}");
            }
        }
    }
    let tests: Vec<BTreeMap<Key, Seen>> = runs.iter().map(|r| tests_of(&r.documents)).collect();
    let mut rows = vec![vec![
        "cell".to_string(),
        "build".into(),
        "pass".into(),
        "flagged".into(),
        "fail".into(),
        "not run".into(),
        "exit".into(),
    ]];
    for (r, t) in runs.iter().zip(&tests) {
        let (pass, flagged, fail, not_run) = counts(t);
        rows.push(vec![
            r.resolved.label(),
            r.resolved.build.clone(),
            pass.to_string(),
            flagged.to_string(),
            fail.to_string(),
            not_run.to_string(),
            r.code.to_string(),
        ]);
    }
    println!("\nMatrix");
    for l in table(&rows) {
        println!("{l}");
    }
    if runs.iter().any(|r| !sources_of(&r.documents).is_empty()) {
        let mut rows = vec![vec![
            "cell".to_string(),
            "part".into(),
            "tests run".into(),
            "newly failing".into(),
            "fails differently".into(),
            "newly refused".into(),
            "not verified".into(),
            "exit".into(),
        ]];
        for r in runs {
            for s in sources_of(&r.documents) {
                let n = |k: &str| s["counts"][k].as_u64().unwrap_or(0);
                let total: u64 = s["counts"]
                    .as_object()
                    .map_or(0, |c| c.values().filter_map(|v| v.as_u64()).sum());
                rows.push(vec![
                    r.resolved.label(),
                    s["source"].as_str().unwrap_or_default().to_string(),
                    total.to_string(),
                    (n("newlyFailing") + n("selectionMiss")).to_string(),
                    n("failsDifferently").to_string(),
                    n("newlyRefused").to_string(),
                    n("notVerified").to_string(),
                    s["exitCode"].to_string(),
                ]);
            }
        }
        println!("\nSources per cell");
        for l in table(&rows) {
            println!("{l}");
        }
    }
    let (differ, gaps) = differs(&tests);
    let labels: Vec<String> = runs.iter().map(|r| r.resolved.label()).collect();
    if !differ.is_empty() {
        println!("\nDiffers between cells ({} tests)", differ.len());
        let mut rows = vec![{
            let mut h = vec!["part".to_string(), "codeunit".into(), "test".into()];
            h.extend(labels.iter().cloned());
            h
        }];
        let limit = if a.verbose { usize::MAX } else { 50 };
        for key in differ.iter().take(limit) {
            let mut row = vec![
                if key.0.is_empty() { "-".into() } else { key.0.clone() },
                key.1.to_string(),
                key.2.clone(),
            ];
            row.extend(tests.iter().map(|t| shown(t.get(key))));
            rows.push(row);
        }
        for l in table(&rows) {
            println!("{l}");
        }
        if differ.len() > limit {
            println!("  ... {} more (--verbose lists them)", differ.len() - limit);
        }
    }
    if !gaps.is_empty() {
        println!(
            "  {} test(s) run in some cells and are not run in others{}",
            gaps.len(),
            if a.verbose { ":" } else { " (--verbose lists them)" }
        );
        if a.verbose {
            for key in &gaps {
                let cells: Vec<String> = labels
                    .iter()
                    .zip(&tests)
                    .map(|(l, t)| format!("{l}: {}", shown(t.get(key))))
                    .collect();
                println!("    {} {}: {}", key.1, key.2, cells.join(", "));
            }
        }
    }
    let code = sources_report::worst(runs.iter().map(|r| r.code));
    let worst = runs.iter().find(|r| r.code == code);
    let verdict = match code {
        0 => "PASS",
        1 => "FAIL",
        6 => "NOTHING SELECTED",
        7 => "TESTS NOT RUN",
        8 => "DOTNET UNAVAILABLE",
        _ => "ERROR",
    };
    match worst {
        Some(w) if code != 0 => println!("Result: {verdict} (worst cell {}: exit {code})", w.resolved.label()),
        _ => println!("Result: {verdict} (exit {code})"),
    }
    if let Some(Some(path)) = &a.output_json {
        let doc = serde_json::json!({
            "format": 1,
            "cells": runs.iter().map(|r| serde_json::json!({
                "bcVersion": r.resolved.cell.version,
                "build": r.resolved.build,
                "country": r.resolved.cell.country,
                "symbols": symbols,
                "commandLine": command_line(&r.args),
                "exitCode": r.code,
                "killed": r.killed,
                "peakMemoryMb": r.peak_mb,
                "startedMs": r.started_ms as u64,
                "endedMs": r.ended_ms as u64,
                "run": match r.documents.as_slice() {
                    [(_, one)] => one.clone(),
                    [] => serde_json::Value::Null,
                    many => serde_json::json!({"documents": many.iter().map(|(n, d)| serde_json::json!({"file": n, "document": d})).collect::<Vec<_>>()}),
                },
            })).collect::<Vec<_>>(),
            "differs": differ.iter().map(|k| {
                let mut outcomes = serde_json::Map::new();
                let mut messages = serde_json::Map::new();
                for (l, t) in labels.iter().zip(&tests) {
                    let s = t.get(k);
                    outcomes.insert(l.clone(), serde_json::json!(s.map_or("absent", |s| s.word.as_str())));
                    if let Some(m) = s.and_then(|s| s.message.as_deref()) {
                        messages.insert(l.clone(), serde_json::json!(m));
                    }
                }
                let mut d = serde_json::json!({"codeunit": k.1, "test": k.2, "outcomes": outcomes, "messages": messages});
                if !k.0.is_empty() {
                    d["part"] = serde_json::json!(k.0);
                }
                d
            }).collect::<Vec<_>>(),
            "notRunInSome": gaps.len(),
            "exitCode": code,
        });
        crate::run::write_file(
            path,
            &serde_json::to_string_pretty(&doc).expect("a JSON value serializes"),
        )?;
    }
    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(v: &str, cc: &str) -> Cell {
        Cell {
            version: Some(v.into()),
            country: cc.into(),
        }
    }

    #[test]
    fn lists_make_every_combination_and_matrix_names_the_cells() {
        assert_eq!(cells(Some("29.0"), Some("us"), None).unwrap(), None);
        assert_eq!(
            cells(Some("27.5,28.5"), Some("w1,US"), None).unwrap().unwrap(),
            vec![c("27.5", "w1"), c("27.5", "us"), c("28.5", "w1"), c("28.5", "us")]
        );
        assert_eq!(
            cells(None, Some("w1,de"), None).unwrap().unwrap()[1],
            Cell {
                version: None,
                country: "de".into()
            }
        );
        assert_eq!(
            cells(None, None, Some("27.5:w1, 29.0:us,28.5")).unwrap().unwrap(),
            vec![c("27.5", "w1"), c("29.0", "us"), c("28.5", "w1")]
        );
        for (bc, cc, m) in [
            (Some("27.5,28.5"), None, Some("27.5")),
            (None, Some("us"), Some("27.5")),
            (Some("27.5,,28.5"), None, None),
            (None, None, Some("27.5:w1,27.5")),
            (None, None, Some(":us")),
            (Some("27.5,27.5"), None, None),
        ] {
            assert!(cells(bc, cc, m).is_err(), "{bc:?} {cc:?} {m:?}");
        }
    }

    #[test]
    fn a_cell_command_line_has_its_own_build_country_and_files() {
        let args: Vec<String> = "test App --bc-version 27.5,28.5 --country w1,us --junit r.xml --output-json o.json --matrix-parallel 2 --dep D --affected --affected-store S"
            .split(' ')
            .map(String::from)
            .collect();
        let got = cell_args(
            &args,
            "27.5-us",
            "27.5.1.2",
            "us",
            Some(Path::new("r.27.5-us.xml")),
            Path::new("/w/cell.json"),
        );
        assert_eq!(
            got.join(" "),
            "test App --dep D --affected --affected-store S.27.5-us --bc-version 27.5.1.2 --country us --junit r.27.5-us.xml --matrix-cell /w/cell.json"
        );
        assert_eq!(
            cell_file(Path::new("out/results.xml"), "27.5-w1"),
            PathBuf::from("out/results.27.5-w1.xml")
        );
        assert_eq!(
            command_line(&got),
            "al-runner test App --dep D --affected --affected-store S.27.5-us --bc-version 27.5.1.2 --country us --junit r.27.5-us.xml"
        );
    }

    #[test]
    fn a_cell_starts_only_within_the_parallel_count_free_memory_and_cap() {
        assert!(
            may_start(&[], 10_000, 1, Some(10), Some(1)),
            "a lone cell always starts"
        );
        assert!(!may_start(&[3072], 3072, 1, Some(100_000), None));
        assert!(may_start(&[3072], 3072, 2, Some(100_000), None));
        assert!(!may_start(&[3072], 3072, 2, Some(3072 + 2047), None), "the reserve");
        assert!(!may_start(&[3072], 3072, 2, Some(100_000), Some(5000)), "the cap");
        assert!(may_start(&[3072], 3072, 2, None, Some(6144)));
    }

    fn seen(word: &str, message: Option<&str>) -> Seen {
        Seen {
            word: word.into(),
            flagged: false,
            message: message.map(str::to_string),
        }
    }

    #[test]
    fn differs_compares_outcomes_where_a_test_ran_and_counts_not_run_apart() {
        let k = |t: &str| (String::new(), 1u32, t.to_string());
        let mut a = BTreeMap::new();
        let mut b = BTreeMap::new();
        a.insert(k("PassFail"), seen("pass", None));
        b.insert(k("PassFail"), seen("fail", Some("x")));
        a.insert(k("Same"), seen("pass", None));
        b.insert(
            k("Same"),
            Seen {
                flagged: true,
                ..seen("pass", None)
            },
        );
        a.insert(k("Gap"), seen("pass", None));
        b.insert(k("Gap"), seen("notRun", Some("refused")));
        a.insert(k("OnlyA"), seen("pass", None));
        a.insert(k("TwoTexts"), seen("fail", Some("one")));
        b.insert(k("TwoTexts"), seen("fail", Some("two")));
        let s = ("Vendor".to_string(), 2u32, "S".to_string());
        a.insert(s.clone(), seen("same, reached", None));
        b.insert(s.clone(), seen("newly failing", None));
        let (d, gaps) = differs(&[a, b]);
        assert_eq!(d, vec![k("OnlyA"), k("PassFail"), k("TwoTexts"), s]);
        assert_eq!(gaps, vec![k("Gap")]);
        assert_eq!(
            shown(Some(&Seen {
                flagged: true,
                ..seen("pass", None)
            })),
            "PASS*"
        );
        assert_eq!(shown(None), "absent");
    }
}
