//! BC 29 test framework features the runner does not run yet are refused, never run as if they were not there.
//!
//! * A data-driven test (`[TestDataSource(Codeunit, DataSet)]`, no `[Test]`) runs once per case on BC
//!   (al-language-testlibraries/lt-bi-test-framework/BiTestDataSource.al:87). It used to be dropped from the
//!   result without a row.
//! * A test codeunit with `TestHandlers` has its ITestHandler callbacks run around it and each test
//!   (BiTestHandler.al:98, :125). Its tests used to run without the callbacks.

use al_test_cli::report::Outcome;
use al_testkit::outcomes_in;

const SRC: &str = r#"
interface ITestContext
{
}

codeunit 50100 "DD Tests"
{
    Subtype = Test;

    [Test]
    procedure Plain()
    begin
    end;

    [TestDataSource(50101, 'Set1')]
    procedure DataDriven(Context: Interface ITestContext)
    begin
    end;
}

codeunit 50102 "TH Tests"
{
    Subtype = Test;
    TestHandlers = "Some Handler";

    [Test]
    procedure First()
    begin
    end;
}
"#;

fn not_run(o: &Outcome) -> Option<&str> {
    match o {
        Outcome::NotRun(why) => Some(why),
        _ => None,
    }
}

#[test]
fn a_data_driven_test_is_reported_as_not_run() {
    let got = outcomes_in(SRC, "DD Tests");
    assert_eq!(got.len(), 2, "{got:?}");
    assert_eq!(got[0], ("Plain".to_string(), Outcome::Pass));
    assert_eq!(got[1].0, "DataDriven");
    assert!(
        not_run(&got[1].1).is_some_and(|w| w.contains("TestDataSource")),
        "{got:?}"
    );
}

#[test]
fn a_codeunit_with_test_handlers_is_not_run() {
    let got = outcomes_in(SRC, "TH Tests");
    assert_eq!(got.len(), 1, "{got:?}");
    assert!(
        not_run(&got[0].1).is_some_and(|w| w.contains("TestHandlers")),
        "{got:?}"
    );
}
