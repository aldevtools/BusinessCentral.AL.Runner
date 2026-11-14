//! The blank-captioned Enum member typed on the rows of a list (`blank_enum_rows.rs`, al-runner issue 7380) and the calls after an
//! action left one row behind the new-row line of an empty List (`empty_list_one_row.rs`, al-runner issue 7478). Each new path has a
//! neighbor that flips one pinned fact and keeps main's refusal.

use al_testkit::outcomes_in;

const SRC: &str = r#"
enum 50380 "BE Action Type" { Extensible = false; value(0; Empty) { Caption = ' ', Locked = true; } value(1; Replace) { Caption = 'Replace'; } }
enum 50381 "BE Check Type" { Extensible = false; value(0; Exists) { Caption = 'Exists'; } value(1; Missing) { Caption = 'Missing'; } }
table 50380 "BE Head" { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { } } }
table 50381 "BE Line"
{
    fields { field(1; "Head No."; Code[20]) { } field(2; "Line No."; Integer) { } field(3; "Check Type"; Enum "BE Check Type") { } field(4; "Action Type"; Enum "BE Action Type") { } }
    keys { key(PK; "Head No.", "Line No.") { } }
}
page 50381 "BE Part"
{
    PageType = ListPart; SourceTable = "BE Line"; AutoSplitKey = true; DelayedInsert = true;
    layout { area(Content) { repeater(R) { field(CheckType; Rec."Check Type") { } field(ActionType; Rec."Action Type") { } } } }
}
page 50382 "BE Card"
{
    PageType = Card; SourceTable = "BE Head"; UsageCategory = None;
    layout { area(Content) { field("No."; Rec."No.") { } part(Lines; "BE Part") { SubPageLink = "Head No." = field("No."); } } }
}
page 50383 "BE List Trigger"
{
    PageType = List; SourceTable = "BE Line"; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(CheckType; Rec."Check Type") { } field(ActionType; Rec."Action Type") { } } } }
    trigger OnAfterGetRecord() begin end;
}
page 50384 "BE Lines Trigger"
{
    PageType = List; SourceTable = "BE Line"; DelayedInsert = true; UsageCategory = None;
    layout { area(Content) { repeater(R) { field("Line No."; Rec."Line No.") { } } } }
    actions { area(Processing) { action(AddOne) { trigger OnAction() var L: Record "BE Line"; begin L."Head No." := 'P1'; L."Line No." := 10000; L.Insert(); end; } } }
    trigger OnAfterGetRecord() begin end;
}
codeunit 50385 T
{
    Subtype = Test;
    local procedure Seed()
    var Head: Record "BE Head"; Line: Record "BE Line";
    begin
        Head.DeleteAll(); Line.DeleteAll();
        Head."No." := 'H1'; Head.Insert();
        Line."Head No." := 'H1'; Line."Line No." := 10000; Line."Action Type" := Line."Action Type"::Replace; Line.Insert();
    end;
    [Test]
    procedure Issue7380_PartNewRowLine()
    var MyCard: TestPage "BE Card"; Line: Record "BE Line";
    begin
        Seed();
        MyCard.OpenEdit();
        MyCard.Lines.Last();
        MyCard.Lines.Next();
        MyCard.Lines.CheckType.Value(Format(Enum::"BE Check Type"::Missing));
        MyCard.Lines.ActionType.Value(Format(Enum::"BE Action Type"::Empty));
        if MyCard.Lines.ActionType.Value() <> ' ' then Error('shown [%1]', MyCard.Lines.ActionType.Value());
        MyCard.Close();
        Line.Get('H1', 20000);
        Line.TestField("Action Type", Line."Action Type"::Empty);
    end;
    [Test]
    procedure ListWithTrigger_Refused()
    var List: TestPage "BE List Trigger";
    begin
        Seed();
        List.OpenEdit();
        List.First();
        List.ActionType.Value(' ');
    end;
    [Test]
    procedure OneRowBehind_PageWithTrigger_OkRefused()
    var Lines: TestPage "BE Lines Trigger"; Line: Record "BE Line";
    begin
        Line.DeleteAll();
        Lines.OpenEdit();
        Lines.Filter.SetFilter("Head No.", 'P1');
        Lines.AddOne.Invoke();
        Lines.OK().Invoke();
    end;
}
"#;

#[test]
fn blank_enum_list_rows_and_one_row_behind() {
    let outcomes = outcomes_in(SRC, "T");
    let get = |name: &str| {
        let o = outcomes.iter().find(|(n, _)| n == name).map(|(_, o)| o);
        format!("{o:?}")
    };
    assert_eq!(get("Issue7380_PartNewRowLine"), "Some(Pass)");
    let list = get("ListWithTrigger_Refused");
    assert!(
        list.contains("NotRun") && list.contains("SetValue of a blank text on an Enum control"),
        "{list}"
    );
    let ok = get("OneRowBehind_PageWithTrigger_OkRefused");
    assert!(
        ok.contains("NotRun") && ok.contains("after an action stored a row behind the new-row line"),
        "{ok}"
    );
}
