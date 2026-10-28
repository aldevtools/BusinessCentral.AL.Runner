//! `--files-dir` gives the `File` type a real directory (`host::set_files_dir`, set once per process, so this test has
//! a binary of its own). What the install code reads from there is not in the key of the kept install baseline, so a
//! run with it keeps no baseline: a changed file is read again by the next run.

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

fn run(cache: &Path) -> String {
    let manifest = |p: &str| Manifest {
        publisher: p.into(),
        version: "1.0.0.0".into(),
        resources: vec![],
        include_source: false,
        target: String::new(),
        layout_files: Vec::new(),
        id_ranges: Vec::new(),
        translations: false,
        translation_files: Vec::new(),
        symbols: vec![],
    };
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
    let files = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    al_testrun::host::set_files_dir(files.path().to_path_buf());
    std::fs::write(files.path().join("setting.txt"), "first").unwrap();
    let first = run(cache.path());
    assert!(first.contains("row: first"), "{first}");
    std::fs::write(files.path().join("setting.txt"), "second").unwrap();
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
