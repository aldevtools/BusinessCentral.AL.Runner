//! The partner dependency apps' login subscribers in the strict-order lane (al-runner issue 8115, scenario s09; review of
//! al-runner PR 8138, probes p2/s_set, s_none, s_bumpfn). The login sets `Cnt` to 10 in every session; in BC's order the
//! value an earlier test codeunit left wins (`Session::adopt_single_instances`), and the readers are flagged.

use al_builtins::assumptions::Assumption;
use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SuiteRun};

const SYS: &str = "codeunit 150 \"System Initialization\" { procedure Raise() begin OnAfterLogin(); end;
        [IntegrationEvent(false, false, false)] procedure OnAfterLogin() begin end; }
    codeunit 151 \"System Initialization Impl.\" {
        [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpenCompleted', '', false, false)]
        local procedure InitCompany() var S: Codeunit \"System Initialization\"; begin S.Raise(); end; }";

const PARTNER: &str = "codeunit 60904 Cache { SingleInstance = true; var Cnt: Integer;
        procedure Set(N: Integer) begin Cnt := N; end; procedure GetCount(): Integer begin exit(Cnt); end; }
    codeunit 60901 DepLogin {
        [EventSubscriber(ObjectType::Codeunit, Codeunit::\"System Initialization\", 'OnAfterLogin', '', false, false)]
        local procedure L() var C: Codeunit Cache; begin C.Set(10); end; }";

/// Test codeunit `A` with one test of body `a`, and `B`, whose first test expects `Cnt` = `b` and whose second touches
/// nothing.
fn tests(a: &str, b: u32) -> String {
    format!(
        "codeunit 60911 Inst {{ Subtype = Install; trigger OnInstallAppPerCompany() begin end; }}
        codeunit 60951 A {{ Subtype = Test; [Test] procedure A1() var C: Codeunit Cache; begin {a} end; }}
        codeunit 60952 B {{ Subtype = Test;
            [Test] procedure B1() var C: Codeunit Cache; begin if C.GetCount() <> {b} then Error('count %1', C.GetCount()); end;
            [Test] procedure B2() begin end; }}"
    )
}

fn run(src: &str) -> CorpusRuntime {
    CorpusRuntime::try_run_suites_with(
        vec![
            app("main", &["dep2"], src),
            app("dep2", &["dep"], PARTNER),
            app("dep", &[], SYS),
        ],
        vec![
            al_sema::AppKind::Workspace,
            al_sema::AppKind::Dependency,
            al_sema::AppKind::Dependency,
        ],
        &[manifest("Me"), manifest("Probe"), manifest("Microsoft")],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions {
            strict_order: true,
            ..Default::default()
        },
    )
    .expect("the run starts")
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
        platform_internals_visible_to: Vec::new(),
        symbols: vec![],
    }
}

/// Every test passes; returns whether each of `B1` and `B2` relies on the partner login.
fn assert_passes(rt: &CorpusRuntime) -> (bool, bool) {
    for r in rt.results() {
        assert_eq!(r.outcome, Outcome::Pass, "{}: {:?}", r.test, r.outcome);
    }
    let flagged = |name: &str| {
        rt.results()
            .iter()
            .any(|r| r.test.ends_with(name) && r.assumptions.contains(&Assumption::PartnerLoginSubscriberInTestSession))
    };
    (flagged("B1"), flagged("B2"))
}

// p2/s_set: A sets 99; in BC's order B reads 99, not the 10 its own session's login set.
#[test]
fn an_earlier_codeunits_value_wins_over_the_login_in_bcs_order() {
    let rt = run(&tests("C.Set(99);", 99));
    assert_passes(&rt);
}

// p2/s_none: A touches nothing; B reads the login's 10 and relies on it, and so does the test after it.
#[test]
fn a_reader_after_an_untouched_codeunit_sees_the_login_value() {
    let rt = run(&tests("", 10));
    assert_eq!(assert_passes(&rt), (true, true));
}

// p2/s_bumpfn: A reads the login's 10 and sets 11; B reads 11.
#[test]
fn a_codeunit_that_reads_the_login_value_and_writes_hands_it_on() {
    let rt = run(&tests("if C.GetCount() = 10 then C.Set(11);", 11));
    assert!(assert_passes(&rt).0);
}
