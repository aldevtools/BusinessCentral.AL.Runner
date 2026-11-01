//! An extension that names its target by id (`pageextension 50101 E extends 50100`, `tableextension .. extends 50100`):
//! the AL compiler takes it, and on BC 29 a TestPage sees the control the page extension adds (al-runner issue 5576,
//! measured on BC 29.0.54011.55407 by the reporter; alc 18.0 compiles both forms).

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
table 69960 "NE Row"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
tableextension 69961 "NE Row Ext" extends 69960
{
    fields { field(69960; Extra; Integer) { } }
}
page 69960 "NE Card"
{
    PageType = Card;
    SourceTable = "NE Row";
    layout { area(Content) { field(Id; Rec.Id) { } } }
}
pageextension 69961 "NE Card Ext" extends 69960
{
    layout { addlast(Content) { field(AddedValue; Rec.Extra) { } } }
}
codeunit 69962 T
{
    Subtype = Test;
    [Test]
    procedure ExtensionByIdAddsControl()
    var Card: TestPage "NE Card"; Row: Record "NE Row";
    begin
        Row.Id := 1; Row.Extra := 7; Row.Insert();
        Card.OpenView(); Card.GoToRecord(Row);
        Card.AddedValue.AssertEquals(7);
        Card.Close();
    end;
}
"#;

#[test]
fn an_extension_by_target_id_extends_that_object() {
    let out = outcomes_in(SRC, "T");
    assert_eq!(out, vec![("ExtensionByIdAddsControl".to_string(), Outcome::Pass)]);
}
