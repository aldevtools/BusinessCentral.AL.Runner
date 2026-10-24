//! TestPage `Editable()` of a repeater control whose Editable expression reads a page variable OnAfterGetRecord sets, after
//! GoToRecord moves to another row (al-runner issue 5808, the reporter's Worksheet). BC answers for the row the TestPage stands
//! on, with the variable as that row's OnAfterGetRecord left it (writer RT's pages/lt-outside-worksheet-2/RtRowEditable.al:162
//! RT_EdWs_OpenEdit_GoToRecord_AB, assumption repeater-editable-follows-row until the corpus bump). An expression that calls
//! code stays refused there.

use al_builtins::assumptions::{Assumption, take_hits};
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
fn editable_of_a_repeater_control_follows_the_row_a_move_goes_to() {
    take_hits();
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "Run")), "A:No;B:Yes");
    assert!(take_hits().contains(&Assumption::RepeaterEditableFollowsRow));
}

#[test]
fn an_editable_expression_that_calls_code_on_another_row_is_refused() {
    let src = SRC
        .replace("Editable = SequenceEditable;", "Editable = IsEditable();")
        .replace(
            "    var\n        SequenceEditable: Boolean;",
            "    local procedure IsEditable(): Boolean\n    begin\n        exit(SequenceEditable);\n    end;\n    var\n        SequenceEditable: Boolean;",
        );
    assert!(src.contains("IsEditable(): Boolean"));
    match al_testkit::run_in(&src, "C", "Run") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("editable"), "{why}"),
        other => panic!("{other:?}"),
    }
}
