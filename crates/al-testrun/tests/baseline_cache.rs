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
        logo: None,
        description: None,
        layout_files: Vec::new(),
        id_ranges: Vec::new(),
        translations: false,
        translation_files: Vec::new(),
        runtime: None,
        app_json_keys: Vec::new(),
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

/// Issue 5635: the install code can read the workspace's resource files, which are not compiled (a warm server hands
/// it new ones without a new compile), so they are part of the key.
#[test]
fn other_resource_files_of_the_workspace_get_a_baseline_of_their_own() {
    let run_with_resource = |dir: &Path, active: &str| {
        // The same source both times: only the resource differs.
        let main = r#"
codeunit 60102 ResourceInstaller
{
    Subtype = Install;
    trigger OnInstallAppPerCompany() var S: Record Setting; begin S.Code := 'RES'; S.Value := NavApp.GetResourceAsText('active.txt'); S.Insert(); end;
}
codeunit 60103 ResourceTests
{
    Subtype = Test;
    [Test] procedure ReadsTheInstallRow() var S: Record Setting; begin S.Get('RES'); if S.Value <> NavApp.GetResourceAsText('active.txt') then Error('got ' + S.Value); end;
}
"#
        .to_string();
        let manifest = |p: &str, resources: Vec<(String, Vec<u8>)>| Manifest {
            publisher: p.into(),
            version: "1.0.0.0".into(),
            resources,
            include_source: false,
            target: String::new(),
            logo: None,
            description: None,
            layout_files: Vec::new(),
            id_ranges: Vec::new(),
            translations: false,
            translation_files: Vec::new(),
            runtime: None,
            app_json_keys: Vec::new(),
            symbols: vec![],
        };
        CorpusRuntime::try_run_suites_with(
            vec![app("main", &["base"], main), app("base", &[], base("yes"))],
            vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
            &[
                manifest("Me", vec![("active.txt".into(), active.as_bytes().to_vec())]),
                manifest("Microsoft", vec![]),
            ],
            &Default::default(),
            None,
            Default::default(),
            &[SuiteRun::everything()],
            &RunOptions {
                compile_cache: Some(CompileCache {
                    dir: dir.to_path_buf(),
                    require: false,
                    rebuild: false,
                    country: String::new(),
                }),
                ..Default::default()
            },
        )
        .expect("the run starts")
    };
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(outcome(&run_with_resource(dir.path(), "M0001")), Outcome::Pass);
    assert_eq!(
        outcome(&run_with_resource(dir.path(), "M0002")),
        Outcome::Pass,
        "the row is the one the install code wrote from the new resource"
    );
    assert_eq!(files(dir.path()).len(), 2, "the resources are part of the key");
}

// The login code's WorkDate (al-testrun `install::login_work_date`) is looked up once per kept baseline: the first run
// records it in the baseline's file, and a later run of the same program over the same baseline replays it without
// running Base Application's `GetDefaultWorkDate` (al-runner issue 5884). Codeunit 40 stands in for Base Application's
// LogInManagement; its `LogInWorkDate` global is read later by Base Application's LogInEnd, so the replay keeps it.
const LOGIN_BASE: &str = r#"
table 50110 Entry { fields { field(1; No; Integer) { } field(2; D; Date) { } } keys { key(PK; No) { } } }
codeunit 50111 Installer
{
    Subtype = Install;
    trigger OnInstallAppPerCompany() var E: Record Entry; begin E.No := 1; E.D := 20240115D; E.Insert(); end;
}
codeunit 40 LogInManagement
{
    SingleInstance = true;
    var LogInWorkDate: Date;
    procedure GetDefaultWorkDate(): Date var E: Record Entry; begin
        if E.FindLast() then begin LogInWorkDate := NormalDate(E.D); exit(NormalDate(E.D)); end;
        exit(WorkDate()); end;
    procedure LoginDate(): Date begin exit(LogInWorkDate); end;
}
"#;

const LOGIN_TESTS: &str = r#"
codeunit 60111 LoginTests
{
    Subtype = Test;
    [Test] procedure SeesTheLoginDate() var L: Codeunit LogInManagement; begin
        if WorkDate() <> 20240115D then Error('WorkDate %1', WorkDate());
        if L.LoginDate() <> 20240115D then Error('LogInWorkDate %1', L.LoginDate()); end;
}
"#;

