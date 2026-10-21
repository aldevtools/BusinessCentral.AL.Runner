//! The install baseline kept on disk next to the compile cache (`baseline_cache.rs`): the second run of the same
//! program reads it instead of running the install code, and anything that could make the baseline another one
//! makes another file, so a test never sees data from a program that is not the one it runs against.

use al_host::testdata::{Cell, TableError, TableProvider, TableRequest, TableRows, TestData};
use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::compile_cache::CompileCache;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SuiteRun};
use std::path::Path;
use std::sync::Arc;

fn base(value: &str) -> String {
    format!(
        r#"
table 50100 Setting {{ fields {{ field(1; Code; Code[20]) {{ }} field(2; Value; Text[50]) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
codeunit 50101 Installer
{{
    Subtype = Install;
    trigger OnInstallAppPerCompany() var S: Record Setting; begin if S.Get('SEEDED') then begin S.Value := '{value}'; S.Modify(); end else begin S.Code := 'SEEDED'; S.Value := '{value}'; S.Insert(); end; end;
}}
"#
    )
}

fn main_app(expected: &str) -> String {
    format!(
        r#"
codeunit 60101 DataTests
{{
    Subtype = Test;
    [Test] procedure ReadsTheInstallRow() var S: Record Setting; begin S.Get('SEEDED'); if S.Value <> '{expected}' then Error('got ' + S.Value); end;
}}
"#
    )
}

fn app(id: &str, deps: &[&str], src: String) -> AppSource {
    AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src)],
    }
}

fn run(installed: &str, expected: &str, cache: Option<&Path>) -> CorpusRuntime {
    run_with_symbols(installed, expected, cache, &Default::default())
}

fn run_with_symbols(
    installed: &str,
    expected: &str,
    cache: Option<&Path>,
    symbols: &std::collections::BTreeSet<String>,
) -> CorpusRuntime {
    run_full(
        installed,
        expected,
        cache.map(|d| (d, false)),
        symbols,
        None,
        Default::default(),
    )
}

fn run_full(
    installed: &str,
    expected: &str,
    cache: Option<(&Path, bool)>,
    symbols: &std::collections::BTreeSet<String>,
    test_data: Option<TestData>,
    dotnet: al_vm::dotnet::DotNetConfig,
) -> CorpusRuntime {
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
    CorpusRuntime::try_run_suites_with(
        vec![
            app("main", &["base"], main_app(expected)),
            app("base", &[], base(installed)),
        ],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[manifest("Me"), manifest("Microsoft")],
        symbols,
        None,
        dotnet,
        &[SuiteRun::everything()],
        &RunOptions {
            test_data,
            compile_cache: cache.map(|(dir, rebuild)| CompileCache {
                dir: dir.to_path_buf(),
                require: false,
                rebuild,
                country: String::new(),
            }),
            ..Default::default()
        },
    )
    .expect("the run starts")
}

fn outcome(rt: &CorpusRuntime) -> Outcome {
    rt.results()
        .iter()
        .find(|r| r.test == "ReadsTheInstallRow")
        .map(|r| r.outcome.clone())
        .expect("the test ran")
}

/// Which file is there: a read leaves it (and marks it used), a write puts a new one in its place.
fn file_id(path: &Path) -> u64 {
    let meta = std::fs::metadata(path).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        meta.ino()
    }
    #[cfg(not(unix))]
    {
        meta.len()
    }
}

fn files(cache: &Path) -> Vec<std::path::PathBuf> {
    let mut v: Vec<_> = std::fs::read_dir(cache)
        .map(|d| {
            d.filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    p.extension()
                        .is_some_and(|x| x == al_testrun::baseline_cache::EXTENSION)
                })
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

#[test]
fn the_second_run_of_a_program_reads_the_baseline_the_first_one_kept() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(outcome(&run("yes", "yes", Some(dir.path()))), Outcome::Pass);
    let kept = files(dir.path());
    assert_eq!(kept.len(), 1, "the first run keeps its baseline");
    let (bytes, modified) = (std::fs::read(&kept[0]).unwrap(), file_id(&kept[0]));
    assert_eq!(outcome(&run("yes", "yes", Some(dir.path()))), Outcome::Pass);
    assert_eq!(files(dir.path()), kept, "no second file");
    assert_eq!(
        std::fs::read(&kept[0]).unwrap(),
        bytes,
        "the file is read, not written again"
    );
    assert_eq!(file_id(&kept[0]), modified);
}

