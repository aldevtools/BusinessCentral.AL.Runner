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

/// Al-runner issue 8125: an entry's own time limit wins over the run's; `Some(None)` is no limit, and an entry without
/// one keeps the run's. The time `Sleep` asks for counts toward it.
#[test]
fn an_entry_runs_its_tests_under_its_own_time_limit_or_the_runs() {
    let main = "codeunit 60104 T { Subtype = Test; [Test] procedure SleepsFive() begin Sleep(5000); end; }";
    let secs = |n| Some(std::time::Duration::from_secs(n));
    let entries = vec![
        EntryRun {
            test_time_limit: Some(secs(2)),
            ..Default::default()
        },
        EntryRun {
            test_time_limit: Some(None),
            ..Default::default()
        },
        EntryRun::default(),
    ];
    let rt = CorpusRuntime::try_run_suites_with(
        vec![app("main", &["base"], main), app("base", &[], BASE)],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[manifest("Me", vec![]), manifest("Microsoft", vec![])],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions {
            entries,
            test_time_limit: secs(3),
            ..Default::default()
        },
    )
    .expect("the run starts");
    let got = outcomes(&rt);
    let stopped_at = |entry: &[Outcome], n: u32| matches!(entry, [Outcome::NotRun(why)] if why.contains(&format!("time limit of {n} s")));
    assert!(stopped_at(&got[0], 2), "the entry's 2 s: {got:?}");
    assert_eq!(got[1], [Outcome::Pass], "no limit: {got:?}");
    assert!(stopped_at(&got[2], 3), "the run's 3 s: {got:?}");
}

/// An app under test with no dependency app: no install baseline runs, so its install code runs for the test codeunits.
fn run_alone(main: &str, entries: Vec<EntryRun>, impact_trace: bool) -> CorpusRuntime {
    run_alone_with(
        main,
        vec![("active.txt".into(), b"M0001".to_vec())],
        entries,
        impact_trace,
    )
}

/// [`run_alone`] with the app's resource files `resources`.
fn run_alone_with(
    main: &str,
    resources: Vec<(String, Vec<u8>)>,
    entries: Vec<EntryRun>,
    impact_trace: bool,
) -> CorpusRuntime {
    CorpusRuntime::try_run_suites_with(
        vec![app("main", &[], main)],
        vec![al_sema::AppKind::Workspace],
        &[manifest("Me", resources)],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions {
            entries,
            impact_trace,
            ..Default::default()
        },
    )
    .expect("the run starts")
}

/// Install code that writes a row and sets WorkDate and GlobalLanguage; the test fails with what it sees of them.
const INSTALLS: &str = "table 60110 Seeded { fields { field(1; Code; Code[20]) { } field(2; Value; Text[50]) { } } keys { key(PK; Code) { } } }
    codeunit 60111 Installer { Subtype = Install;
        trigger OnInstallAppPerCompany() var S: Record Seeded; begin S.Code := 'INST'; S.Value := 'V'; S.Insert(); WorkDate(DMY2Date(3, 4, 2031)); GlobalLanguage(1031); end; }
    codeunit 60112 T { Subtype = Test;
        [Test] procedure Sees() var S: Record Seeded; begin S.Get('INST'); Error('rows=%1 wd=%2 lang=%3 res=%4', S.Count(), Date2DMY(WorkDate(), 3), GlobalLanguage(), NavApp.GetResourceAsText('active.txt')); end; }";

/// Al-runner issue 8144: install code that reads no resource file runs once for all entries of a request, and every entry
/// starts from what it left: the row, WorkDate and GlobalLanguage, as when it ran in each entry.
#[test]
fn install_code_that_reads_no_resource_runs_once_for_all_entries() {
    let rt = run_alone(
        INSTALLS,
        vec![override_with("M0002"), EntryRun::default(), override_with("M0003")],
        false,
    );
    let got = outcomes(&rt);
    for (entry, res) in got.iter().zip(["M0002", "M0001", "M0003"]) {
        let want = format!("rows=1 wd=2031 lang=1031 res={res}");
        assert!(
            matches!(&entry[..], [Outcome::Fail(m)] if m.contains(&want)),
            "{want}: {got:?}"
        );
    }
    assert_eq!(rt.phases.install_runs, 1, "{got:?}");
}

