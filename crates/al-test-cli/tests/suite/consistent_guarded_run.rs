//! Consistent(false) marks and a guarded Codeunit.Run (al-runner issue 7586 item 4). The pinned shapes raise the
//! inconsistency error as the run ends (errors/ConsistentGuardedRun.al); their neighbors refuse: a Commit after another
//! variable of the table called Consistent(true), an inner codeunit that marks and then fails, two marked tables.

use super::y48_platform_probes::outcomes;
use al_test_cli::report::Outcome;

const SRC: &str = r#"
table 50100 "C Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
table 50101 "C Other" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
codeunit 50101 "C Nothing" { trigger OnRun() begin end; }
codeunit 50102 "C Mark Fail" { trigger OnRun() var R: Record "C Row"; begin R.Consistent(false); Error('inner failed'); end; }
codeunit 50000 T { Subtype = Test;
  [Test] procedure Pinned() var R: Record "C Row"; Ok: Boolean; begin
    Commit(); R.Consistent(false); asserterror Ok := Codeunit.Run(Codeunit::"C Nothing"); Error(GetLastErrorCode()); end;
  [Test] procedure CommitAfterOther() var M: Record "C Row"; U: Record "C Row"; begin
    M.Consistent(false); U.Consistent(true); Commit(); end;
  [Test] procedure InnerMarksAndFails() var Ok: Boolean; begin
    Commit(); Ok := Codeunit.Run(Codeunit::"C Mark Fail"); end;
  [Test] procedure TwoTables() var R: Record "C Row"; O: Record "C Other"; Ok: Boolean; begin
    Commit(); R.Consistent(false); O.Consistent(false); asserterror Ok := Codeunit.Run(Codeunit::"C Nothing"); end;
}
"#;

#[test]
fn guarded_run_with_consistent_marks() {
    let o = outcomes(SRC);
    let get = |name: &str| {
        o.iter()
            .find(|(n, _)| n == name)
            .unwrap_or_else(|| panic!("{name} in {o:?}"))
            .1
            .clone()
    };
    assert_eq!(get("Pinned"), Outcome::Fail("DB:TableInConsistent".into()));
    match get("CommitAfterOther") {
        Outcome::NotRun(why) => assert!(why.contains("another variable"), "{why}"),
        other => panic!("CommitAfterOther: {other:?}"),
    }
    for name in ["InnerMarksAndFails", "TwoTables"] {
        match get(name) {
            Outcome::NotRun(why) => assert!(why.contains("only Commit() is pinned"), "{name}: {why}"),
            other => panic!("{name}: {other:?}"),
        }
    }
}
