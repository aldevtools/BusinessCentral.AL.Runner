//! A table method reads xRec after a partial fetch (al-runner issue 7375). xRec of a plain procedure is a blank record: every
//! field reads empty, the key too, and no load from the stored row happens for it, while a field of Rec that the load set
//! omits still loads from the row (corpus record/load-fields/XRecUnloadedFieldRead.al, PartialGet_IsBlocked_IssueShape,
//! PartialGet_XRecFields_ReadInTableMethod, PartialGet_RowChangedAfterGet_XRecReadFirst,
//! PartialGet_KeyAssignedAfterGet_ReadInTableMethod, LineOnInsert_PartialHeaderGet_IsBlocked).

use al_testkit::{boolean, run_in, text};

const SRC: &str = r#"
table 50871 "XU Header"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; "Template Code"; Code[20]) { }
        field(3; Status; Text[10]) { }
    }
    keys { key(PK; "No.") { Clustered = true; } }
    procedure IsBlocked(): Boolean
    begin
        if xRec.IsInitial() or Rec.IsInitial() then
            exit(true);
    end;
    procedure IsInitial(): Boolean
    begin
        if "Template Code" = '' then
            exit(false);
        exit(Status = 'NEW');
    end;
    procedure XStatus(): Text
    begin
        exit(xRec.Status);
    end;
    procedure XNo(): Text
    begin
        exit(xRec."No.");
    end;
    procedure RStatus(): Text
    begin
        exit(Status);
    end;
}
codeunit 50871 "XU Tests"
{
    local procedure Seed()
    var H: Record "XU Header";
    begin
        H.DeleteAll();
        H."No." := 'A1'; H."Template Code" := 'T1'; H.Status := 'NEW'; H.Insert();
    end;
    procedure IsBlocked(): Boolean
    var H: Record "XU Header";
    begin
        Seed(); H.SetLoadFields("No."); H.Get('A1'); exit(H.IsBlocked());
    end;
    procedure XStatus(): Text
    var H: Record "XU Header";
    begin
        Seed(); H.SetLoadFields("No."); H.Get('A1'); exit(H.XStatus() + '|' + H.XNo());
    end;
    procedure ChangedElsewhere(): Text
    var H: Record "XU Header"; O: Record "XU Header";
    begin
        Seed(); H.SetLoadFields("No."); H.Get('A1');
        O.Get('A1'); O.Status := 'CHANGED'; O.Modify();
        exit(H.XStatus() + '|' + H.RStatus());
    end;
    procedure KeyAssigned(): Text
    var H: Record "XU Header";
    begin
        Seed(); H.SetLoadFields("No."); H.Get('A1'); H."No." := 'A2';
        exit(H.RStatus() + '|' + H.XNo() + '|' + H.XStatus());
    end;
}
"#;

#[test]
fn xrec_of_a_plain_table_method_reads_empty_after_a_partial_get() {
    assert!(boolean(run_in(SRC, "XU Tests", "IsBlocked")));
    assert_eq!(text(run_in(SRC, "XU Tests", "XStatus")), "|");
}

#[test]
fn xrec_stays_empty_when_the_row_changed_after_the_get() {
    assert_eq!(text(run_in(SRC, "XU Tests", "ChangedElsewhere")), "|CHANGED");
}

#[test]
fn xrec_stays_empty_when_the_key_was_assigned_after_the_get() {
    assert_eq!(text(run_in(SRC, "XU Tests", "KeyAssigned")), "NEW||");
}
