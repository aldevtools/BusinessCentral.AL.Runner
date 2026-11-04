//! The caption of a list whose DataCaptionFields field relates with where(field()) to a table with a two-field key: the first
//! related row with that name, whatever the template; a blank description is left out
//! (corpus testpage/Y72CaptionRelationWhere.al).

use al_testkit::text;

const SRC: &str = r#"
table 69970 "CW Batch"
{
    DataCaptionFields = Name, Description;
    fields { field(1; Tmpl; Code[10]) { } field(2; Name; Code[10]) { } field(3; Description; Text[50]) { } }
    keys { key(PK; Tmpl, Name) { } }
}
table 69971 "CW Line"
{
    fields
    {
        field(1; Tmpl; Code[10]) { }
        field(2; Batch; Code[10]) { TableRelation = "CW Batch".Name where(Tmpl = field(Tmpl)); }
        field(3; "Line No."; Integer) { }
    }
    keys { key(PK; Tmpl, Batch, "Line No.") { } }
}
page 69972 "CW Journal"
{
    Caption = 'CW Journal'; PageType = List; SourceTable = "CW Line"; DataCaptionFields = Batch; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(Batch; Rec.Batch) { } } } }
}
codeunit 69973 C
{
    local procedure Seed()
    var B: Record "CW Batch"; L: Record "CW Line";
    begin
        B.Tmpl := 'T1'; B.Name := 'BAT'; B.Description := 'Batch one'; B.Insert();
        B.Tmpl := 'T2'; B.Name := 'BAT'; B.Description := 'Batch two'; B.Insert();
        B.Tmpl := 'T1'; B.Name := 'OTHER'; B.Description := ''; B.Insert();
        L.Tmpl := 'T1'; L.Batch := 'BAT'; L."Line No." := 1; L.Insert();
        L.Tmpl := 'T2'; L.Batch := 'BAT'; L."Line No." := 1; L.Insert();
        L.Tmpl := 'T1'; L.Batch := 'OTHER'; L."Line No." := 1; L.Insert();
    end;

    local procedure Cap(Tmpl: Code[10]; Batch: Code[10]): Text
    var L: Record "CW Line"; P: TestPage "CW Journal";
    begin
        L.SetRange(Tmpl, Tmpl);
        L.SetRange(Batch, Batch);
        P.Trap();
        Page.Run(Page::"CW Journal", L);
        exit(P.Caption());
    end;

    procedure Captions(): Text
    begin
        Seed();
        exit(Cap('T2', 'BAT') + '|' + Cap('T1', 'OTHER'));
    end;
}
"#;

#[test]
fn a_where_field_relation_to_a_two_field_key_takes_the_first_row_with_the_name() {
    // Y72CaptionRelationWhere.al Caption_WhereField_OtherTemplate, Caption_WhereField_NoDescription,
    // Caption_WhereField_FilterGroupZeroFromCaller.
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "Captions")),
        "CW Journal - BAT \u{2219} Batch one|CW Journal - OTHER"
    );
}
