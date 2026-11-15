//! The partner dependency apps' login subscribers in the strict-order lane (al-runner issue 8115, scenario s09; review of
//! al-runner PR 8138, probes p2/s_set, s_none, s_bumpfn). The login sets `Cnt` to 10 in every session; in BC's order the
//! value an earlier test codeunit left wins (`Session::adopt_single_instances`; PlLoginSingleTests.al:109-128). In the
//! default order each test codeunit starts with fresh SingleInstance state and its own login (ADR-066 amendment 12).

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SuiteRun};

const SYS: &str = "codeunit 150 \"System Initialization\" { procedure Raise() begin OnAfterLogin(); end;
        [IntegrationEvent(false, false, false)] procedure OnAfterLogin() begin end; }
    codeunit 151 \"System Initialization Impl.\" {
        [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpenCompleted', '', false, false)]
        local procedure InitCompany() var S: Codeunit \"System Initialization\"; begin S.Raise(); end; }";

const PARTNER: &str = "codeunit 60904 Cache { SingleInstance = true; var Cnt: Integer;
        procedure Set(N: Integer) begin Cnt := N; end; procedure GetCount(): Integer begin exit(Cnt); end;
        procedure Bump() begin Cnt += 1; end; }
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
    run_in(src, true)
}

fn run_in(src: &str, strict_order: bool) -> CorpusRuntime {
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
            strict_order,
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

/// Every test passes.
fn assert_passes(rt: &CorpusRuntime) {
    for r in rt.results() {
        assert_eq!(r.outcome, Outcome::Pass, "{}: {:?}", r.test, r.outcome);
    }
}

// p2/s_set: A sets 99; in BC's order B reads 99, not the 10 its own session's login set.
#[test]
fn an_earlier_codeunits_value_wins_over_the_login_in_bcs_order() {
    assert_passes(&run(&tests("C.Set(99);", 99)));
}

// p2/s_none: A touches nothing; B reads the login's 10.
#[test]
fn a_reader_after_an_untouched_codeunit_sees_the_login_value() {
    assert_passes(&run(&tests("", 10)));
}

// p2/s_bumpfn: A reads the login's 10 and sets 11; B reads 11.
#[test]
fn a_codeunit_that_reads_the_login_value_and_writes_hands_it_on() {
    assert_passes(&run(&tests("if C.GetCount() = 10 then C.Set(11);", 11)));
}

// PlLoginSingleTests.al:109-128: BC runs the login once per session and keeps the single across test codeunits, so B
// reads the 11 that A's `Cnt += 1` left.
#[test]
fn a_later_codeunit_reads_the_value_an_earlier_codeunit_bumped_in_bcs_order() {
    assert_passes(&run(&tests("C.Bump();", 11)));
}

// The same shape in the default order: B starts fresh and reads its own login's 10 (ADR-066 amendment 12), as for the
// SingleInstance codeunits of the apps under test. A test that expects BC's 11 fails there.
#[test]
fn a_later_codeunit_after_a_bump_in_the_default_order_starts_fresh() {
    assert_passes(&run_in(&tests("C.Bump();", 10), false));
    let rt = run_in(&tests("C.Bump();", 11), false);
    let b1 = rt.results().iter().find(|r| r.test.ends_with("B1")).expect("B1 ran");
    assert!(
        matches!(&b1.outcome, Outcome::Fail(m) if m.contains("count 10")),
        "{:?}",
        b1.outcome
    );
}
