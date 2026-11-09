//! A part whose Visible is bound to a Boolean field of Rec (on the host page, and added by a page extension to the FactBoxes)
//! reports Visible() false, and so does a control inside it, when the field is false on the opened record; true when it is
//! true (corpus master pages/parts/PartVisibleBoundToRecField.al; al-runner issue 7455).

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
table 69984 "PVR Host"
{
    fields { field(1; Code; Code[10]) { } field(2; "Show Part"; Boolean) { } field(3; Name; Text[30]) { } }
    keys { key(PK; Code) { Clustered = true; } }
}
table 69985 "PVR Line"
{
    fields { field(1; Code; Code[10]) { } field(2; "Line No."; Integer) { } field(3; Description; Text[30]) { } }
    keys { key(PK; Code, "Line No.") { Clustered = true; } }
}
page 69986 "PVR Lines"
{
    PageType = ListPart; SourceTable = "PVR Line"; UsageCategory = None;
    layout { area(Content) { repeater(Rep) { field(Description; Rec.Description) { ApplicationArea = All; } } } }
}
page 69987 "PVR Host Card"
{
    PageType = Card; SourceTable = "PVR Host"; UsageCategory = None;
    layout
    {
        area(Content) { field(Name; Rec.Name) { ApplicationArea = All; } }
        area(FactBoxes)
        {
            part(BasePart; "PVR Lines") { ApplicationArea = All; SubPageLink = Code = field(Code); Visible = Rec."Show Part"; }
        }
    }
}
pageextension 69988 "PVR Host Card Ext" extends "PVR Host Card"
{
    layout
    {
        addlast(FactBoxes)
        {
            part(ExtPart; "PVR Lines") { ApplicationArea = All; SubPageLink = Code = field(Code); Visible = Rec."Show Part"; }
        }
    }
}
codeunit 69989 T
{
    Subtype = Test;
    local procedure Seed()
    var Host: Record "PVR Host"; Line: Record "PVR Line";
    begin
        Host.DeleteAll(); Line.DeleteAll();
        Host.Init(); Host.Code := 'HIDDEN'; Host."Show Part" := false; Host.Insert();
        Host.Code := 'SHOWN'; Host."Show Part" := true; Host.Insert();
        Line.Code := 'HIDDEN'; Line."Line No." := 1; Line.Description := 'hidden line'; Line.Insert();
        Line.Code := 'SHOWN'; Line.Description := 'shown line'; Line.Insert();
    end;
    local procedure Seen(Code: Code[10]): Text
    var Host: Record "PVR Host"; Card: TestPage "PVR Host Card";
    begin
        Seed();
        Host.Get(Code);
        Card.Trap();
        Page.Run(Page::"PVR Host Card", Host);
        exit(Format(Card.BasePart.Visible()) + Format(Card.BasePart.Description.Visible()) + Format(Card.ExtPart.Visible())
            + Format(Card.ExtPart.Description.Visible()));
    end;
    [Test] procedure Hidden() var S: Text; begin S := Seen('HIDDEN'); if S <> 'NoNoNoNo' then Error(S); end;
    [Test] procedure Shown() var S: Text; begin S := Seen('SHOWN'); if S <> 'YesYesYesYes' then Error(S); end;
}
"#;

#[test]
fn a_part_bound_to_a_false_rec_field_and_its_controls_are_not_visible() {
    let mut got = outcomes_in(SRC, "T");
    got.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        got,
        vec![
            ("Hidden".to_string(), Outcome::Pass),
            ("Shown".to_string(), Outcome::Pass),
        ]
    );
}
