//! Consistent(false) marks and a guarded Codeunit.Run (al-runner issues 7586 item 4 and 8970). The pinned shapes raise
//! the inconsistency error as the run ends (errors/ConsistentGuardedRun.al) and are clean passes. The shapes of corpus
//! PR 1568 (ConsistentGuardedRunEnd.al) pass flagged with `guarded-run-consistent-end`. Every refusal the end-of-run check
//! and the Commit check have is reached here.

use super::y48_platform_probes::outcomes_with_hits;
use al_builtins::assumptions::Assumption;
use al_test_cli::report::Outcome;

const SRC: &str = r#"
table 50100 "C Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
table 50101 "C Other" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
table 50102 "C Ext" { Caption = 'C Own Caption'; fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
tableextension 50100 "C Ext Caption" extends "C Ext" { Caption = 'C Ext Caption Text'; }
codeunit 50101 "C Nothing" { trigger OnRun() begin end; }
codeunit 50103 "C Row Run" { TableNo = "C Row"; trigger OnRun() begin Rec.Id := 7; end; }
codeunit 50105 "C Other Run" { TableNo = "C Other"; trigger OnRun() begin Rec.Id := 7; end; }
codeunit 50104 "C Refused" { trigger OnRun() begin if TaskScheduler.SetTaskReady(CreateGuid()) then; end; }
codeunit 50102 "C Mark Fail" { trigger OnRun() var R: Record "C Row"; begin R.Consistent(false); Error('inner failed'); end; }
codeunit 50106 "C Mark Commit" { trigger OnRun() var R: Record "C Row"; begin R.Consistent(false); Commit(); end; }
codeunit 50107 "C Mark Divide" { trigger OnRun() var R: Record "C Row"; I: Integer; begin R.Consistent(false); I := 0; I := 1 div I; end; }
codeunit 50108 "C Nested" { trigger OnRun() var R: Record "C Row"; O: Record "C Other"; Ok: Boolean; begin
    R.Consistent(false); Ok := Codeunit.Run(Codeunit::"C Other Run", O); end; }
codeunit 50109 "C Mark Ext" { trigger OnRun() var E: Record "C Ext"; begin E.Consistent(false); end; }
codeunit 50111 "C Mark Two Fail" { trigger OnRun() var R: Record "C Row"; E: Record "C Ext"; begin
    R.Consistent(false); E.Consistent(false); Error('inner failed'); end; }
codeunit 50110 "C Get Fail" { trigger OnRun() var R: Record "C Row"; begin R.Get(99); end; }
codeunit 50000 T { Subtype = Test;
  [Test] procedure Pinned() var R: Record "C Row"; Ok: Boolean; begin
    Commit(); R.Consistent(false); asserterror Ok := Codeunit.Run(Codeunit::"C Nothing"); Error(GetLastErrorCode()); end;
  [Test] procedure CommitAfterOther() var M: Record "C Row"; U: Record "C Row"; begin
    M.Consistent(false); U.Consistent(true); asserterror Commit(); Error(GetLastErrorCode()); end;
  [Test] procedure CommitAfterTemporaryOther() var M: Record "C Row"; U: Record "C Row" temporary; begin
    M.Consistent(false); U.Consistent(true); Commit(); end;
  [Test] procedure InnerMarksAndFails() var Ok: Boolean; begin
    Commit(); Ok := Codeunit.Run(Codeunit::"C Mark Fail"); if Ok then Error('run succeeded'); Commit(); end;
  [Test] procedure InnerMarksAndFailsOtherCode() var Ok: Boolean; begin
    Commit(); Ok := Codeunit.Run(Codeunit::"C Mark Divide"); end;
  [Test] procedure InnerMarksAndCommits() var Ok: Boolean; begin
    Commit(); Ok := Codeunit.Run(Codeunit::"C Mark Commit"); end;
  [Test] procedure WithRecordOfOtherTable() var R: Record "C Row"; O: Record "C Other"; Ok: Boolean; begin
    Commit(); R.Consistent(false); asserterror Ok := Codeunit.Run(Codeunit::"C Other Run", O); Error(GetLastErrorCode()); end;
  [Test] procedure WithRecordOfMarkedTable() var R: Record "C Row"; Row: Record "C Row"; Ok: Boolean; begin
    Commit(); R.Consistent(false); asserterror Ok := Codeunit.Run(Codeunit::"C Row Run", Row); end;
  [Test] procedure WithTemporaryRecord() var R: Record "C Row"; O: Record "C Other" temporary; Ok: Boolean; begin
    Commit(); R.Consistent(false); asserterror Ok := Codeunit.Run(Codeunit::"C Other Run", O); end;
  [Test] procedure NestedRaise() var Ok: Boolean; begin
    Commit(); Ok := Codeunit.Run(Codeunit::"C Nested"); end;
  [Test] procedure ExtensionCaption() var Ok: Boolean; begin
    Commit(); asserterror Ok := Codeunit.Run(Codeunit::"C Mark Ext"); end;
  [Test] procedure CallerMarksGetFails() var R: Record "C Row"; Ok: Boolean; begin
    Commit(); R.Consistent(false); asserterror Ok := Codeunit.Run(Codeunit::"C Get Fail"); Error(GetLastErrorCode()); end;
  [Test] procedure CallerReleasedGetFails() var M: Record "C Row"; U: Record "C Row"; Ok: Boolean; begin
    Commit(); M.Consistent(false); U.Consistent(true); asserterror Ok := Codeunit.Run(Codeunit::"C Get Fail"); end;
  [Test] procedure CallerMarksOtherInnerMarksFails() var O: Record "C Other"; Ok: Boolean; begin
    Commit(); O.Consistent(false); asserterror Ok := Codeunit.Run(Codeunit::"C Mark Fail"); Error(GetLastErrorCode()); end;
  [Test] procedure CallerMarksOtherInnerMarksDivides() var O: Record "C Other"; Ok: Boolean; begin
    Commit(); O.Consistent(false); asserterror Ok := Codeunit.Run(Codeunit::"C Mark Divide"); end;
  [Test] procedure CallerMarksOtherInnerMarksTwoFails() var O: Record "C Other"; Ok: Boolean; begin
    Commit(); O.Consistent(false); asserterror Ok := Codeunit.Run(Codeunit::"C Mark Two Fail"); end;
  [Test] procedure CallerWritesNoMark() var R: Record "C Row"; Ok: Boolean; begin
    Commit(); R.Id := 1; R.Insert(); asserterror Ok := Codeunit.Run(Codeunit::"C Mark Fail"); Error(GetLastErrorCode()); end;
  [Test] procedure CallerWritesThenMarks() var R: Record "C Row"; Ok: Boolean; begin
    Commit(); R.Id := 1; R.Insert(); R.Consistent(false); asserterror Ok := Codeunit.Run(Codeunit::"C Mark Fail"); Error(GetLastErrorCode()); end;
  [Test] procedure InnerRefused() var R: Record "C Row"; Ok: Boolean; begin
    Commit(); R.Consistent(false); Ok := Codeunit.Run(Codeunit::"C Refused"); end;
  [Test] procedure TwoTables() var R: Record "C Row"; O: Record "C Other"; Ok: Boolean; begin
    Commit(); R.Consistent(false); O.Consistent(false); asserterror Ok := Codeunit.Run(Codeunit::"C Nothing"); end;
}
"#;

#[test]
fn guarded_run_with_consistent_marks() {
    let o = outcomes_with_hits(SRC, "T");
    let get = |name: &str| {
        o.iter()
            .find(|(n, _, _)| n == name)
            .map(|(_, out, hits)| (out.clone(), hits.contains(&Assumption::GuardedRunConsistentEnd)))
            .unwrap_or_else(|| panic!("{name} in {o:?}"))
    };
    // Pinned: a clean failure with the code of the inconsistency error.
    assert_eq!(get("Pinned"), (Outcome::Fail("DB:TableInConsistent".into()), false));
    // Assumed: a Commit after another database variable's Consistent(true) raises (Commit_FalseOnOneVariable_TrueOnAnother);
    // a failed inner run drops its mark (GuardedRunId_InnerMarksLocalWritesNothingAndFails); a record argument of another
    // table (GuardedRunIdRec_CallerMarks_InnerSucceeds).
    assert_eq!(
        get("CommitAfterOther"),
        (Outcome::Fail("DB:TableInConsistent".into()), true)
    );
    assert_eq!(get("InnerMarksAndFails"), (Outcome::Pass, true));
    // A caller mark raises after an inner Get fails (GuardedRunId_CallerMarks_InnerGetFails), and after the inner
    // codeunit marks another table and fails (GuardedRunId_CallerMarksOther_InnerStartsStateAndFails).
    assert_eq!(
        get("CallerMarksGetFails"),
        (Outcome::Fail("DB:TableInConsistent".into()), true)
    );
    assert_eq!(
        get("CallerMarksOtherInnerMarksFails"),
        (Outcome::Fail("DB:TableInConsistent".into()), true)
    );
    assert_eq!(
        get("WithRecordOfOtherTable"),
        (Outcome::Fail("DB:TableInConsistent".into()), true)
    );
    let refused = |name: &str, text: &str| match get(name).0 {
        Outcome::NotRun(why) => assert!(why.contains(text), "{name}: {why}"),
        other => panic!("{name}: {other:?}"),
    };
    refused(
        "CommitAfterTemporaryOther",
        "called Consistent on its Consistent(false) mark (not measured)",
    );
    for name in [
        "InnerMarksAndFailsOtherCode",
        "InnerMarksAndCommits",
        "CallerReleasedGetFails",
        "CallerMarksOtherInnerMarksDivides",
        "CallerMarksOtherInnerMarksTwoFails",
        "WithRecordOfMarkedTable",
        "WithTemporaryRecord",
        "NestedRaise",
        "TwoTables",
    ] {
        refused(name, "in a shape no probe measured");
    }
    refused(
        "ExtensionCaption",
        "whose caption a tableextension or a translation sets",
    );
    // A guarded run while the caller holds a write: the pending-write error, with or without a mark. With a mark BC gave
    // this error or the inconsistency error between CI runs of corpus PR 1568 (ConsistentGuardedRunEnd.al:955, :973);
    // main's answer is one of them and stays, unflagged.
    assert_eq!(get("CallerWritesNoMark"), (Outcome::Fail("DB:Ok".into()), false));
    assert_eq!(get("CallerWritesThenMarks"), (Outcome::Fail("DB:Ok".into()), false));
    // An inner refusal keeps its own reason: the Consistent mark does not replace it.
    refused("InnerRefused", "SetTaskReady");
}
