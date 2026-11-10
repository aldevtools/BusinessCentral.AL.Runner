//! `TestField.Lookup()` over a TableRelation onto a list page with OnFindRecord: which rows the open asks for (`Which`) is not
//! pinned. `=>` is pinned for LookupMode(true) on a page variable and for the static Page.RunModal only
//! (pages/list-order/QgGoToKeyNextCall.al:629, pages/run-modal/StaticRunModalFindRecordLookup.al); the one corpus file with a
//! relation lookup next to OnFindRecord (testpage/AnTestPageTests.al:444) logs no trigger. A page that does `Rec.Find(Which)`
//! on a filtered view finds another row with `=>` than with `=><`, so the lookup is refused before the page opens.

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
table 50130 "RF Target"
{
    LookupPageId = "RF Target List";
    fields { field(1; Code; Code[10]) { } field(2; Name; Text[30]) { } }
    keys { key(PK; Code) { Clustered = true; } }
}
table 50136 "RF Plain"
{
    LookupPageId = "RF Plain List";
    fields { field(1; Code; Code[10]) { } }
    keys { key(PK; Code) { Clustered = true; } }
}
table 50131 "RF Host"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Ref; Code[10]) { TableRelation = "RF Target"; }
        field(3; Plain; Code[10]) { TableRelation = "RF Plain"; }
    }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 50132 "RF Log"
{
    SingleInstance = true;
    var
        L: Text;
    procedure Add(T: Text) begin L += T + ';'; end;
    procedure Take() R: Text begin R := L; L := ''; end;
}
page 50133 "RF Target List"
{
    PageType = List; SourceTable = "RF Target"; Editable = true;
    layout { area(Content) { repeater(R) { field(Code; Rec.Code) { } field(Name; Rec.Name) { } } } }
    trigger OnFindRecord(Which: Text): Boolean
    var
        Log: Codeunit "RF Log";
        Found: Boolean;
    begin
        Found := Rec.Find(Which);
        Log.Add('F:' + Which + '=' + Format(Found));
        exit(Found);
    end;
}
page 50137 "RF Plain List"
{
    PageType = List; SourceTable = "RF Plain";
    layout { area(Content) { repeater(R) { field(Code; Rec.Code) { } } } }
}
page 50134 "RF Host Card"
{
    PageType = Card; SourceTable = "RF Host";
    layout { area(Content) { field(Id; Rec.Id) { } field(Ref; Rec.Ref) { } field(Plain; Rec.Plain) { } } }
}
codeunit 50135 "RF Tests"
{
    Subtype = Test;
    var
        Seen: Text;

    local procedure Seed()
    var
        T: Record "RF Target";
        P: Record "RF Plain";
        H: Record "RF Host";
    begin
        T.DeleteAll(); P.DeleteAll(); H.DeleteAll();
        T.Code := 'A1'; T.Insert(); T.Code := 'A2'; T.Insert();
        P.Code := 'P1'; P.Insert(); P.Code := 'P2'; P.Insert();
        H.Id := 1; H.Insert();
    end;

    [Test]
    [HandlerFunctions('PlainHandler')]
    procedure RelationLookup_NoFindRecord()
    var
        Pg: TestPage "RF Host Card";
    begin
        Seed();
        Pg.OpenEdit();
        Pg.GoToKey(1);
        Pg.Plain.Lookup();
        Error('OBS seen=%1 value=%2', Seen, Pg.Plain.Value());
    end;

    [Test]
    [HandlerFunctions('LookupHandler')]
    procedure RelationLookup_FindWhich()
    var
        Pg: TestPage "RF Host Card";
        Log: Codeunit "RF Log";
    begin
        Seed();
        Pg.OpenEdit();
        Pg.GoToKey(1);
        Log.Take();
        Pg.Ref.Lookup();
        Error('OBS log=%1 seen=%2 value=%3', Log.Take(), Seen, Pg.Ref.Value());
    end;

    [ModalPageHandler]
    procedure LookupHandler(var L: TestPage "RF Target List")
    begin
        Seen := 'rows=';
        if L.First() then
            repeat
                Seen += L.Code.Value() + ',';
            until not L.Next();
        L.First();
        L.OK().Invoke();
    end;

    [ModalPageHandler]
    procedure PlainHandler(var L: TestPage "RF Plain List")
    begin
        Seen := 'rows=';
        if L.First() then
            repeat
                Seen += L.Code.Value() + ',';
            until not L.Next();
        L.First();
        L.OK().Invoke();
    end;
}
"#;

#[test]
fn a_relation_lookup_onto_a_list_with_onfindrecord_is_refused() {
    let outcomes = outcomes_in(SRC, "RF Tests");
    let of = |name: &str| {
        outcomes
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, o)| o.clone())
            .unwrap_or_else(|| panic!("no test {name}"))
    };
    match of("RelationLookup_FindWhich") {
        Outcome::NotRun(why) => assert!(
            why.contains("TableRelation") && why.contains("OnFindRecord") && why.contains("not pinned"),
            "{why}"
        ),
        other => panic!("expected a refusal, got {other:?}"),
    }
    // A lookup list without OnFindRecord opens as before and has no new-row line: Next from its last row answers false
    // (testpage/P133RelationLookups.al:271-284).
    assert_eq!(
        of("RelationLookup_NoFindRecord"),
        Outcome::Fail("OBS seen=rows=P1,P2, value=P1".into())
    );
}
