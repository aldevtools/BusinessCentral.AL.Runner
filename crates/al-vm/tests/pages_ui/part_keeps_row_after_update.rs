//! Writer TH (al-runner issue 6038), pages/lt-th-part-update-cursor/ThPartUpdateCursor.al on branch al-runtime-rs-staging-pub-th:
//! a ListPart of a Document page whose control's OnValidate runs CurrPage.Update(true) or CurrPage.Update(false) stays on its row,
//! and the usual First/SetValue/Next walk visits every line once. The objects are copied from the probe; the procedures return
//! what the probe observes, and each Rust test compares it with the values the probe asserts on real BC.

use al_builtins::assumptions::{clear_hits, take_hits};

const SRC: &str = r#"
table 89880 "LT TH Doc"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "No."; Code[20]) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

table 89881 "LT TH Line"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Document No."; Code[20]) { }
        field(2; "Line No."; Integer) { }
        field(3; Quantity; Decimal) { }
        field(4; "Qty. to Handle"; Decimal) { }
        field(5; "Qty. to Ship"; Decimal) { }
        field(6; Note; Text[30]) { }
        field(7; "Bin Code"; Code[20]) { }
    }

    keys
    {
        key(PK; "Document No.", "Line No.") { Clustered = true; }
        key(Qty; "Document No.", Quantity) { }
    }
}

codeunit 89884 "LT TH Log"
{
    SingleInstance = true;

    var
        Entries: Text;

    procedure Reset()
    begin
        Entries := '';
    end;

    procedure Add(Entry: Text)
    begin
        Entries += Entry + ' ';
    end;

    procedure Get(): Text
    begin
        exit(Entries);
    end;
}

page 89882 "LT TH Doc"
{
    PageType = Document;
    SourceTable = "LT TH Doc";
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
            part(Lines; "LT TH Lines")
            {
                ApplicationArea = All;
                SubPageLink = "Document No." = field("No.");
            }
        }
    }

    trigger OnAfterGetRecord()
    var
        Log: Codeunit "LT TH Log";
    begin
        Log.Add('HR');
    end;

    trigger OnAfterGetCurrRecord()
    var
        Log: Codeunit "LT TH Log";
    begin
        Log.Add('HC');
    end;
}

page 89886 "LT TH Doc Sorted"
{
    PageType = Document;
    SourceTable = "LT TH Doc";
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
            part(Lines; "LT TH Lines")
            {
                ApplicationArea = All;
                SubPageLink = "Document No." = field("No.");
                SubPageView = sorting(Quantity);
            }
        }
    }
}

page 89883 "LT TH Lines"
{
    PageType = ListPart;
    SourceTable = "LT TH Line";
    InsertAllowed = false;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Group)
            {
                field("Line No."; Rec."Line No.") { ApplicationArea = All; }
                field(Quantity; Rec.Quantity) { ApplicationArea = All; }
                field("Qty. to Handle"; Rec."Qty. to Handle")
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    var
                        Log: Codeunit "LT TH Log";
                    begin
                        Log.Add('QH.V');
                        CurrPage.Update(true);
                        Log.Add('QH.V.end');
                    end;
                }
                field("Qty. to Ship"; Rec."Qty. to Ship")
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    var
                        Log: Codeunit "LT TH Log";
                    begin
                        Log.Add('QS.V');
                        CurrPage.Update(false);
                        Log.Add('QS.V.end');
                    end;
                }
                field(Note; Rec.Note) { ApplicationArea = All; }
                field("Bin Code"; Rec."Bin Code")
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    var
                        Log: Codeunit "LT TH Log";
                    begin
                        Log.Add('BC.V');
                        CurrPage.SaveRecord();
                        Log.Add('BC.V.end');
                    end;
                }
            }
        }
    }

    trigger OnAfterGetRecord()
    var
        Log: Codeunit "LT TH Log";
    begin
        Log.Add('R' + Format(Rec."Line No." div 10000));
    end;

    trigger OnAfterGetCurrRecord()
    var
        Log: Codeunit "LT TH Log";
    begin
        Log.Add('C' + Format(Rec."Line No." div 10000));
    end;
}

