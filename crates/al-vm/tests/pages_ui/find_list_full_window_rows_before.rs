//! The open read of a list or part with OnFindRecord, OnNextRecord and OnAfterGetRecord but no OnAfterGetCurrRecord, whose first
//! window of 20 rows is full and whose OnNextRecord(-1) from the first row answers a step: the client steps back over every row
//! above the first, reading each, until OnNextRecord(-1) answers 0; no OnFindRecord('=') and no read of the first row again
//! (al-runner issue 5887, the reporter's run on real BC 29.0.54011.55407, 2/2 tests; assumption list-open-full-window-rows-before
//! until corpus writer SF's pin lands). The fixture is the reporter's repro.

use al_testkit::{Built, text};

const SRC: &str = r##"table 50100 "Day Probe Row"
{
    TableType = Temporary;

    fields
    {
        field(1; Day; Date) { }
        field(2; "Day No."; Integer) { }
    }

    keys
    {
        key(PK; Day) { Clustered = true; }
    }
}

table 50101 "Day Probe Host"
{
    fields
    {
        field(1; Code; Code[10]) { }
    }

    keys
    {
        key(PK; Code) { Clustered = true; }
    }
}

codeunit 50100 "Day Probe Log"
{
    SingleInstance = true;

    var
        Entries: Text;

    procedure Add(Entry: Text)
    begin
        Entries += Entry;
    end;

    procedure Take(): Text
    var
        Result: Text;
    begin
        Result := Entries;
        Entries := '';
        exit(Result);
    end;
}

codeunit 50101 "Day Probe Walker"
{
    var
        Log: Codeunit "Day Probe Log";

    procedure DayNo(Day: Date): Integer
    begin
        if Day = 0D then
            exit(0);
        exit(Day - 20291231D);
    end;

    local procedure FirstDay(): Date
    begin
        exit(20300101D);
    end;

    local procedure LastDay(): Date
    begin
        exit(20300209D);
    end;

    local procedure StartDay(): Date
    begin
        exit(20300110D);
    end;

    procedure Find(var Row: Record "Day Probe Row" temporary; var Pos: Date; Which: Text): Boolean
    var
        i: Integer;
        Found: Boolean;
    begin
        if Pos = 0D then
            Pos := StartDay();
        for i := 1 to StrLen(Which) do
            if not Found then
                case Which[i] of
                    '=':
                        Found := (Pos >= FirstDay()) and (Pos <= LastDay());
                    '>':
                        if Pos < LastDay() then begin
                            Pos := Pos + 1;
                            Found := true;
                        end;
                    '<':
                        if Pos > FirstDay() then begin
                            Pos := Pos - 1;
                            Found := true;
                        end;
                    '-':
                        begin
                            Pos := FirstDay();
                            Found := true;
                        end;
                    '+':
                        begin
                            Pos := LastDay();
                            Found := true;
                        end;
                end;
        if Found then
            Fill(Row, Pos);
        Log.Add('F(' + Which + ')=' + YesNo(Found) + ':' + Format(DayNo(Pos)) + ';');
        exit(Found);
    end;

    procedure Next(var Row: Record "Day Probe Row" temporary; var Pos: Date; Steps: Integer): Integer
    var
        Target: Date;
        Result: Integer;
    begin
        if Row.Day <> Pos then
            Pos := Row.Day;
        Target := Pos + Steps;
        if Target < FirstDay() then
            Target := FirstDay();
        if Target > LastDay() then
            Target := LastDay();
        Result := Target - Pos;
        if Result <> 0 then begin
            Pos := Target;
            Fill(Row, Pos);
        end;
        Log.Add('N(' + Format(Steps) + ')=' + Format(Result) + ':' + Format(DayNo(Pos)) + ';');
        exit(Result);
    end;

    procedure Read(var Row: Record "Day Probe Row" temporary; var Pos: Date)
    begin
        Pos := Row.Day;
        Log.Add('R' + Format(DayNo(Row.Day)) + ';');
    end;

    local procedure Fill(var Row: Record "Day Probe Row" temporary; Day: Date)
    begin
        if not Row.Get(Day) then begin
            Row.Init();
            Row.Day := Day;
            Row."Day No." := DayNo(Day);
            Row.Insert();
        end;
    end;

    local procedure YesNo(Value: Boolean): Text
    begin
        if Value then
            exit('Y');
        exit('N');
    end;
}

page 50102 "Day Probe Part"
{
    PageType = ListPart;
    SourceTable = "Day Probe Row";
    SourceTableTemporary = true;
    Editable = false;
    InsertAllowed = false;
    DeleteAllowed = false;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Day No."; Rec."Day No.") { ApplicationArea = All; }
            }
        }
    }

    trigger OnFindRecord(Which: Text): Boolean
    begin
        exit(Walker.Find(Rec, Pos, Which));
    end;

    trigger OnNextRecord(Steps: Integer): Integer
    begin
        exit(Walker.Next(Rec, Pos, Steps));
    end;

    trigger OnAfterGetRecord()
    begin
        Walker.Read(Rec, Pos);
    end;

    var
        Walker: Codeunit "Day Probe Walker";
        Log: Codeunit "Day Probe Log";
        Pos: Date;

    procedure Refill(HostCode: Code[10])
    begin
        Log.Add('U(' + HostCode + '):' + Format(Walker.DayNo(Pos)) + ';');
        Rec.DeleteAll();
        CurrPage.Update(false);
    end;
}

