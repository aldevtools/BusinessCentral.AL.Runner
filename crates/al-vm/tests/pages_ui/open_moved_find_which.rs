//! A list with OnFindRecord whose OnOpenPage left Rec on a row that is not the first of its view is asked `=>` when it opens
//! (pages/lt-ai-list-order/AiListOrderTests.al:851 OpenOnRow_SetPositionInOnOpenPage_LastRow_OpenLog `F:=>=Yes;R:R1006;N:1=0;
//! N:-1=-1;R:R1005;...`, :871 _SecondLastRow_OpenLog `F:=>=Yes;R:R1005;N:1=1;R:R1006;N:1=0;R:R1005;N:-1=-1;...`). On the last row
//! the opening row is not read again before OnNextRecord(-1); on the row before it, it is.

use al_testkit::{Built, text};

const SRC: &str = r##"table 50100 "Pos Row"
{
    fields
    {
        field(1; "No."; Code[20]) { }
    }
    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

codeunit 50100 "Pos Log"
{
    SingleInstance = true;
    var
        Entries: Text;
        StartKey: Code[20];

    procedure Add(Entry: Text)
    begin
        if Entries <> '' then
            Entries += ';';
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

    procedure SetStart(K: Code[20])
    begin
        StartKey := K;
    end;

    procedure Start(): Code[20]
    begin
        exit(StartKey);
    end;
}

page 50100 "Pos List"
{
    PageType = List;
    SourceTable = "Pos Row";
    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(NoCtl; Rec."No.") { }
            }
        }
    }

    trigger OnOpenPage()
    var
        Log: Codeunit "Pos Log";
    begin
        if Log.Start() <> '' then begin
            Rec."No." := Log.Start();
            Rec.SetPosition(Rec.GetPosition());
            Rec.Get(Log.Start());
        end;
    end;

    trigger OnFindRecord(Which: Text): Boolean
    var
        Log: Codeunit "Pos Log";
        Found: Boolean;
    begin
        Found := Rec.Find(Which);
        Log.Add('F:' + Which + '=' + Format(Found));
        exit(Found);
    end;

    trigger OnNextRecord(Steps: Integer): Integer
    var
        Log: Codeunit "Pos Log";
        Moved: Integer;
    begin
        Moved := Rec.Next(Steps);
        Log.Add('N:' + Format(Steps) + '=' + Format(Moved));
        exit(Moved);
    end;

    trigger OnAfterGetRecord()
    var
        Log: Codeunit "Pos Log";
    begin
        Log.Add('R:' + Rec."No.");
    end;
}

codeunit 50101 T
{
    procedure Open(Start: Code[20]): Text
    var
        Row: Record "Pos Row";
        Log: Codeunit "Pos Log";
        P: TestPage "Pos List";
        i: Integer;
        Result: Text;
    begin
        Row.DeleteAll();
        for i := 1 to 6 do begin
            Row."No." := 'R' + Format(1000 + i);
            Row.Insert();
        end;
        Log.SetStart(Start);
        Log.Take();
        P.OpenView();
        Result := Log.Take() + '|' + P.NoCtl.Value();
        P.Close();
        exit(Result);
    end;

    procedure OpenLast(): Text
    begin
        exit(Open('R1006'));
    end;

    procedure OpenSecondLast(): Text
    begin
        exit(Open('R1005'));
    end;

    procedure OpenPlain(): Text
    begin
        exit(Open(''));
    end;
}
"##;

#[test]
fn the_last_row_set_in_onopenpage_is_found_with_greater_and_not_read_again() {
    let mut b = Built::from_source(SRC);
    assert_eq!(
        text(b.run_in("T", "OpenLast")),
        "F:=>=Yes;R:R1006;N:1=0;N:-1=-1;R:R1005;N:-1=-1;R:R1004;N:-1=-1;R:R1003;N:-1=-1;R:R1002;N:-1=-1;R:R1001;N:-1=0|R1006"
    );
}

#[test]
fn the_second_last_row_set_in_onopenpage_is_read_again_before_stepping_back() {
    let mut b = Built::from_source(SRC);
    assert_eq!(
        text(b.run_in("T", "OpenSecondLast")),
        "F:=>=Yes;R:R1005;N:1=1;R:R1006;N:1=0;R:R1005;N:-1=-1;R:R1004;N:-1=-1;R:R1003;N:-1=-1;R:R1002;N:-1=-1;R:R1001;N:-1=0|R1005"
    );
}

/// Without a row set in OnOpenPage the list is asked `=><` (AiListOrderTests.al:37).
#[test]
fn without_a_row_set_in_onopenpage_the_open_find_is_unchanged() {
    let mut b = Built::from_source(SRC);
    assert!(text(b.run_in("T", "OpenPlain")).starts_with("F:=><=Yes;R:R1001;"));
}
