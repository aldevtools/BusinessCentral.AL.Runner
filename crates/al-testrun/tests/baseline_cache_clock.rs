//! The install baseline on disk and the run clock. The rows of a baseline carry the time the run that made them began.
//! A later run that reads the file must start its own clock at that instant, as if it had begun then and run the
//! install code: a run that does not sees rows stamped with a time that its own clock never showed, which an uncached
//! run cannot show.
//!
//! The run clock is read once per process, so the run that writes the file is another process (this test starts
//! itself with `BASELINE_CLOCK_CHILD` set).

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::compile_cache::CompileCache;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SuiteRun};
use std::path::Path;

const SOURCE_BASE: &str = r#"
table 50100 Setting { fields { field(1; Code; Code[20]) { } field(2; Stamp; DateTime) { } } keys { key(PK; Code) { } } }
codeunit 50101 Installer
{
    Subtype = Install;
    trigger OnInstallAppPerCompany() var S: Record Setting; begin S.Code := 'SEEDED'; S.Stamp := CurrentDateTime; S.Insert(); end;
}
"#;

const SOURCE_MAIN: &str = r#"
codeunit 60101 ClockTests
{
    Subtype = Test;
    [Test] procedure RowStampEqualsTheRunClock() var S: Record Setting; begin S.Get('SEEDED'); if S.Stamp <> CurrentDateTime then Error('row stamped %1, clock says %2', S.Stamp, CurrentDateTime); end;
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

fn run(cache: &Path) -> Outcome {
    let manifest = |p: &str| Manifest {
        publisher: p.into(),
        version: "1.0.0.0".into(),
        resources: vec![],
        include_source: false,
        target: String::new(),
        layout_files: Vec::new(),
        id_ranges: Vec::new(),
        symbols: vec![],
    };
    let rt = CorpusRuntime::try_run_suites_with(
        vec![app("main", &["base"], SOURCE_MAIN), app("base", &[], SOURCE_BASE)],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[manifest("Me"), manifest("Microsoft")],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions {
            compile_cache: Some(CompileCache {
                dir: cache.to_path_buf(),
                require: false,
                rebuild: false,
                country: String::new(),
            }),
            ..Default::default()
        },
    )
    .expect("the run starts");
    rt.results()
        .iter()
        .find(|r| r.test == "RowStampEqualsTheRunClock")
        .map(|r| r.outcome.clone())
        .expect("the test ran")
}

#[test]
fn a_run_that_reads_the_kept_baseline_starts_its_clock_where_that_run_began() {
    if let Some(dir) = std::env::var_os("BASELINE_CLOCK_CHILD") {
        assert_eq!(run(Path::new(&dir)), Outcome::Pass, "the run that writes the file");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "a_run_that_reads_the_kept_baseline_starts_its_clock_where_that_run_began",
        ])
        .env("BASELINE_CLOCK_CHILD", dir.path())
        .status()
        .unwrap();
    assert!(status.success(), "the first process");
    let kept = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|x| x == al_testrun::baseline_cache::EXTENSION)
        })
        .count();
    assert_eq!(kept, 1, "the first process kept its baseline");
    // This process began later. Its run reads the file; the rows say when the first process began.
    assert_eq!(run(dir.path()), Outcome::Pass);
}