codeunit 89990 "LT TH Probe"
{
    local procedure Seed()
    var
        Doc: Record "LT TH Doc";
        Line: Record "LT TH Line";
        Log: Codeunit "LT TH Log";
    begin
        Line.DeleteAll();
        Doc.DeleteAll();
        Doc."No." := 'D1';
        Doc.Insert();
        InsertLine(10000, 300, 'FIRST');
        InsertLine(20000, 300, 'SECOND');
        InsertLine(30000, 100, 'THIRD');
        Log.Reset();
    end;

    local procedure InsertLine(LineNo: Integer; Qty: Decimal; Bin: Code[20])
    var
        Line: Record "LT TH Line";
    begin
        Line."Document No." := 'D1';
        Line."Line No." := LineNo;
        Line.Quantity := Qty;
        Line."Bin Code" := Bin;
        Line.Insert();
    end;

    local procedure SetField(var Doc: TestPage "LT TH Doc"; FieldNo: Integer; NewValue: Text)
    begin
        case FieldNo of
            1:
                Doc.Lines."Qty. to Handle".SetValue(NewValue);
            2:
                Doc.Lines."Qty. to Ship".SetValue(NewValue);
            3:
                Doc.Lines.Note.SetValue(NewValue);
        end;
    end;

    local procedure Stored(LineNo: Integer): Text
    var
        Line: Record "LT TH Line";
    begin
        Line.Get('D1', LineNo);
        exit(Format(Line."Qty. to Handle") + '|' + Format(Line."Qty. to Ship") + '|' + Line.Note + '|' + Line."Bin Code");
    end;

    procedure OneLine(FieldNo: Integer; LineNo: Integer) Seen: Text
    var
        Doc: TestPage "LT TH Doc";
        Log: Codeunit "LT TH Log";
    begin
        Seed();
        Doc.OpenEdit();
        Doc.GoToKey('D1');
        Doc.Lines.GoToKey('D1', LineNo);
        Log.Reset();
        SetField(Doc, FieldNo, '7');
        Seen := Doc.Lines."Line No.".Value() + ';' + Doc.Lines."Qty. to Handle".Value() + ';' + Doc.Lines."Bin Code".Value() + ';'
            + Doc."No.".Value() + ';' + Log.Get();
        Doc.Close();
        Seen += ';' + Stored(LineNo);
    end;

    procedure Walk(FieldNo: Integer) Seen: Text
    var
        Doc: TestPage "LT TH Doc";
        Log: Codeunit "LT TH Log";
        Rows: Integer;
    begin
        Seed();
        Doc.OpenEdit();
        Doc.GoToKey('D1');
        Doc.Lines.First();
        Log.Reset();
        repeat
            Rows += 1;
            Seen += Doc.Lines."Line No.".Value() + '->';
            SetField(Doc, FieldNo, Format(Rows));
            Seen += Doc.Lines."Line No.".Value() + ';';
        until (not Doc.Lines.Next()) or (Rows = 6);
        Seen += Format(Rows) + ';' + Doc.Lines."Line No.".Value() + ';' + Log.Get();
        Doc.Close();
        Seen += ';' + Stored(10000) + ';' + Stored(20000) + ';' + Stored(30000);
    end;

    procedure WalkSorted(FieldNo: Integer) Seen: Text
    var
        Doc: TestPage "LT TH Doc Sorted";
        Rows: Integer;
    begin
        Seed();
        Doc.OpenEdit();
        Doc.GoToKey('D1');
        Doc.Lines.First();
        repeat
            Rows += 1;
            Seen += Doc.Lines."Line No.".Value() + '->';
            case FieldNo of
                1:
                    Doc.Lines."Qty. to Handle".SetValue(Format(Rows));
                2:
                    Doc.Lines."Qty. to Ship".SetValue(Format(Rows));
            end;
            Seen += Doc.Lines."Line No.".Value() + ';';
        until (not Doc.Lines.Next()) or (Rows = 6);
        Doc.Close();
    end;
}
"#;