page 50103 "Day Probe List"
{
    PageType = List;
    SourceTable = "Day Probe Row";
    SourceTableTemporary = true;
    Editable = false;
    InsertAllowed = false;
    DeleteAllowed = false;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Day No."; Rec."Day No.") { ApplicationArea = All; }
            }
        }
    }

    trigger OnFindRecord(Which: Text): Boolean
    begin
        exit(Walker.Find(Rec, Pos, Which));
    end;

    trigger OnNextRecord(Steps: Integer): Integer
    begin
        exit(Walker.Next(Rec, Pos, Steps));
    end;

    trigger OnAfterGetRecord()
    begin
        Walker.Read(Rec, Pos);
    end;

    var
        Walker: Codeunit "Day Probe Walker";
        Pos: Date;
}

page 50104 "Day Probe Host"
{
    PageType = ListPlus;
    SourceTable = "Day Probe Host";
    InsertAllowed = false;
    DeleteAllowed = false;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(General)
            {
                field(Code; Rec.Code) { ApplicationArea = All; }
            }
            part(Lines; "Day Probe Part")
            {
                ApplicationArea = All;
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
        CurrPage.Lines.Page.Refill(Rec.Code);
    end;
}

codeunit 50120 T
{
    var Log: Codeunit "Day Probe Log";

    procedure ListOpens(): Text
    var List: TestPage "Day Probe List"; Observed: Text;
    begin
        Log.Take();
        List.OpenView();
        Observed := 'open=' + List."Day No.".Value() + ':' + Log.Take();
        List.Next();
        Observed += ' next=' + List."Day No.".Value() + ':' + Log.Take();
        List.Previous();
        Observed += ' previous=' + List."Day No.".Value() + ':' + Log.Take();
        List.Close();
        exit(Observed);
    end;

    procedure ListLast()
    var List: TestPage "Day Probe List";
    begin
        List.OpenView();
        List.Last();
    end;

    procedure ListFirst()
    var List: TestPage "Day Probe List";
    begin
        List.OpenView();
        List.First();
    end;

    procedure HostOpensPart(): Text
    var HostRow: Record "Day Probe Host"; Host: TestPage "Day Probe Host"; Observed: Text;
    begin
        HostRow.DeleteAll();
        HostRow.Code := 'A'; HostRow.Insert();
        HostRow.Code := 'B'; HostRow.Insert();
        Log.Take();
        HostRow.SetRange(Code, 'A');
        Host.Trap();
        Page.Run(Page::"Day Probe Host", HostRow);
        Observed := 'open=' + Host.Lines."Day No.".Value() + ':' + Log.Take();
        Observed += ' filter=' + Host.Filter.GetFilter(Code);
        Host.Close();
        exit(Observed);
    end;
}
"##;

const LIST: &str = "open=10:F(=><)=Y:10;R10;N(1)=1:11;R11;N(1)=1:12;R12;N(1)=1:13;R13;N(1)=1:14;R14;N(1)=1:15;R15;N(1)=1:16;R16;N(1)=1:17;R17;N(1)=1:18;R18;N(1)=1:19;R19;N(1)=1:20;R20;N(1)=1:21;R21;N(1)=1:22;R22;N(1)=1:23;R23;N(1)=1:24;R24;N(1)=1:25;R25;N(1)=1:26;R26;N(1)=1:27;R27;N(1)=1:28;R28;N(1)=1:29;R29;R10;N(-1)=-1:9;R9;N(-1)=-1:8;R8;N(-1)=-1:7;R7;N(-1)=-1:6;R6;N(-1)=-1:5;R5;N(-1)=-1:4;R4;N(-1)=-1:3;R3;N(-1)=-1:2;R2;N(-1)=-1:1;R1;N(-1)=0:1; next=11: previous=10:";
const HOST: &str = "open=10:U(A):0;F(=><)=Y:10;R10;N(1)=1:11;R11;N(1)=1:12;R12;N(1)=1:13;R13;N(1)=1:14;R14;N(1)=1:15;R15;N(1)=1:16;R16;N(1)=1:17;R17;N(1)=1:18;R18;N(1)=1:19;R19;N(1)=1:20;R20;N(1)=1:21;R21;N(1)=1:22;R22;N(1)=1:23;R23;N(1)=1:24;R24;N(1)=1:25;R25;N(1)=1:26;R26;N(1)=1:27;R27;N(1)=1:28;R28;N(1)=1:29;R29;R10;N(-1)=-1:9;R9;N(-1)=-1:8;R8;N(-1)=-1:7;R7;N(-1)=-1:6;R6;N(-1)=-1:5;R5;N(-1)=-1:4;R4;N(-1)=-1:3;R3;N(-1)=-1:2;R2;N(-1)=-1:1;R1;N(-1)=0:1; filter=A";

#[test]
fn a_full_first_window_with_rows_before_steps_back_over_every_row_above() {
    use al_builtins::assumptions::{self, Assumption};
    let mut b = Built::from_source(SRC);
    assumptions::clear_hits();
    assert_eq!(text(b.run_in("T", "ListOpens")), LIST);
    // Nine rows above: pinned (PnF06FindListMoves.al:1947); only the stop at a full window above is assumed.
    assert!(!assumptions::take_hits().contains(&Assumption::ListOpenFullWindowRowsBefore));
}

#[test]
fn a_part_with_a_full_first_window_steps_back_the_same_way() {
    let mut b = Built::from_source(SRC);
    assert_eq!(text(b.run_in("T", "HostOpensPart")), HOST);
}

// Not pinned: the other moves on the rows such an open holds are refused.
#[test]
fn other_moves_after_such_an_open_are_refused() {
    let mut b = Built::from_source(SRC);
    for p in ["ListLast", "ListFirst"] {
        let r = b.run_in("T", p);
        assert!(
            matches!(r, Err(al_builtins::native::Signal::Unsupported(_))),
            "{p}: {r:?}"
        );
    }
}
