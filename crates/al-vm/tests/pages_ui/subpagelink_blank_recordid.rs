//! `SubPageLink = Target = const()` on a RecordId field shows only the child whose RecordId is blank (al-runner issue 7267).
//! Pinned by corpus pull request 1199 (measured on real BC 27.5, 28.5 and 29.0):
//! pages/subpagelink-empty-term/TestSubPageLinkRecordId.al:41 (the link) and :53 SubPageLink_BlankRecordId_SelectsBlankRow.
//! Any other text filter on a RecordId field stays refused.

const SRC: &str = r#"
table 69980 "SRL Host" { fields { field(1; "Entry No."; Integer) { } } keys { key(PK; "Entry No.") { Clustered = true; } } }
table 69992 "PAG RecordId Link Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Target; RecordId) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}
page 69993 "PAG RecordId Link Part"
{
    PageType = ListPart;
    SourceTable = "PAG RecordId Link Row";
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { ApplicationArea = All; } } } }
}
page 69994 "PAG RecordId Link Card"
{
    PageType = Card;
    SourceTable = "SRL Host";
    layout
    {
        area(Content)
        {
            part(Lines; "PAG RecordId Link Part") { ApplicationArea = All; SubPageLink = Target = const(); }
        }
    }
}
codeunit 69995 C
{
    procedure F(): Text
    var
        Host: Record "SRL Host";
        Line: Record "PAG RecordId Link Row";
        Card: TestPage "PAG RecordId Link Card";
        s: Text;
    begin
        Host."Entry No." := 101;
        Host.Insert(false);
        Line.Id := 202;
        Line.Insert(false);
        Line.Id := 303;
        Line.Target := Host.RecordId();
        Line.Insert(false);
        Card.OpenView();
        Card.Lines.First();
        s := Card.Lines.Id.Value();
        if Card.Lines.Next() then
            s += '|more:' + Card.Lines.Id.Value();
        Card.Close();
        exit(s);
    end;
}
"#;

#[test]
fn an_empty_const_on_a_record_id_shows_the_blank_child_only() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "F")), "202");
}
