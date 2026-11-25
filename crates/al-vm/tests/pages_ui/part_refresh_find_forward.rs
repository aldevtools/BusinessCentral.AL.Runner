//! The merged issue 7820 probe's exact cells: the local variable is `Log`, the object names differ from the reporter's, and
//! the original page properties are present. The 1/2/5-row first-row cells and the 3-row second-cursor cell all refresh with
//! `OnFindRecord =>` (`R7820PartUpdateFindForward.al:169`, :181, :193 and :205, assumption
//! `part-update-first-row-find-forward`). A 3-row first-row cell and mixed properties keep main's values unflagged.

use al_builtins::assumptions::{Assumption, clear_hits, take_hits};
use al_testkit::text;

const SRC: &str = r#"
table 51600 "PG R7820 Header"
{
    fields { field(1; "No."; Code[20]) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}
table 51601 "PG R7820 Line"
{
    fields
    {
        field(1; "Order No."; Code[20]) { }
        field(2; "Line No."; Integer) { }
        field(3; Qty; Integer) { }
    }
    keys { key(PK; "Order No.", "Line No.") { Clustered = true; } }
}
codeunit 51602 "PG R7820 Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Reset() begin Entries := ''; end;
    procedure Add(Entry: Text) begin Entries += Entry + ';'; end;
    procedure Get(): Text begin exit(Entries); end;
}
page 51603 "PG R7820 Doc"
{
    PageType = Document;
    SourceTable = "PG R7820 Header";
    ApplicationArea = All;
    UsageCategory = None;
    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
            part(Lines; "PG R7820 Part")
            {
                ApplicationArea = All;
                SubPageLink = "Order No." = field("No.");
            }
        }
    }
}
page 51604 "PG R7820 Part"
{
    PageType = ListPart;
    SourceTable = "PG R7820 Line";
    ApplicationArea = All;
    AutoSplitKey = true;
    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Line No."; Rec."Line No.") { ApplicationArea = All; Editable = false; }
                field(Qty; Rec.Qty)
                {
                    ApplicationArea = All;
                    trigger OnValidate()
                    begin
                        CurrPage.Update(true);
                    end;
                }
            }
        }
    }
    trigger OnFindRecord(Which: Text): Boolean
    var Log: Codeunit "PG R7820 Log";
    begin
        Log.Add('F' + Which);
        exit(Rec.Find(Which));
    end;
    trigger OnAfterGetRecord()
    var Log: Codeunit "PG R7820 Log";
    begin
        Log.Add('R' + Format(Rec."Line No." div 10000));
    end;
    trigger OnAfterGetCurrRecord()
    var Log: Codeunit "PG R7820 Log";
    begin
        Log.Add('C' + Format(Rec."Line No." div 10000));
    end;
}
codeunit 51605 C
{
    local procedure Run(Rows: Integer; Moves: Integer; QtyOffset: Integer): Text
    var
        Header: Record "PG R7820 Header";
        Line: Record "PG R7820 Line";
        Log: Codeunit "PG R7820 Log";
        P: TestPage "PG R7820 Doc";
        Answer: Text;
        i: Integer;
    begin
        Header."No." := 'O1';
        Header.Insert();
        for i := 1 to Rows do begin
            Line."Order No." := 'O1';
            Line."Line No." := i * 10000;
            Line.Qty := i + QtyOffset;
            Line.Insert();
        end;
        P.OpenEdit();
        P.GoToRecord(Header);
        P.Lines.First();
        for i := 1 to Moves do
            P.Lines.Next();
        Log.Reset();
        P.Lines.Qty.SetValue(100);
        Answer := Log.Get() + '|' + P.Lines."Line No.".Value() + '|' + P.Lines.Qty.Value();
        P.Close();
        exit(Answer);
    end;
    procedure One(): Text begin exit(Run(1, 0, 0)); end;
    procedure Two(): Text begin exit(Run(2, 0, 0)); end;
    procedure Five(): Text begin exit(Run(5, 0, 0)); end;
    procedure ThreeSecond(): Text begin exit(Run(3, 1, 0)); end;
    procedure OtherSeedValues(): Text begin exit(Run(1, 0, 10)); end;
    procedure ThreeFirstValues(): Text
    var
        Header: Record "PG R7820 Header";
        Line: Record "PG R7820 Line";
        P: TestPage "PG R7820 Doc";
        i: Integer;
    begin
        Header."No." := 'O1';
        Header.Insert();
        for i := 1 to 3 do begin
            Line."Order No." := 'O1';
            Line."Line No." := i * 10000;
            Line.Qty := i;
            Line.Insert();
        end;
        P.OpenEdit();
        P.GoToRecord(Header);
        P.Lines.First();
        P.Lines.Qty.SetValue(100);
        exit(P.Lines."Line No.".Value() + '|' + P.Lines.Qty.Value());
    end;
}
"#;

fn run_flagged(routine: &str) -> String {
    clear_hits();
    let answer = text(al_testkit::run_in(SRC, "C", routine));
    assert!(take_hits().contains(&Assumption::PartUpdateFirstRowFindForward));
    answer
}

#[test]
fn merged_probe_one_row_refreshes_forward_with_log_slot() {
    assert_eq!(run_flagged("One"), "F=>;R1;F=;R1;C1;|10000|100");
}

#[test]
fn merged_probe_two_rows_refreshes_forward() {
    assert_eq!(run_flagged("Two"), "F=>;R1;R2;R1;F=;R1;C1;|10000|100");
}

#[test]
fn merged_probe_five_rows_refreshes_forward() {
    assert_eq!(run_flagged("Five"), "F=>;R1;R2;R3;R4;R5;R1;F=;R1;C1;|10000|100");
}

#[test]
fn merged_probe_second_of_three_uses_the_exact_later_cursor_walk() {
    assert_eq!(run_flagged("ThreeSecond"), "F=>;R2;R3;R2;R1;R2;F=;R2;C2;|20000|100");
}

#[test]
fn three_rows_on_the_first_keep_main_values_without_an_assumption() {
    clear_hits();
    assert_eq!(text(al_testkit::run_in(SRC, "C", "ThreeFirstValues")), "10000|100");
    assert!(!take_hits().contains(&Assumption::PartUpdateFirstRowFindForward));
}

#[test]
fn other_seed_values_keep_main_values_without_an_assumption() {
    clear_hits();
    assert!(text(al_testkit::run_in(SRC, "C", "OtherSeedValues")).ends_with("|10000|100"));
    assert!(!take_hits().contains(&Assumption::PartUpdateFirstRowFindForward));
}

#[test]
fn mixed_page_properties_keep_main_values_without_an_assumption() {
    let src = SRC.replacen(
        "    SourceTable = \"PG R7820 Line\";\n    ApplicationArea = All;\n    AutoSplitKey = true;",
        "    SourceTable = \"PG R7820 Line\";\n    AutoSplitKey = true;",
        1,
    );
    clear_hits();
    assert!(text(al_testkit::run_in(&src, "C", "One")).ends_with("|10000|100"));
    assert!(!take_hits().contains(&Assumption::PartUpdateFirstRowFindForward));
}