fn login_run(cache: &Path) -> Outcome {
    let manifest = |p: &str| Manifest {
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
        translation_files: Vec::new(),
        runtime: None,
        app_json_keys: Vec::new(),
        symbols: vec![],
    };
    let rt = CorpusRuntime::try_run_suites_with(
        vec![
            app("main", &["base"], LOGIN_TESTS.into()),
            app("base", &[], LOGIN_BASE.into()),
        ],
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
        .find(|r| r.test == "SeesTheLoginDate")
        .map(|r| r.outcome.clone())
        .expect("the test ran")
}

#[test]
fn the_login_work_date_is_kept_with_the_baseline_and_replayed_by_the_next_run() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(login_run(dir.path()), Outcome::Pass);
    let kept = files(dir.path());
    assert_eq!(kept.len(), 1);
    let first = std::fs::read(&kept[0]).unwrap();
    let (run, at) = al_testrun::baseline_cache::decode(&first).unwrap();
    assert_eq!(
        run.login.len(),
        1,
        "the first run records the login code's answer in the file"
    );
    // The next run takes the date from the file: a record changed to another date shows in what the test sees, so the
    // login code did not run.
    let (mut changed, _) = al_testrun::baseline_cache::decode(&first).unwrap();
    changed.login[0].day += 1;
    for g in &mut changed.login[0].globals {
        g.1 += 1;
    }
    std::fs::write(&kept[0], al_testrun::baseline_cache::encode(&changed, at).unwrap()).unwrap();
    let second = login_run(dir.path());
    assert!(
        matches!(&second, Outcome::Fail(m) if m.contains("WorkDate")),
        "the replayed date is the file's: {second:?}"
    );
    // Put back as the first run left it, the next run passes and reads the file without writing it.
    std::fs::write(&kept[0], &first).unwrap();
    let before = file_id(&kept[0]);
    assert_eq!(login_run(dir.path()), Outcome::Pass);
    assert_eq!(file_id(&kept[0]), before, "read, not written");
}

/// A run of the dependency `base` and a workspace app of `main_src`, with `opts` and the compile cache in `cache`.
fn run_main(main_src: &str, cache: &Path, dotnet: al_vm::dotnet::DotNetConfig, opts: RunOptions) -> CorpusRuntime {
    run_apps(
        vec![
            app("main", &["base"], main_src.to_string()),
            app("base", &[], base("yes")),
        ],
        cache,
        dotnet,
        opts,
    )
}

fn run_apps(
    apps: Vec<AppSource>,
    cache: &Path,
    dotnet: al_vm::dotnet::DotNetConfig,
    opts: RunOptions,
) -> CorpusRuntime {
    let manifest = |p: &str| Manifest {
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
        translation_files: Vec::new(),
        runtime: None,
        app_json_keys: Vec::new(),
        symbols: vec![],
    };
    CorpusRuntime::try_run_suites_with(
        apps,
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[manifest("Me"), manifest("Microsoft")],
        &Default::default(),
        None,
        dotnet,
        &[SuiteRun::everything()],
        &RunOptions {
            compile_cache: Some(CompileCache {
                dir: cache.to_path_buf(),
                require: false,
                rebuild: false,
                country: String::new(),
            }),
            ..opts
        },
    )
    .expect("the run starts")
}

fn outcome_of(rt: &CorpusRuntime, test: &str) -> Outcome {
    rt.results()
        .iter()
        .find(|r| r.test == test)
        .map(|r| r.outcome.clone())
        .unwrap_or_else(|| panic!("the test {test} ran"))
}

