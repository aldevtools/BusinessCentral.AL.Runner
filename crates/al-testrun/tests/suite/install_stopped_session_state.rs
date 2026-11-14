//! Al-runner issue 8072: an install routine the runner stops is rolled back, and the session language and WorkDate it set
//! go back with its rows, as for a test refused while it ran (reports/runtime/BdRegionTests.al, `BdSetterLanguage_*` after
//! `BdSummaryML_1040`; al-testrun `run.rs`). The later install routines run under the state the session had before it.
//! A routine that runs to its end keeps what it set (main's behavior).

use al_sema::workspace::AppSource;
use al_testrun::runtime::CorpusRuntime;
use al_testrun::{CaseId, Outcome};

const DEP_ID: &str = "11111111-1111-1111-1111-111111118072";
const MAIN_ID: &str = "22222222-2222-2222-2222-222222228072";

/// The log table and the routine that records what the session runs under.
const LOG: &str = r#"
table 50100 "Lang Log"
{
    fields { field(1; "Entry No."; Integer) { } field(2; "Language Id"; Integer) { } field(3; Day; Date) { } }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}
"#;

/// Switches the language and the WorkDate and is stopped by the runner before it switches back (a language id the runtime
/// refuses at run time), or runs to its end without switching back.
fn switcher(id: u32, stops: bool) -> String {
    let stop = if stops { "GlobalLanguage(-5);" } else { "" };
    format!(
        r#"
codeunit {id} "Lang Switch {id}"
{{
    Subtype = Install;
    trigger OnInstallAppPerCompany()
    begin
        GlobalLanguage(1031);
        WorkDate(20300105D);
        {stop}
    end;
}}
"#
    )
}

/// A later install routine: logs the language and the WorkDate it runs under.
fn logger(id: u32) -> String {
    format!(
        r#"
codeunit {id} "Lang Logger {id}"
{{
    Subtype = Install;
    trigger OnInstallAppPerCompany()
    var
        L: Record "Lang Log";
    begin
        L."Entry No." := {id};
        L."Language Id" := GlobalLanguage();
        L.Day := WorkDate();
        L.Insert();
    end;
}}
"#
    )
}

fn tests(entry: u32) -> String {
    format!(
        r#"
codeunit 50400 "Lang Tests"
{{
    Subtype = Test;

    [Test]
    procedure LaterRoutineLanguage()
    var
        L: Record "Lang Log";
    begin
        L.Get({entry});
        if L."Language Id" <> 1033 then Error('language %1', L."Language Id");
        if L.Day = 20300105D then Error('work date %1', L.Day);
    end;

    [Test]
    procedure LaterRoutineKeptLanguage()
    var
        L: Record "Lang Log";
    begin
        L.Get({entry});
        if L."Language Id" <> 1031 then Error('language %1', L."Language Id");
        if L.Day <> 20300105D then Error('work date %1', L.Day);
    end;
}}
"#
    )
}

fn app(id: &str, deps: &[&str], src: String) -> AppSource {
    AppSource {
        id: id.into(),
        name: format!("Lang {}", &id[..1]),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![("f.al".into(), src)],
    }
}

/// The install routines in a dependency app (the install baseline, `baseline_roots.rs`) or in the app under test
/// (`install.rs`).
fn outcome(in_dependency: bool, stops: bool, name: &str) -> Outcome {
    let routines = format!("{LOG}{}{}", switcher(50100, stops), logger(50101));
    let (apps, kinds) = if in_dependency {
        (
            vec![app(MAIN_ID, &[DEP_ID], tests(50101)), app(DEP_ID, &[], routines)],
            vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        )
    } else {
        (
            vec![app(MAIN_ID, &[], format!("{routines}{}", tests(50101)))],
            vec![al_sema::AppKind::Workspace],
        )
    };
    let rt = CorpusRuntime::run_for_apps(apps, kinds, &[], &Default::default(), None, Default::default());
    rt.first_outcome(&CaseId {
        codeunit: 50400,
        name: name.into(),
    })
}

#[test]
fn a_stopped_dependency_install_routine_leaves_no_language_or_work_date() {
    assert_eq!(outcome(true, true, "LaterRoutineLanguage"), Outcome::Pass);
}

#[test]
fn a_stopped_install_routine_of_the_app_under_test_leaves_no_language_or_work_date() {
    assert_eq!(outcome(false, true, "LaterRoutineLanguage"), Outcome::Pass);
}

/// Main's behavior: a routine that ran to its end and did not switch back leaves its state to the next routine.
#[test]
fn a_finished_install_routine_keeps_its_language_and_work_date() {
    assert_eq!(outcome(false, false, "LaterRoutineKeptLanguage"), Outcome::Pass);
    assert_eq!(outcome(true, false, "LaterRoutineKeptLanguage"), Outcome::Pass);
}
