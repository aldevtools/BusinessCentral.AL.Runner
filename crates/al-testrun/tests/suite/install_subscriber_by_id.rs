//! An Extension Triggers install subscriber written with the bare id 2000000010 runs as the one that names the codeunit
//! (assumption `subscriber-publisher-by-id`, corpus PR 1466, SbiSubscriberByIdTests.al:276). A test that reads a table it
//! writes is a flagged pass, for a subscriber of the app under test and of a dependency app (`install::assume_by_id_install_tables`); the name
//! form stays an unflagged pass.

use al_builtins::assumptions::Assumption;
use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SuiteRun};

/// The table "BI Inst Row" and the install subscriber that inserts its row, the publisher written as `publisher`.
fn installer(publisher: &str) -> String {
    format!(
        r#"
table 50230 "BI Inst Row"
{{
    fields {{ field(1; Id; Integer) {{ }} }}
    keys {{ key(PK; Id) {{ Clustered = true; }} }}
}}
codeunit 50231 "BI Inst Sub"
{{
    [EventSubscriber(ObjectType::Codeunit, {publisher}, 'OnInstallAppPerCompany', '', false, false)]
    local procedure OnInstall()
    var R: Record "BI Inst Row";
    begin
        R.Id := 7;
        R.Insert();
    end;
}}
"#
    )
}

const TEST: &str = r#"
codeunit 50932 "BI Inst Tests"
{
    Subtype = Test;
    [Test] procedure ReadsRow() var R: Record "BI Inst Row"; begin if not R.Get(7) then Error('no install row'); end;
}
"#;

/// The outcome of `ReadsRow` and whether it ran through `subscriber-publisher-by-id`; with `dependency` the table and the
/// subscriber are in a dependency app, else in the app under test.
fn run(publisher: &str, dependency: bool) -> (Outcome, bool) {
    let app = |id: &str, deps: &[&str], src: String| AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src)],
    };
    let (apps, kinds) = if dependency {
        (
            vec![
                app("test", &["lib"], TEST.into()),
                app("lib", &[], installer(publisher)),
            ],
            vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        )
    } else {
        (
            vec![app("test", &[], format!("{}{TEST}", installer(publisher)))],
            vec![al_sema::AppKind::Workspace],
        )
    };
    let manifests = vec![Manifest::default(); apps.len()];
    let rt = CorpusRuntime::try_run_suites_with(
        apps,
        kinds,
        &manifests,
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    )
    .expect("runs");
    let r = rt
        .results()
        .iter()
        .find(|t| t.test == "ReadsRow")
        .expect("ReadsRow ran")
        .clone();
    (r.outcome, r.assumptions.contains(&Assumption::SubscriberPublisherById))
}

const NAME: &str = "Codeunit::\"Extension Triggers\"";

#[test]
fn a_test_reading_the_row_of_a_bare_id_install_subscriber_of_the_app_under_test_is_flagged() {
    let (o, flagged) = run("2000000010", false);
    assert!(matches!(o, Outcome::Pass) && flagged, "{o:?}, flagged {flagged}");
}

#[test]
fn a_test_reading_the_row_of_a_bare_id_install_subscriber_of_a_dependency_is_flagged() {
    let (o, flagged) = run("2000000010", true);
    assert!(matches!(o, Outcome::Pass) && flagged, "{o:?}, flagged {flagged}");
}

#[test]
fn the_name_form_install_subscriber_keeps_an_unflagged_pass() {
    for dependency in [false, true] {
        let (o, flagged) = run(NAME, dependency);
        assert!(
            matches!(o, Outcome::Pass) && !flagged,
            "dependency {dependency}: {o:?}, flagged {flagged}"
        );
    }
}
