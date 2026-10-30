//! A warm server keeps the install baseline across an edit the install code cannot reach (the edit only changes code
//! no install root, `Company-Initialize` subscriber or anything they call can enter), and runs the install code again
//! after any edit it can reach. Either way a request has the outcomes a fresh run of the same texts has.

use al_sema::workspace::AppSource;
use al_testrun::compile_cache::CompileCache;
use al_testrun::runtime::{CorpusRuntime, FileChange, Manifest, RunOptions, SuiteRun, Warm, WarmUse};

/// A dependency app: a table the install code fills, its own install code, and a `Company-Initialize` that raises an
/// event the workspace subscribes to.
const DEP: &str = r#"
table 50100 Seeded { fields { field(1; K; Code[20]) { } field(2; N; Integer) { } } keys { key(PK; K) { } } }
codeunit 50102 DepInstall
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        S: Record Seeded;
    begin
        S.K := 'DEP';
        S.N := 1;
        S.Insert();
    end;
}
codeunit 2 "Company-Initialize"
{
    trigger OnRun()
    begin
        OnCompanyInitialize();
    end;

    [IntegrationEvent(false, false)]
    local procedure OnCompanyInitialize()
    begin
    end;
}
"#;

const INSTALL: &str = r#"codeunit 60200 WsInstall
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        S: Record Seeded;
        H: Codeunit Helper;
        E: Record Extra;
    begin
        S.K := 'WS';
        S.N := H.Value();
        S.Insert();
        E.K := 'E';
        E.Insert();
    end;
}

codeunit 60201 Helper
{
    procedure Value(): Integer
    begin
        exit(2);
    end;
}
"#;

const SUB: &str = r#"codeunit 60202 Sub
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Company-Initialize", 'OnCompanyInitialize', '', false, false)]
    local procedure OnInit()
    var
        S: Record Seeded;
    begin
        S.K := 'SUB';
        S.N := 7;
        S.Insert();
    end;
}
"#;

const TABLES: &str = r#"table 60210 Extra
{
    fields
    {
        field(1; K; Code[20]) { }
        field(2; N; Integer) { InitValue = 4; }
    }
    keys { key(PK; K) { } }
}
"#;

const LIB: &str = r#"codeunit 60203 TestLib
{
    procedure Prefix(): Text
    begin
        exit('v');
    end;
}
"#;

const TESTS: &str = r#"codeunit 60300 Reads
{
    Subtype = Test;

    [Test]
    procedure ShowsTheInstallRows()
    var
        S: Record Seeded;
        T: Record Seeded;
        D: Record Seeded;
        E: Record Extra;
        L: Codeunit TestLib;
    begin
        D.Get('DEP');
        S.Get('WS');
        T.Get('SUB');
        E.Get('E');
        Error('%1 %2 %3 %4 %5', L.Prefix(), D.N, S.N, T.N, E.N);
    end;
}
"#;

const NAMES: [&str; 5] = ["Install.al", "Sub.al", "Tables.al", "Lib.al", "Tests.al"];

fn texts() -> [String; 5] {
    [INSTALL, SUB, TABLES, LIB, TESTS].map(String::from)
}

fn manifest(publisher: &str) -> Manifest {
    Manifest {
        publisher: publisher.to_string(),
        version: "1.0.0.0".into(),
        resources: vec![],
        include_source: false,
        target: String::new(),
        logo: None,
        layout_files: Vec::new(),
        id_ranges: Vec::new(),
        translations: false,
        translation_files: Vec::new(),
        symbols: vec![],
    }
}

fn build(t: &[String; 5]) -> (Vec<AppSource>, Vec<al_sema::AppKind>, Vec<Manifest>) {
    (
        vec![
            AppSource {
                id: "main".into(),
                name: "main".into(),
                dependencies: vec!["dep".into()],
                files: NAMES.iter().zip(t).map(|(n, t)| (n.to_string(), t.clone())).collect(),
            },
            AppSource {
                id: "dep".into(),
                name: "dep".into(),
                dependencies: vec![],
                files: vec![("Dep.al".into(), DEP.into())],
            },
        ],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        vec![manifest("Me"), manifest("Someone")],
    )
}

fn opts(dir: &std::path::Path) -> RunOptions {
    RunOptions {
        compile_cache: Some(CompileCache {
            dir: dir.to_path_buf(),
            require: false,
            rebuild: false,
            country: String::new(),
        }),
        ..Default::default()
    }
}

fn outcomes(rt: &CorpusRuntime) -> Vec<String> {
    let mut v: Vec<String> = rt
        .results()
        .iter()
        .map(|r| format!("{} {} {:?}", r.codeunit, r.test, r.outcome))
        .collect();
    v.sort();
    v
}

