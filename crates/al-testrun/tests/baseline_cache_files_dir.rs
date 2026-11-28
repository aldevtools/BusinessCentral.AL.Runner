//! `--files-dir` gives the `File` type a real directory (`host::set_files_dir`, set once per process, so this test has
//! a binary of its own). What the install code reads from there is not in the key of the kept install baseline, so a
//! run with it keeps no baseline: a changed file is read again by the next run. Both tests share one directory
//! ([`files_dir`]); the second has no dependency app and reads no file: the install code of the apps under test runs in
//! each codeunit's session, and `--timings` says why.

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::compile_cache::CompileCache;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SuiteRun};
use std::path::Path;

const BASE: &str = r#"
table 50100 Setting { fields { field(1; Code; Code[20]) { } field(2; Value; Text[50]) { } } keys { key(PK; Code) { } } }
codeunit 50101 Installer
{
    Subtype = Install;
    trigger OnInstallAppPerCompany() var S: Record Setting; F: File; I: InStream; T: Text; begin
        F.Open('setting.txt'); F.CreateInStream(I); I.ReadText(T); F.Close();
        S.Code := 'FILE'; S.Value := T; S.Insert(); end;
}
"#;

const TESTS: &str = r#"
codeunit 60101 FileTests
{
    Subtype = Test;
    [Test] procedure Row() var S: Record Setting; begin S.Get('FILE'); Error('row: %1', S.Value); end;
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

/// The process's `--files-dir`, the same for both tests (`host::set_files_dir` keeps the first).
fn files_dir() -> &'static Path {
    static DIR: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();
    let dir = DIR.get_or_init(|| tempfile::tempdir().unwrap()).path();
    al_testrun::host::set_files_dir(dir.to_path_buf());
    dir
}

fn manifest(p: &str) -> Manifest {
    Manifest {
        publisher: p.into(),
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

fn run(cache: &Path) -> String {
    let rt = CorpusRuntime::try_run_suites_with(
        vec![app("main", &["base"], TESTS), app("base", &[], BASE)],
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
    match rt.results().iter().find(|r| r.test == "Row").map(|r| r.outcome.clone()) {
        Some(Outcome::Fail(m)) => m,
        other => format!("{other:?}"),
    }
}

#[test]
fn a_changed_file_the_install_code_reads_is_read_again_by_the_next_run() {
    let files = files_dir();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(files.join("setting.txt"), "first").unwrap();
    let first = run(cache.path());
    assert!(first.contains("row: first"), "{first}");
    std::fs::write(files.join("setting.txt"), "second").unwrap();
    let second = run(cache.path());
    assert!(
        second.contains("row: second"),
        "the install code read the file again: {second}"
    );
    let kept: Vec<_> = std::fs::read_dir(cache.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|x| x == al_testrun::baseline_cache::EXTENSION)
        })
        .collect();
    assert!(kept.is_empty(), "no install baseline is kept with --files-dir");
}

/// With `--files-dir` the install code of the apps under test does not run in a session of its own: each test codeunit's
/// session runs it, and `--timings` names the reason (al-testrun `install_once.rs`).
#[test]
fn the_install_code_runs_per_codeunit_on_a_real_folder_and_timings_says_why() {
    files_dir();
    let src = "codeunit 50201 Inst { Subtype = Install; trigger OnInstallAppPerCompany() begin end; }
        codeunit 60201 T1 { Subtype = Test; [Test] procedure A() begin end; }
        codeunit 60202 T2 { Subtype = Test; [Test] procedure B() begin end; }";
    let rt = CorpusRuntime::try_run_suites_with(
        vec![app("own", &[], src)],
        vec![al_sema::AppKind::Workspace],
        &[manifest("Me")],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    )
    .expect("the run starts");
    assert_eq!(rt.phases.install_runs, 2);
    let lines = rt.phases.baseline_line().unwrap_or_default();
    assert!(
        lines.lines().any(|l| l
            == "install code: ran 2 times for the test codeunits; first reason noted that its run cannot be shared: files on a real folder"),
        "{lines}"
    );
}
