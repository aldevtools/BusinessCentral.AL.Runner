//! TestField.Lookup on a header control bound to a field of a global record of the page, whose OnLookup sets Text and
//! returns true and whose OnValidate saves the record and calls `CurrPage.Update(false)`: the control then reads the
//! picked value and the record holds it (al-runner issue 5843, the reporter's run on real BC 29.0.54011.55407, 2/2 tests).
//! The read after the Lookup is pinned by pages/lt-sc-outside-pages/ScLookupGlobalRecordField.al (run in outside_sc_pins.rs).

use al_test_cli::report::Outcome;

const SRC: &str = r#"
table 50110 "Repro Planner Setting 50110"
{
    fields
    {
        field(1; "Primary Key"; Code[10]) { }
        field(2; "Resource No."; Code[20]) { }
    }
    keys { key(PK; "Primary Key") { } }
}
table 50111 "Repro Resource 50111"
{
    LookupPageId = "Repro Resource List 50112";
    fields { field(1; "No."; Code[20]) { } }
    keys { key(PK; "No.") { } }
}
page 50112 "Repro Resource List 50112"
{
    PageType = List;
    SourceTable = "Repro Resource 50111";
    Editable = false;
    layout { area(Content) { repeater(Lines) { field("No."; Rec."No.") { } } } }
}
page 50113 "Repro Schedule 50113"
{
    PageType = List;
    SourceTable = "Repro Resource 50111";
    layout
    {
        area(Content)
        {
            group(Filters)
            {
                field(ResourceNo; Setting."Resource No.")
                {
                    Caption = 'Resource No.';
                    trigger OnLookup(var Text: Text): Boolean
                    var
                        Resource: Record "Repro Resource 50111";
                    begin
                        if Page.RunModal(Page::"Repro Resource List 50112", Resource) <> Action::LookupOK then
                            exit(false);
                        Text := Resource."No.";
                        exit(true);
                    end;
                    trigger OnValidate()
                    begin
                        Setting.Modify();
                        CurrPage.Update(false);
                    end;
                }
            }
            repeater(Lines) { field("No."; Rec."No.") { } }
        }
    }
    trigger OnOpenPage()
    begin
        if not Setting.Get() then
            Setting.Insert();
    end;
    var
        Setting: Record "Repro Planner Setting 50110";
}
codeunit 50114 T
{
    Subtype = Test;
    [Test]
    [HandlerFunctions('PickSecondResource')]
    procedure LookupShowsThePickedResource()
    var
        Schedule: TestPage "Repro Schedule 50113";
    begin
        Setup();
        Schedule.OpenEdit();
        Schedule.ResourceNo.Lookup();
        if Schedule.ResourceNo.Value() <> 'R2' then
            Error('Resource No. shows [%1] after the lookup.', Schedule.ResourceNo.Value());
        Schedule.Close();
    end;

    [Test]
    [HandlerFunctions('PickSecondResource')]
    procedure LookupSavesThePickedResource()
    var
        Setting: Record "Repro Planner Setting 50110";
        Schedule: TestPage "Repro Schedule 50113";
    begin
        Setup();
        Schedule.OpenEdit();
        Schedule.ResourceNo.Lookup();
        Schedule.Close();
        Setting.Get();
        if Setting."Resource No." <> 'R2' then
            Error('Setting holds [%1] after the lookup.', Setting."Resource No.");
    end;

    [Test]
    [HandlerFunctions('PickSecondResource')]
    procedure LookupWithoutUpdateShowsThePickedResource()
    var
        Schedule: TestPage "Repro Schedule 50115";
    begin
        Setup();
        Schedule.OpenEdit();
        Schedule.ResourceNo.Lookup();
        if Schedule.ResourceNo.Value() <> 'R2' then
            Error('Resource No. shows [%1] after the lookup.', Schedule.ResourceNo.Value());
    end;

    [ModalPageHandler]
    procedure PickSecondResource(var ResourceList: TestPage "Repro Resource List 50112")
    begin
        ResourceList.GoToKey('R2');
        ResourceList.OK().Invoke();
    end;

    local procedure Setup()
    var
        Setting: Record "Repro Planner Setting 50110";
        Resource: Record "Repro Resource 50111";
    begin
        Setting.DeleteAll();
        Resource.DeleteAll();
        Resource."No." := 'R1';
        Resource.Insert();
        Resource."No." := 'R2';
        Resource.Insert();
        Resource."No." := 'R3';
        Resource.Insert();
    end;
}
page 50115 "Repro Schedule 50115"
{
    PageType = List;
    SourceTable = "Repro Resource 50111";
    layout
    {
        area(Content)
        {
            group(Filters)
            {
                field(ResourceNo; Setting."Resource No.")
                {
                    trigger OnLookup(var Text: Text): Boolean
                    var
                        Resource: Record "Repro Resource 50111";
                    begin
                        if Page.RunModal(Page::"Repro Resource List 50112", Resource) <> Action::LookupOK then
                            exit(false);
                        Text := Resource."No.";
                        exit(true);
                    end;
                    trigger OnValidate()
                    begin
                        Setting.Modify();
                    end;
                }
            }
            repeater(Lines) { field("No."; Rec."No.") { } }
        }
    }
    trigger OnOpenPage()
    begin
        if not Setting.Get() then
            Setting.Insert();
    end;
    var
        Setting: Record "Repro Planner Setting 50110";
}
"#;

