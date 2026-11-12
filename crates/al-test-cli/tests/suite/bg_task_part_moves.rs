//! A page part that enqueues a background task in OnAfterGetCurrRecord, as Base Application's FactBoxes do (Customer
//! and Vendor Statistics FactBox, Sales Doc. Check Factbox). Every enqueue runs its task, also a second one for the
//! record of the part's previous enqueue (OpenView and then GoToRecord of the same row): Microsoft's tests pass on BC
//! with this (Tests-SCM 137275 CalcInventoryWithIncludeItemWithoutTransaction*, 137033
//! ShowPositiveEntriesOnApplyToEntryLookup*, 137285 ServOrderStatistics*; Tests-ERM-Purchase 133772). Whether BC runs
//! both tasks is not pinned yet (al-runner issue 7576). A host that moves to another row runs the task again
//! (Customer Card OpenEdit + GoToRecord, Customer List OpenView + Next and OpenView + GoToRecord, Vendor Card GoToRecord
//! all pass on BC 29).

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
table 69990 "BPM Row"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 69991 "BPM Worker"
{
    trigger OnRun()
    var Results: Dictionary of [Text, Text];
    begin
        Results.Add('ok', 'yes');
        Page.SetBackgroundTaskResult(Results);
    end;
}
page 69992 "BPM Part"
{
    PageType = CardPart; SourceTable = "BPM Row";
    layout { area(Content) { field(Id; Rec.Id) { ApplicationArea = All; } } }
    var TaskId: Integer;
    trigger OnAfterGetCurrRecord()
    begin
        if Rec.Id = 0 then exit;
        CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"BPM Worker");
    end;
}
page 69993 "BPM List"
{
    PageType = List; SourceTable = "BPM Row"; UsageCategory = None;
    layout
    {
        area(Content) { repeater(Rows) { field(Id; Rec.Id) { ApplicationArea = All; } } }
        area(FactBoxes) { part(Details; "BPM Part") { ApplicationArea = All; SubPageLink = Id = field(Id); } }
    }
}
page 69994 "BPM Card"
{
    PageType = Card; SourceTable = "BPM Row"; UsageCategory = None;
    layout
    {
        area(Content) { field(Id; Rec.Id) { ApplicationArea = All; } }
        area(FactBoxes) { part(Details; "BPM Part") { ApplicationArea = All; SubPageLink = Id = field(Id); } }
    }
}
codeunit 69995 T
{
    Subtype = Test;
    TestPermissions = Disabled;
    local procedure Rows()
    var Row: Record "BPM Row";
    begin
        Row.DeleteAll();
        Row.Id := 1; Row.Insert();
        Row.Id := 2; Row.Insert();
        Commit();
    end;
    [Test] procedure CardOpenEditGoToOtherRecord()
    var Row: Record "BPM Row"; P: TestPage "BPM Card";
    begin
        Rows(); Row.Get(2); P.OpenEdit(); P.GoToRecord(Row); P.Close();
    end;
    [Test] procedure ListOpenViewNext()
    var P: TestPage "BPM List";
    begin
        Rows(); P.OpenView(); P.Next(); P.Close();
    end;
    [Test] procedure ListOpenViewGoToOtherRecord()
    var Row: Record "BPM Row"; P: TestPage "BPM List";
    begin
        Rows(); Row.Get(2); P.OpenView(); P.GoToRecord(Row); P.Close();
    end;
    [Test] procedure CardOpenViewOnly()
    var P: TestPage "BPM Card";
    begin
        Rows(); P.OpenView(); P.Close();
    end;
    [Test] procedure CardOpenViewGoToOtherRecord()
    var Row: Record "BPM Row"; P: TestPage "BPM Card";
    begin
        Rows(); Row.Get(2); P.OpenView(); P.GoToRecord(Row); P.Close();
    end;
    [Test] procedure ListOpenViewGoToSameRecord()
    var Row: Record "BPM Row"; P: TestPage "BPM List";
    begin
        Rows(); Row.Get(1); P.OpenView(); P.GoToRecord(Row); P.Close();
    end;
}
"#;

#[test]
fn a_part_enqueues_again_after_a_row_move_and_for_the_same_row() {
    let out = outcomes_in(SRC, "T");
    let get = |n: &str| out.iter().find(|(t, _)| t == n).map(|(_, o)| o.clone()).unwrap();
    for name in [
        "CardOpenEditGoToOtherRecord",
        "ListOpenViewNext",
        "ListOpenViewGoToOtherRecord",
        "CardOpenViewOnly",
        "CardOpenViewGoToOtherRecord",
        "ListOpenViewGoToSameRecord",
    ] {
        assert_eq!(get(name), Outcome::Pass, "{name}");
    }
}
