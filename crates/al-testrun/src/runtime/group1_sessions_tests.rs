//! A filter in filter group 1 across install sessions (al-runner issue 8071; assumption
//! `group1-filter-ends-with-install-session`, corpus PR 1405, owner al-vm `group1_sessions.rs`).
use super::tests::outcomes_and_hits_of_apps;
use super::*;
use al_builtins::assumptions::{self, Assumption};
use al_sema::AppSource;

/// App A's table, a Company Triggers subscriber that sets a group 1 filter on it, and an install codeunit; `extra` adds
/// to A.
fn app_a(extra: &str) -> String {
    format!(
        "table 60950 Item {{ fields {{ field(1; No; Code[20]) {{ }} field(2; Blocked; Boolean) {{ }} }} }}
        codeunit 60951 Setter {{
            [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpenCompleted', '', false, false)]
            local procedure Login() var I: Record Item; begin I.FilterGroup(1); I.SetRange(Blocked, false); I.FilterGroup(0); end; }}
        codeunit 60952 InstA {{ Subtype = Install; trigger OnInstallAppPerCompany() begin end; }}
        {extra}"
    )
}

/// App B: an install codeunit that writes a row, and a test that reads it.
const APP_B: &str = "table 60960 Setup { fields { field(1; K; Code[10]) { } } }
    codeunit 60961 InstB { Subtype = Install;
        trigger OnInstallAppPerCompany() var S: Record Setup; begin if S.Insert() then; end; }
    codeunit 60962 T { Subtype = Test;
        [Test] procedure InstallRan() var S: Record Setup; begin S.Get(); end; }";

fn outcome(a: &str, b: &str) -> Outcome {
    let (o, _) = outcomes_and_hits_of_apps(&[("a", &[], a), ("b", &["a"], b)], "T");
    o[0].1.clone()
}

// The reporter's repro (issue 8071): a group 1 filter a Company Triggers subscriber set in A's install session is not
// there in B's install session, where the subscriber sets it again.
#[test]
fn a_group1_filter_of_one_install_session_does_not_reach_the_next() {
    assert_eq!(outcome(&app_a(""), APP_B), Outcome::Pass);
}

// Cell b (G1sInstallSessionTests.al:58): a filter A's install routine left set, read with a new record in B's install
// session: both rows, no filter.
#[test]
fn the_next_install_session_reads_the_table_without_the_filter() {
    let a = "table 60950 Item { fields { field(1; No; Code[20]) { } field(2; Qty; Integer) { } } }
        codeunit 60952 InstA { Subtype = Install;
            trigger OnInstallAppPerCompany() var I: Record Item; S: Record Item;
            begin S.No := 'R1'; S.Qty := 1; S.Insert(); S.No := 'R2'; S.Qty := 5; S.Insert();
                I.FilterGroup(1); I.SetFilter(Qty, '>=3'); I.FilterGroup(0); end; }";
    let b = "table 60960 Log { fields { field(1; K; Code[10]) { } field(2; Seen; Text[100]) { } } }
        codeunit 60961 InstB { Subtype = Install;
            trigger OnInstallAppPerCompany() var I: Record Item; L: Record Log; G1: Text;
            begin I.FilterGroup(1); G1 := I.GetFilters(); I.FilterGroup(0); L.Seen := StrSubstNo('count=%1;g1=%2', I.Count(), G1); L.Insert(); end; }
        codeunit 60962 T { Subtype = Test;
            [Test] procedure Seen() var L: Record Log; begin L.Get(); if L.Seen <> 'count=2;g1=' then Error('%1', L.Seen); end; }";
    assert_eq!(outcome(a, b), Outcome::Pass);
}

// The drop is marked: the install part relied on the assumption (the baseline notes name it).
#[test]
fn the_dropped_filter_marks_the_assumption() {
    let a = app_a("");
    let (ws, l) = al_sema::lower::compile(
        vec![
            AppSource {
                id: "a".into(),
                name: "a".into(),
                dependencies: vec![],
                files: vec![("a.al".into(), a)],
            },
            AppSource {
                id: "b".into(),
                name: "b".into(),
                dependencies: vec!["a".into()],
                files: vec![("b.al".into(), APP_B.into())],
            },
        ],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let engine = Engine::new(l.program.clone(), Vec::new());
    let mut session = Session::new(engine);
    assumptions::clear_hits();
    let skipped = crate::install::run_apps_under_test(&l, &mut session);
    assert!(skipped.is_empty(), "{skipped:?}");
    assert!(assumptions::hit(Assumption::Group1FilterEndsWithInstallSession));
}

// Out of the measured shape, main's refusal stays: a group 1 filter set in the per-database part of A's install session.
#[test]
fn a_filter_of_the_per_database_part_keeps_mains_refusal() {
    let a = "table 60950 Item { DataPerCompany = false; fields { field(1; No; Code[20]) { } field(2; Blocked; Boolean) { } } }
        codeunit 60952 InstA { Subtype = Install;
            trigger OnInstallAppPerDatabase() var I: Record Item; begin I.FilterGroup(1); I.SetRange(Blocked, false); I.FilterGroup(0); end; }";
    let b = "table 60960 Setup { fields { field(1; K; Code[10]) { } } }
        codeunit 60961 InstB { Subtype = Install;
            trigger OnInstallAppPerCompany() var I: Record Item; S: Record Setup; begin if I.IsEmpty() then; if S.Insert() then; end; }
        codeunit 60962 T { Subtype = Test;
            [Test] procedure InstallRan() var S: Record Setup; begin S.Get(); end; }";
    assert!(matches!(outcome(a, b), Outcome::NotRun(r) if r.contains("filter group 1")));
}

// Out of the measured shape, main's refusal stays: a group 1 filter set through a FieldRef.
#[test]
fn a_filter_set_through_a_fieldref_keeps_mains_refusal() {
    let a = "table 60950 Item { fields { field(1; No; Code[20]) { } field(2; Blocked; Boolean) { } } }
        codeunit 60951 Setter {
            [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpenCompleted', '', false, false)]
            local procedure Login() var R: RecordRef; F: FieldRef; begin R.Open(60950); R.FilterGroup(1); F := R.Field(2); F.SetRange(false); end; }
        codeunit 60952 InstA { Subtype = Install; trigger OnInstallAppPerCompany() begin end; }";
    assert!(matches!(outcome(a, APP_B), Outcome::NotRun(r) if r.contains("filter group 1")));
}