/// Al-runner issue 8144, ADR-028 decision 11: install code that read a resource file runs again for an entry whose resource
/// texts differ from those it read, and reads that entry's texts.
#[test]
fn install_code_that_reads_a_resource_runs_again_for_an_entry_with_other_texts() {
    let main = "table 60120 Seeded { fields { field(1; Code; Code[20]) { } field(2; Value; Text[50]) { } } keys { key(PK; Code) { } } }
        codeunit 60121 Installer { Subtype = Install;
            trigger OnInstallAppPerCompany() var S: Record Seeded; begin S.Code := 'RES'; S.Value := NavApp.GetResourceAsText('active.txt'); S.Insert(); end; }
        codeunit 60122 T { Subtype = Test;
            [Test] procedure Installed() var S: Record Seeded; begin S.Get('RES'); Error('inst=%1', S.Value); end; }";
    let rt = run_alone(
        main,
        vec![
            override_with("M0002"),
            override_with("M0002"),
            EntryRun::default(),
            override_with("M0003"),
        ],
        false,
    );
    let got = outcomes(&rt);
    for (entry, want) in got.iter().zip(["inst=M0002", "inst=M0002", "inst=M0001", "inst=M0003"]) {
        assert!(
            matches!(&entry[..], [Outcome::Fail(m)] if m.contains(want)),
            "{want}: {got:?}"
        );
    }
    // The second entry has the first one's texts; each other entry has texts of its own.
    assert_eq!(rt.phases.install_runs, 3, "{got:?}");
    assert_eq!(
        crate::install_once::install_line(&rt).as_deref(),
        Some(
            "install code: ran 3 times for the test codeunits; first reason noted that its run cannot be shared: other texts of the resource files it read"
        ),
    );
}

/// An entry that replaces `name` with `text`.
fn override_file(name: &str, text: &str) -> EntryRun {
    EntryRun {
        resources: vec![(0, vec![(name.into(), text.as_bytes().to_vec())])],
        ..Default::default()
    }
}

/// Install code that reads `active.txt` with `read`; the test fails with the row it wrote and the text of `other.txt`.
fn reads_active(read: &str) -> String {
    format!(
        "table 60120 Seeded {{ fields {{ field(1; Code; Code[20]) {{ }} field(2; Value; Text[50]) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
        codeunit 60121 Installer {{ Subtype = Install;
            trigger OnInstallAppPerCompany() var S: Record Seeded; L: List of [Text]; begin S.Code := 'RES'; {read} S.Insert(); end; }}
        codeunit 60122 T {{ Subtype = Test;
            [Test] procedure Installed() var S: Record Seeded; begin S.Get('RES'); Error('inst=%1 other=%2', S.Value, NavApp.GetResourceAsText('other.txt')); end; }}"
    )
}

/// The app's files for [`reads_active`].
fn two_files() -> Vec<(String, Vec<u8>)> {
    vec![
        ("active.txt".into(), b"M0001".to_vec()),
        ("other.txt".into(), b"O0001".to_vec()),
    ]
}

/// Install code that read one resource file runs again only for an entry whose text of that file differs from the text
/// it read: an entry that replaces only another file starts from what it left.
#[test]
fn install_code_runs_again_only_for_an_entry_with_another_text_of_the_file_it_read() {
    let rt = run_alone_with(
        &reads_active("S.Value := NavApp.GetResourceAsText('active.txt');"),
        two_files(),
        vec![
            EntryRun::default(),
            override_file("other.txt", "X0002"),
            override_file("active.txt", "M0002"),
            override_file("other.txt", "X0003"),
        ],
        false,
    );
    let got = outcomes(&rt);
    let wants = [
        "inst=M0001 other=O0001",
        "inst=M0001 other=X0002",
        "inst=M0002 other=O0001",
        "inst=M0001 other=X0003",
    ];
    for (entry, want) in got.iter().zip(wants) {
        assert!(
            matches!(&entry[..], [Outcome::Fail(m)] if m.contains(want)),
            "{want}: {got:?}"
        );
    }
    // The second entry keeps the first one's text of active.txt; the third and the fourth each change it.
    assert_eq!(rt.phases.install_runs, 3, "{got:?}");
}

/// The neighbor: install code that listed the app's resource files counts as having read every file of the app, so it runs
/// again for an entry that replaces any of them.
#[test]
fn install_code_that_lists_the_resource_files_runs_again_for_an_entry_with_any_other_text() {
    let rt = run_alone_with(
        &reads_active("L := NavApp.ListResources(); S.Value := Format(L.Count());"),
        two_files(),
        vec![EntryRun::default(), override_file("other.txt", "X0002")],
        false,
    );
    let got = outcomes(&rt);
    for (entry, want) in got.iter().zip(["inst=2 other=O0001", "inst=2 other=X0002"]) {
        assert!(
            matches!(&entry[..], [Outcome::Fail(m)] if m.contains(want)),
            "{want}: {got:?}"
        );
    }
    assert_eq!(rt.phases.install_runs, 2, "{got:?}");
}

/// A run that notes the code it runs (here the impact trace) keeps running the install code in each test codeunit's
/// session, so that what it notes of the install code is noted for each codeunit.
#[test]
fn a_run_that_notes_the_code_it_runs_runs_the_install_code_for_each_entry() {
    let rt = run_alone(
        INSTALLS,
        vec![EntryRun::default(), EntryRun::default(), EntryRun::default()],
        true,
    );
    let got = outcomes(&rt);
    assert!(
        got.iter()
            .all(|e| matches!(&e[..], [Outcome::Fail(m)] if m.contains("rows=1 wd=2031 lang=1031 res=M0001"))),
        "{got:?}"
    );
    assert_eq!(rt.phases.install_runs, 3, "{got:?}");
    assert_eq!(
        crate::install_once::install_line(&rt).as_deref(),
        Some(
            "install code: ran 3 times for the test codeunits; first reason noted that its run cannot be shared: the run follows the code it runs"
        ),
    );
}
