//! Several entries over one compile and one install baseline (`RunOptions::entries`, al-runner issue 5635).

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, EntryRun, Manifest, RunOptions, SuiteRun};

fn app(id: &str, deps: &[&str], src: &str) -> AppSource {
    AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.to_string())],
    }
}

fn manifest(publisher: &str, resources: Vec<(String, Vec<u8>)>) -> Manifest {
    Manifest {
        publisher: publisher.into(),
        version: "1.0.0.0".into(),
        resources,
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

/// A dependency whose install code writes a row the tests can read: the install baseline runs.
const BASE: &str = "table 50100 Setting { fields { field(1; Code; Code[20]) { } field(2; Value; Text[50]) { } } keys { key(PK; Code) { } } }
codeunit 50101 BaseInstaller { Subtype = Install;
    trigger OnInstallAppPerCompany() var S: Record Setting; begin S.Code := 'SEEDED'; S.Insert(); end; }";

fn run(main: &str, entries: Vec<EntryRun>) -> CorpusRuntime {
    run_with(main, vec![("active.txt".into(), b"M0001".to_vec())], entries)
}

fn run_with(main: &str, resources: Vec<(String, Vec<u8>)>, entries: Vec<EntryRun>) -> CorpusRuntime {
    CorpusRuntime::try_run_suites_with(
        vec![app("main", &["base"], main), app("base", &[], BASE)],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[manifest("Me", resources), manifest("Microsoft", vec![])],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions {
            entries,
            ..Default::default()
        },
    )
    .expect("the run starts")
}

fn override_with(text: &str) -> EntryRun {
    EntryRun {
        resources: vec![(0, vec![("active.txt".into(), text.as_bytes().to_vec())])],
        ..Default::default()
    }
}

fn outcomes(rt: &CorpusRuntime) -> Vec<Vec<Outcome>> {
    rt.entries
        .iter()
        .map(|r| rt.results()[r.clone()].iter().map(|t| t.outcome.clone()).collect())
        .collect()
}

#[test]
fn each_entry_reads_its_own_resource_texts_from_the_same_baseline() {
    let main = "codeunit 60101 T { Subtype = Test;
        [Test] procedure Active() begin Error('active=%1', NavApp.GetResourceAsText('active.txt')); end; }";
    let rt = run(
        main,
        vec![override_with("M0002"), EntryRun::default(), override_with("M0003")],
    );
    let got = outcomes(&rt);
    assert_eq!(got.len(), 3);
    for (entry, want) in got.iter().zip(["active=M0002", "active=M0001", "active=M0003"]) {
        assert!(matches!(&entry[..], [Outcome::Fail(m)] if m.contains(want)), "{got:?}");
    }
}

#[test]
fn an_entry_with_resource_texts_is_not_run_when_the_install_baseline_read_a_resource() {
    let main = "codeunit 60102 Installer { Subtype = Install;
        trigger OnInstallAppPerCompany() var S: Record Setting; begin S.Code := 'RES'; S.Value := NavApp.GetResourceAsText('active.txt'); S.Insert(); end; }
        codeunit 60103 T { Subtype = Test;
        [Test] procedure Installed() var S: Record Setting; begin S.Get('RES'); Error('inst=%1', S.Value); end; }";
    let rt = run(main, vec![override_with("M0002"), EntryRun::default()]);
    let got = outcomes(&rt);
    assert!(
        matches!(&got[0][..], [Outcome::NotRun(why)] if why.contains("install baseline of its own")),
        "{got:?}"
    );
    assert!(
        matches!(&got[1][..], [Outcome::Fail(m)] if m.contains("inst=M0001")),
        "{got:?}"
    );
}

/// Al-runner issue 8026: an entry holds only the resource texts it replaces; the app's other resource files read as the
/// program's, and the replaced one as the entry's, in each entry alone.
#[test]
fn an_entry_reads_the_files_it_does_not_replace_from_the_app() {
    let main = "codeunit 60104 T { Subtype = Test;
        [Test] procedure Both() begin Error('active=%1 other=%2', NavApp.GetResourceAsText('active.txt'), NavApp.GetResourceAsText('other.txt')); end; }";
    let files = vec![
        ("active.txt".into(), b"M0001".to_vec()),
        ("other.txt".into(), b"O0001".to_vec()),
    ];
    let rt = run_with(
        main,
        files,
        vec![override_with("M0002"), EntryRun::default(), override_with("M0003")],
    );
    let got = outcomes(&rt);
    assert_eq!(got.len(), 3);
    for (entry, want) in got.iter().zip([
        "active=M0002 other=O0001",
        "active=M0001 other=O0001",
        "active=M0003 other=O0001",
    ]) {
        assert!(matches!(&entry[..], [Outcome::Fail(m)] if m.contains(want)), "{got:?}");
    }
}
