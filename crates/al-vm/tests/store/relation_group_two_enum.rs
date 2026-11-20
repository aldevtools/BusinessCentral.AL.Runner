//! A source FlowFilter set in filter group 2 constrains a TableRelation check only in the shapes
//! record/filters/TestRecordEnumFlowFilterGroup.al pins (:68-108): `Record.Validate` on a record never read or inserted,
//! and a TestPage SetValue on the line `New()` started on a Card a trapped static `Page.Run(Id, Rec)` opened. The enum's
//! ordinals equal its positions there. Every other shape is refused.

use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
enum 69930 "G2E Kind" { value(0; Other) { } value(1; Selected) { } }
enum 69931 "G2E Gap Kind" { value(3; Other) { } value(7; Selected) { } }
table 69930 "G2E Target"
{
    fields { field(1; "Code"; Code[20]) { } field(2; Kind; Enum "G2E Kind") { } field(3; GKind; Enum "G2E Gap Kind") { } }
    keys { key(PK; "Code") { } }
}
table 69931 "G2E Source"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; "Kind Filter"; Enum "G2E Kind") { FieldClass = FlowFilter; }
        field(3; Target; Code[20]) { TableRelation = "G2E Target".Code where(Kind = field("Kind Filter")); }
        field(4; "G Filter"; Enum "G2E Gap Kind") { FieldClass = FlowFilter; }
        field(5; GTarget; Code[20]) { TableRelation = "G2E Target".Code where(GKind = field("G Filter")); }
    }
    keys { key(PK; Id) { } }
}
page 69930 "G2E Card"
{
    PageType = Card;
    SourceTable = "G2E Source";
    layout { area(Content) { group(General) { field(Id; Rec.Id) { } field(Target; Rec.Target) { } } } }
}
codeunit 69932 C
{
    local procedure Init()
    var
        T: Record "G2E Target";
    begin
        T.Code := 'MATCH'; T.Kind := T.Kind::Selected; T.GKind := T.GKind::Selected; T.Insert();
        T.Code := 'OTHER'; T.Kind := T.Kind::Other; T.GKind := T.GKind::Other; T.Insert();
    end;

    local procedure G2(var S: Record "G2E Source")
    begin
        S.FilterGroup(2);
        S.SetRange("Kind Filter", S."Kind Filter"::Selected);
        S.FilterGroup(0);
    end;

    procedure RecordValidate(): Text
    var
        S: Record "G2E Source";
    begin
        Init();
        G2(S);
        S.Validate(Target, 'MATCH');
        exit(S.Target);
    end;

    procedure NewLineOfTrappedCard(): Text
    var
        S: Record "G2E Source";
        Card: TestPage "G2E Card";
    begin
        Init();
        G2(S);
        Card.Trap();
        Page.Run(Page::"G2E Card", S);
        Card.New();
        Card.Id.SetValue(1);
        Card.Target.SetValue('MATCH');
        exit(Card.Target.Value());
    end;

    procedure TemporarySource(): Text
    var
        S: Record "G2E Source" temporary;
    begin
        Init();
        G2(S);
        S.Validate(Target, 'MATCH');
        exit(S.Target);
    end;

    procedure FieldRefValidate(): Text
    var
        S: Record "G2E Source";
        RR: RecordRef;
        FR: FieldRef;
    begin
        Init();
        G2(S);
        RR.GetTable(S);
        FR := RR.Field(3);
        FR.Validate('MATCH');
        exit(Format(FR.Value));
    end;

    procedure ExistingRowOfTrappedCard(): Text
    var
        S: Record "G2E Source";
        Card: TestPage "G2E Card";
    begin
        Init();
        S.Id := 5;
        S.Insert();
        G2(S);
        Card.Trap();
        Page.Run(Page::"G2E Card", S);
        Card.Target.SetValue('MATCH');
        exit(Card.Target.Value());
    end;

    procedure EnumWithGaps(): Text
    var
        S: Record "G2E Source";
    begin
        Init();
        S.FilterGroup(2);
        S.SetRange("G Filter", S."G Filter"::Selected);
        S.FilterGroup(0);
        S.Validate(GTarget, 'MATCH');
        exit(S.GTarget);
    end;

    procedure SourceReadByGet(): Text
    var
        S: Record "G2E Source";
    begin
        Init();
        S.Id := 5;
        S.Insert();
        S.Get(5);
        G2(S);
        S.Validate(Target, 'MATCH');
        exit(S.Target);
    end;
}
"#;

fn refused(routine: &str) {
    match al_testkit::run_in(SRC, "C", routine) {
        Err(Signal::Unsupported(why)) => assert!(why.contains("not pinned"), "{routine}: {why}"),
        other => panic!("{routine}: expected a refusal, got {other:?}"),
    }
}

#[test]
fn the_pinned_shapes_check_the_group_two_value() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "RecordValidate")), "MATCH");
    assert_eq!(text(al_testkit::run_in(SRC, "C", "NewLineOfTrappedCard")), "MATCH");
}

#[test]
fn a_temporary_source_is_refused() {
    refused("TemporarySource");
}

#[test]
fn fieldref_validate_is_refused() {
    refused("FieldRefValidate");
}

#[test]
fn setvalue_on_an_existing_row_is_refused() {
    refused("ExistingRowOfTrappedCard");
}

#[test]
fn an_enum_whose_ordinals_are_not_its_positions_is_refused() {
    refused("EnumWithGaps");
}

#[test]
fn a_source_read_by_get_is_refused() {
    refused("SourceReadByGet");
}

#[test]
fn a_group_zero_filter_keeps_its_check() {
    // Out of scope of the group 2 rule: a group 0 equality on the FlowFilter (RelationWhereFlowFilter.al:426-441).
    let src = SRC.replace("S.FilterGroup(2);", "S.FilterGroup(0);");
    assert_eq!(text(al_testkit::run_in(&src, "C", "TemporarySource")), "MATCH");
}
