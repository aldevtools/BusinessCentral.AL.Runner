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

/// Neighbors of the two shapes the rule serves (PR 7816 review round 2, the reviewer's probe): each keeps main's outcome.
const NEIGHBORS: &str = r#"
table 50300 "N Row" { fields { field(1; Id; Guid) { } field(2; Label; Text[30]) { } } keys { key(PK; Id) { Clustered = true; } } }
table 50302 "N TT" { TableType = Temporary; fields { field(1; Id; Guid) { } field(2; Label; Text[30]) { } } keys { key(PK; Id) { Clustered = true; } } }
table 50303 "N Sub" { fields { field(1; Id; Guid) { } field(2; Label; Text[30]) { } } keys { key(PK; Id) { Clustered = true; } } }
codeunit 50305 "N Subs"
{
    SingleInstance = true;
    var L: Text;
    procedure Take(): Text var T: Text; begin T := L; L := ''; exit(T); end;
    procedure Log(T: Text) begin L += T + ';'; end;
    [EventSubscriber(ObjectType::Table, Database::"N Sub", 'OnAfterInsertEvent', '', false, false)]
    local procedure Ins(var Rec: Record "N Sub"; RunTrigger: Boolean) begin L += 'ins;'; end;
    [EventSubscriber(ObjectType::Table, Database::"N Sub", 'OnAfterModifyEvent', '', false, false)]
    local procedure Mod(var Rec: Record "N Sub"; var xRec: Record "N Sub"; RunTrigger: Boolean) begin L += 'mod;'; end;
}
page 50318 "N SubPg" { PageType = NavigatePage; SourceTable = "N Sub"; SourceTableTemporary = true;
  layout { area(Content) { field(Label; Rec.Label) { ApplicationArea = All; } } }
  trigger OnOpenPage() begin Rec.Init(); Rec.Insert(); end; }
page 50319 "N TTPg" { PageType = NavigatePage; SourceTable = "N TT";
  layout { area(Content) { field(Label; Rec.Label) { ApplicationArea = All; } } }
  trigger OnOpenPage() begin Rec.Init(); Rec.Insert(); end; }
page 50320 "N Val" { PageType = NavigatePage; SourceTable = "N Row"; SourceTableTemporary = true;
  layout { area(Content) { field(Label; Rec.Label) { ApplicationArea = All; } } }
  trigger OnOpenPage() begin Rec.Init(); Rec.Id := CreateGuid(); Rec.Label := 'X'; Rec.Insert(); end; }
page 50324 "N Sub Part" { PageType = CardPart; SourceTable = "N Row";
  layout { area(Content) { field(Label; Rec.Label) { ApplicationArea = All; } } } }
page 50325 "N WithPart" { PageType = NavigatePage; SourceTable = "N Row"; SourceTableTemporary = true;
  layout { area(Content) { field(Label; Rec.Label) { ApplicationArea = All; } part(Sub; "N Sub Part") { ApplicationArea = All; } } }
  trigger OnOpenPage() begin Rec.Init(); Rec.Insert(); end; }
page 50326 "N NoInsert" { PageType = NavigatePage; SourceTable = "N Row"; SourceTableTemporary = true; InsertAllowed = false;
  layout { area(Content) { field(Label; Rec.Label) { ApplicationArea = All; } } }
  trigger OnOpenPage() begin Rec.Init(); Rec.Insert(); end; }
page 50327 "N NoEdit" { PageType = NavigatePage; SourceTable = "N Row"; SourceTableTemporary = true; Editable = false;
  layout { area(Content) { field(Label; Rec.Label) { ApplicationArea = All; } } }
  trigger OnOpenPage() begin Rec.Init(); Rec.Insert(); end; }
page 50328 "N NoModify" { PageType = NavigatePage; SourceTable = "N Row"; SourceTableTemporary = true; ModifyAllowed = false;
  layout { area(Content) { field(Label; Rec.Label) { ApplicationArea = All; } } }
  trigger OnOpenPage() begin Rec.Init(); Rec.Insert(); end; }
codeunit 50340 "N Tests"
{
    Subtype = Test;
    var Subs: Codeunit "N Subs";
    {TEST}
}
"#;

/// The neighbor tests and main's outcome of each (the reviewer's main run, 672097846).
const NEIGHBOR_TESTS: &[(&str, &str, &str)] = &[
    (
        "T09_TableSubscriber",
        r#"[Test] procedure T09_TableSubscriber() var P: TestPage "N SubPg"; begin P.OpenNew(); P.Label.SetValue('Draft'); P.Close(); Error('OBS log=%1', Subs.Take()); end;"#,
        "already exists",
    ),
    (
        "T10_TableTypeTemporary",
        r#"[Test] procedure T10_TableTypeTemporary() var P: TestPage "N TTPg"; begin P.OpenNew(); P.Label.SetValue('Draft'); P.Close(); Error('OBS closed %1', Subs.Take()); end;"#,
        "already exists",
    ),
    (
        "T11_NonBlankRow",
        r#"[Test] procedure T11_NonBlankRow() var P: TestPage "N Val"; begin P.OpenNew(); Subs.Log(P.Label.Value() + '/' + Format(P.Editable())); P.Close(); Error('OBS closed %1', Subs.Take()); end;"#,
        "OBS closed /Yes;",
    ),
    (
        "T15_WithPart",
        r#"[Test] procedure T15_WithPart() var P: TestPage "N WithPart"; begin P.OpenNew(); P.Label.SetValue('Draft'); P.Close(); Error('OBS closed %1', Subs.Take()); end;"#,
        "already exists",
    ),
    (
        "T16_NoInsert",
        r#"[Test] procedure T16_NoInsert() var P: TestPage "N NoInsert"; begin P.OpenNew(); P.Label.SetValue('Draft'); P.Close(); Error('OBS closed %1', Subs.Take()); end;"#,
        "already exists",
    ),
    (
        "T17_NoEdit",
        r#"[Test] procedure T17_NoEdit() var P: TestPage "N NoEdit"; begin P.OpenNew(); P.Label.SetValue('Draft'); P.Close(); Error('OBS closed %1', Subs.Take()); end;"#,
        "already exists",
    ),
    (
        "T18_NoModify",
        r#"[Test] procedure T18_NoModify() var P: TestPage "N NoModify"; begin P.OpenNew(); P.Label.SetValue('Draft'); P.Close(); Error('OBS closed %1', Subs.Take()); end;"#,
        "already exists",
    ),
];

#[test]
fn neighbors_of_the_two_shapes_keep_mains_open_new() {
    let mut wrong = Vec::new();
    for (name, test, main) in NEIGHBOR_TESTS {
        let o = al_testkit::outcomes_in(&NEIGHBORS.replace("{TEST}", test), "N Tests");
        match outcome(&o, name) {
            Outcome::Fail(why) if why.contains(main) => {}
            other => wrong.push(format!("{name}: {other:?} (main: {main})")),
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
