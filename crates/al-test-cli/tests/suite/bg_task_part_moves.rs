//! A page part that enqueues a background task in OnAfterGetCurrRecord, as Base Application's FactBoxes do (Customer
//! and Vendor Statistics FactBox, Sales Doc. Check Factbox). A second enqueue for the record of the part's previous
//! enqueue is refused (session-system/isolated-storage/IsolatedStorageInBackgroundTask.al:242-251: OpenView and then
//! GoToRecord of the same row, where BC reports one task's outcome). A host that moves to another row runs the task again
//! as before (al-runner review of pull request 7828: Customer Card OpenEdit + GoToRecord, Customer List OpenView + Next
//! and OpenView + GoToRecord, Vendor Card GoToRecord all pass on BC 29).

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
    [Test] procedure ListOpenViewGoToSameRecordRefused()
    var Row: Record "BPM Row"; P: TestPage "BPM List";
    begin
        Rows(); Row.Get(1); P.OpenView(); P.GoToRecord(Row); P.Close();
    end;
}
"#;

#[test]
fn a_part_enqueues_again_after_a_row_move_and_refuses_a_second_enqueue_for_the_same_row() {
    let out = outcomes_in(SRC, "T");
    let get = |n: &str| out.iter().find(|(t, _)| t == n).map(|(_, o)| o.clone()).unwrap();
    for name in [
        "CardOpenEditGoToOtherRecord",
        "ListOpenViewNext",
        "ListOpenViewGoToOtherRecord",
        "CardOpenViewOnly",
        "CardOpenViewGoToOtherRecord",
    ] {
        assert_eq!(get(name), Outcome::Pass, "{name}");
    }
    match get("ListOpenViewGoToSameRecordRefused") {
        Outcome::NotRun(reason) => assert!(reason.contains("for the same record"), "{reason}"),
        other => panic!("{other:?}"),
    }
}
