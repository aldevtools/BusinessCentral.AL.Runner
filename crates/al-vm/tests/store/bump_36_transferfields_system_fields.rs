//! TransferFields from another table and the buffer's SystemCreatedAt and SystemModifiedAt (corpus bump 36, writer QE).

use al_testkit::text;

const SRC: &str = r#"
table 69991 "QE Row"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } }
    keys { key(PK; Id) { } }
}
table 69992 "QE Other"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } }
    keys { key(PK; Id) { } }
}
codeunit 69993 C
{
    procedure CreatedAtAfterTransferFromOther(): Text
    var R: Record "QE Row"; O: Record "QE Other";
    begin
        O.Id := 1; O.Name := 'other'; O.Insert();
        R.Id := 1; R.Name := 'own'; R.Insert();
        R.TransferFields(O);
        exit(Format(R.SystemCreatedAt = O.SystemCreatedAt));
    end;

    procedure ModifiedAtAfterTransferFromOther(): Text
    var R: Record "QE Row"; O: Record "QE Other";
    begin
        O.Id := 1; O.Name := 'other'; O.Insert();
        R.Id := 1; R.Name := 'own'; R.Insert();
        R.TransferFields(O);
        exit(Format(R.SystemModifiedAt = O.SystemModifiedAt));
    end;

    procedure StaleBufferSourceNewer(): Text
    var R: Record "QE Row"; R2: Record "QE Row"; O: Record "QE Other";
    begin
        R.Id := 1; R.Name := 'own'; R.Insert();
        Commit();
        R.Get(1);
        R2.Get(1); R2.Name := 'second'; R2.Modify();
        Commit();
        O.Id := 1; O.Name := 'other'; O.Insert();
        Commit();
        O.Get(1);
        R.TransferFields(O);
        asserterror R.Modify();
        R2.Get(1);
        exit(GetLastErrorCode() + '|' + R2.Name);
    end;

    procedure CurrentBufferSourceOlder(): Text
    var R: Record "QE Row"; O: Record "QE Other";
    begin
        O.Id := 1; O.Name := 'other'; O.Insert();
        Commit();
        R.Id := 1; R.Name := 'own'; R.Insert();
        Commit();
        R.Get(1);
        O.Get(1);
        R.TransferFields(O);
        R.Modify();
        R.Get(1);
        exit(R.Name);
    end;

    procedure AfterModify(): Text
    var R: Record "QE Row"; R4: Record "QE Row"; O: Record "QE Other"; OwnCreated: DateTime; OwnId: Guid;
    begin
        R.Id := 1; R.Name := 'own'; R.Insert();
        Commit();
        Sleep(50);
        O.Id := 1; O.Name := 'other'; O.Insert();
        Commit();
        R.Get(1);
        OwnCreated := R.SystemCreatedAt;
        OwnId := R.SystemId;
        O.Get(1);
        R.TransferFields(O);
        R.Modify();
        R4.Get(1);
        exit(Format(R4.SystemCreatedAt = OwnCreated) + '|' + Format(R4.SystemId = OwnId) + '|' + Format(R4.SystemModifiedAt > O.SystemModifiedAt) + '|' + R4.Name);
    end;

    procedure CreatedByAfterTransferFromOther(): Text
    var R: Record "QE Row"; O: Record "QE Other";
    begin
        O.Id := 1; O.Name := 'other'; O.Insert();
        R.Id := 1; R.Name := 'own'; R.Insert();
        R.TransferFields(O);
        exit(Format(R.SystemCreatedBy = O.SystemCreatedBy) + Format(IsNullGuid(R.SystemCreatedBy)));
    end;

    procedure CreatedAtAfterTransferFromMemory(): Text
    var R: Record "QE Row"; O: Record "QE Other";
    begin
        R.Id := 1; R.Name := 'own'; R.Insert();
        O.Id := 1; O.Name := 'mem';
        R.TransferFields(O);
        exit(Format(R.SystemCreatedAt));
    end;

    procedure NameAfterTransferFromOther(): Text
    var R: Record "QE Row"; O: Record "QE Other";
    begin
        O.Id := 1; O.Name := 'other'; O.Insert();
        R.Id := 1; R.Name := 'own'; R.Insert();
        R.TransferFields(O);
        exit(R.Name + '|' + Format(R.SystemId = O.SystemId));
    end;
}
"#;

// record/crud/CnTransferFieldsModify.al:414-479 QE_TF_Other_SystemFields_AfterModify
// (`buffer createdat = other`, `buffer modifiedat = other`): from another table the buffer takes the source's
// SystemCreatedAt and SystemModifiedAt.
#[test]
fn system_created_at_after_a_transfer_from_another_table_is_the_source_one() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "CreatedAtAfterTransferFromOther")),
        "Yes"
    );
}

#[test]
fn system_modified_at_after_a_transfer_from_another_table_is_the_source_one() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "ModifiedAtAfterTransferFromOther")),
        "Yes"
    );
}

// From a row built in memory the buffer takes its blank SystemCreatedAt (CnTransferFieldsModify.al:543
// QH_TF_Other_SourceInMemory_AuditFields `buffer createdat blank`).
#[test]
fn system_created_at_after_a_transfer_from_a_row_in_memory_is_blank() {
    let r = al_testkit::run_in(SRC, "C", "CreatedAtAfterTransferFromMemory");
    assert_eq!(text(r), "");
}

// SystemCreatedBy comes from the source too (CnTransferFieldsModify.al:501 QH_TF_Other_ByFields_SourceFromDb `buffer createdby =
// other`): the source's stamp, the session's user.
#[test]
fn system_created_by_after_a_transfer_from_another_table_is_the_sources() {
    let r = al_testkit::run_in(SRC, "C", "CreatedByAfterTransferFromOther");
    assert_eq!(text(r), "YesNo");
}

// CnTransferFieldsModify.al:271-290 QE_TF_Other_StaleBuffer_SourceNewer: the buffer keeps its own row version, so a stale
// buffer stays stale whatever the source.
#[test]
fn a_stale_buffer_stays_stale_after_a_transfer_from_another_table() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "StaleBufferSourceNewer")),
        "DB:RecordChanged|second"
    );
}

// CnTransferFieldsModify.al:336-355 QE_TF_Other_CurrentBuffer_SourceOlder: a current buffer modifies fine.
#[test]
fn a_current_buffer_modifies_after_a_transfer_from_an_older_row() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "CurrentBufferSourceOlder")),
        "other"
    );
}

// CnTransferFieldsModify.al:414-479: after the Modify the row keeps its own SystemId and SystemCreatedAt and gets a new
// SystemModifiedAt.
#[test]
fn the_row_keeps_its_own_created_at_after_the_modify() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "AfterModify")),
        "Yes|Yes|Yes|other"
    );
}

// The same test pins that the buffer keeps its own SystemId (`buffer systemid = own`) and that normal fields move.
#[test]
fn normal_fields_and_system_id_after_a_transfer_from_another_table() {
    let r = al_testkit::run_in(SRC, "C", "NameAfterTransferFromOther");
    assert_eq!(al_testkit::text(r), "other|No");
}
