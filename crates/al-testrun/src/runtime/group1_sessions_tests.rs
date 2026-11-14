//! A filter in filter group 1 across install sessions (al-runner issue 8071; assumption
//! `global-filter-ends-with-install-session`, corpus PR 1405, owner al-vm `group1_sessions.rs`).
use super::tests::outcomes_and_hits_of_apps;
use super::*;
use al_builtins::assumptions::{self, Assumption};
use al_sema::AppSource;

const ID: &str = "global-filter-ends-with-install-session";

/// System Application's login, as the install session raises it: "System Initialization Impl." subscribes to Company
/// Triggers OnCompanyOpenCompleted and raises "System Initialization".OnAfterLogin.
const DEP: &str = "codeunit 150 \"System Initialization\" { procedure Raise() begin OnAfterLogin(); end;
        [IntegrationEvent(false, false)] procedure OnAfterLogin() begin end; }
    codeunit 151 \"System Initialization Impl.\" {
        [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpenCompleted', '', false, false)]
        local procedure InitCompany() var S: Codeunit \"System Initialization\"; begin S.Raise(); end; }";

/// The reporter's app A (issue 8071): a table, a subscriber that sets a group 1 filter on it, and an empty install
/// codeunit; `event` is the subscriber's attribute.
fn app_a(event: &str, body: &str) -> String {
    format!(
        "table 60950 Item {{ fields {{ field(1; No; Code[20]) {{ }} field(2; Blocked; Boolean) {{ }} }} }}
        codeunit 60951 Setter {{
            [EventSubscriber(ObjectType::Codeunit, {event}, '', false, false)]
            local procedure Login() {body} }}
        codeunit 60952 InstA {{ Subtype = Install; trigger OnInstallAppPerCompany() begin end; }}"
    )
}

const LOGIN: &str = "Codeunit::\"System Initialization\", 'OnAfterLogin'";
const SET_BY_RECORD: &str =
    "var I: Record Item; begin I.FilterGroup(1); I.SetRange(Blocked, false); I.FilterGroup(0); end;";

/// App B: an install codeunit that writes a row, a test that reads it, and a test that touches no table.
const APP_B: &str = "table 60960 Setup { fields { field(1; K; Code[10]) { } } }
    codeunit 60961 InstB { Subtype = Install;
        trigger OnInstallAppPerCompany() var S: Record Setup; begin if S.Insert() then; end; }
    codeunit 60962 T { Subtype = Test;
        [Test] procedure InstallRan() var S: Record Setup; begin S.Get(); end;
        [Test] procedure NoTable() begin end; }";

/// App B, whose install code reads A's table.
const APP_B_READS: &str = "table 60960 Setup { fields { field(1; K; Code[10]) { } } }
    codeunit 60961 InstB { Subtype = Install;
        trigger OnInstallAppPerCompany() var I: Record Item; S: Record Setup; begin if I.IsEmpty() then; if S.Insert() then; end; }
    codeunit 60962 T { Subtype = Test;
        [Test] procedure InstallRan() var S: Record Setup; begin S.Get(); end; }";

/// The outcomes and hits of B's tests, with the dependency app `DEP` and the apps under test A and B.
fn run(a: &str, b: &str) -> super::tests::OutcomesAndHits {
    outcomes_and_hits_of_apps(&[("dep", &[], DEP), ("a", &["dep"], a), ("b", &["a"], b)], "T")
}

fn outcome(a: &str, b: &str) -> Outcome {
    run(a, b).0[0].1.clone()
}

/// Asserts B's first test passed flagged and its second passed clean.
fn first_flagged(a: &str, b: &str) {
    let (o, hits) = run(a, b);
    let hit = |k: usize| hits.iter().any(|(n, h)| *n == o[k].0 && h.contains(&ID));
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
    assert!(hit(0), "{hits:?}");
    if o.len() > 1 {
        assert_eq!(o[1].1, Outcome::Pass, "{o:?}");
        assert!(!hit(1), "{hits:?}");
    }
}

// The reporter's repro (issue 8071; G1sXaInstall.al:88-96): a group 1 filter A's OnAfterLogin subscriber set in A's install
// session is not there in B's install session, where the subscriber sets it again. B's install code ran through the
// assumption: the test that reads what it wrote is flagged, the one that touches no table is clean.
#[test]
fn a_group1_filter_of_one_install_session_does_not_reach_the_next() {
    first_flagged(&app_a(LOGIN, SET_BY_RECORD), APP_B);
}

// Cell b (G1sInstallSessionTests.al:58; G1sXaInstall.al:115-117): a filter A's install routine left set, read with a new
// record in B's install session: both rows, no filter.
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
    first_flagged(a, b);
}