fn outcome(o: &[(String, Outcome)], name: &str) -> Outcome {
    o.iter()
        .find(|(n, _)| n == name)
        .map(|(_, o)| o.clone())
        .unwrap_or_else(|| panic!("no test {name}: {o:?}"))
}

#[test]
fn a_lookup_whose_on_validate_updates_the_page_shows_the_picked_value() {
    let o = al_testkit::outcomes_in(SRC, "T");
    assert_eq!(outcome(&o, "LookupShowsThePickedResource"), Outcome::Pass, "{o:?}");
    assert_eq!(outcome(&o, "LookupSavesThePickedResource"), Outcome::Pass, "{o:?}");
    // Without CurrPage.Update(false) in the OnValidate too (ScLookupGlobalRecordField.al LookupWithoutUpdateShowsThePickedResource).
    assert_eq!(
        outcome(&o, "LookupWithoutUpdateShowsThePickedResource"),
        Outcome::Pass,
        "{o:?}"
    );
}

/// The same page with a ListPart on the same table without a SubPageLink (al-runner issue 6009, the reporter's run on real
/// BC 29.0.54011.55407: 2/2 tests, `ResourceNo` shows `R2` after the lookup), and the forms nobody measured: a part with a
/// SubPageLink, and the part with no `CurrPage.Update(false)` in the OnValidate. `PAGE` is filled in per form.
const PART_SRC: &str = r#"
table 50120 "Repro Planner Setting 50120"
{
    fields
    {
        field(1; "Primary Key"; Code[10]) { }
        field(2; "Resource No."; Code[20]) { }
    }
    keys { key(PK; "Primary Key") { } }
}
table 50121 "Repro Resource 50121"
{
    LookupPageId = "Repro Resource List 50122";
    fields { field(1; "No."; Code[20]) { } }
    keys { key(PK; "No.") { } }
}
page 50122 "Repro Resource List 50122"
{
    PageType = List;
    SourceTable = "Repro Resource 50121";
    Editable = false;
    layout { area(Content) { repeater(Lines) { field("No."; Rec."No.") { } } } }
}
page 50123 "Repro Resource Part 50123"
{
    PageType = ListPart;
    SourceTable = "Repro Resource 50121";
    Editable = false;
    layout { area(Content) { repeater(Lines) { field("No."; Rec."No.") { } } } }
}
page 50124 "Repro Schedule 50124"
{
    PageType = List;
    SourceTable = "Repro Resource 50121";
    layout
    {
        area(Content)
        {
            group(Filters)
            {
                field(ResourceNo; Setting."Resource No.")
                {
                    trigger OnLookup(var Text: Text): Boolean
                    var
                        Resource: Record "Repro Resource 50121";
                    begin
                        if Page.RunModal(Page::"Repro Resource List 50122", Resource) <> Action::LookupOK then
                            exit(false);
                        Text := Resource."No.";
                        exit(true);
                    end;
                    trigger OnValidate()
                    begin
                        Setting.Modify();
                        UPDATE
                    end;
                }
            }
            repeater(Lines) { field("No."; Rec."No.") { } }
            part(Resources; "Repro Resource Part 50123")
            {
                LINK
            }
        }
    }
    trigger OnOpenPage()
    begin
        if not Setting.Get() then
            Setting.Insert();
    end;
    var
        Setting: Record "Repro Planner Setting 50120";
}
codeunit 50125 T
{
    Subtype = Test;
    [Test]
    [HandlerFunctions('PickSecondResource')]
    procedure LookupShowsThePickedResourceOnPageWithPart()
    var
        Schedule: TestPage "Repro Schedule 50124";
    begin
        Setup();
        Schedule.OpenEdit();
        Schedule.ResourceNo.Lookup();
        if Schedule.ResourceNo.Value() <> 'R2' then
            Error('Resource No. shows [%1] after the lookup.', Schedule.ResourceNo.Value());
        Schedule.Close();
    end;

    [ModalPageHandler]
    procedure PickSecondResource(var ResourceList: TestPage "Repro Resource List 50122")
    begin
        ResourceList.GoToKey('R2');
        ResourceList.OK().Invoke();
    end;

    local procedure Setup()
    var
        Setting: Record "Repro Planner Setting 50120";
        Resource: Record "Repro Resource 50121";
    begin
        Setting.DeleteAll();
        Resource.DeleteAll();
        Resource."No." := 'R1';
        Resource.Insert();
        Resource."No." := 'R2';
        Resource.Insert();
        Resource."No." := 'R3';
        Resource.Insert();
    end;
}
"#;

