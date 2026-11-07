//! IsolatedStorage.Delete of a missing key answers false and opens the write transaction
//! (corpus session-system/Y76IsolatedStorage.al Y76_PlainStorage_Transactions `delete_missing`).

use al_testkit::text;

const SRC: &str = r#"
codeunit 69990 C
{
    procedure F(): Text
    var Ret: Boolean; s: Text;
    begin
        Commit();
        s := Format(Database.IsInWriteTransaction());
        Ret := IsolatedStorage.Delete('LTMISSING', DataScope::Module);
        exit(s + '|' + Format(Ret) + '|' + Format(Database.IsInWriteTransaction()));
    end;
}
"#;

#[test]
fn deleting_a_missing_key_opens_the_write_transaction() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "F")), "No|No|Yes");
}
