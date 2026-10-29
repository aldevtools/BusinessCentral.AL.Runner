//! A warm server keeps the engines and the install baseline of a request for the next one over the same compile
//! (al-runner issue 5856). What a request starts from must not change: every request sees the rows the install code
//! left and none of the rows an earlier request wrote, whatever it selects.

use al_sema::workspace::AppSource;
use al_testrun::compile_cache::CompileCache;
use al_testrun::runtime::{CorpusRuntime, FileChange, Manifest, RunOptions, SuiteRun, Warm};

/// A dependency app whose install code leaves one row, and a number in a second table.
const DEP: &str = r#"
table 50100 Seeded { fields { field(1; K; Code[20]) { } field(2; N; Integer) { } } keys { key(PK; K) { } } }
table 50101 Counter { fields { field(1; K; Code[20]) { } field(2; N; Integer) { } } keys { key(PK; K) { } } }
codeunit 50102 DepInstall
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        S: Record Seeded;
        C: Record Counter;
    begin
        S.K := 'INSTALL';
        S.N := 1;
        S.Insert();
        C.K := 'C';
        C.N := 10;
        C.Insert();
    end;
}
"#;

/// Two test codeunits that each check the install rows, then write: insert, modify and commit.
const TESTS: &str = r#"
codeunit 60100 WritesA
{
    Subtype = Test;

    [Test]
    procedure SeesOnlyTheInstallRows()
    var
        S: Record Seeded;
        C: Record Counter;
    begin
        if S.Count() <> 1 then
            Error('Seeded holds %1 rows', S.Count());
        C.Get('C');
        if C.N <> 10 then
            Error('Counter is %1', C.N);
        S.K := 'A';
        S.Insert();
        C.N += 1;
        C.Modify();
        Commit();
    end;
}

codeunit 60101 WritesB
{
    Subtype = Test;

    [Test]
    procedure SeesOnlyTheInstallRowsToo()
    var
        S: Record Seeded;
        C: Record Counter;
    begin
        if S.Count() <> 1 then
            Error('Seeded holds %1 rows', S.Count());
        C.Get('C');
        if C.N <> 10 then
            Error('Counter is %1', C.N);
        S.K := 'B';
        S.Insert();
        C.N += 5;
        C.Modify();
        Commit();
    end;
}
"#;

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

fn build() -> (Vec<AppSource>, Vec<al_sema::AppKind>, Vec<Manifest>) {
    (
        vec![
            AppSource {
                id: "main".into(),
                name: "main".into(),
                dependencies: vec!["dep".into()],
                files: vec![("Tests.al".into(), TESTS.into())],
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

fn opts(dir: &std::path::Path, only: Option<Vec<(usize, u32)>>) -> RunOptions {
    RunOptions {
        compile_cache: Some(CompileCache {
            dir: dir.to_path_buf(),
            require: false,
            rebuild: false,
            country: String::new(),
        }),
        only,
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

/// One request of a warm server over the same texts: its outcomes, where its baseline came from, whether its
/// engine was kept from an earlier request.
fn request(warm: &mut Warm, dir: &std::path::Path, only: Option<Vec<(usize, u32)>>) -> (Vec<String>, String, bool) {
    let files = [("Tests.al".to_string(), TESTS.to_string())];
    let changes: Vec<FileChange> = files
        .iter()
        .map(|(p, t)| FileChange {
            app: 0,
            path: p,
            text: t,
        })
        .collect();
    let o = opts(dir, only);
    let (rt, _) = warm
        .run(
            "k",
            &changes,
            build,
            &Default::default(),
            None,
            Default::default(),
            &[SuiteRun::everything()],
            &o,
        )
        .unwrap();
    let p = &rt.phases;
    (outcomes(&rt), format!("{:?}", p.baseline_source), p.engine_kept)
}

#[test]
fn every_request_starts_from_the_install_rows_and_none_of_an_earlier_requests_writes() {
    let dir = tempfile::tempdir().unwrap();
    let mut warm = Warm::new();
    let both = request(&mut warm, dir.path(), None);
    assert!(both.0.iter().all(|o| o.ends_with("Pass")), "{both:?}");
    assert_eq!(both.0.len(), 2, "{both:?}");
    // The first request ran the install code on an engine it built.
    assert_eq!((both.1.as_str(), both.2), ("Some(\"ran\")", false), "{both:?}");
    let mut seen = Vec::new();
    for only in [
        None,
        Some(vec![(0, 60100)]),
        Some(vec![(0, 60101)]),
        None,
        Some(vec![(0, 60100)]),
    ] {
        seen.push(request(&mut warm, dir.path(), only));
    }
    for (o, _, _) in &seen {
        assert!(o.iter().all(|o| o.ends_with("Pass")), "{seen:?}");
    }
    assert_eq!(seen[0].0, both.0, "{seen:?}");
    assert_eq!(seen[3].0, both.0, "{seen:?}");
    assert_eq!(seen[1].0, vec![both.0[0].clone()], "{seen:?}");
    assert_eq!(seen[2].0, vec![both.0[1].clone()], "{seen:?}");
    // The later requests, whatever they select, used the engines and the install baseline an earlier one kept.
    for (_, source, kept) in &seen[1..] {
        assert_eq!((source.as_str(), *kept), ("Some(\"kept\")", true), "{seen:?}");
    }
    // A run of one request over a compile of its own has the same outcomes.
    let (apps, kinds, m) = build();
    let o = opts(dir.path(), None);
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
    assert_eq!(
        outcomes(&CorpusRuntime::run_compiled(&c, None, Default::default(), &o)),
        both.0
    );
}