fn part_outcome(update: &str, link: &str) -> Outcome {
    let src = PART_SRC.replace("UPDATE", update).replace("LINK", link);
    al_builtins::assumptions::clear_hits();
    let o = al_testkit::outcomes_in(&src, "T");
    outcome(&o, "LookupShowsThePickedResourceOnPageWithPart")
}

#[test]
fn a_lookup_on_a_page_with_an_unlinked_part_shows_the_picked_value() {
    // Pinned: pages/lt-sc-outside-pages/ScLookupGlobalRecordField.al:469 LookupUpdateFalseOnPart `shows=[R2]`.
    assert_eq!(part_outcome("CurrPage.Update(false);", ""), Outcome::Pass);
    assert!(al_builtins::assumptions::take_hits().is_empty());
}

#[test]
fn a_lookup_on_a_page_with_a_part_keeps_the_unmeasured_forms_refused() {
    // A part with a SubPageLink.
    let linked = part_outcome("CurrPage.Update(false);", "SubPageLink = \"No.\" = field(\"No.\");");
    assert!(
        matches!(linked, Outcome::NotRun(ref r) if r.contains("not pinned")),
        "{linked:?}"
    );
    // No CurrPage.Update in the OnValidate.
    let no_update = part_outcome("", "");
    assert!(
        matches!(no_update, Outcome::NotRun(ref r) if r.contains("not pinned")),
        "{no_update:?}"
    );
    // CurrPage.Update(true).
    let update_true = part_outcome("CurrPage.Update(true);", "");
    assert!(
        matches!(update_true, Outcome::NotRun(ref r) if r.contains("not pinned")),
        "{update_true:?}"
    );
}