fn run(call: &str) -> String {
    let src = SRC.replacen(
        "codeunit 89990 \"LT TH Probe\"\n{",
        &format!("codeunit 89990 \"LT TH Probe\"\n{{\n    procedure Probe(): Text begin exit({call}); end;"),
        1,
    );
    al_testkit::text(al_testkit::run_in(&src, "LT TH Probe", "Probe"))
}

// ThPartUpdateCursor.al:357 TH_Line2_UpdateTrue, :373 TH_Line2_UpdateFalse, :437 TH_Line3_UpdateFalse.
#[test]
fn a_part_keeps_a_later_row_after_curr_page_update_in_on_validate() {
    clear_hits();
    assert_eq!(
        run("OneLine(1, 20000)"),
        "20000;7.00;SECOND;D1;QH.V QH.V.end R2 R3 R2 R1 R2 R2 C2 ;7|0||SECOND"
    );
    assert_eq!(
        run("OneLine(2, 20000)"),
        "20000;0.00;SECOND;D1;QS.V QS.V.end R2 R3 R2 R1 R2 R2 C2 ;0|0||SECOND"
    );
    assert_eq!(
        run("OneLine(2, 30000)"),
        "30000;0.00;THIRD;D1;QS.V QS.V.end R3 R2 R1 R3 R3 C3 ;0|0||THIRD"
    );
    assert!(take_hits().is_empty());
}

// ThPartUpdateCursor.al:421 TH_Line1_UpdateTrue: on the first row the part reads as with its host (pinned before).
#[test]
fn a_part_on_its_first_row_reads_from_the_first_row() {
    assert_eq!(
        run("OneLine(1, 10000)"),
        "10000;7.00;FIRST;D1;QH.V QH.V.end R1 R2 R3 R1 R1 C1 ;7|0||FIRST"
    );
}

// ThPartUpdateCursor.al:453 TH_Walk_UpdateTrue, :469 TH_Walk_UpdateFalse, :485 TH_Walk_NoTrigger, :517-546 TH_WalkSorted_*.
#[test]
fn the_first_set_value_next_walk_visits_every_line_once() {
    assert_eq!(
        run("Walk(1)"),
        "10000->10000;20000->20000;30000->30000;3;30000;QH.V QH.V.end R1 R2 R3 R1 R1 C1 R2 R2 C2 QH.V QH.V.end R2 R3 R2 R1 R2 R2 C2 R3 R3 C3 QH.V QH.V.end R3 R2 R1 R3 R3 C3 ;1|0||FIRST;2|0||SECOND;3|0||THIRD"
    );
    assert_eq!(
        run("Walk(2)"),
        "10000->10000;20000->20000;30000->30000;3;30000;QS.V QS.V.end R1 R2 R3 R1 R1 C1 R2 R2 C2 QS.V QS.V.end R2 R3 R2 R1 R2 R2 C2 R3 R3 C3 QS.V QS.V.end R3 R2 R1 R3 R3 C3 ;0|0||FIRST;0|0||SECOND;0|0||THIRD"
    );
    assert_eq!(
        run("Walk(3)"),
        "10000->10000;20000->20000;30000->30000;3;30000;R2 R2 C2 R3 R3 C3 ;0|0|1|FIRST;0|0|2|SECOND;0|0|3|THIRD"
    );
    assert_eq!(run("WalkSorted(1)"), "30000->30000;10000->10000;20000->20000;");
    assert_eq!(run("WalkSorted(2)"), "30000->30000;10000->10000;20000->20000;");
}
