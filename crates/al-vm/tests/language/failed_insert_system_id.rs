//! A guarded Insert that fails on an existing key leaves the variable's SystemId null, and the Modify after it gives the variable
//! the stored row's SystemId (al-runner issue 6153; corpus events/lt-uf-failed-insert-sysid/UfFailedInsertSystemId.al, writer UF
//! branch al-runtime-rs-staging-uf, commit 3d7d7b66).

use al_testkit::text;

const SRC: &str = r#"
table 69796 "UF Row" { fields { field(1; "No."; Code[20]) { } field(2; Name; Text[50]) { } } keys { key(PK; "No.") { } } }
codeunit 69797 C
{
    procedure SysId(): Text
    var First: Record "UF Row"; Entry: Record "UF Row"; Stored: Record "UF Row"; Null: Guid; T: Text;
    begin
        First."No." := 'C'; First.Insert(false);
        Entry."No." := 'C'; Entry.Name := 'changed';
        T += Format(Entry.Insert(false)) + ';' + Format(Entry.SystemId = Null) + ';';
        Entry.Modify(false);
        Stored.Get('C');
        T += Format(Entry.SystemId = Stored.SystemId) + ';' + Format(Stored.SystemId = First.SystemId) + ';' + Stored.Name;
        exit(T);
    end;

}
"#;

#[test]
fn failed_insert_then_modify_takes_the_stored_system_id() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "SysId")), "No;Yes;Yes;Yes;changed");
}