#[test]
fn a_changed_install_codeunit_does_not_get_the_old_baseline() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(outcome(&run("yes", "yes", Some(dir.path()))), Outcome::Pass);
    // The same test against an install code that writes another value. The baseline of the first run holds `yes`: a
    // run that took it would pass.
    assert!(
        matches!(outcome(&run("no", "yes", Some(dir.path()))), Outcome::Fail(_)),
        "the row is the one the new install code wrote"
    );
    assert_eq!(files(dir.path()).len(), 2, "another program, another file");
    // The first program still finds its own.
    assert_eq!(outcome(&run("yes", "yes", Some(dir.path()))), Outcome::Pass);
    assert_eq!(files(dir.path()).len(), 2);
}

#[test]
fn a_changed_workspace_app_gets_a_baseline_of_its_own() {
    let dir = tempfile::tempdir().unwrap();
    run("yes", "yes", Some(dir.path()));
    run("yes", "other", Some(dir.path()));
    assert_eq!(files(dir.path()).len(), 2, "the workspace's source is part of the key");
}

#[test]
fn a_run_without_the_compile_cache_keeps_nothing() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(outcome(&run("yes", "yes", None)), Outcome::Pass);
    assert!(files(dir.path()).is_empty());
}

#[test]
fn other_preprocessor_symbols_of_the_workspace_get_a_baseline_of_their_own() {
    // The symbols select the code that is compiled, and the workspace's install codeunits are part of the baseline.
    let dir = tempfile::tempdir().unwrap();
    let none = std::collections::BTreeSet::new();
    let some: std::collections::BTreeSet<String> = ["SPECIAL".to_string()].into_iter().collect();
    run_with_symbols("yes", "yes", Some(dir.path()), &none);
    run_with_symbols("yes", "yes", Some(dir.path()), &some);
    assert_eq!(files(dir.path()).len(), 2, "the symbols are part of the key");
}

/// A usable .NET host that nothing calls: the run has .NET, as every corpus run does.
struct HasDotNet;

impl al_host::dotnet::DotNetHost for HasDotNet {
    fn availability(&self) -> Result<(), al_host::dotnet::DotNetUnavailable> {
        Ok(())
    }
    fn open_session(&self, cfg: &al_host::dotnet::DotNetSessionConfig) -> Box<dyn al_host::dotnet::DotNetSession> {
        al_host::dotnet::NoDotNet.open_session(cfg)
    }
}

fn with_dotnet() -> al_vm::dotnet::DotNetConfig {
    al_vm::dotnet::DotNetConfig {
        host: Arc::new(HasDotNet),
        ..Default::default()
    }
}

#[test]
fn a_run_with_dotnet_keeps_and_reads_its_baseline_like_any_other() {
    let dir = tempfile::tempdir().unwrap();
    let go = || {
        run_full(
            "yes",
            "yes",
            Some((dir.path(), false)),
            &Default::default(),
            None,
            with_dotnet(),
        )
    };
    assert_eq!(outcome(&go()), Outcome::Pass);
    let kept = files(dir.path());
    assert_eq!(kept.len(), 1, "the first run keeps its baseline");
    let modified = file_id(&kept[0]);
    assert_eq!(outcome(&go()), Outcome::Pass);
    assert_eq!(file_id(&kept[0]), modified, "the second run reads the file");
    // A baseline made with .NET is not the one a run without .NET starts from: another file.
    assert_eq!(outcome(&run("yes", "yes", Some(dir.path()))), Outcome::Pass);
    assert_eq!(files(dir.path()).len(), 2);
}