// Nothing a test writes reaches the file: not its rows, not its number sequences, and not what the login code would
// record over the database the test changed. The second run starts from what the install code left, as the first did.
const WRITES_BASE: &str = r#"
table 50120 Setting { fields { field(1; Code; Code[20]) { } field(2; Value; Text[50]) { } } keys { key(PK; Code) { } } }
table 50121 Entry { fields { field(1; No; Integer) { } field(2; D; Date) { } } keys { key(PK; No) { } } }
codeunit 50122 Installer
{
    Subtype = Install;
    trigger OnInstallAppPerCompany() var S: Record Setting; E: Record Entry; begin
        S.Code := 'SEEDED'; S.Value := 'installed'; S.Insert();
        E.No := 1; E.D := 20240115D; E.Insert(); end;
}
codeunit 40 LogInManagement
{
    SingleInstance = true;
    procedure GetDefaultWorkDate(): Date var E: Record Entry; begin
        if E.FindLast() then exit(NormalDate(E.D));
        exit(WorkDate()); end;
}
"#;

const WRITES_TESTS: &str = r#"
codeunit 60120 WritingTests
{
    Subtype = Test;
    [Test] procedure WritesEverything() var S: Record Setting; E: Record Entry; begin
        if S.Get('TESTROW') then Error('a row a test of an earlier run wrote: %1', S.Value);
        if not S.Get('SEEDED') then Error('the install row is missing');
        if NumberSequence.Exists('TESTSEQUENCE') then Error('a number sequence of an earlier run');
        if WorkDate() <> 20240115D then Error('WorkDate %1', WorkDate());
        S.Code := 'TESTROW'; S.Value := 'TESTVALUE'; S.Insert();
        S.Get('SEEDED'); S.Value := 'TESTCHANGED'; S.Modify();
        NumberSequence.Insert('TESTSEQUENCE', 1000, 1);
        if NumberSequence.Next('TESTSEQUENCE') <> 1000 then Error('the sequence');
        E.No := 2; E.D := 20300101D; E.Insert(); end;
}
"#;

#[test]
fn nothing_a_test_writes_reaches_the_kept_baseline() {
    let dir = tempfile::tempdir().unwrap();
    let go = || {
        run_apps(
            vec![
                app("main", &["base"], WRITES_TESTS.into()),
                app("base", &[], WRITES_BASE.into()),
            ],
            dir.path(),
            Default::default(),
            Default::default(),
        )
    };
    assert_eq!(outcome_of(&go(), "WritesEverything"), Outcome::Pass);
    let kept = files(dir.path());
    assert_eq!(kept.len(), 1, "the first run keeps its baseline");
    let first = std::fs::read(&kept[0]).unwrap();
    // The second run reads the file and its test sees none of what the first run's test wrote.
    assert_eq!(outcome_of(&go(), "WritesEverything"), Outcome::Pass);
    assert_eq!(files(dir.path()), kept);
    let second = std::fs::read(&kept[0]).unwrap();
    for (bytes, which) in [(&first, "first"), (&second, "second")] {
        for text in ["TESTROW", "TESTVALUE", "TESTCHANGED", "TESTSEQUENCE"] {
            assert!(
                !bytes.windows(text.len()).any(|w| w == text.as_bytes()),
                "{text} is in the file after the {which} run"
            );
        }
    }
    // The login code's record is the one over the install code's data (2024-01-15), not over the test's entry.
    let (run, _) = al_testrun::baseline_cache::decode(&second).unwrap();
    let (run1, _) = al_testrun::baseline_cache::decode(&first).unwrap();
    assert_eq!(run.login, run1.login, "the login records did not change");
    assert!(run.login.len() <= 1);
}

/// A .NET host whose answer to `Value()` is set by the test: what an assembly of `--dotnet-assembly-dir`, a file, an
/// environment variable or the machine's clock would answer, none of which is in the key.
#[derive(Clone)]
struct Answering(Arc<std::sync::Mutex<String>>);

struct AnsweringSession(Arc<std::sync::Mutex<String>>);

