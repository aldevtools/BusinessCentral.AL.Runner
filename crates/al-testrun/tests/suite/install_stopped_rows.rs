//! Al-runner issue 6973: a dependency app's install routine stops before it inserts a row (here a feature flag), and another
//! app's install already put rows into the same table. A test that reads the table through an ordinary read (a filter and
//! `IsEmpty`) finds no row for the flag and raises its own error. On BC the install runs to its end and the row exists, so
//! the answer "no row" is not BC's: the test is not run, as for a table that holds no row at all (ADR-013 decision 5).

use al_sema::workspace::AppSource;
use al_testrun::runtime::CorpusRuntime;
use al_testrun::{CaseId, Outcome};

const FLAGS_ID: &str = "11111111-1111-1111-1111-111111116973";
const OTHER_ID: &str = "22222222-2222-2222-2222-222222226973";
const STOPS_ID: &str = "33333333-3333-3333-3333-333333336973";
const MAIN_ID: &str = "44444444-4444-4444-4444-444444446973";

/// The feature flag app: its table and the procedures the other apps call.
const FLAGS: &str = r#"
table 50100 "My Feature Flag"
{
    fields { field(1; Code; Code[50]) { } field(2; Enabled; Boolean) { } }
    keys { key(PK; Code) { Clustered = true; } }
}

codeunit 50100 "My Feature Flags"
{
    procedure AddFeature(FlagCode: Code[50])
    var
        FeatureFlag: Record "My Feature Flag";
    begin
        FeatureFlag.Code := FlagCode;
        if FeatureFlag.Insert() then;
    end;

    procedure IsEnabled(FlagCode: Code[50]): Boolean
    var
        FeatureFlag: Record "My Feature Flag";
    begin
        FeatureFlag.SetRange(Code, FlagCode);
        if FeatureFlag.IsEmpty() then
            Error('Feature flag %1 is not defined.', FlagCode);
        exit(false);
    end;
}
"#;

/// Another app whose install adds its own flag: the table holds a row in the baseline.
const OTHER: &str = r#"
codeunit 50200 "Other Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Flags: Codeunit "My Feature Flags";
    begin
        Flags.AddFeature('OTHERFLAG');
    end;
}
"#;

/// The app whose install stops before it adds its flag.
const STOPS: &str = r#"
codeunit 50300 "My Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Flags: Codeunit "My Feature Flags";
    begin
        CreateJobQueueEntries();
        Flags.AddFeature('MYFLAG');
    end;

    local procedure CreateJobQueueEntries()
    begin
        Error('stops here');
    end;
}
"#;

const MAIN: &str = r#"
codeunit 50400 "My Tests"
{
    Subtype = Test;

    [Test]
    procedure ReadsTheFlag()
    var
        Flags: Codeunit "My Feature Flags";
    begin
        Flags.IsEnabled('MYFLAG');
    end;

    [Test]
    procedure ReadsTheOtherFlag()
    var
        FeatureFlag: Record "My Feature Flag";
    begin
        FeatureFlag.Get('OTHERFLAG');
    end;
}
"#;

fn app(id: &str, deps: &[&str], src: &str) -> AppSource {
    AppSource {
        id: id.into(),
        name: format!("Stopped {}", &id[..1]),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![("f.al".into(), src.into())],
    }
}

fn outcome(name: &str) -> Outcome {
    let apps = vec![
        app(MAIN_ID, &[FLAGS_ID, OTHER_ID, STOPS_ID], MAIN),
        app(FLAGS_ID, &[], FLAGS),
        app(OTHER_ID, &[FLAGS_ID], OTHER),
        app(STOPS_ID, &[FLAGS_ID], STOPS),
    ];
    let kinds = vec![
        al_sema::AppKind::Workspace,
        al_sema::AppKind::Dependency,
        al_sema::AppKind::Dependency,
        al_sema::AppKind::Dependency,
    ];
    let rt = CorpusRuntime::run_for_apps(apps, kinds, &[], &Default::default(), None, Default::default());
    rt.first_outcome(&CaseId {
        codeunit: 50400,
        name: name.into(),
    })
}

#[test]
fn a_test_that_reads_rows_a_stopped_install_would_have_inserted_is_not_run() {
    match outcome("ReadsTheFlag") {
        Outcome::NotRun(why) => assert!(why.contains("My Feature Flag"), "{why}"),
        other => panic!("expected not run, got {other:?}"),
    }
}

/// A row the table holds is BC's answer: another app's install wrote it, and the stopped install could only have added rows.
#[test]
fn a_row_another_install_wrote_is_still_found() {
    assert_eq!(outcome("ReadsTheOtherFlag"), Outcome::Pass);
}
