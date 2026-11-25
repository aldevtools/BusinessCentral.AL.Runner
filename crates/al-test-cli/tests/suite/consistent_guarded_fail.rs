//! The guarded Codeunit.Run ends of corpus PR 1586 (ConsistentGuardedRunFail.al, codeunit 88974, section (2); al-runner
//! issue 8970) that pass flagged with `guarded-run-consistent-fail`, and the neighbors of each that stay refused.

use super::y48_platform_probes::outcomes_with_hits;
use al_builtins::assumptions::Assumption;
use al_test_cli::report::Outcome;

const SRC: &str = r#"
table 50130 "F X" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
table 50131 "F Y" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
codeunit 50130 "F Steps" { SingleInstance = true; var S: Text;
    procedure Set(T: Text) begin S := T; end;
    procedure Get(): Text begin exit(S); end; }
codeunit 50131 "F Inner" { trigger OnRun() begin DoSteps(); end;
    var G: Record "F X";
    procedure DoSteps() var St: Codeunit "F Steps"; XA: Record "F X"; XB: Record "F X"; XC: Record "F X";
        YA: Record "F Y"; YC: Record "F Y"; T: Record "F X" temporary; I: Integer; Step: Text;
    begin
        foreach Step in St.Get().Split(',') do
            case Step of
                'startx': StartX();
                'fxa': XA.Consistent(false); 'fxc': XC.Consistent(false); 'txa': XA.Consistent(true);
                'txb': XB.Consistent(true); 'txc': XC.Consistent(true); 'tya': YA.Consistent(true); 'fya': YA.Consistent(false); 'fyc': YC.Consistent(false);
                'fxg': G.Consistent(false); 'ftmp': T.Consistent(false);
                'err': Error('inner failed');
                'div': begin I := 0; I := 1 div I; end;
                'fielderr': begin YA.Init(); YA.FieldError(Id, 'must be set'); end;
                'commit': Commit();
                'nested': if Codeunit.Run(Codeunit::"F Wrap") then;
            end;
    end;
    local procedure StartX() var X: Record "F X"; begin X.Consistent(false); end; }
codeunit 50132 "F Rec Inner" { TableNo = "F Y"; trigger OnRun() var I: Codeunit "F Inner"; begin I.DoSteps(); end; }
codeunit 50133 "F Wrap" { trigger OnRun() var St: Codeunit "F Steps"; begin St.Set('startx,fya,fyc,err'); Codeunit.Run(Codeunit::"F Inner"); end; }
codeunit 50000 T { Subtype = Test;
  local procedure Fails(Steps: Text) var St: Codeunit "F Steps"; begin
    Commit(); St.Set(Steps); if Codeunit.Run(Codeunit::"F Inner") then Error('run succeeded'); Commit(); end;
  local procedure Ends(Steps: Text) var St: Codeunit "F Steps"; Ok: Boolean; begin
    Commit(); St.Set(Steps); asserterror Ok := Codeunit.Run(Codeunit::"F Inner"); Error(GetLastErrorCode()); end;
  [Test] procedure HeldAgainFails() begin Fails('startx,fya,fyc,err'); end;
  [Test] procedure ReleasedHeldFails() begin Fails('startx,txb,fya,err'); end;
  [Test] procedure AgainHeldFails() begin Fails('fxa,txb,fxc,fya,err'); end;
  [Test] procedure FieldErrorFails() var St: Codeunit "F Steps"; begin
    Commit(); St.Set('fxg,fielderr'); if Codeunit.Run(Codeunit::"F Inner") then Error('run succeeded');
    if GetLastErrorCode() <> 'NCLCSRTS:TableErrorStr' then Error(GetLastErrorCode()); Commit(); end;
  [Test] procedure HeldAgainEnds() begin Ends('startx,fya,fyc'); end;
  [Test] procedure AgainHeldEnds() begin Ends('fxa,txb,fxc,fya'); end;
  [Test] procedure AgainLiftedByTrueEnds() begin Ends('fxa,txb,fxc,fya,txa,tya'); end;
  [Test] procedure AgainLiftedByAgainTrueEnds() begin Ends('fxa,txb,fxc,fya,txc,tya'); end;
  [Test] procedure ReleasedAloneFails() begin Fails('startx,txb,err'); end;
  [Test] procedure AgainAloneFails() begin Fails('fya,fyc,err'); end;
  [Test] procedure HeldAgainOtherCode() begin Fails('startx,fya,fyc,div'); end;
  [Test] procedure FieldErrorTwoMarks() begin Fails('fxg,fya,fielderr'); end;
  [Test] procedure HeldHeldEndsAfterTemporary() begin Ends('fxa,ftmp,fya'); end;
  [Test] procedure AgainThenCommit() begin Fails('fya,fyc,commit'); end;
  [Test] procedure HeldAgainNested() begin Fails('nested,err'); end;
  [Test] procedure HeldAgainRecordArgument() var St: Codeunit "F Steps"; Y: Record "F Y"; Ok: Boolean; begin
    Commit(); St.Set('startx,fya,fyc,err'); Ok := Codeunit.Run(Codeunit::"F Rec Inner", Y); end;
  [Test] procedure HeldAgainCallerMark() var St: Codeunit "F Steps"; Y: Record "F Y"; Ok: Boolean; begin
    Commit(); Y.Consistent(false); St.Set('startx,fya,fyc,err'); asserterror Ok := Codeunit.Run(Codeunit::"F Inner"); end;
}
"#;

