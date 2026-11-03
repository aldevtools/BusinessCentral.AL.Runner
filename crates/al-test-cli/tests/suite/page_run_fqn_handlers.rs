//! Page.Run and Page.RunModal with a fully qualified page name, with page handlers (corpus master pages/lt-y74-fqn-run/
//! Y74FqnRunForms.al Run_Name_Only, Run_Name_Record, Run_Name_Record_FieldValue, Run_Name_Spellings, RunModal_Name_Only).

use al_testkit::outcomes_in;

const SRC: &str = r#"
namespace My.Ns;
table 51700 "FQ Row" { fields { field(1; Id; Integer) { } field(2; Txt; Text[20]) { } } keys { key(PK; Id) { } } }
page 51701 "FQ Card" { PageType = Card; SourceTable = "FQ Row";
    layout { area(Content) { field(IdCtl; Rec.Id) { } } } }
page 51702 "FQ List" { PageType = List; SourceTable = "FQ Row";
    layout { area(Content) { repeater(R) { field(IdCtl; Rec.Id) { } } } } }
codeunit 51703 "FQ Tests"
{
    Subtype = Test;
    var Seen: Text;
    local procedure Seed() var R: Record "FQ Row"; begin R.DeleteAll(); R.Id := 1; R.Insert(); R.Id := 2; R.Insert(); end;
    local procedure Check(What: Text; Actual: Text; Expected: Text)
    begin if Actual <> Expected then Error('%1: [%2], expected [%3]', What, Actual, Expected); end;
    [PageHandler]
    procedure CardHandler(var P: TestPage "FQ Card") begin Seen := Seen + 'card ' + P.IdCtl.Value() + ';'; end;
    [ModalPageHandler]
    procedure ListHandler(var P: TestPage "FQ List") begin Seen := Seen + 'list ' + P.IdCtl.Value() + ';'; P.OK().Invoke(); end;
    [Test] [HandlerFunctions('CardHandler')]
    procedure ByName()
    var R: Record "FQ Row";
    begin
        Seed(); Seen := '';
        Page.Run('My.Ns.FQ Card');
        Page.Run('my.ns.fq card');
        Page.Run('My.Ns."FQ Card"');
        R.Get(2);
        Page.Run('My.Ns.FQ Card', R);
        Page.Run('My.Ns.FQ Card', R, R.Txt);
        Check('seen', Seen, 'card 1;card 1;card 1;card 2;card 2;');
    end;
    [Test] [HandlerFunctions('ListHandler')]
    procedure ModalByName()
    var A: Action;
    begin
        Seed(); Seen := '';
        A := Page.RunModal('My.Ns.FQ List');
        Check('seen', Seen + Format(A), 'list 1;LookupOK');
    end;
}
"#;

#[test]
fn a_page_runs_by_its_fully_qualified_name_with_a_handler() {
    let o = outcomes_in(SRC, "FQ Tests");
    for n in ["ByName", "ModalByName"] {
        let got = format!("{:?}", o.iter().find(|(name, _)| name == n).map(|(_, x)| x));
        assert_eq!(got, "Some(Pass)", "{n}");
    }
}
