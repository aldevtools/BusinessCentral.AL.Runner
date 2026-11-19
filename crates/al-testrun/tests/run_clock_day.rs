//! The run clock's day is the day the run began, also in a process that ran earlier runs (al-runner issue 8457). The
//! run clock starts at 12:34:56.789 UTC of the wall clock's UTC day (ADR-045 amendment 2026-11-15); a `serve` daemon
//! kept past midnight must answer `Today` with the new day, as a daemon started after midnight does.
//!
//! The kept process stands in for a daemon whose earlier request began yesterday ([`as_if_a_run_began_yesterday`]).
//! Every AL run happens in a child process (this test starts itself with `RUN_CLOCK_DAY_MODE` set), so the parent's
//! own state cannot decide what a child sees.

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::compile_cache::CompileCache;
use al_testrun::host::{SystemClock, run_clock_start};
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SuiteRun};
use std::path::Path;

const DAY: i64 = 86_400_000;

/// What AL reads of the run clock, in the message of a test that fails on purpose (the repro of issue 8457).
const SHOW_CLOCK: &str = r#"
codeunit 50910 "Clock Day Probe Tests"
{
    Subtype = Test;
    [Test]
    procedure ShowClock()
    begin
        Error('Today=%1 WorkDate=%2 CurrentDateTime=%3 Time=%4',
            Format(Today, 0, 9), Format(WorkDate, 0, 9), Format(CurrentDateTime, 0, 9), Format(Time, 0, 9));
    end;
}
"#;

/// A dependency app whose install code stamps a row with the run clock: its install baseline is kept on disk.
const STAMPING_DEP: &str = r#"
table 50100 Setting { fields { field(1; Code; Code[20]) { } field(2; Stamp; Text[50]) { } } keys { key(PK; Code) { } } }
codeunit 50101 Installer
{
    Subtype = Install;
    trigger OnInstallAppPerCompany() var S: Record Setting; begin S.Code := 'SEEDED'; S.Stamp := Format(CurrentDateTime, 0, 9); S.Insert(); end;
}
"#;

/// Shows the run clock's day and the stamp the install code left.
const SHOW_STAMP: &str = r#"
codeunit 60101 ClockTests
{
    Subtype = Test;
    [Test] procedure ShowClock() var S: Record Setting; begin S.Get('SEEDED'); Error('Today=%1 Stamp=%2', Format(Today, 0, 9), S.Stamp); end;
}
"#;

fn app(id: &str, deps: &[&str], src: &str) -> AppSource {
    AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.to_string())],
    }
}

fn manifest(publisher: &str) -> Manifest {
    Manifest {
        publisher: publisher.into(),
        version: "1.0.0.0".into(),
        resources: vec![],
        include_source: false,
        target: String::new(),
        logo: None,
        description: None,
        layout_files: Vec::new(),
        id_ranges: Vec::new(),
        translations: false,
        package_data: false,
        package_data_files: Vec::new(),
        translation_files: Vec::new(),
        translation_source: None,
        runtime: None,
        app_json_keys: Vec::new(),
        platform_internals_visible_to: Vec::new(),
        symbols: vec![],
    }
}

/// The message of `ShowClock`: the program alone, or over the stamping dependency with the install baseline kept in
/// `cache`.
fn show_clock(cache: Option<&Path>, opts: RunOptions) -> String {
    let (apps, kinds, manifests) = match cache {
        None => (
            vec![app("main", &[], SHOW_CLOCK)],
            vec![al_sema::AppKind::Workspace],
            vec![manifest("Probe")],
        ),
        Some(_) => (
            vec![app("main", &["base"], SHOW_STAMP), app("base", &[], STAMPING_DEP)],
            vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
            vec![manifest("Me"), manifest("Microsoft")],
        ),
    };
    let opts = RunOptions {
        compile_cache: cache.map(|dir| CompileCache {
            dir: dir.to_path_buf(),
            require: false,
            rebuild: false,
            country: String::new(),
        }),
        ..opts
    };
    let rt = CorpusRuntime::try_run_suites_with(
        apps,
        kinds,
        &manifests,
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &opts,
    )
    .expect("the run starts");
    let outcome = rt
        .results()
        .iter()
        .find(|r| r.test == "ShowClock")
        .map(|r| r.outcome.clone())
        .expect("the test ran");
    match outcome {
        Outcome::Fail(m) => m,
        other => panic!("ShowClock fails on purpose, got {other:?}"),
    }
}

