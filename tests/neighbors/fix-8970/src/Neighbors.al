table 50130 "F X" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
table 50131 "F Y" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
codeunit 50130 "F Steps" { SingleInstance = true; var S: Text;
    procedure Set(T: Text) begin S := T; end;
    procedure Get(): Text begin exit(S); end; }
codeunit 50131 "F Inner" { trigger OnRun() begin DoSteps(); end;
    var G: Record "F X";
    procedure DoSteps() var St: Codeunit "F Steps"; XA: Record "F X"; XB: Record "F X"; XC: Record "F X"; XD: Record "F X";
        YA: Record "F Y"; YC: Record "F Y"; T: Record "F X" temporary; I: Integer; Step: Text;
    begin
        foreach Step in St.Get().Split(',') do
            case Step of
                'startx': StartX();
                'fxa': XA.Consistent(false); 'fxc': XC.Consistent(false); 'txa': XA.Consistent(true);
                'txb': XB.Consistent(true); 'fxb': XB.Consistent(false); 'txd': XD.Consistent(true); 'txc': XC.Consistent(true); 'tya': YA.Consistent(true); 'fya': YA.Consistent(false); 'fyc': YC.Consistent(false);
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
table 50132 "F Z" { fields { field(1; Id; Integer) { } field(2; "Date Filter"; Date) { FieldClass = FlowFilter; } } keys { key(PK; Id) { Clustered = true; } } }
codeunit 50135 "F Z Inner" { TableNo = "F Z"; trigger OnRun() var I: Codeunit "F Inner"; begin I.DoSteps(); end; }
codeunit 50137 "F Zf Inner" { TableNo = "F Z"; trigger OnRun() var I: Codeunit "F Inner"; begin Rec.SetRange(Id, 2); I.DoSteps(); end; }
codeunit 50138 "F Zg Inner" { TableNo = "F Z"; trigger OnRun() var I: Codeunit "F Inner"; begin Rec.FilterGroup(2); I.DoSteps(); end; }
codeunit 50140 "F X Mark Inner" { TableNo = "F X"; trigger OnRun() var I: Codeunit "F Inner"; begin Rec.Consistent(false); I.DoSteps(); end; }
codeunit 50136 "F X Inner" { TableNo = "F X"; trigger OnRun() var I: Codeunit "F Inner"; begin I.DoSteps(); end; }
codeunit 50133 "F Wrap" { trigger OnRun() var St: Codeunit "F Steps"; begin St.Set('startx,fya,fyc,err'); Codeunit.Run(Codeunit::"F Inner"); end; }
codeunit 50134 "F Neighbor Tests" { Subtype = Test;
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
  // Record arguments (ConsistentGuardedRunRecFail.al): a row of a table no mark names, a row of the table marked first, a
  // temporary record of it; and one fact flipped each: a filter, a FieldError, a temporary record in another sequence.
  local procedure RecFails(Kind: Text; Steps: Text) var St: Codeunit "F Steps"; Z: Record "F Z"; X: Record "F X"; TX: Record "F X" temporary; TY: Record "F Y" temporary; Ok: Boolean; begin
    Commit(); St.Set(Steps);
    case Kind of
      'z': Ok := Codeunit.Run(Codeunit::"F Z Inner", Z);
      'zfilter': begin Z.SetRange(Id, 1); Ok := Codeunit.Run(Codeunit::"F Z Inner", Z); end;
      'zgroup2': begin Z.FilterGroup(2); Z.SetRange(Id, 1); Z.FilterGroup(0); Ok := Codeunit.Run(Codeunit::"F Z Inner", Z); end;
      'zor': begin Z.SetFilter(Id, '1|2'); Ok := Codeunit.Run(Codeunit::"F Z Inner", Z); end;
      'zinnerfilter': Ok := Codeunit.Run(Codeunit::"F Zf Inner", Z);
      'zinnergroup': Ok := Codeunit.Run(Codeunit::"F Zg Inner", Z);
      'zflowfilter': begin Z.SetRange("Date Filter", 20260101D); Ok := Codeunit.Run(Codeunit::"F Z Inner", Z); end;
      'zmarked': begin Z.Id := 1; Z.Insert(); Commit(); Z.Mark(true); Ok := Codeunit.Run(Codeunit::"F Z Inner", Z); end;
      'tmpyfilter': begin TY.Id := 1; TY.Insert(); TY.SetRange(Id, 1); Ok := Codeunit.Run(Codeunit::"F Rec Inner", TY); end;
      'x': Ok := Codeunit.Run(Codeunit::"F X Inner", X);
      'xmark': Ok := Codeunit.Run(Codeunit::"F X Mark Inner", X);
      'zgroup2nofilter': begin Z.FilterGroup(2); Ok := Codeunit.Run(Codeunit::"F Z Inner", Z); end;
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
  // Filtered record arguments (ConsistentGuardedRunRecFiltered.al), one fact flipped each: a filter in filter group 2, another
  // SetFilter pattern, a filter the inner codeunit sets, a filtered temporary record of the table marked second, a mark on
  // the argument's table set by another variable, a FieldError after a filter, a Commit inside a run without a record.
  [Test] procedure RecFilterGroup2HeldAgainFails() begin RecFails('zgroup2', 'startx,fya,fyc,err'); end;
  [Test] procedure RecFilterOrValuesHeldAgainFails() begin RecFails('zor', 'startx,fya,fyc,err'); end;
  [Test] procedure RecFilterSetByInnerHeldAgainFails() begin RecFails('zinnerfilter', 'startx,fya,fyc,err'); end;
  [Test] procedure RecFilteredTemporarySecondHeldAgainFails() begin RecFails('tmpyfilter', 'startx,fya,fyc,err'); end;
  [Test] procedure RecMarkedByOtherVariableFails() begin RecFails('x', 'fxa,err'); end;
  [Test] procedure RecFilteredFieldError() begin RecFails('zfilter', 'fxg,fielderr'); end;
  [Test] procedure HeldHeldCommitNoRecord() begin Fails('fxa,fya,commit'); end;
  // Fix round 1: a mark a temporary variable set; the releaser marking again; Consistent(true) on a mark marked again by a
  // variable other than the setter and the one that marked it again; a FlowFilter range; marked rows without MarkedOnly; a
  // filter group the inner codeunit leaves changed.
  [Test] procedure TemporarySetterHeldAgainFails() begin Fails('ftmp,fya,fyc,err'); end;
  [Test] procedure TemporarySetterMarkedAgainFails() begin Fails('ftmp,fxa,fya,err'); end;
  [Test] procedure ReleaserMarksAgainFails() begin Fails('fxa,txb,fxb,fya,err'); end;
  [Test] procedure RecFlowFilterSingleValueHeldAgainFails() begin RecFails('zflowfilter', 'startx,fya,fyc,err'); end;
  [Test] procedure RecMarkedRowNoMarkedOnlyHeldAgainFails() begin RecFails('zmarked', 'startx,fya,fyc,err'); end;
  [Test] procedure RecFilterGroupLeftByInnerHeldAgainFails() begin RecFails('zinnergroup', 'startx,fya,fyc,err'); end;
  // Fix round 2: the argument marks its own table and a second mark follows (the probe measures it only alone); the caller's
  // filter group is 2 with no filter.
  [Test] procedure RecXArgMarksThenHeldFails() begin RecFails('xmark', 'fya,err'); end;
  [Test] procedure RecXArgMarksThenHeldAgainFails() begin RecFails('xmark', 'fya,fyc,err'); end;
  [Test] procedure RecCallerGroupTwoNoFilterHeldAgainFails() begin RecFails('zgroup2nofilter', 'startx,fya,fyc,err'); end;
}
// The successful runs of fix round 1, in a codeunit of their own: an earlier refused test of a codeunit turns a later
// failing one into not run.
codeunit 50139 "F Neighbor Ends Tests" { Subtype = Test;
  local procedure Ends(Steps: Text) var St: Codeunit "F Steps"; Ok: Boolean; begin
    Commit(); St.Set(Steps); asserterror Ok := Codeunit.Run(Codeunit::"F Inner"); Error(GetLastErrorCode()); end;
  [Test] procedure TemporarySetterHeldAgainEnds() begin Ends('ftmp,fya,fyc'); end;
  [Test] procedure ReleaserTrueOnAgainEnds() begin Ends('fxa,txb,fxc,fya,txb,tya'); end;
  [Test] procedure OtherTrueOnAgainEnds() begin Ends('fxa,txb,fxc,fya,txd,tya'); end;
}
