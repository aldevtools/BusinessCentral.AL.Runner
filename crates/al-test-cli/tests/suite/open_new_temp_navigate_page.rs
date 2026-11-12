//! TestPage.OpenNew() on a NavigatePage over a temporary source whose OnOpenPage inserted its blank record (al-runner
//! issue 7811, the reporter's repro unchanged): a SetValue edits the row OnOpenPage inserted, and the real table stays
//! empty. Microsoft's SMTP Account Auth Tests open the SMTP Account Wizard this way and pass on BC; the path is flagged
//! (assumption `open-new-temp-navigate-page-edits-inserted-row`) until the corpus probe lands.

use al_testrun::Outcome;

const SRC: &str = r#"
table 50270 "Probe Setting"
{
    fields
    {
        field(1; Id; Guid) { }
        field(2; Label; Text[30]) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

page 50271 "Probe Setup Guide"
{
    PageType = NavigatePage;
    SourceTable = "Probe Setting";
    SourceTableTemporary = true;

    layout
    {
        area(Content)
        {
            field(Label; Rec.Label) { ApplicationArea = All; }
        }
    }

    trigger OnOpenPage()
    begin
        Rec.Init();
        Rec.Insert();
    end;
}

codeunit 50272 "Probe Temp Page Tests"
{
    Subtype = Test;

    [Test]
    procedure OpenNewThenSetValue()
    var
        Guide: TestPage "Probe Setup Guide";
        Setting: Record "Probe Setting";
    begin
        Guide.OpenNew();
        Guide.Label.SetValue('Draft');   // fails here on al-runner
        Guide.Close();
        if not Setting.IsEmpty() then
            Error('the real table holds %1 record(s), expected 0', Setting.Count());
    end;

    [Test]
    procedure OpenNewWithoutSetValue()
    var
        Guide: TestPage "Probe Setup Guide";
    begin
        Guide.OpenNew();
        Guide.Close();
        Guide.OpenNew();
        Guide.Close();
    end;
}
"#;

fn outcome(outcomes: &[(String, Outcome)], name: &str) -> Outcome {
    outcomes
        .iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no test {name}"))
        .1
        .clone()
}

/// A neighbor outside the shape: the same page as a Card keeps the new row of OpenNew (main's behavior).
const CARD: &str = r#"
table 50273 "Probe Card Setting"
{
    fields { field(1; Id; Guid) { } field(2; Label; Text[30]) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
page 50274 "Probe Card Guide"
{
    PageType = Card; SourceTable = "Probe Card Setting"; SourceTableTemporary = true;
    layout { area(Content) { field(Label; Rec.Label) { ApplicationArea = All; } } }
    trigger OnOpenPage() begin Rec.Init(); Rec.Insert(); end;
}
codeunit 50275 "Probe Card Tests"
{
    Subtype = Test;
    [Test] procedure CardOpenNewThenSetValue()
    var Guide: TestPage "Probe Card Guide";
    begin Guide.OpenNew(); Guide.Label.SetValue('Draft'); Guide.Close(); end;
}
"#;

#[test]
fn a_card_of_the_same_shape_keeps_the_new_row() {
    let o = al_testkit::outcomes_in(CARD, "Probe Card Tests");
    match outcome(&o, "CardOpenNewThenSetValue") {
        Outcome::Fail(why) => assert!(why.contains("already exists"), "{why}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn open_new_edits_the_row_onopenpage_inserted_into_the_temporary_source() {
    let o = al_testkit::outcomes_in(SRC, "Probe Temp Page Tests");
    assert_eq!(outcome(&o, "OpenNewThenSetValue"), Outcome::Pass, "{o:?}");
    assert_eq!(outcome(&o, "OpenNewWithoutSetValue"), Outcome::Pass, "{o:?}");
}