impl al_host::dotnet::DotNetSession for AnsweringSession {
    fn construct(
        &mut self,
        _: &al_host::dotnet::DotNetType,
        _: Vec<al_host::dotnet::DotNetArg>,
    ) -> Result<al_host::dotnet::DotNetReply, al_host::dotnet::DotNetFailure> {
        Err(al_host::dotnet::DotNetFailure::Refused("not served".into()))
    }
    fn invoke(
        &mut self,
        _: al_host::dotnet::DotNetTarget,
        member: &str,
        _: Vec<al_host::dotnet::DotNetArg>,
    ) -> Result<al_host::dotnet::DotNetReply, al_host::dotnet::DotNetFailure> {
        match member {
            "Value" => Ok(al_host::dotnet::DotNetReply::value(al_host::dotnet::DotNetValue::Str(
                self.0.lock().unwrap().clone(),
            ))),
            _ => Err(al_host::dotnet::DotNetFailure::Refused("not served".into())),
        }
    }
    fn get_property(
        &mut self,
        _: al_host::dotnet::DotNetTarget,
        _: &str,
    ) -> Result<al_host::dotnet::DotNetReply, al_host::dotnet::DotNetFailure> {
        Err(al_host::dotnet::DotNetFailure::Refused("not served".into()))
    }
    fn set_property(
        &mut self,
        _: al_host::dotnet::DotNetTarget,
        _: &str,
        _: al_host::dotnet::DotNetValue,
    ) -> Result<al_host::dotnet::DotNetReply, al_host::dotnet::DotNetFailure> {
        Err(al_host::dotnet::DotNetFailure::Refused("not served".into()))
    }
    fn release(&mut self, _: &[al_host::dotnet::DotNetHandle]) {}
    fn close(self: Box<Self>) {}
}

impl al_host::dotnet::DotNetHost for Answering {
    fn availability(&self) -> Result<(), al_host::dotnet::DotNetUnavailable> {
        Ok(())
    }
    fn open_session(&self, _: &al_host::dotnet::DotNetSessionConfig) -> Box<dyn al_host::dotnet::DotNetSession> {
        Box::new(AnsweringSession(self.0.clone()))
    }
}

const DOTNET_MAIN: &str = r#"
dotnet { assembly("MyAssembly") { type("MyCompany.Settings"; "MySettings") { } } }
codeunit 60130 DotNetInstaller
{
    Subtype = Install;
    trigger OnInstallAppPerCompany() var S: Record Setting; M: DotNet MySettings; begin
        S.Code := 'NET'; S.Value := M.Value(); S.Insert(); end;
}
codeunit 60131 DotNetTests
{
    Subtype = Test;
    [Test] procedure ReadsTheInstallRow() var S: Record Setting; M: DotNet MySettings; begin
        S.Get('NET'); if S.Value <> M.Value() then Error('got ' + S.Value); end;
}
"#;

#[test]
fn a_baseline_whose_install_code_asked_dotnet_is_not_kept() {
    let dir = tempfile::tempdir().unwrap();
    let answer = Arc::new(std::sync::Mutex::new("one".to_string()));
    let go = || {
        run_main(
            DOTNET_MAIN,
            dir.path(),
            al_vm::dotnet::DotNetConfig {
                host: Arc::new(Answering(answer.clone())),
                ..Default::default()
            },
            Default::default(),
        )
    };
    assert_eq!(outcome_of(&go(), "ReadsTheInstallRow"), Outcome::Pass);
    // .NET answers something else now (another file, another assembly): the install code must ask it again.
    *answer.lock().unwrap() = "two".to_string();
    assert_eq!(
        outcome_of(&go(), "ReadsTheInstallRow"),
        Outcome::Pass,
        "the row holds what .NET answers in this run"
    );
    assert!(files(dir.path()).is_empty(), "nothing kept: {:?}", files(dir.path()));
}

const TDD_MAIN: &str = r#"
codeunit 60140 Helper { procedure There() begin end; }
codeunit 60141 TddInstaller
{
    Subtype = Install;
    trigger OnInstallAppPerCompany() var S: Record Setting; H: Codeunit Helper; begin
        H.NotThere(); S.Code := 'TDD'; S.Value := 'stubbed'; S.Insert(); end;
}
codeunit 60142 TddTests
{
    Subtype = Test;
    [Test] procedure SeesTheInstallRow() var S: Record Setting; begin if not S.Get('TDD') then Error('no row'); end;
    [Test] procedure SeesNoInstallRow() var S: Record Setting; begin if S.Get('TDD') then Error('a row'); end;
}
"#;

