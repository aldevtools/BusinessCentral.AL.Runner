//! Al-runner issue 8072: the session language and WorkDate after install code the runner stopped.
//!
//! Restored to the state before the stopped routine only where the routine saved it first and sets it back as its last
//! statements, after the statement it was stopped in (the shape of reports/runtime/BdRegionTests.al:1327-1339). A routine
//! that changed the state and holds no such set-back leaves a state the runner does not know: the later install code of
//! that install session is not run, and a test that reads what it writes is not run either. A routine that changed
//! nothing, or ran to its end, keeps main's behavior.

use al_sema::workspace::AppSource;
use al_testrun::runtime::CorpusRuntime;
use al_testrun::{CaseId, Outcome};

const DEP_ID: &str = "11111111-1111-1111-1111-111111118072";
const MAIN_ID: &str = "22222222-2222-2222-2222-222222228072";

/// The log table.
const LOG: &str = r#"
table 50100 "Lang Log"
{
    DataPerCompany = false;
    fields { field(1; "Entry No."; Integer) { } field(2; "Language Id"; Integer) { } field(3; Day; Date) { } }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}
"#;

/// How the first install code ends.
#[derive(Clone, Copy)]
enum Shape {
    /// Saves the language and the WorkDate, switches both, is stopped (a language id the runtime refuses at run time),
    /// and sets both back as its last statements (the reporter's shape).
    StoppedBeforeSetBack,
    /// Switches both and is stopped; no set-back (al-runner PR 8105 review, `p8105/lang`).
    StoppedWithoutSetBack,
    /// Saves, switches, is stopped, sets back, then switches again: the set-back is not its last statement.
    StoppedSetBackNotLast,
    /// Saves, switches, fails with an AL error, sets back: real BC ends it at the error too, before the set-back.
    ErrorBeforeSetBack,
    /// Is stopped before it changes anything.
    StoppedUnchanged,
    /// Switches both and runs to its end without switching back.
    Finished,
}

fn body(shape: Shape) -> &'static str {
    match shape {
        Shape::StoppedBeforeSetBack => {
            "Prev := GlobalLanguage(); PrevDay := WorkDate(); GlobalLanguage(1031); WorkDate(20300105D); GlobalLanguage(-5); \
             GlobalLanguage(Prev); WorkDate(PrevDay);"
        }
        Shape::StoppedWithoutSetBack => "GlobalLanguage(1031); WorkDate(20300105D); GlobalLanguage(-5);",
        Shape::StoppedSetBackNotLast => {
            "Prev := GlobalLanguage(); PrevDay := WorkDate(); GlobalLanguage(1031); WorkDate(20300105D); GlobalLanguage(-5); \
             GlobalLanguage(Prev); WorkDate(PrevDay); GlobalLanguage(1031);"
        }
        Shape::ErrorBeforeSetBack => {
            "Prev := GlobalLanguage(); PrevDay := WorkDate(); GlobalLanguage(1031); WorkDate(20300105D); Error('stop'); \
             GlobalLanguage(Prev); WorkDate(PrevDay);"
        }
        Shape::StoppedUnchanged => "GlobalLanguage(-5); GlobalLanguage(1031);",
        Shape::Finished => "GlobalLanguage(1031); WorkDate(20300105D);",
    }
}

/// The first install routine (`trigger`), or a Company Triggers subscriber of the app. The subscriber also runs when the
/// test session opens the company (session 1), where it is not stopped.
fn switcher(shape: Shape, company_triggers: bool) -> String {
    let body = if company_triggers {
        body(shape).replace("GlobalLanguage(-5);", "if SessionId() <> 1 then GlobalLanguage(-5);")
    } else {
        body(shape).to_string()
    };
    let head = if company_triggers {
        "codeunit 50100 \"Lang Switch\"\n{\n    [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpenCompleted', '', false, false)]\n    local procedure OnOpen()"
    } else {
        "codeunit 50100 \"Lang Switch\"\n{\n    Subtype = Install;\n    trigger OnInstallAppPerCompany()"
    };
    format!(
        "{head}\n    var\n        Prev: Integer;\n        PrevDay: Date;\n    begin\n        {body}\n    end;\n}}\n"
    )
}

/// A later install routine of the same app: logs the language and the WorkDate it runs under.
fn logger(trigger: &str) -> String {
    format!(
        r#"
codeunit 50101 "Lang Logger"
{{
    Subtype = Install;
    trigger {trigger}()
    var
        L: Record "Lang Log";
    begin
        L."Entry No." := 1;
        L."Language Id" := GlobalLanguage();
        L.Day := WorkDate();
        L.Insert();
    end;
}}
"#
    )
}

const TESTS: &str = r#"
codeunit 50400 "Lang Tests"
{
    Subtype = Test;

    [Test]
    procedure LaterRoutineLanguage()
    var
        L: Record "Lang Log";
    begin
        L.Get(1);
        if L."Language Id" <> 1033 then Error('language %1', L."Language Id");
        if L.Day = 20300105D then Error('work date %1', L.Day);
    end;

    [Test]
    procedure LaterRoutineKeptLanguage()
    var
        L: Record "Lang Log";
    begin
        L.Get(1);
        if L."Language Id" <> 1031 then Error('language %1', L."Language Id");
        if L.Day <> 20300105D then Error('work date %1', L.Day);
    end;
}
"#;

fn app(id: &str, deps: &[&str], src: String) -> AppSource {
    AppSource {
        id: id.into(),
        name: format!("Lang {}", &id[..1]),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![("f.al".into(), src)],
    }
}