#[test]
fn the_baseline_sits_next_to_the_layers_and_goes_with_the_flush() {
    let dir = tempfile::tempdir().unwrap();
    run("yes", "yes", Some(dir.path()));
    let names: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(names.iter().any(|n| n.ends_with(".alcc")), "layers: {names:?}");
    assert!(names.iter().any(|n| n.ends_with(".albr")), "baseline: {names:?}");
    let (removed, _) = al_testrun::compile_cache::clear(dir.path());
    assert!(removed >= 2);
    assert!(
        std::fs::read_dir(dir.path()).unwrap().next().is_none(),
        "nothing is left"
    );
}

#[test]
fn a_rebuild_does_not_read_the_file_and_writes_it_again() {
    let dir = tempfile::tempdir().unwrap();
    run("yes", "yes", Some(dir.path()));
    let kept = files(dir.path());
    // Damage what the file holds without making it unreadable: a run that read it would not get "yes".
    let mut run0 = al_testrun::baseline_cache::decode(&std::fs::read(&kept[0]).unwrap()).unwrap();
    run0.0.notes.push("planted".into());
    let planted = al_testrun::baseline_cache::encode(&run0.0, run0.1).unwrap();
    std::fs::write(&kept[0], &planted).unwrap();
    let rebuilt = run_full(
        "yes",
        "yes",
        Some((dir.path(), true)),
        &Default::default(),
        None,
        Default::default(),
    );
    assert_eq!(outcome(&rebuilt), Outcome::Pass);
    let now = std::fs::read(&kept[0]).unwrap();
    assert_ne!(now, planted, "the file was written again, not read");
    let (back, _) = al_testrun::baseline_cache::decode(&now).unwrap();
    assert!(!back.notes.iter().any(|n| n == "planted"));
}

/// A restored database of a few rows. `identity` is what the key holds of it.
struct Backup {
    identity: Option<String>,
}

impl TableProvider for Backup {
    fn company(&self) -> &str {
        "CRONUS"
    }
    fn source(&self) -> String {
        "fake.bak".into()
    }
    fn identity(&self) -> Option<String> {
        self.identity.clone()
    }
    fn read(&self, t: &TableRequest) -> Result<Arc<TableRows>, TableError> {
        match t.name.to_lowercase().as_str() {
            "setting" => Ok(Arc::new(TableRows {
                columns: vec!["Code".into(), "Value".into()],
                rows: vec![
                    vec![Cell::Text("RESTORED".into()), Cell::Text("kept".into())],
                    vec![Cell::Text("SEEDED".into()), Cell::Text("from the backup".into())],
                ],
            })),
            _ => Err(TableError::Absent),
        }
    }
}

fn backup(identity: Option<&str>) -> TestData {
    TestData::new(Arc::new(Backup {
        identity: identity.map(String::from),
    }))
}

#[test]
fn a_run_with_test_data_keeps_its_baseline_and_the_second_run_reads_it() {
    let dir = tempfile::tempdir().unwrap();
    let go = |data: TestData| {
        run_full(
            "installed",
            "installed",
            Some((dir.path(), false)),
            &Default::default(),
            Some(data),
            Default::default(),
        )
    };
    // The install code overwrites the restored row `SEEDED` (the install code of this app modifies, not inserts, over
    // restored data) and the test reads it; the other restored row stays as the backup has it.
    assert_eq!(outcome(&go(backup(Some("backup-1")))), Outcome::Pass);
    let kept = files(dir.path());
    assert_eq!(kept.len(), 1);
    let modified = file_id(&kept[0]);
    assert_eq!(outcome(&go(backup(Some("backup-1")))), Outcome::Pass);
    assert_eq!(files(dir.path()), kept);
    assert_eq!(file_id(&kept[0]), modified, "read, not written");
    // Another backup (a new download, another build) is another baseline.
    assert_eq!(outcome(&go(backup(Some("backup-2")))), Outcome::Pass);
    assert_eq!(files(dir.path()).len(), 2);
    // A source that cannot say what it is gets nothing kept.
    assert_eq!(outcome(&go(backup(None))), Outcome::Pass);
    assert_eq!(files(dir.path()).len(), 2);
}
