//! `TestField.Lookup()` over a TableRelation with several arms: the arm whose condition holds for the page's row opens its
//! table's lookup page; no arm, no page (testpage/P133RelationLookups.al:262-356; al-runner issue 7454).

const SRC: &str = r#"
table 69975 "RA A" { LookupPageId = "RA A List"; fields { field(1; "Code"; Code[10]) { } } keys { key(PK; "Code") { } } }
table 69976 "RA B" { LookupPageId = "RA B List"; fields { field(1; "Code"; Code[10]) { } } keys { key(PK; "Code") { } } }
table 69977 "RA Host"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Kind; Integer) { }
        field(3; Ref; Code[10]) { TableRelation = if (Kind = const(1)) "RA A" else if (Kind = const(2)) "RA B"; }
        field(5; RefP; Code[10]) { TableRelation = "RA A"; }
        field(4; RefW; Code[10]) { TableRelation = if (Kind = const(1)) "RA A" where("Code" = const('A2')) else if (Kind = const(2)) "RA B"; }
    }
    keys { key(PK; Id) { } }
}
page 69978 "RA A List" { PageType = List; SourceTable = "RA A"; layout { area(Content) { repeater(R) { field("Code"; Rec.Code) { } } } } }
page 69979 "RA B List" { PageType = List; SourceTable = "RA B"; layout { area(Content) { repeater(R) { field("Code"; Rec.Code) { } } } } }
page 69980 "RA Card"
{
    PageType = Card; SourceTable = "RA Host";
    layout { area(Content) { field(Id; Rec.Id) { } field(Kind; Rec.Kind) { } field(Ref; Rec.Ref) { } field(RefW; Rec.RefW) { } field(RefP; Rec.RefP) { } } }
}
codeunit 69981 "RA Tests"
{
    Subtype = Test;
    var Seen: Text;

    local procedure Seed()
    var A: Record "RA A"; B: Record "RA B"; H: Record "RA Host";
    begin
        A.DeleteAll(); B.DeleteAll(); H.DeleteAll();
        A.Code := 'A1'; A.Insert(); A.Code := 'A2'; A.Insert();
        B.Code := 'B1'; B.Insert();
        H.Id := 1; H.Kind := 1; H.Insert();
        H.Id := 2; H.Kind := 2; H.Insert();
        H.Id := 3; H.Kind := 0; H.Insert();
        Seen := '';
    end;

    [ModalPageHandler]
    procedure LookupA(var P: TestPage "RA A List")
    begin
        P.First(); Seen += 'A:' + P.Code.Value + ';'; P.OK().Invoke();
    end;

    [ModalPageHandler]
    procedure LookupB(var P: TestPage "RA B List")
    begin
        P.First(); Seen += 'B:' + P.Code.Value + ';'; P.OK().Invoke();
    end;

    [ModalPageHandler]
    procedure WalkA(var P: TestPage "RA A List")
    begin
        if P.First() then
            repeat
                Seen += P.Code.Value + ',';
            until not P.Next();
        if P.First() then;
        P.OK().Invoke();
    end;

    [Test]
    [HandlerFunctions('WalkA')]
    procedure WalkPlain()
    var Pg: TestPage "RA Card";
    begin
        Seed();
        Pg.OpenEdit();
        Pg.GoToKey(1);
        Pg.RefP.Lookup();
        Expect(Pg.RefP.Value + '|' + Seen, 'A1|A1,A2,');
        Pg.Close();
    end;

    [Test]
    [HandlerFunctions('WalkA')]
    procedure WalkArm()
    var Pg: TestPage "RA Card";
    begin
        Seed();
        Pg.OpenEdit();
        Pg.GoToKey(1);
        Pg.Ref.Lookup();
        Expect(Pg.Ref.Value + '|' + Seen, 'A1|A1,A2,');
        Pg.Close();
    end;

    local procedure Expect(Actual: Text; Wanted: Text)
    begin
        if Actual <> Wanted then Error('expected [%1] but was [%2]', Wanted, Actual);
    end;

    [Test]
    [HandlerFunctions('LookupA,LookupB')]
    procedure ArmsByTheRow()
    var Pg: TestPage "RA Card";
    begin
        Seed();
        Pg.OpenEdit();
        Pg.GoToKey(1);
        Pg.Ref.Lookup();
        Expect(Pg.Ref.Value + '|' + Seen, 'A1|A:A1;');
        Pg.Kind.SetValue(2);
        Pg.Ref.Lookup();
        Expect(Pg.Ref.Value + '|' + Seen, 'B1|A:A1;B:B1;');
        Pg.GoToKey(2);
        Pg.Ref.Lookup();
        Expect(Pg.Ref.Value + '|' + Seen, 'B1|A:A1;B:B1;B:B1;');
        Pg.Close();
    end;

    [Test]
    procedure NoArmNoPage()
    var Pg: TestPage "RA Card";
    begin
        Seed();
        Pg.OpenEdit();
        Pg.GoToKey(3);
        Pg.Ref.Lookup();
        Expect(Pg.Ref.Value + '|' + Seen, '|');
        Pg.Close();
    end;

    [Test]
    [HandlerFunctions('LookupA')]
    procedure ArmWithWhere()
    var Pg: TestPage "RA Card";
    begin
        Seed();
        Pg.OpenEdit();
        Pg.GoToKey(1);
        Pg.RefW.Lookup();
    end;
}
"#;

#[test]
fn the_arm_whose_condition_holds_opens_its_lookup_page() {
    use al_testrun::Outcome;
    let outcomes = al_testkit::outcomes_in(SRC, "RA Tests");
    let of = |name: &str| {
        outcomes
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, o)| o.clone())
            .expect(name)
    };
    assert_eq!(of("ArmsByTheRow"), Outcome::Pass);
    assert_eq!(of("NoArmNoPage"), Outcome::Pass);
    // The handler walks the rows of the lookup page: the same rows for a plain relation and for an arm (P133RelationLookups.al:271).
    assert_eq!(of("WalkPlain"), Outcome::Pass);
    assert_eq!(of("WalkArm"), Outcome::Pass);
    // An arm with a where() stays refused: the filter of the lookup page is not pinned.
    assert!(
        matches!(of("ArmWithWhere"), Outcome::NotRun(_)),
        "{:?}",
        of("ArmWithWhere")
    );
}