#[test]
fn guarded_run_fail_probe_shapes() {
    let o = outcomes_with_hits(SRC, "T");
    let get = |name: &str| {
        o.iter()
            .find(|(n, _, _)| n == name)
            .map(|(_, out, hits)| (out.clone(), hits.contains(&Assumption::GuardedRunConsistentFail)))
            .unwrap_or_else(|| panic!("{name} in {o:?}"))
    };
    // A failed run drops its marks: held then marked again (Inner_StartX_MarkYByA_MarkYByC_Fails), released then held
    // (Inner_StartX_TrueXByB_MarkYByA_Fails), marked again then held (Inner_MarkXByA_TrueXByB_MarkXByC_MarkYByA_Fails), a
    // FieldError after a global's mark (Inner_MarkGlobal_FieldError).
    for name in [
        "HeldAgainFails",
        "ReleasedHeldFails",
        "AgainHeldFails",
        "FieldErrorFails",
    ] {
        assert_eq!(get(name), (Outcome::Pass, true), "{name}");
    }
    // A successful run with a mark marked again raises (Inner_StartX_MarkYByA_MarkYByC_Ends,
    // Inner_MarkXByA_TrueXByB_MarkXByC_MarkYByA_Ends, Inner_MarkXByA_TrueXByB_MarkXByC_TrueXByA_TrueYByA_Ends, _TrueXByC_TrueYByA_Ends).
    for name in [
        "HeldAgainEnds",
        "AgainHeldEnds",
        "AgainLiftedByTrueEnds",
        "AgainLiftedByAgainTrueEnds",
    ] {
        assert_eq!(
            get(name),
            (Outcome::Fail("DB:TableInConsistent".into()), true),
            "{name}"
        );
    }
    let refused = |name: &str, text: &str| match get(name).0 {
        Outcome::NotRun(why) => assert!(why.contains(text), "{name}: {why}"),
        other => panic!("{name}: {other:?}"),
    };
    // One fact flipped from a cell each: the list of marks, the error code, the record argument, a caller mark, nesting,
    // a temporary variable in the sequence.
    for name in [
        "ReleasedAloneFails",
        "AgainAloneFails",
        "HeldAgainOtherCode",
        "FieldErrorTwoMarks",
        "HeldHeldEndsAfterTemporary",
        "HeldAgainNested",
        "HeldAgainRecordArgument",
        "HeldAgainCallerMark",
    ] {
        refused(name, "in a shape no probe measured");
    }
    // A Commit while a mark is marked again: not measured.
    refused(
        "AgainThenCommit",
        "called Consistent on its Consistent(false) mark (not measured)",
    );
}