/// One request of the warm server: what it did about the compile, its outcomes and where its baseline came from.
fn request(warm: &mut Warm, dir: &std::path::Path, t: &[String; 5]) -> (WarmUse, Vec<String>, String) {
    let changes: Vec<FileChange> = NAMES
        .iter()
        .zip(t)
        .map(|(p, t)| FileChange {
            app: 0,
            path: p,
            text: t,
        })
        .collect();
    let o = opts(dir);
    let (rt, used) = warm
        .run(
            "k",
            &changes,
            || build(t),
            &Default::default(),
            None,
            Default::default(),
            &[SuiteRun::everything()],
            &o,
        )
        .unwrap();
    (used, outcomes(&rt), format!("{:?}", rt.phases.baseline_source))
}

/// The outcomes of a run of its own over a fresh compile cache.
fn fresh(t: &[String; 5]) -> Vec<String> {
    let dir = tempfile::tempdir().unwrap();
    let (apps, kinds, m) = build(t);
    let o = opts(dir.path());
    let c = CorpusRuntime::compile_suites(
        apps,
        kinds,
        &m,
        &Default::default(),
        None,
        &[SuiteRun::everything()],
        &o,
    )
    .unwrap();
    outcomes(&CorpusRuntime::run_compiled(&c, None, Default::default(), &o))
}

fn edited(file: usize, from: &str, to: &str) -> [String; 5] {
    let mut t = texts();
    assert!(t[file].contains(from), "{from}");
    t[file] = t[file].replacen(from, to, 1);
    t
}

/// A warm server's first request over the original texts, then one over `t`: (what the second did, its outcomes,
/// where its baseline came from), checked against a fresh run of `t`.
fn after_edit(t: &[String; 5]) -> (WarmUse, Vec<String>, String) {
    let dir = tempfile::tempdir().unwrap();
    let mut warm = Warm::new();
    let first = request(&mut warm, dir.path(), &texts());
    assert_eq!(first.2, "Some(\"ran\")", "{first:?}");
    assert!(first.1[0].contains("v 1 2 7 4"), "{first:?}");
    let second = request(&mut warm, dir.path(), t);
    assert_eq!(
        second.1,
        fresh(t),
        "a kept or rerun baseline gives the outcomes of a fresh run: {second:?}"
    );
    second
}

#[test]
fn an_edit_of_a_test_method_keeps_the_baseline() {
    let (used, out, source) = after_edit(&edited(4, "Error('%1", "Error('edited %1"));
    assert_eq!(used, WarmUse::Updated);
    assert!(out[0].contains("edited v 1 2 7 4"), "{out:?}");
    assert_eq!(source, "Some(\"kept\")");
}

#[test]
fn an_edit_of_code_only_the_tests_call_keeps_the_baseline() {
    let (used, out, source) = after_edit(&edited(3, "exit('v')", "exit('w')"));
    assert_eq!(used, WarmUse::Updated);
    assert!(out[0].contains("w 1 2 7 4"), "{out:?}");
    assert_eq!(source, "Some(\"kept\")");
}

#[test]
fn a_kept_baseline_survives_several_edits_in_a_row() {
    let dir = tempfile::tempdir().unwrap();
    let mut warm = Warm::new();
    request(&mut warm, dir.path(), &texts());
    for i in 0..3 {
        let t = edited(4, "Error('%1", &format!("Error('e{i} %1"));
        let (used, out, source) = request(&mut warm, dir.path(), &t);
        assert_eq!((used, source.as_str()), (WarmUse::Updated, "Some(\"kept\")"), "{out:?}");
        assert_eq!(out, fresh(&t));
    }
}

#[test]
fn an_edit_of_an_install_codeunit_runs_the_install_code_again() {
    let (used, out, source) = after_edit(&edited(0, "E.K := 'E';", "E.K := 'E'; E.N := 9;"));
    assert_eq!(used, WarmUse::Updated);
    assert!(out[0].contains("v 1 2 7 9"), "{out:?}");
    assert_eq!(source, "Some(\"ran\")");
}

#[test]
fn an_edit_of_code_the_install_code_calls_runs_it_again() {
    let (used, out, source) = after_edit(&edited(0, "exit(2);", "exit(3);"));
    assert_eq!(used, WarmUse::Updated);
    assert!(out[0].contains("v 1 3 7 4"), "{out:?}");
    assert_eq!(source, "Some(\"ran\")");
}

#[test]
fn an_edit_of_a_company_initialize_subscriber_runs_the_install_code_again() {
    let (used, out, source) = after_edit(&edited(1, "S.N := 7;", "S.N := 8;"));
    assert_eq!(used, WarmUse::Updated);
    assert!(out[0].contains("v 1 2 8 4"), "{out:?}");
    assert_eq!(source, "Some(\"ran\")");
}

#[test]
fn an_edit_of_a_table_field_the_install_code_uses_runs_it_again() {
    let (_, out, source) = after_edit(&edited(2, "InitValue = 4;", "InitValue = 5;"));
    assert!(out[0].contains("v 1 2 7 5"), "{out:?}");
    assert_eq!(source, "Some(\"ran\")");
}
