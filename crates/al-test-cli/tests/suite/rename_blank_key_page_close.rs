//! The page client's rename at close of a StandardDialog in lookup mode (pages/save-deleted-row/Y17SaveRowGone.al:551, a
//! database source) from a blank key while another table relates to the key: the cells of the assumption
//! rename-blank-key-with-relation are Renames that AL code calls (al-runner issue 7937), so this one is refused.

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
table 69980 "PC Master" { fields { field(1; Code; Code[20]) { } field(2; Name; Text[20]) { } } keys { key(PK; Code) { } } }
table 69981 "PC Child"
{
    fields { field(1; Id; Integer) { } field(2; Ref; Code[20]) { TableRelation = "PC Master"; } }
    keys { key(PK; Id) { } }
}
page 69982 "PC Dialog"
{
    PageType = StandardDialog;
    SourceTable = "PC Master";
    layout { area(Content) { field(Name; Rec.Name) { trigger OnValidate() begin Rec.Code := 'X'; end; } } }
}
codeunit 69983 T
{
    Subtype = Test;
    [Test]
    [HandlerFunctions('SetName')]
    procedure CloseRenamesFromBlank()
    var
        M: Record "PC Master";
        C: Record "PC Child";
        P: Page "PC Dialog";
    begin
        M.Insert();
        C.Id := 1;
        C.Insert();
        P.SetRecord(M);
        P.LookupMode(true);
        asserterror P.RunModal();
    end;
    [ModalPageHandler]
    procedure SetName(var TP: TestPage "PC Dialog")
    begin
        TP.Name.SetValue('n');
        TP.OK().Invoke();
    end;
}
"#;

#[test]
fn the_page_clients_rename_from_a_blank_key_is_refused() {
    let out = outcomes_in(SRC, "T");
    assert_eq!(out.len(), 1);
    match &out[0].1 {
        Outcome::NotRun(why) => assert!(why.contains("blank or zero key value"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
