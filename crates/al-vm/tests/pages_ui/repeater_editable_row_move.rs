//! TestPage `Editable()` of a repeater control whose Editable expression reads a page variable OnAfterGetRecord sets, after
//! GoToRecord moves to another row (al-runner issue 5808, the reporter's Worksheet). BC answers per row; the runtime keeps one
//! value of the variable. Only a Card is pinned (handlers/TestPageEditable_Tests.al:93-112), so the read on a row other than the
//! one OnAfterGetRecord ran with last is refused until a probe pins the list (docs/probes-needed.md).

use al_builtins::native::Signal;

const SRC: &str = r#"
table 69960 "RE Plan Row"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Sequence No."; Integer) { }
        field(3; "New Start"; DateTime) { }
    }
    keys
    {
        key(PK; "Code") { Clustered = true; }
        key(Seq; "Sequence No.") { }
    }
}
page 69961 "RE Plan Worksheet"
{
    PageType = Worksheet;
    SourceTable = "RE Plan Row";
    SourceTableView = sorting("Sequence No.");
    ApplicationArea = All;
    UsageCategory = None;
    InsertAllowed = false;
    DelayedInsert = true;
    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Sequence No."; Rec."Sequence No.") { ApplicationArea = All; Editable = SequenceEditable; }
                field("Code"; Rec."Code") { ApplicationArea = All; }
                field("New Start"; Rec."New Start") { ApplicationArea = All; }
            }
        }
    }
    trigger OnAfterGetRecord()
    begin
        SequenceEditable := Rec."New Start" = 0DT;
    end;
    var
        SequenceEditable: Boolean;
}
codeunit 69962 C
{
    procedure Run(): Text
    var
        RowA: Record "RE Plan Row";
        RowB: Record "RE Plan Row";
        Worksheet: TestPage "RE Plan Worksheet";
        Seen: Text;
    begin
        RowA.DeleteAll();
        RowA."Code" := 'A';
        RowA."Sequence No." := 1;
        RowA."New Start" := CreateDateTime(DMY2Date(1, 1, 2030), 080000T);
        RowA.Insert();
        RowB."Code" := 'B';
        RowB."Sequence No." := 2;
        RowB.Insert();
        Worksheet.OpenEdit();
        Worksheet.GoToRecord(RowA);
        Seen := 'A:' + Format(Worksheet."Sequence No.".Editable());
        Worksheet.GoToRecord(RowB);
        Seen += ';B:' + Format(Worksheet."Sequence No.".Editable());
        Worksheet.Close();
        exit(Seen);
    end;
}
"#;

#[test]
fn editable_of_a_repeater_control_after_a_row_move_is_refused() {
    match al_testkit::run_in(SRC, "C", "Run") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("Editable"), "{why}"),
        other => panic!("{other:?}"),
    }
}
