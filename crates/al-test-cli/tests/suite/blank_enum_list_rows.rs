//! The blank-captioned Enum member typed on the rows of a list (`blank_enum_rows.rs`, al-runner issue 7380). The pinned shapes
//! run; each neighbor flips one pinned fact of pages/enum-setvalue-blank/EnumBlankCaptionListRows.al and keeps main's refusal.

use al_testkit::outcomes_in;

const SRC: &str = r#"
enum 50380 "BE Action Type" { Extensible = false; value(0; Empty) { Caption = ' ', Locked = true; } value(1; Replace) { Caption = 'Replace'; } }
enum 50381 "BE Check Type" { Extensible = false; value(0; Exists) { Caption = 'Exists'; } value(1; Missing) { Caption = 'Missing'; } }
enum 50382 "BE Name Blank" { Extensible = false; value(0; Empty) { Caption = ' ', Locked = true; } value(1; Replace) { Caption = 'Replace'; } value(2; " ") { } }
enum 50383 "BE Extended" { Extensible = true; value(0; Empty) { Caption = ' ', Locked = true; } value(1; Replace) { Caption = 'Replace'; } }
enumextension 50384 "BE Extended Ext" extends "BE Extended" { value(5; Other) { Caption = 'Other'; } }
table 50380 "BE Head" { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { } } }
table 50381 "BE Line"
{
    fields
    {
        field(1; "Head No."; Code[20]) { } field(2; "Line No."; Integer) { } field(3; "Check Type"; Enum "BE Check Type") { }
        field(4; "Action Type"; Enum "BE Action Type") { } field(5; "Name Blank"; Enum "BE Name Blank") { } field(6; Extended; Enum "BE Extended") { }
    }
    keys { key(PK; "Head No.", "Line No.") { } }
}
table 50382 "BE Line V"
{
    fields
    {
        field(1; "Head No."; Code[20]) { } field(2; "Line No."; Integer) { }
        field(4; "Action Type"; Enum "BE Action Type") { trigger OnValidate() begin end; }
    }
    keys { key(PK; "Head No.", "Line No.") { } }
}
table 50383 "BE Line T"
{
    fields { field(1; "Head No."; Code[20]) { } field(2; "Line No."; Integer) { } field(3; "Check Type"; Enum "BE Check Type") { } field(4; "Action Type"; Enum "BE Action Type") { } }
    keys { key(PK; "Head No.", "Line No.") { } }
    trigger OnInsert() begin end;
}
table 50384 "BE Line S"
{
    fields { field(1; "Head No."; Code[20]) { } field(2; "Line No."; Integer) { } field(4; "Action Type"; Enum "BE Action Type") { } }
    keys { key(PK; "Head No.", "Line No.") { } }
}
codeunit 50386 "BE Subscriber"
{
    [EventSubscriber(ObjectType::Table, Database::"BE Line S", 'OnAfterValidateEvent', 'Action Type', false, false)]
    local procedure OnAfterValidateActionType(var Rec: Record "BE Line S") begin end;
}
page 50381 "BE Part"
{
    PageType = ListPart; SourceTable = "BE Line"; AutoSplitKey = true; DelayedInsert = true;
    layout { area(Content) { repeater(R) { field(CheckType; Rec."Check Type") { ApplicationArea = All; } field(ActionType; Rec."Action Type") { ApplicationArea = All; } } } }
}
page 50382 "BE Card"
{
    PageType = Card; SourceTable = "BE Head"; UsageCategory = None;
    layout { area(Content) { field("No."; Rec."No.") { ApplicationArea = All; } part(Lines; "BE Part") { ApplicationArea = All; SubPageLink = "Head No." = field("No."); } } }
}
page 50383 "BE List Trigger"
{
    PageType = List; SourceTable = "BE Line"; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(CheckType; Rec."Check Type") { ApplicationArea = All; } field(ActionType; Rec."Action Type") { ApplicationArea = All; } } } }
    trigger OnAfterGetRecord() begin end;
}
page 50390 "BE List"
{
    PageType = List; SourceTable = "BE Line"; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(CheckType; Rec."Check Type") { ApplicationArea = All; } field(ActionType; Rec."Action Type") { ApplicationArea = All; } field(NameBlank; Rec."Name Blank") { ApplicationArea = All; } field(Extended; Rec.Extended) { ApplicationArea = All; } } } }
}
page 50391 "BE List Ctl Validate"
{
    PageType = List; SourceTable = "BE Line"; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(ActionType; Rec."Action Type") { ApplicationArea = All; trigger OnValidate() begin end; } } } }
}
page 50392 "BE List V"
{
    PageType = List; SourceTable = "BE Line V"; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(ActionType; Rec."Action Type") { ApplicationArea = All; } } } }
}
page 50393 "BE Part T"
{
    PageType = ListPart; SourceTable = "BE Line T"; AutoSplitKey = true; DelayedInsert = true;
    layout { area(Content) { repeater(R) { field(CheckType; Rec."Check Type") { ApplicationArea = All; } field(ActionType; Rec."Action Type") { ApplicationArea = All; } } } }
}
page 50394 "BE Card T"
{
    PageType = Card; SourceTable = "BE Head"; UsageCategory = None;
    layout { area(Content) { field("No."; Rec."No.") { ApplicationArea = All; } part(Lines; "BE Part T") { ApplicationArea = All; SubPageLink = "Head No." = field("No."); } } }
}
page 50395 "BE List S"
{
    PageType = List; SourceTable = "BE Line S"; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(ActionType; Rec."Action Type") { ApplicationArea = All; } } } }
}
page 50396 "BE Part AutoSplit"
{
    PageType = ListPart; SourceTable = "BE Line"; AutoSplitKey = true;
    layout { area(Content) { repeater(R) { field(CheckType; Rec."Check Type") { ApplicationArea = All; } field(ActionType; Rec."Action Type") { ApplicationArea = All; } } } }
}
page 50397 "BE Card AutoSplit"
{
    PageType = Card; SourceTable = "BE Head"; UsageCategory = None;
    layout { area(Content) { field("No."; Rec."No.") { ApplicationArea = All; } part(Lines; "BE Part AutoSplit") { ApplicationArea = All; SubPageLink = "Head No." = field("No."); } } }
}
page 50398 "BE List Not Editable"
{
    PageType = List; SourceTable = "BE Line"; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(ActionType; Rec."Action Type") { ApplicationArea = All; Editable = false; } } } }
}
page 50399 "BE List Outside"
{
    PageType = List; SourceTable = "BE Line"; UsageCategory = None;
    layout { area(Content) { group(G) { field(ActionType; Rec."Action Type") { ApplicationArea = All; } } repeater(R) { field(CheckType; Rec."Check Type") { ApplicationArea = All; } } } }
}
codeunit 50385 T
{
    Subtype = Test;
    local procedure Seed()
    var Head: Record "BE Head"; Line: Record "BE Line"; LineV: Record "BE Line V"; LineT: Record "BE Line T"; LineS: Record "BE Line S";
    begin
        Head.DeleteAll(); Line.DeleteAll(); LineV.DeleteAll(); LineT.DeleteAll(); LineS.DeleteAll();
        Head."No." := 'H1'; Head.Insert();
        Line."Head No." := 'H1'; Line."Line No." := 10000; Line."Action Type" := Line."Action Type"::Replace; Line.Insert();
        LineV."Head No." := 'H1'; LineV."Line No." := 10000; LineV."Action Type" := LineV."Action Type"::Replace; LineV.Insert();
        LineT."Head No." := 'H1'; LineT."Line No." := 10000; LineT."Action Type" := LineT."Action Type"::Replace; LineT.Insert();
        LineS."Head No." := 'H1'; LineS."Line No." := 10000; LineS."Action Type" := LineS."Action Type"::Replace; LineS.Insert();
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
    procedure List_NewRowLine_Space_StoresNothing()
    var List: TestPage "BE List"; Line: Record "BE Line";
    begin
        Seed();
        List.OpenEdit();
        List.Last();
        List.Next();
        List.ActionType.SetValue(' ');
        if List.ActionType.Value() <> ' ' then Error('shown [%1]', List.ActionType.Value());
        List.Close();
        if Line.Count() <> 1 then Error('stored %1', Line.Count());
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
    procedure ControlOnValidate_NewRowLine_Refused()
    var List: TestPage "BE List Ctl Validate";
    begin
        Seed();
        List.OpenEdit();
        List.Last();
        List.Next();
        List.ActionType.SetValue(' ');
    end;
    [Test]
    procedure FieldOnValidate_NewRowLine_Refused()
    var List: TestPage "BE List V";
    begin
        Seed();
        List.OpenEdit();
        List.Last();
        List.Next();
        List.ActionType.SetValue(' ');
    end;
    [Test]
    procedure TableOnInsert_PartAfterOtherField_Refused()
    var MyCard: TestPage "BE Card T";
    begin
        Seed();
        MyCard.OpenEdit();
        MyCard.Lines.Last();
        MyCard.Lines.Next();
        MyCard.Lines.CheckType.Value(Format(Enum::"BE Check Type"::Missing));
        MyCard.Lines.ActionType.Value(' ');
    end;
    [Test]
    procedure TableSubscriber_NewRowLine_Refused()
    var List: TestPage "BE List S";
    begin
        Seed();
        List.OpenEdit();
        List.Last();
        List.Next();
        List.ActionType.SetValue(' ');
    end;
    [Test]
    procedure PartAutoSplitOnly_Refused()
    var MyCard: TestPage "BE Card AutoSplit";
    begin
        Seed();
        MyCard.OpenEdit();
        MyCard.Lines.Last();
        MyCard.Lines.Next();
        MyCard.Lines.CheckType.Value(Format(Enum::"BE Check Type"::Missing));
        MyCard.Lines.ActionType.Value(' ');
    end;
    [Test]
    procedure ControlEditableFalse_ExistingRow_Refused()
    var List: TestPage "BE List Not Editable";
    begin
        Seed();
        List.OpenEdit();
        List.First();
        List.ActionType.SetValue(' ');
    end;
    [Test]
    procedure ControlOutsideRepeater_Refused()
    var List: TestPage "BE List Outside";
    begin
        Seed();
        List.OpenEdit();
        List.First();
        List.ActionType.SetValue(' ');
    end;
    [Test]
    procedure MemberBlankByName_Refused()
    var List: TestPage "BE List";
    begin
        Seed();
        List.OpenEdit();
        List.First();
        List.NameBlank.SetValue(0);
    end;
    [Test]
    procedure EnumExtended_Refused()
    var List: TestPage "BE List";
    begin
        Seed();
        List.OpenEdit();
        List.First();
        List.Extended.SetValue(0);
    end;
    [Test]
    procedure TwoSpaces_Refused()
    var List: TestPage "BE List";
    begin
        Seed();
        List.OpenEdit();
        List.First();
        List.ActionType.SetValue('  ');
    end;
    [Test]
    procedure ListTestFilter_Refused()
    var List: TestPage "BE List";
    begin
        Seed();
        List.OpenEdit();
        List.Filter.SetFilter("Head No.", 'H1');
        List.First();
        List.ActionType.SetValue(' ');
    end;
    [Test]
    procedure VariantMember_Refused()
    var List: TestPage "BE List"; V: Variant;
    begin
        Seed();
        List.OpenEdit();
        List.First();
        V := Enum::"BE Action Type"::Empty;
        List.ActionType.SetValue(V);
    end;
}
"#;

