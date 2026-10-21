//! A test that ends while a duplicate-key `Insert` is still deferred (al-runner issue 5603). The corpus pins that the
//! statement form returns without an error and the next access to the table or a `Commit` raises it
//! (record/TestRecordRuntimeTextsAndViews.al:180-209); what the test runner's commit at the end of the test does with
//! it is not pinned, so the test is not run, never passed.

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
table 69970 "DI Row"
{
    fields { field(1; "Key"; Code[20]) { } field(2; Value; Integer) { } }
    keys { key(PK; "Key") { Clustered = true; } }
}
codeunit 69971 T
{
    Subtype = Test;
    [Test]
    procedure EndsDeferred()
    var Row: Record "DI Row";
    begin Row.DeleteAll(); Row."Key" := 'K1'; Row.Insert(); Row."Key" := 'K1'; Row.Insert(); end;
}
codeunit 69972 U
{
    Subtype = Test;
    [Test]
    procedure RaisedByTheNextRead()
    var Row: Record "DI Row";
    begin Row.DeleteAll(); Row."Key" := 'K2'; Row.Insert(); Row."Key" := 'K2'; Row.Insert(); if Row.Count() = 0 then; end;
    [Test]
    procedure GuardedReturnsFalse()
    var Row: Record "DI Row";
    begin Row.DeleteAll(); Row."Key" := 'K3'; Row.Insert(); Row."Key" := 'K3'; if Row.Insert() then Error('true'); end;
}
"#;

#[test]
fn a_test_that_ends_with_a_deferred_duplicate_insert_is_not_run() {
    let mut out = outcomes_in(SRC, "T");
    out.extend(outcomes_in(SRC, "U"));
    let of = |n: &str| out.iter().find(|(t, _)| t == n).map(|(_, o)| o.clone()).unwrap();
    match of("EndsDeferred") {
        Outcome::NotRun(r) => assert!(r.contains("deferred"), "{r}"),
        other => panic!("{other:?}"),
    }
    match of("RaisedByTheNextRead") {
        Outcome::Fail(m) => assert!(m.contains("already exists"), "{m}"),
        other => panic!("{other:?}"),
    }
    assert!(
        matches!(of("GuardedReturnsFalse"), Outcome::Pass),
        "{:?}",
        of("GuardedReturnsFalse")
    );
}