// A dependency app's install session (the install baseline, `baseline_roots.rs`): a filter its install routine left set
// is not there in the app under test's install session. The test that reads what that wrote is flagged through the
// baseline.
#[test]
fn a_dependency_apps_install_session_ends_its_group1_filter() {
    let dep = "table 60940 Feature { fields { field(1; K; Code[10]) { } field(2; On; Boolean) { } } }
        codeunit 60941 DepInst { Subtype = Install;
            trigger OnInstallAppPerCompany() var F: Record Feature; G: Record Feature;
            begin F.K := 'A'; F.Insert(); F.K := 'B'; F.On := true; F.Insert(); G.FilterGroup(1); G.SetRange(On, true); G.FilterGroup(0); end; }";
    let main = "table 60960 Log { fields { field(1; K; Code[10]) { } field(2; N; Integer) { } } }
        codeunit 60961 InstB { Subtype = Install;
            trigger OnInstallAppPerCompany() var F: Record Feature; L: Record Log; begin L.N := F.Count(); L.Insert(); end; }
        codeunit 60962 T { Subtype = Test;
            [Test] procedure Seen() var L: Record Log; begin L.Get(); if L.N <> 2 then Error('%1', L.N); end;
            [Test] procedure NoTable() begin end; }";
    // The install baseline's roots: the dependency app's install codeunit is compiled.
    let (ws, l) = al_sema::lower::compile_apps_with(
        vec![
            AppSource {
                id: "dep".into(),
                name: "dep".into(),
                dependencies: vec![],
                files: vec![("dep.al".into(), dep.into())],
            },
            AppSource {
                id: "main".into(),
                name: "main".into(),
                dependencies: vec!["dep".into()],
                files: vec![("main.al".into(), main.into())],
            },
        ],
        vec![al_sema::AppKind::Dependency, al_sema::AppKind::Workspace],
        &Default::default(),
        None,
        al_sema::check::Roots {
            baseline: true,
            ..Default::default()
        },
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let engine = Engine::new(l.program.clone(), Vec::new());
    let base = crate::baseline::run(&engine, &l, None).expect("the dependency app has install code");
    assert!(base.notes.iter().any(|n| n.contains(ID)), "{:?}", base.notes);
    // The baseline as the cache keeps it.
    let back = al_vm::Baseline::from_bytes(&base.baseline.to_bytes().expect("plain rows")).expect("read back");
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    let mut hits = Vec::new();
    let o = run_codeunit_full(
        &engine,
        &l,
        oi,
        Some(&back),
        TestIsolation::Codeunit,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut hits,
        None,
    );
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
    assert_eq!(o[1].1, Outcome::Pass, "{o:?}");
    let hit = |k: usize| {
        hits.iter()
            .any(|(n, h)| *n == o[k].0 && h.iter().any(|a| a.info().id == ID))
    };
    assert!(hit(0), "{o:?}");
    assert!(!hit(1), "{o:?}");
}

// The drop is marked: the install part relied on the assumption (the baseline notes name it).
#[test]
fn the_dropped_filter_marks_the_assumption() {
    let a = app_a(LOGIN, SET_BY_RECORD);
    let (ws, l) = al_sema::lower::compile_apps(
        [
            ("dep", vec![], DEP.to_string()),
            ("a", vec!["dep".to_string()], a),
            ("b", vec!["a".to_string()], APP_B.into()),
        ]
        .into_iter()
        .map(|(id, dependencies, src)| AppSource {
            id: id.into(),
            name: id.into(),
            dependencies,
            files: vec![(format!("{id}.al"), src)],
        })
        .collect(),
        vec![
            al_sema::AppKind::Dependency,
            al_sema::AppKind::Workspace,
            al_sema::AppKind::Workspace,
        ],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let engine = Engine::new(l.program.clone(), Vec::new());
    let mut session = Session::new(engine);
    assumptions::clear_hits();
    let skipped = crate::install::run_apps_under_test(&l, &mut session);
    assert!(skipped.is_empty(), "{skipped:?}");
    assert!(assumptions::hit(Assumption::GlobalFilterEndsWithInstallSession));
}

fn refused_by_group1(o: &Outcome) -> bool {
    matches!(o, Outcome::NotRun(r) if r.contains("filter group 1"))
}

// Out of the measured shape, main's refusal stays: a group 1 filter a Company Triggers subscriber set (not an
// OnAfterLogin subscriber).
#[test]
fn a_filter_of_a_company_triggers_subscriber_keeps_mains_refusal() {
    let a = app_a(
        "Codeunit::\"Company Triggers\", 'OnCompanyOpenCompleted'",
        SET_BY_RECORD,
    );
    assert!(refused_by_group1(&outcome(&a, APP_B)));
}

// Out of the measured shape, main's refusal stays: a group 1 filter an "Extension Triggers" install subscriber set (not
// the install codeunit's trigger).
#[test]
fn a_filter_of_an_extension_triggers_subscriber_keeps_mains_refusal() {
    let a = app_a(
        "Codeunit::\"Extension Triggers\", 'OnInstallAppPerCompany'",
        SET_BY_RECORD,
    );
    let o = outcome(&a, APP_B_READS);
    assert!(refused_by_group1(&o), "{o:?}");
}

// Out of the measured shape, main's refusal stays: a group 1 filter set in the per-database part of A's install session.
#[test]
fn a_filter_of_the_per_database_part_keeps_mains_refusal() {
    let a = "table 60950 Item { DataPerCompany = false; fields { field(1; No; Code[20]) { } field(2; Blocked; Boolean) { } } }
        codeunit 60952 InstA { Subtype = Install;
            trigger OnInstallAppPerDatabase() var I: Record Item; begin I.FilterGroup(1); I.SetRange(Blocked, false); I.FilterGroup(0); end; }";
    assert!(refused_by_group1(&outcome(a, APP_B_READS)));
}

// Out of the measured shape, main's refusal stays: a group 1 filter an OnAfterLogin subscriber set through a FieldRef.
#[test]
fn a_filter_set_through_a_fieldref_keeps_mains_refusal() {
    let a = app_a(
        LOGIN,
        "var R: RecordRef; F: FieldRef; begin R.Open(60950); R.FilterGroup(1); F := R.Field(2); F.SetRange(false); end;",
    );
    assert!(refused_by_group1(&outcome(&a, APP_B)));
}