#[test]
fn blank_enum_list_rows() {
    let outcomes = outcomes_in(SRC, "T");
    let get = |name: &str| {
        let o = outcomes.iter().find(|(n, _)| n == name).map(|(_, o)| o);
        format!("{o:?}")
    };
    let mut wrong = Vec::new();
    for pass in ["Issue7380_PartNewRowLine", "List_NewRowLine_Space_StoresNothing"] {
        if get(pass) != "Some(Pass)" {
            wrong.push(format!("{pass}: {}", get(pass)));
        }
    }
    // Each neighbor keeps main's refusal of a blank Enum input on a list.
    for refused in [
        "ListWithTrigger_Refused",
        "ControlOnValidate_NewRowLine_Refused",
        "FieldOnValidate_NewRowLine_Refused",
        "TableOnInsert_PartAfterOtherField_Refused",
        "TableSubscriber_NewRowLine_Refused",
        "PartAutoSplitOnly_Refused",
        "ControlEditableFalse_ExistingRow_Refused",
        "ControlOutsideRepeater_Refused",
        "MemberBlankByName_Refused",
        "EnumExtended_Refused",
        "TwoSpaces_Refused",
        "VariantMember_Refused",
        "ListTestFilter_Refused",
    ] {
        let o = get(refused);
        if !(o.contains("NotRun") && o.contains("(not pinned)")) {
            wrong.push(format!("{refused}: {o}"));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}
