//! `RecordRef.LockTable` and the write transaction (recordref/lt-wc/WcRecordRefLockTable.al, al-runner issue 6312): on an open
//! database table it starts the write transaction, which a read keeps and a Commit ends; on a temporary table it does not; on a
//! RecordRef that is not open (never opened, `Open(0)`, closed) it raises "The record is not open.".

use al_testkit::text;

const SRC: &str = r#"
table 50495 "RLT Entry" { fields { field(1; "Entry No."; Integer) { } } keys { key(PK; "Entry No.") { Clustered = true; } } }
codeunit 50495 C
{
    local procedure Tx(): Text begin exit(Format(IsInWriteTransaction())); end;
    [TryFunction] local procedure TryLock(var RR: RecordRef) begin RR.LockTable(); end;
    procedure WriteTx(): Text
    var RR: RecordRef; Out: Text;
    begin
        Commit();
        RR.Open(Database::"RLT Entry");
        Out := Tx();
        RR.LockTable();
        Out += '|' + Tx();
        if RR.FindFirst() then;
        Out += '|' + Tx();
        Commit();
        exit(Out + '|' + Tx());
    end;
    procedure Temporary(): Text
    var RR: RecordRef;
    begin
        Commit();
        RR.Open(Database::"RLT Entry", true);
        RR.LockTable();
        exit(Tx());
    end;
    procedure NotOpen(): Text
    var RR: RecordRef; Out: Text;
    begin
        Commit();
        if not TryLock(RR) then Out := GetLastErrorText() + '|' + GetLastErrorCode();
        RR.Open(0);
        if not TryLock(RR) then Out += '/' + GetLastErrorText() + '|' + GetLastErrorCode();
        RR.Open(Database::"RLT Entry");
        RR.Close();
        if not TryLock(RR) then Out += '/' + GetLastErrorText() + '|' + GetLastErrorCode();
        exit(Out + '/' + Tx());
    end;
}
"#;

fn run(routine: &str) -> String {
    text(al_testkit::run_in(SRC, "C", routine))
}

/// WcRecordRefLockTable.al:28-51 WC_RecordRef_LockTable_WriteTransaction.
#[test]
fn locktable_on_a_database_table_starts_the_write_transaction() {
    assert_eq!(run("WriteTx"), "No|Yes|Yes|No");
}

/// WcRecordRefLockTable.al:76-87 WC_RecordRef_LockTable_Temporary.
#[test]
fn locktable_on_a_temporary_table_starts_none() {
    assert_eq!(run("Temporary"), "No");
}

/// WcRecordRefLockTable.al:89-137: never opened, Open(0), closed.
#[test]
fn locktable_on_a_ref_that_is_not_open_raises() {
    assert_eq!(
        run("NotOpen"),
        "The record is not open.|RecordNotOpened/The record is not open.|RecordNotOpened/The record is not open.|RecordNotOpened/No"
    );
}
