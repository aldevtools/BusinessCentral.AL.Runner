//! From BC 28 a TestPage read of the Value of a page extension's control whose source is a bare quoted name of a table
//! extension's field raises TestFieldNotFound (pages/lt-wd/WdBareQuotedField.al:185-198 WD_BareNameExt_ValueReadError,
//! al-runner issue 6314). The same field bound through `Rec.` is read as usual.

use al_test_cli::report::Outcome;

const SRC: &str = r#"
table 69990 "BQ Doc" { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { Clustered = true; } } }
tableextension 69991 "BQ Doc Ext" extends "BQ Doc" { fields { field(69991; "BQ Profile Code"; Code[20]) { } } }
page 69992 "BQ Card"
{
    PageType = Card; SourceTable = "BQ Doc"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { group(General) { field("No."; Rec."No.") { } } } }
}
pageextension 69993 "BQ Card Ext" extends "BQ Card"
{
    layout
    {
        addlast(General)
        {
            field("BQ Profile"; "BQ Profile Code") { }
            field("BQ Profile Rec"; Rec."BQ Profile Code") { }
        }
    }
}
codeunit 69994 T
{
    Subtype = Test;
    local procedure Setup()
    var D: Record "BQ Doc";
    begin
        D.DeleteAll(); D."No." := 'D1'; D."BQ Profile Code" := 'P1'; D.Insert();
    end;
    [Test]
    procedure BareRead()
    var Card: TestPage "BQ Card";
    begin
        Setup();
        Card.OpenView();
        Card.First();
        asserterror if Card."BQ Profile".Value() = '' then;
        if GetLastErrorCode() <> 'TestFieldNotFound' then Error('code %1', GetLastErrorCode());
        if not GetLastErrorText().StartsWith('The field with ID = ') then Error('text %1', GetLastErrorText());
        if not GetLastErrorText().EndsWith(' is not found on the page.') then Error('text %1', GetLastErrorText());
    end;
    [Test]
    procedure RecRead()
    var Card: TestPage "BQ Card";
    begin
        Setup();
        Card.OpenView();
        Card.First();
        if Card."BQ Profile Rec".Value() <> 'P1' then Error('%1', Card."BQ Profile Rec".Value());
    end;
}
"#;

#[test]
fn a_bare_quoted_table_extension_field_in_a_page_extension_is_not_found_by_a_testpage() {
    let out = al_testkit::outcomes_in(SRC, "T");
    assert_eq!(out.len(), 2);
    for (n, o) in out {
        assert_eq!(o, Outcome::Pass, "{n}");
    }
}
