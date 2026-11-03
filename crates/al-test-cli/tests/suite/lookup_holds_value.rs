//! TestField.Lookup over a TableRelation when the field already holds a value: the lookup page stands on the related row that holds
//! it (pages/lt-y80-lookup-holds-value/Y80LookupHoldsValue.al, same on BC 27, 28 and 29). The corpus file is the full pin; these
//! tests keep the shapes in the unit run.

use al_test_cli::report::Outcome;

const SRC: &str = r#"
table 69980 "LH Group"
{
    LookupPageId = "LH Group List";
    fields { field(1; "Code"; Code[10]) { } field(2; Descr; Text[30]) { } }
    keys { key(PK; "Code") { Clustered = true; } }
}
table 69981 "LH Item"
{
    fields
    {
        field(1; "No."; Code[10]) { }
        field(2; Grp; Code[10]) { TableRelation = "LH Group".Code; }
        field(3; Alt; Text[30]) { TableRelation = "LH Group".Descr; }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}
page 69982 "LH Group List"
{
    PageType = List; SourceTable = "LH Group"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(CodeCtl; Rec.Code) { } field(DescrCtl; Rec.Descr) { } } } }
}
page 69983 "LH Card"
{
    PageType = Card; SourceTable = "LH Item"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(NoCtl; Rec."No.") { } field(GrpCtl; Rec.Grp) { } field(AltCtl; Rec.Alt) { } } }
}
codeunit 69984 "LH Tests"
{
    Subtype = Test;
    var First: Text; Mode: Text; Rows: Integer;

    local procedure Seed(Held: Code[10]; HeldAlt: Text)
    var G: Record "LH Group"; I: Record "LH Item"; N: Integer;
    begin
        G.DeleteAll(); I.DeleteAll();
        for N := 1 to Rows do begin G.Code := 'G' + Format(N); G.Descr := 'Desc ' + Format(N); G.Insert(); end;
        I."No." := 'I1'; I.Grp := Held; I.Alt := CopyStr(HeldAlt, 1, 30); I.Insert();
    end;

    [ModalPageHandler]
    procedure H(var P: TestPage "LH Group List")
    begin
        First := P.CodeCtl.Value();
        if Mode = 'next' then P.Next();
        P.OK().Invoke();
    end;

    local procedure Run(Held: Code[10]; Alt: Text; OnAlt: Boolean; M: Text): Text
    begin
        exit(RunRows(4, Held, Alt, OnAlt, M));
    end;

    local procedure RunRows(Count: Integer; Held: Code[10]; Alt: Text; OnAlt: Boolean; M: Text): Text
    var P: TestPage "LH Card";
    begin
        Rows := Count;
        Seed(Held, Alt); Mode := M;
        P.OpenEdit(); P.GoToKey('I1');
        if OnAlt then P.AltCtl.Lookup() else P.GrpCtl.Lookup();
        exit(First + '|' + P.GrpCtl.Value() + '|' + P.AltCtl.Value());
    end;

    [Test] [HandlerFunctions('H')] procedure Middle() begin if Run('G2', '', false, 'ok') <> 'G2|G2|' then Error('middle'); end;
    [Test] [HandlerFunctions('H')] procedure Last() begin if Run('G4', '', false, 'ok') <> 'G4|G4|' then Error('last'); end;
    [Test] [HandlerFunctions('H')] procedure MiddleNext() begin if Run('G2', '', false, 'next') <> 'G2|G3|' then Error('next'); end;
    [Test] [HandlerFunctions('H')] procedure NoRow() begin if Run('ZZ', '', false, 'ok') <> 'G1|G1|' then Error('norow'); end;
    [Test] [HandlerFunctions('H')] procedure LowerCase() begin if Run('g3', '', false, 'ok') <> 'G3|G3|' then Error('lower'); end;
    // Y87BigLookup.al (Y87_BL_30_Middle, _30_Last, _30_NoRow, _30_Middle_Next): a related table of more than four rows
    [Test] [HandlerFunctions('H')] procedure Big30Middle() begin if RunRows(30, 'G16', '', false, 'ok') <> 'G16|G16|' then Error('big middle'); end;
    [Test] [HandlerFunctions('H')] procedure Big30Last() begin if RunRows(30, 'G30', '', false, 'ok') <> 'G30|G30|' then Error('big last'); end;
    [Test] [HandlerFunctions('H')] procedure Big30NoRow() begin if RunRows(30, 'ZZ', '', false, 'ok') <> 'G1|G1|' then Error('big norow'); end;
    [Test] [HandlerFunctions('H')] procedure Big30MiddleNext() begin if RunRows(30, 'G16', '', false, 'next') <> 'G16|G17|' then Error('big next'); end;
    [Test] [HandlerFunctions('H')] procedure Alt() begin if Run('G1', 'Desc 3', true, 'ok') <> 'G3|G1|Desc 3' then Error('alt'); end;
}
"#;

#[test]
fn a_lookup_over_a_held_value_stands_on_the_related_row() {
    for (name, outcome) in al_testkit::outcomes_in(SRC, "LH Tests") {
        assert_eq!(outcome, Outcome::Pass, "{name}: {outcome:?}");
    }
}