fn run(in_dependency: bool, routines: String, name: &str) -> Outcome {
    let (apps, kinds) = if in_dependency {
        (
            vec![app(MAIN_ID, &[DEP_ID], TESTS.to_string()), app(DEP_ID, &[], routines)],
            vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        )
    } else {
        (
            vec![app(MAIN_ID, &[], format!("{routines}{TESTS}"))],
            vec![al_sema::AppKind::Workspace],
        )
    };
    let rt = CorpusRuntime::run_for_apps(apps, kinds, &[], &Default::default(), None, Default::default());
    rt.first_outcome(&CaseId {
        codeunit: 50400,
        name: name.into(),
    })
}

/// An install routine that is stopped, and a later per-company install routine of the same app that logs, in a dependency
/// app (the install baseline, `baseline_roots.rs`) or in the app under test (`install.rs`).
fn outcome(in_dependency: bool, shape: Shape, name: &str) -> Outcome {
    let routines = format!("{LOG}{}{}", switcher(shape, false), logger("OnInstallAppPerCompany"));
    run(in_dependency, routines, name)
}

/// Not run: in the app under test the reason names the stopped code; a dependency app's refused root shows as its rows
/// that could not be created (the run log names the root).
fn not_run(o: &Outcome, in_dependency: bool) -> bool {
    matches!(o, Outcome::NotRun(why) if in_dependency || why.contains("Lang Switch"))
}

/// A per-company install routine that does nothing: the Company Triggers run only before per-company install code.
const PER_COMPANY: &str = r#"
codeunit 50102 "Lang Per Company"
{
    Subtype = Install;
    trigger OnInstallAppPerCompany()
    begin
    end;
}
"#;

#[test]
fn a_stopped_dependency_install_routine_with_its_set_back_leaves_no_language_or_work_date() {
    assert_eq!(
        outcome(true, Shape::StoppedBeforeSetBack, "LaterRoutineLanguage"),
        Outcome::Pass
    );
}

#[test]
fn a_stopped_install_routine_of_the_app_under_test_with_its_set_back_leaves_no_language_or_work_date() {
    assert_eq!(
        outcome(false, Shape::StoppedBeforeSetBack, "LaterRoutineLanguage"),
        Outcome::Pass
    );
}

/// The reviewer's `p8105/lang` shape: no set-back, so the state after the routine is not known. The later routine of the
/// session is not run, and the test that reads its row is not run.
#[test]
fn a_stopped_install_routine_without_a_set_back_refuses_the_later_routines_of_its_session() {
    for in_dependency in [true, false] {
        for name in ["LaterRoutineLanguage", "LaterRoutineKeptLanguage"] {
            let o = outcome(in_dependency, Shape::StoppedWithoutSetBack, name);
            assert!(
                not_run(&o, in_dependency),
                "in dependency {in_dependency}, {name}: {o:?}"
            );
        }
    }
}

#[test]
fn a_set_back_that_is_not_the_last_statement_does_not_count() {
    for in_dependency in [true, false] {
        let o = outcome(in_dependency, Shape::StoppedSetBackNotLast, "LaterRoutineLanguage");
        assert!(not_run(&o, in_dependency), "in dependency {in_dependency}: {o:?}");
    }
}

#[test]
fn an_install_routine_that_failed_with_an_error_before_its_set_back_refuses_the_later_routines() {
    for in_dependency in [true, false] {
        let o = outcome(in_dependency, Shape::ErrorBeforeSetBack, "LaterRoutineLanguage");
        assert!(not_run(&o, in_dependency), "in dependency {in_dependency}: {o:?}");
    }
}

/// Main's behavior: a routine stopped before it changed the state leaves nothing to restore, and the later routine runs.
#[test]
fn a_stopped_install_routine_that_changed_nothing_keeps_the_later_routines() {
    assert_eq!(
        outcome(true, Shape::StoppedUnchanged, "LaterRoutineLanguage"),
        Outcome::Pass
    );
    assert_eq!(
        outcome(false, Shape::StoppedUnchanged, "LaterRoutineLanguage"),
        Outcome::Pass
    );
}

/// Main's behavior: a routine that ran to its end and did not switch back leaves its state to the next routine.
#[test]
fn a_finished_install_routine_keeps_its_language_and_work_date() {
    assert_eq!(
        outcome(false, Shape::Finished, "LaterRoutineKeptLanguage"),
        Outcome::Pass
    );
    assert_eq!(
        outcome(true, Shape::Finished, "LaterRoutineKeptLanguage"),
        Outcome::Pass
    );
}

/// The Company Triggers of an app under test (`install.rs`, `company_open`): a subscriber stopped before its set-back
/// leaves the per-database routine of the same install session the state from before it.
#[test]
fn a_stopped_company_triggers_subscriber_with_its_set_back_leaves_no_language() {
    let routines = format!(
        "{LOG}{}{}",
        switcher(Shape::StoppedBeforeSetBack, true),
        logger("OnInstallAppPerDatabase")
    );
    let routines = format!("{routines}{PER_COMPANY}");
    assert_eq!(run(false, routines, "LaterRoutineLanguage"), Outcome::Pass);
}

#[test]
fn a_stopped_company_triggers_subscriber_without_a_set_back_refuses_the_later_routines() {
    let routines = format!(
        "{LOG}{}{}",
        switcher(Shape::StoppedWithoutSetBack, true),
        logger("OnInstallAppPerDatabase")
    );
    let routines = format!("{routines}{PER_COMPANY}");
    let o = run(false, routines, "LaterRoutineLanguage");
    assert!(not_run(&o, false), "{o:?}");
}
