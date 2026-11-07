//! A list with Editable = false (ModifyAllowed left true) over a table with a Blob field: an action that releases the selected row
//! through its own record, and leaves Rec as it was, must not be overwritten by the page saving a stale Rec after the action
//! (Tests-ERM-Sales "Sales Batch Document Posting" ReleaseSelectedQuotes and 13 siblings: Sales Quotes, Sales Order List, ...
//! have Editable = false and the Work Description Blob). The page-unwritten-blob-no-change-not-editable assumption.

use al_testkit::{run_in, text};

const SRC: &str = r#"
table 69985 "EB Doc"
{
    fields { field(1; Code; Code[10]) { } field(2; Status; Option) { OptionMembers = Open,Released; } field(3; Note; Blob) { } }
    keys { key(PK; Code) { } }
}
page 69986 "EB List"
{
    PageType = List; SourceTable = "EB Doc"; Editable = false;
    ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(Code; Rec.Code) { } field(Status; Rec.Status) { } } } }
    actions { area(Processing) {
        action(Release) { trigger OnAction() var D: Record "EB Doc"; begin CurrPage.SetSelectionFilter(D); D.FindFirst(); D.Status := D.Status::Released; D.Modify(); end; }
    } }
}
codeunit 69987 C
{
    procedure Release(): Text
    var R: Record "EB Doc"; P: TestPage "EB List";
    begin
        R.Code := 'A'; R.Insert();
        R.Code := 'B'; R.Insert();
        P.OpenView();
        P.GoToKey('B');
        P.Release.Invoke();
        R.Get('B');
        exit(Format(R.Status));
    end;
}
"#;

#[test]
fn an_action_that_releases_the_row_through_its_own_record_keeps_the_release() {
    assert_eq!(text(run_in(SRC, "C", "Release")), "Released");
}
