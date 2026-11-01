//! A test that ends with a table still marked by `Consistent(false)`: the runner's commit there is not pinned (only
//! `Commit()` is, corpus misc/lt-cu-odd-leads/CuCommitTests.al:405-419), so the test is not run.

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
table 69980 "IC Row"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 69981 T
{
    Subtype = Test;
    [Test]
    procedure LeavesMark()
    var Row: Record "IC Row";
    begin Row.Consistent(false); Row.Id := 1; Row.Insert(); end;
    [Test]
    procedure LiftsMark()
    var Row: Record "IC Row";
    begin Row.Consistent(false); Row.Id := 2; Row.Insert(); Row.Consistent(true); end;
}
"#;

#[test]
fn a_test_that_ends_with_an_inconsistent_table_is_not_run() {
    let out = outcomes_in(SRC, "T");
    let of = |n: &str| out.iter().find(|(t, _)| t == n).map(|(_, o)| o.clone()).unwrap();
    match of("LeavesMark") {
        Outcome::NotRun(r) => assert!(r.contains("Consistent(false)"), "{r}"),
        other => panic!("{other:?}"),
    }
    assert!(matches!(of("LiftsMark"), Outcome::Pass), "{:?}", of("LiftsMark"));
}
