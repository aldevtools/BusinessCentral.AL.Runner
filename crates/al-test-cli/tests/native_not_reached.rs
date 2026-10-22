//! al-runner issue 5684: a `[Native]` procedure without an implementation here refuses when it is called, not when a test
//! or an install routine can merely reach it. BC installs an app whose install code reaches `Agent Utilities.SetInstructions`
//! only for rows that do not exist (corpus-writer RH: nothing to pin; a reached SetInstructions stops BC with a CLR
//! exception, so it stays refused when it runs).

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
codeunit 50190 "NN Platform"
{
    [Native] procedure Unlisted(String: Text): Text begin end;
}
table 50190 "NN Row"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 50191 "NN Install"
{
    Subtype = Install;
    trigger OnInstallAppPerCompany()
    var R: Record "NN Row"; P: Codeunit "NN Platform";
    begin
        if R.FindSet() then
            repeat
                P.Unlisted('x');
            until R.Next() = 0;
    end;
}
codeunit 50192 T
{
    Subtype = Test;
    [Test]
    procedure NotReached()
    var R: Record "NN Row";
    begin
        if not R.IsEmpty() then
            Error('rows');
    end;
    [Test]
    procedure Reached()
    var P: Codeunit "NN Platform";
    begin
        P.Unlisted('x');
    end;
}
"#;

#[test]
fn a_native_refuses_only_where_it_is_called() {
    let out = outcomes_in(SRC, "T");
    let get = |n: &str| out.iter().find(|(t, _)| t == n).map(|(_, o)| o.clone()).unwrap();
    assert_eq!(get("NotReached"), Outcome::Pass);
    assert!(
        matches!(get("Reached"), Outcome::NotRun(ref why) if why.contains("Native") && why.contains("Unlisted")),
        "{:?}",
        get("Reached")
    );
}