#[test]
fn a_run_with_tdd_gets_a_baseline_of_its_own() {
    let dir = tempfile::tempdir().unwrap();
    let go = |tdd: bool| {
        run_main(
            TDD_MAIN,
            dir.path(),
            Default::default(),
            RunOptions {
                tdd,
                ..Default::default()
            },
        )
    };
    // With `--tdd` the missing procedure is a stub and the install code runs to its end; without, it does not run.
    let with = go(true);
    assert_eq!(outcome_of(&with, "SeesTheInstallRow"), Outcome::Pass);
    // Without, the install routine does not compile, and a test of the table it writes is held back. A run that took
    // the baseline of the run with `--tdd` would find the row.
    let without = go(false);
    let held = outcome_of(&without, "SeesNoInstallRow");
    assert!(
        matches!(&held, Outcome::NotRun(why) if why.contains("could not run")),
        "the baseline of the run with --tdd is not taken: {held:?}"
    );
    assert_eq!(files(dir.path()).len(), 2, "--tdd is part of the key");
}

const ROW_TESTS: &str = r#"
codeunit 60150 RowTests
{
    Subtype = Test;
    [Test] procedure Row() var S: Record Setting; begin if not S.Get('ROW') then Error('no row'); Error('row: %1', S.Value); end;
}
"#;

/// What the test `Row` says: `no row`, or the value of the row the baseline code wrote.
fn row(rt: &CorpusRuntime) -> String {
    match outcome_of(rt, "Row") {
        Outcome::Fail(m) => m,
        other => format!("{other:?}"),
    }
}

#[test]
fn other_skipped_dependency_apps_get_a_baseline_of_their_own() {
    // A run of an object id no loaded app declares raises the missing-object error, unless a skipped app may declare it:
    // then it is refused (`al_vm::Engine::skipped_apps`). What the install code leaves differs, so the key does.
    let base = r#"
table 50100 Setting { fields { field(1; Code; Code[20]) { } field(2; Value; Text[50]) { } } keys { key(PK; Code) { } } }
codeunit 50151 Installer
{
    Subtype = Install;
    trigger OnInstallAppPerCompany() begin Codeunit.Run(70000); end;
}
"#;
    let dir = tempfile::tempdir().unwrap();
    let go = |skipped: Vec<String>| {
        run_apps(
            vec![app("main", &["base"], ROW_TESTS.into()), app("base", &[], base.into())],
            dir.path(),
            Default::default(),
            RunOptions {
                skipped_dependencies: skipped,
                ..Default::default()
            },
        )
    };
    go(Vec::new());
    go(vec!["`Other` by Someone".into()]);
    assert_eq!(files(dir.path()).len(), 2, "the skipped apps are part of the key");
}

#[test]
fn another_setting_of_the_company_build_gets_a_baseline_of_its_own() {
    let base = r#"
table 50100 Setting { fields { field(1; Code; Code[20]) { } field(2; Value; Text[50]) { } } keys { key(PK; Code) { } } }
codeunit 50152 "Build Company"
{
    trigger OnRun() var S: Record Setting; begin S.Code := 'ROW'; S.Value := 'gui ' + Format(GuiAllowed()); S.Insert(); end;
}
"#;
    let dir = tempfile::tempdir().unwrap();
    let go = |no_client: bool| {
        run_apps(
            vec![app("main", &["base"], ROW_TESTS.into()), app("base", &[], base.into())],
            dir.path(),
            Default::default(),
            RunOptions {
                company_data: Some(al_testrun::company_data::CompanyData {
                    codeunit: 50152,
                    name: "Build Company".into(),
                    resources: None,
                    // The same app and files: only how the company is built differs.
                    identity: "the same".into(),
                    environment: None,
                    installed_first: None,
                    method: None,
                    more_steps: Vec::new(),
                    no_client,
                    evaluation_company: false,
                }),
                ..Default::default()
            },
        )
    };
    assert!(row(&go(false)).contains("row: gui"));
    assert!(row(&go(true)).contains("row: gui"));
    assert_eq!(
        files(dir.path()).len(),
        2,
        "every setting of the company build is part of the key"
    );
}
