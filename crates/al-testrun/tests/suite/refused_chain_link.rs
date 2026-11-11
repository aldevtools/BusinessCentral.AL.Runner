//! A chain codeunit that was not run as a whole (`corpus.rs` `not_run_link`) holds back only the later codeunits that
//! read what it can leave them. Under runner 130450 (the BC 27 legs, codeunits/run-onrun/RqIsolationModes.al:357-369
//! #else) a codeunit with Codeunit isolation leaves its OnRun trigger's writes and the numbers its tests took
//! (testlibraries test-framework/onrun-storage/Y35OnRunStorage.al:30-31); its tests' rows are rolled back with it. The
//! BC 27.5 shape of al-runner issue 7343: corpus LgnLoginChecklistTests.al (codeunit 77323) is refused because
//! Microsoft's login code cannot run here, and held back every later codeunit that read a table its tests name.

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SuiteRun};

/// Stands in for System Application's login code, which stops here (the BC 27.5 corpus run reads G/L Entry demo rows).
const DEP: &str = r#"
table 1990 "Guided Experience Item" { fields { field(1; "Object ID"; Integer) { } } keys { key(PK; "Object ID") { Clustered = true; } } }
codeunit 150 "System Initialization"
{
    [IntegrationEvent(false, false)] internal procedure OnAfterLogin() begin end;
}
codeunit 151 "System Initialization Impl."
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Company Triggers", 'OnCompanyOpenCompleted', '', false, false)]
    local procedure InitCompany()
    var SystemInitialization: Codeunit "System Initialization";
    begin
        SystemInitialization.OnAfterLogin();
    end;
}
codeunit 152 "Login Stops"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Company Triggers", 'OnCompanyOpenCompleted', '', false, false)]
    local procedure StopsHere()
    begin
        Error('The login code cannot run here.');
    end;
}
codeunit 1990 "Guided Experience"
{
    procedure InsertAssistedSetup(ObjectID: Integer)
    var G: Record "Guided Experience Item";
    begin
        G."Object ID" := ObjectID; G.Insert();
    end;
    procedure Exists(ObjectID: Integer): Boolean
    var G: Record "Guided Experience Item";
    begin
        exit(G.Get(ObjectID));
    end;
}
"#;

/// `on_run` is the body of the refused codeunit's OnRun trigger (`None`: it has none).
fn main_src(on_run: Option<&str>) -> String {
    let trigger = on_run.map_or(String::new(), |b| {
        format!("    trigger OnRun() var R: Record \"Row\"; begin {b} end;\n")
    });
    format!(
        r#"
table 51100 "Row" {{ fields {{ field(1; K; Code[20]) {{ }} }} keys {{ key(PK; K) {{ Clustered = true; }} }} }}
codeunit 51101 "Login Subs"
{{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure AfterLogin()
    var G: Codeunit "Guided Experience";
    begin
        G.InsertAssistedSetup(51101);
    end;
}}
codeunit 51102 "Sees The Login"
{{
    Subtype = Test;
{trigger}
    [Test]
    procedure ReadsTheLoginRowAndWritesRows()
    var G: Codeunit "Guided Experience"; R: Record "Row";
    begin
        NumberSequence.Insert('OWN51102');
        if NumberSequence.Next('OWN51102') <> 0 then Error('own sequence');
        R.K := 'TEST'; R.Insert();
        if not G.Exists(51101) then Error('no login row');
    end;
}}
codeunit 51103 "Reads Rows Later"
{{
    Subtype = Test;
    [Test]
    procedure SeesNoRow()
    var R: Record "Row";
    begin
        if not R.IsEmpty() then Error('a row is there');
    end;
}}
"#
    )
}

fn app(id: &str, deps: &[&str], src: &str) -> AppSource {
    AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.into())],
    }
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
        symbols: vec![],
    }
}

fn outcomes(main: &str) -> Vec<(String, Outcome)> {
    outcomes_over(main, DEP)
}

fn outcomes_over(main: &str, dep: &str) -> Vec<(String, Outcome)> {
    outcomes_with(main, dep, &RunOptions::default())
}

fn outcomes_with(main: &str, dep: &str, opts: &RunOptions) -> Vec<(String, Outcome)> {
    let rt = CorpusRuntime::try_run_suites_with(
        vec![app("main", &["dep"], main), app("dep", &[], dep)],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[manifest("Me"), manifest("Microsoft")],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        opts,
    )
    .expect("the run starts");
    let mut o: Vec<_> = rt
        .results()
        .iter()
        .map(|r| (r.test.clone(), r.outcome.clone()))
        .collect();
    o.sort_by(|a, b| a.0.cmp(&b.0));
    o
}

fn outcome<'a>(o: &'a [(String, Outcome)], test: &str) -> &'a Outcome {
    &o.iter()
        .find(|(t, _)| t == test)
        .unwrap_or_else(|| panic!("no {test} in {o:?}"))
        .1
}

#[test]
fn a_refused_codeunit_without_on_run_does_not_hold_back_a_reader_of_its_tests_rows() {
    let o = outcomes(&main_src(None));
    assert!(
        matches!(outcome(&o, "ReadsTheLoginRowAndWritesRows"), Outcome::NotRun(m) if m.contains("The login code cannot run here")),
        "{o:?}"
    );
    assert_eq!(outcome(&o, "SeesNoRow"), &Outcome::Pass, "{o:?}");
}

#[test]
fn a_refused_codeunit_whose_on_run_writes_a_table_holds_back_its_readers() {
    let o = outcomes(&main_src(Some("R.K := 'ONRUN'; R.Insert();")));
    assert!(
        matches!(outcome(&o, "ReadsTheLoginRowAndWritesRows"), Outcome::NotRun(_)),
        "{o:?}"
    );
    assert!(
        matches!(outcome(&o, "SeesNoRow"), Outcome::NotRun(m) if m.contains("Sees The Login")),
        "{o:?}"
    );
}

// The numbers the refused codeunit's tests took from a sequence the install code made are not rolled back with it
// (Y35OnRunStorage.al:30-31, C302_B2_TestWritesStorage :457): what it left there is not known.
#[test]
fn a_refused_codeunit_whose_tests_take_numbers_holds_back_a_reader_of_number_sequences() {
    let main = main_src(None)
        .replace(
            "        NumberSequence.Insert('OWN51102');\n        if NumberSequence.Next('OWN51102') <> 0 then Error('own sequence');",
            "        if NumberSequence.Next('SHARED51100') < 0 then Error('shared sequence');",
        )
        .replace(
            "        if not R.IsEmpty() then Error('a row is there');",
            "        if NumberSequence.Next('SHARED51100') <> 0 then Error('a number was taken');",
        )
;
    assert!(main.contains("Next('SHARED51100') < 0") && main.contains("<> 0 then Error('a number was taken')"));
    let dep = format!("{DEP}{}", INSTALLS);
    // ADR-066 amendment 12: by default every test codeunit starts from the baseline's sequence values and nothing is held
    // back for numbers an earlier codeunit took; `--strict-order` keeps BC's order and the hold-back.
    let strict = RunOptions {
        strict_order: true,
        ..RunOptions::default()
    };
    let o = outcomes_with(&main, &dep, &strict);
    assert!(
        matches!(outcome(&o, "SeesNoRow"), Outcome::NotRun(m) if m.contains("Sees The Login")),
        "{o:?}"
    );
}

const INSTALLS: &str = r#"
codeunit 50221 "Installs A Sequence"
{
    Subtype = Install;
    trigger OnInstallAppPerCompany() begin NumberSequence.Insert('SHARED51100'); end;
}
"#;
