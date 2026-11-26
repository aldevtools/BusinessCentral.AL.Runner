//! The guarded Codeunit.Run ends of corpus PR 1586 (ConsistentGuardedRunFail.al, codeunit 88974, section (2); al-runner
//! issue 8970) and their record-argument twins of corpus PR 1590 (ConsistentGuardedRunRecFail.al) that pass flagged with
//! `guarded-run-consistent-fail`, and the neighbors of each that stay refused.

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
table 50132 "F Z" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
codeunit 50135 "F Z Inner" { TableNo = "F Z"; trigger OnRun() var I: Codeunit "F Inner"; begin I.DoSteps(); end; }
codeunit 50136 "F X Inner" { TableNo = "F X"; trigger OnRun() var I: Codeunit "F Inner"; begin I.DoSteps(); end; }
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
  local procedure RecFails(Kind: Text; Steps: Text) var St: Codeunit "F Steps"; Z: Record "F Z"; X: Record "F X"; TX: Record "F X" temporary; Ok: Boolean; begin
    Commit(); St.Set(Steps);
    case Kind of
      'z': Ok := Codeunit.Run(Codeunit::"F Z Inner", Z);
      'zfilter': begin Z.SetRange(Id, 1); Ok := Codeunit.Run(Codeunit::"F Z Inner", Z); end;
      'x': Ok := Codeunit.Run(Codeunit::"F X Inner", X);
      'tmp': begin TX.Id := 1; TX.Insert(); Ok := Codeunit.Run(Codeunit::"F X Inner", TX); end;
    end;
    if Ok then Error('run succeeded'); Commit(); end;
  local procedure RecEnds(Kind: Text; Steps: Text) var St: Codeunit "F Steps"; Z: Record "F Z"; X: Record "F X"; TX: Record "F X" temporary; Ok: Boolean; begin
    Commit(); St.Set(Steps);
    case Kind of
      'z': asserterror Ok := Codeunit.Run(Codeunit::"F Z Inner", Z);
      'x': asserterror Ok := Codeunit.Run(Codeunit::"F X Inner", X);
      'tmp': begin TX.Id := 1; TX.Insert(); asserterror Ok := Codeunit.Run(Codeunit::"F X Inner", TX); end;
    end;
    Error(GetLastErrorCode()); end;
  [Test] procedure RecUnmarkedHeldAgainFails() begin RecFails('z', 'startx,fya,fyc,err'); end;
  [Test] procedure RecUnmarkedAgainHeldEnds() begin RecEnds('z', 'fxa,txb,fxc,fya'); end;
  [Test] procedure RecMarkedHeldAgainFails() begin RecFails('x', 'startx,fya,fyc,err'); end;
  [Test] procedure RecMarkedReleasedHeldFails() begin RecFails('x', 'startx,txb,fya,err'); end;
  [Test] procedure RecMarkedAgainLiftedEnds() begin RecEnds('x', 'fxa,txb,fxc,fya,txa,tya'); end;
  [Test] procedure RecTemporaryHeldAgainFails() begin RecFails('tmp', 'startx,fya,fyc,err'); end;
  [Test] procedure RecTemporaryHeldAgainEnds() begin RecEnds('tmp', 'startx,fya,fyc'); end;
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
  [Test] procedure RecTemporaryAgainHeldFails() begin RecFails('tmp', 'fxa,txb,fxc,fya,err'); end;
  [Test] procedure RecFilteredHeldAgainFails() begin RecFails('zfilter', 'startx,fya,fyc,err'); end;
  [Test] procedure RecUnmarkedFieldError() begin RecFails('z', 'fxg,fielderr'); end;
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
        // The same with a record argument of corpus PR 1590 (ConsistentGuardedRunRecFail.al RecZ_StartX_MarkYByA_MarkYByC_Fails,
        // RecX_StartX_MarkYByA_MarkYByC_Fails, RecX_StartX_TrueXByB_MarkYByA_Fails, RecTemp_StartX_MarkYByA_MarkYByC_Fails).
        "RecUnmarkedHeldAgainFails",
        "RecMarkedHeldAgainFails",
        "RecMarkedReleasedHeldFails",
        "RecTemporaryHeldAgainFails",
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
        // RecZ_MarkXByA_TrueXByB_MarkXByC_MarkYByA_Ends, RecX_MarkXByA_TrueXByB_MarkXByC_TrueXByA_TrueYByA_Ends,
        // RecTemp_StartX_MarkYByA_MarkYByC_Ends.
        "RecUnmarkedAgainHeldEnds",
        "RecMarkedAgainLiftedEnds",
        "RecTemporaryHeldAgainEnds",
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
    // One fact flipped from a cell each: the list of marks, the error code, the record argument (of the table marked
    // second, a temporary record in a shape PR 1590 does not measure with one, a filtered record, a record with the
    // FieldError), a caller mark, nesting, a temporary variable in the sequence.
    for name in [
        "ReleasedAloneFails",
        "AgainAloneFails",
        "HeldAgainOtherCode",
        "FieldErrorTwoMarks",
        "HeldHeldEndsAfterTemporary",
        "HeldAgainNested",
        "HeldAgainRecordArgument",
        "HeldAgainCallerMark",
        "RecTemporaryAgainHeldFails",
        "RecFilteredHeldAgainFails",
        "RecUnmarkedFieldError",
    ] {
        refused(name, "in a shape no probe measured");
    }
    // A Commit while a mark is marked again: not measured.
    refused(
        "AgainThenCommit",
        "called Consistent on its Consistent(false) mark (not measured)",
    );
}
