//! The cell tap (`RunOptions::cell_tap`): per test, how many `Expect` cells of the corpus fixture "Probe Observer" matched,
//! which were wrong, which were never looked at. Report only: outcomes are the same with the tap off.

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun, TestResult};

const TESTS: &str = r#"
codeunit 50970 "Cell Tap Tests"
{
    Subtype = Test;

    [Test]
    procedure AllMatch()
    var
        O: Codeunit "Probe Observer";
    begin
        O.Observe('a', '1');
        O.Observe('b', '2');
        O.Expect('a', '1');
        O.Expect('b', '2');
        O.Verify();
    end;

    [Test]
    procedure TwoWrongOfFour()
    var
        O: Codeunit "Probe Observer";
    begin
        O.Observe('a', '1');
        O.Observe('b', 'x');
        O.Observe('c', '3');
        O.Observe('d', 'y');
        O.Expect('a', '1');
        O.Expect('b', '2');
        O.Expect('c', '3');
        O.Expect('d', '4');
        O.Verify();
    end;

    [Test]
    procedure FewerObservedThanExpected()
    var
        O: Codeunit "Probe Observer";
    begin
        O.Observe('a', '1');
        O.Expect('a', '1');
        O.Expect('b', '2');
        O.Verify();
    end;

    [Test]
    procedure IgnoredNamesAreNotCells()
    var
        O: Codeunit "Probe Observer";
    begin
        O.Ignore('dump');
        O.Observe('dump', 'zzz');
        O.Observe('a', '1');
        O.Expect('a', '1');
        O.Verify();
    end;

    [Test]
    procedure FailsBeforeVerify()
    var
        O: Codeunit "Probe Observer";
    begin
        O.Expect('a', '1');
        O.Expect('b', '2');
        Error('stopped');
        O.Verify();
    end;

    [Test]
    procedure NoObserver()
    begin
    end;
}
"#;

fn run(cell_tap: bool) -> CorpusRuntime {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/tests/al-language/_fixtures/ProbeObserver.Codeunit.al"
    );
    let fixture = std::fs::read_to_string(path).expect("the corpus submodule is checked out");
    CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: vec![],
            files: vec![("fixture.al".into(), fixture), ("tests.al".into(), TESTS.into())],
        }],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions {
            cell_tap,
            ..Default::default()
        },
    )
}

fn result<'a>(rt: &'a CorpusRuntime, test: &str) -> &'a TestResult {
    rt.results()
        .iter()
        .find(|r| r.test == test)
        .unwrap_or_else(|| panic!("{test} did not run"))
}

#[test]
fn a_matching_test_counts_every_cell() {
    let rt = run(true);
    let r = result(&rt, "AllMatch");
    assert_eq!(r.outcome, Outcome::Pass);
    let c = r.cells.as_ref().expect("cells");
    assert_eq!((c.expected, c.matched, c.mismatched.len(), c.unreached()), (2, 2, 0, 0));
}

#[test]
fn wrong_cells_are_counted_past_the_first_one() {
    let rt = run(true);
    let r = result(&rt, "TwoWrongOfFour");
    assert!(matches!(r.outcome, Outcome::Fail(_)), "{:?}", r.outcome);
    let c = r.cells.as_ref().expect("cells");
    assert_eq!((c.expected, c.matched), (4, 2));
    let keys: Vec<(&str, &str, Option<&str>)> = c
        .mismatched
        .iter()
        .map(|m| (m.key.as_str(), m.expected.as_str(), m.actual.as_deref()))
        .collect();
    assert_eq!(keys, vec![("b", "2", Some("b=x")), ("d", "4", Some("d=y"))]);
}

#[test]
fn a_cell_the_test_never_observed_is_missing() {
    let rt = run(true);
    let c = result(&rt, "FewerObservedThanExpected").cells.clone().expect("cells");
    assert_eq!((c.expected, c.matched), (2, 1));
    assert_eq!(c.mismatched.len(), 1);
    assert_eq!(
        (c.mismatched[0].key.as_str(), c.mismatched[0].actual.clone()),
        ("b", None)
    );
}

#[test]
fn ignored_observations_do_not_shift_the_cells() {
    let rt = run(true);
    let r = result(&rt, "IgnoredNamesAreNotCells");
    assert_eq!(r.outcome, Outcome::Pass);
    let c = r.cells.as_ref().expect("cells");
    assert_eq!((c.expected, c.matched), (1, 1));
}

#[test]
fn a_test_that_fails_before_verify_leaves_its_cells_unreached() {
    let rt = run(true);
    let r = result(&rt, "FailsBeforeVerify");
    assert!(matches!(r.outcome, Outcome::Fail(_)));
    let c = r.cells.as_ref().expect("cells");
    assert_eq!((c.expected, c.matched, c.unreached(), c.verifies), (2, 0, 2, 0));
}

#[test]
fn a_test_without_the_fixture_has_no_cells() {
    assert!(result(&run(true), "NoObserver").cells.is_none());
}

#[test]
fn outcomes_are_the_same_with_the_tap_off_and_it_records_nothing() {
    let (on, off) = (run(true), run(false));
    assert_eq!(on.results().len(), off.results().len());
    for (a, b) in on.results().iter().zip(off.results()) {
        assert_eq!((&a.test, &a.outcome), (&b.test, &b.outcome));
        assert!(b.cells.is_none(), "{}", b.test);
    }
}