/// This process ran a run that began yesterday, as a daemon whose earlier request began before midnight did.
fn as_if_a_run_began_yesterday(cache: Option<&Path>) {
    let _ = cache;
    assert!(SystemClock::begin_at(run_clock_start(SystemClock::wall()) - DAY));
}

/// The wall clock's UTC day as AL formats a date (`Format(Today, 0, 9)`).
fn today_text() -> String {
    let days = SystemClock::wall().div_euclid(DAY);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Runs this test again in a child process with `mode`, and returns the lines it printed with the mode's prefix.
fn child(test: &str, mode: &str, cache: Option<&Path>) -> Vec<String> {
    let mut cmd = std::process::Command::new(std::env::current_exe().unwrap());
    cmd.args(["--exact", test, "--nocapture"])
        .env("RUN_CLOCK_DAY_MODE", mode);
    if let Some(dir) = cache {
        cmd.env("RUN_CLOCK_DAY_CACHE", dir);
    }
    let out = cmd.output().unwrap();
    assert!(out.status.success(), "{mode}: {}", String::from_utf8_lossy(&out.stderr));
    let prefix = format!("{mode} ");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.strip_prefix(&prefix).map(str::to_string))
        .collect()
}

/// Runs `body` while the wall clock stays on one UTC day; `None` when a midnight passed (the comparison says nothing).
fn on_one_day<T>(body: impl FnOnce(&str) -> T) -> Option<T> {
    let day = today_text();
    let out = body(&day);
    (today_text() == day).then_some(out)
}

#[test]
fn a_process_whose_earlier_run_began_yesterday_answers_today_as_a_fresh_process_does() {
    const TEST: &str = "a_process_whose_earlier_run_began_yesterday_answers_today_as_a_fresh_process_does";
    match std::env::var("RUN_CLOCK_DAY_MODE").as_deref() {
        Ok("KEPT") => {
            as_if_a_run_began_yesterday(None);
            for request in 1..=2 {
                println!("KEPT {request} {}", show_clock(None, RunOptions::default()));
            }
            return;
        }
        Ok("FRESH") => {
            for request in 1..=2 {
                println!("FRESH {request} {}", show_clock(None, RunOptions::default()));
            }
            return;
        }
        _ => {}
    }
    let Some((kept, fresh, day)) =
        on_one_day(|day| (child(TEST, "KEPT", None), child(TEST, "FRESH", None), day.to_string()))
    else {
        return;
    };
    for line in kept.iter().chain(&fresh) {
        println!("{line}");
    }
    assert_eq!(fresh.len(), 2);
    assert!(
        fresh[0].contains(&format!("Today={day} WorkDate={day} CurrentDateTime={day}T")),
        "{}",
        fresh[0]
    );
    assert_eq!(kept, fresh, "the kept process answers as the fresh one");
}

#[test]
fn a_baseline_file_a_kept_process_wrote_today_holds_todays_instant() {
    const TEST: &str = "a_baseline_file_a_kept_process_wrote_today_holds_todays_instant";
    let cache = std::env::var_os("RUN_CLOCK_DAY_CACHE");
    match std::env::var("RUN_CLOCK_DAY_MODE").as_deref() {
        Ok("WRITER") => {
            let cache = Path::new(cache.as_deref().expect("the cache folder"));
            as_if_a_run_began_yesterday(Some(cache));
            // Today's request misses yesterday's file (the day is in the key) and writes one of its own.
            println!("WRITER {}", show_clock(Some(cache), RunOptions::default()));
            return;
        }
        Ok("READER") => {
            let cache = Path::new(cache.as_deref().expect("the cache folder"));
            println!("READER {}", show_clock(Some(cache), RunOptions::default()));
            return;
        }
        _ => {}
    }
    let dir = tempfile::tempdir().unwrap();
    let Some((writer, reader, day)) = on_one_day(|day| {
        let writer = child(TEST, "WRITER", Some(dir.path()));
        let reader = child(TEST, "READER", Some(dir.path()));
        (writer, reader, day.to_string())
    }) else {
        return;
    };
    for line in writer.iter().chain(&reader) {
        println!("{line}");
    }
    // A fresh process that reads the file today begins today, and the rows carry today's instant.
    assert_eq!(reader.len(), 1);
    assert!(
        reader[0].starts_with(&format!("Today={day} Stamp={day}T")),
        "{}",
        reader[0]
    );
}
