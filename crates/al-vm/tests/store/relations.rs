//! TableRelation: FieldRef.Relation and Validate against the related table.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::text;
use al_vm::{Engine, Session};

const TABLES: &str = r#"
table 50950 "Rel Parent" {
    fields { field(1; Code; Code[20]) { } field(5; Grp; Code[20]) { } }
    keys { key(PK; Code) { } }
}
table 50951 "Rel Parent B" {
    fields { field(1; Code; Code[20]) { } }
    keys { key(PK; Code) { } }
}
table 50952 "Rel Child" {
    fields {
        field(1; Id; Integer) { }
        field(2; ChildGrp; Code[20]) { }
        field(3; Kind; Option) { OptionMembers = A,B; }
        field(4; Plain; Code[20]) { TableRelation = "Rel Parent".Code; }
        field(5; WhereRef; Code[20]) { TableRelation = "Rel Parent".Code where(Grp = field(ChildGrp)); }
        field(6; Cond; Code[20]) { TableRelation = if (Kind = const(A)) "Rel Parent".Code else "Rel Parent B".Code; }
        field(7; NoRel; Code[20]) { }
        field(8; NoCheck; Code[20]) { TableRelation = "Rel Parent".Code; ValidateTableRelation = false; }
        field(9; KindFilter; Option) { FieldClass = FlowFilter; OptionMembers = A,B; }
        field(10; CondOnFilter; Code[20]) {
            FieldClass = FlowFilter;
            TableRelation = if (KindFilter = const(A)) "Rel Parent".Code else if (KindFilter = const(B)) "Rel Parent B".Code;
        }
        field(11; OnlyA; Code[20]) { TableRelation = if (Kind = const(A)) "Rel Parent".Code; }
        field(12; Unknown; Code[20]) { TableRelation = "Not In Workspace".Code; }
    }
    keys { key(PK; Id) { } }
}
table 50955 "Rel Flow Parent" {
    fields { field(1; Id; Integer) { } field(2; Grp; Code[20]) { } }
    keys { key(PK; Id) { } }
}
table 50956 "Rel Flow Child" {
    fields {
        field(1; Id; Integer) { }
        field(2; ParentId; Integer) { }
        field(3; FlowGrp; Code[20]) { FieldClass = FlowField; CalcFormula = lookup("Rel Flow Parent".Grp where(Id = field(ParentId))); }
        field(4; ByFlow; Code[20]) { TableRelation = "Rel Parent".Code where(Grp = field(FlowGrp)); }
        field(5; CondByFlow; Code[20]) { TableRelation = if (FlowGrp = const('G1')) "Rel Parent".Code else "Rel Parent B".Code; }
    }
    keys { key(PK; Id) { } }
}
table 50953 "Rel Setup" {
    fields {
        field(1; Code; Code[20]) { }
        field(3; Direction; Option) { OptionMembers = Export,Import,"Export-Positive Pay"; }
    }
    keys { key(PK; Code) { } }
}
table 50954 "Rel Bank" {
    fields {
        field(1; Id; Integer) { }
        field(2; PosPay; Code[20]) { TableRelation = "Rel Setup".Code where(Direction = const("Export-Positive Pay")); }
    }
    keys { key(PK; Id) { } }
}
"#;

fn run(body: &str) -> Result<Value, Signal> {
    let src = format!("{TABLES}\ncodeunit 50100 C {{ {body} }}");
    let l = al_testkit::compile(&src);
    let blocked: Vec<Vec<Option<String>>> = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    if let Some(Some(r)) = blocked.get(oi).and_then(|o| o.first()) {
        panic!("F is blocked: {r}");
    }
    let engine = Engine::new(l.program, blocked);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    s.call(oi, 0, inst, vec![]).map(|(v, _)| v)
}

#[test]
fn relation_answers_the_table_of_the_arm_that_applies() {
    // corpus fieldref/TestFieldRefRelation.al:36-130, TestFieldRefRelationFieldClass.al:60-145
    let r = run(
        "procedure F(): Text var RR: RecordRef; s: Text; begin
            RR.Open(50952);
            s := Format(RR.Field(4).Relation()) + '/' + Format(RR.Field(5).Relation()) + '/' + Format(RR.Field(7).Relation());
            RR.Field(3).Value := 0;
            s += '|' + Format(RR.Field(6).Relation());
            RR.Field(3).Value := 1;
            s += '/' + Format(RR.Field(6).Relation()) + '/' + Format(RR.Field(11).Relation());
            RR.Field(9).Value := 1;
            s += '|' + Format(RR.Field(10).Relation());
            exit(s);
        end;",
    );
    assert_eq!(text(r), "50950/50950/0|50950/50951/0|50950");
}

#[test]
fn relation_of_a_target_outside_the_workspace_is_refused() {
    let r = run("procedure F(): Integer var RR: RecordRef; begin RR.Open(50952); exit(RR.Field(12).Relation()); end;");
    assert!(
        matches!(r, Err(Signal::Unsupported(ref m)) if m.contains("Not In Workspace")),
        "{r:?}"
    );
}

#[test]
fn system_created_by_relates_to_the_user_table() {
    // corpus record/TestFieldDataClassificationVirtualTable.al:92-100
    let r = run(
        "procedure F(): Integer var RR: RecordRef; begin RR.Open(50952); exit(RR.Field(2000000002).Relation()); end;",
    );
    assert!(matches!(r, Ok(Value::Integer(2_000_000_120))), "{r:?}");
}

#[test]
fn validate_refuses_a_value_the_related_table_lacks() {
    // corpus fieldref/TestFieldRefRelation.al:182-262 ('cannot be found in the related table')
    let m = text(run("procedure F(): Text var C: Record \"Rel Child\"; begin
            asserterror C.Validate(Plain, 'NOPE');
            exit(GetLastErrorText() + '[' + C.Plain + ']');
        end;"));
    assert!(m.contains("cannot be found in the related table"), "{m}");
    assert!(
        m.contains("Plain") && m.contains("NOPE") && m.contains("Rel Parent"),
        "{m}"
    );
    assert!(m.ends_with("[]"), "a refused value is not stored: {m}");
}

#[test]
fn validate_accepts_known_blank_unchecked_and_directly_assigned_values() {
    // corpus fieldref/TestFieldRefRelationFieldClass.al:130-170, tableextension/TestTableExtFieldTableRelation.al:94-160
    let r = run(
        "procedure F(): Text var C: Record \"Rel Child\"; P: Record \"Rel Parent\"; s: Text; begin
            P.Code := 'P1'; P.Insert();
            C.Validate(Plain, 'P1'); s := C.Plain;
            C.Validate(Plain, ''); s += '[' + C.Plain + ']';
            C.Validate(NoCheck, 'ANYTHING'); s += C.NoCheck;
            C.Plain := 'DIRECT'; s += C.Plain;
            exit(s);
        end;",
    );
    assert_eq!(text(r), "P1[]ANYTHINGDIRECT");
}

#[test]
fn validate_where_field_narrows_the_related_rows_to_the_referencing_rows_group() {
    // corpus fieldref/TestFieldRefRelation.al:215-262
    let r = run(
        "procedure F(): Text var C: Record \"Rel Child\"; P: Record \"Rel Parent\"; s: Text; begin
            P.Code := 'P-G1'; P.Grp := 'G1'; P.Insert();
            P.Code := 'P-G2'; P.Grp := 'G2'; P.Insert();
            C.ChildGrp := 'G2';
            C.Validate(WhereRef, 'P-G2'); s := C.WhereRef;
            asserterror C.Validate(WhereRef, 'P-G1');
            exit(s + '|' + GetLastErrorText());
        end;",
    );
    let r = text(r);
    assert!(r.starts_with("P-G2|"), "{r}");
    assert!(r.contains("cannot be found in the related table"), "{r}");
}

#[test]
fn validate_follows_the_arm_the_condition_selects() {
    let r = run(
        "procedure F(): Text var C: Record \"Rel Child\"; PA: Record \"Rel Parent\"; PB: Record \"Rel Parent B\"; s: Text; begin
            PA.Code := 'ONLYA'; PA.Insert();
            PB.Code := 'ONLYB'; PB.Insert();
            Commit();
            C.Kind := C.Kind::A;
            C.Validate(Cond, 'ONLYA'); s := C.Cond;
            asserterror C.Validate(Cond, 'ONLYB');
            s += '|' + Format(StrPos(GetLastErrorText(), 'cannot be found') > 0);
            C.Kind := C.Kind::B;
            C.Validate(Cond, 'ONLYB'); s += C.Cond;
            exit(s);
        end;",
    );
    assert_eq!(text(r), "ONLYA|YesONLYB");
}

#[test]
fn record_relation_answers_the_arm_and_refuses_a_field_without_relation() {
    // corpus record/TestRecordExtended.al:15-40, fieldref/TestFieldRefRelationFieldClass.al:168-190
    let r = run("procedure F(): Text var C: Record \"Rel Child\"; s: Text; begin
            s := Format(C.Relation(Plain));
            C.Kind := C.Kind::B;
            s += '/' + Format(C.Relation(Cond));
            exit(s);
        end;");
    assert_eq!(text(r), "50950/50951");
    let r = run("procedure F(): Integer var C: Record \"Rel Child\"; begin exit(C.Relation(NoRel)); end;");
    assert!(
        matches!(r, Err(Signal::Unsupported(ref m)) if m.contains("Relation")),
        "{r:?}"
    );
}

#[test]
fn a_relation_onto_system_row_version_relates_to_the_primary_key() {
    // corpus record/TestCalcFormulaSystemFieldsTests.Codeunit.al:229-262: even an existing row's
    // own rowversion is refused, because the target is the primary key
    let src = r#"
table 50960 "RV Line" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
table 50961 "RV Head" { fields { field(1; Id; Integer) { } field(2; Ref; BigInteger) { TableRelation = "RV Line".SystemRowVersion; } } keys { key(PK; Id) { } } }
codeunit 50100 C { procedure F(): Text var L: Record "RV Line"; H: Record "RV Head"; begin
    L.Id := 100; L.Insert(); L.Get(100);
    asserterror H.Validate(Ref, L.SystemRowVersion);
    exit(GetLastErrorText()); end; }
"#;
    let l = al_testkit::compile(src);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let engine = Engine::new(l.program, blocked);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    let r = s.call(oi, 0, inst, vec![]).map(|(v, _)| v);
    assert!(text(r).contains("cannot be found in the related table"));
}

#[test]
fn a_where_const_naming_an_option_member_with_a_hyphen_is_enforced() {
    // The shape of Bank Account."Positive Pay Export Code" (Base Application table 270). Tests-ERM
    // Pos. Pay Test Unit.TestPositivePayExportCodeInvalid expects the error; the member name is
    // "Export-Positive Pay".
    let r = run(
        "procedure F(): Text var S: Record \"Rel Setup\"; B: Record \"Rel Bank\"; Wrong: Text; Right: Text; begin
            S.Code := 'X'; S.Direction := S.Direction::Export; S.Insert();
            S.Code := 'P'; S.Direction := S.Direction::\"Export-Positive Pay\"; S.Insert(); Commit();
            asserterror B.Validate(PosPay, 'X'); Wrong := GetLastErrorText();
            B.Validate(PosPay, 'P'); Right := B.PosPay;
            exit(Wrong + '|' + Right);
        end;",
    );
    let t = text(r);
    assert!(
        t.contains("cannot be found in the related table") && t.ends_with("|P"),
        "{t}"
    );
}

/// A relation filtered by `field(F)` where F is a FlowField of the same record: the check calculates F
/// itself, so `Validate` passes without a prior CalcFields and fails when the calculated value does not
/// match (staging types/PZMisc.al:Validate_TableRelationFilteredByAFlowField, BC 29.0; a Pageworks test).
#[test]
fn a_relation_filtered_by_a_flowfield_of_the_record_calculates_it() {
    let r = run(
        "procedure F(): Text var P: Record \"Rel Parent\"; FP: Record \"Rel Flow Parent\"; C: Record \"Rel Flow Child\"; Seen: Text; begin
            P.Code := 'X'; P.Grp := 'G'; P.Insert();
            P.Code := 'Y'; P.Grp := 'H'; P.Insert();
            FP.Id := 7; FP.Grp := 'G'; FP.Insert(); Commit();
            C.Id := 1; C.ParentId := 7;
            C.Validate(ByFlow, 'X'); Seen := C.ByFlow;
            asserterror C.Validate(ByFlow, 'Y'); exit(Seen + '|' + GetLastErrorText());
        end;",
    );
    let t = text(r);
    assert!(
        t.starts_with("X|") && t.contains("cannot be found in the related table"),
        "{t}"
    );
}

// misc/lt-probes-needed-6/PnF29RelationByFlowField.al:242-260 (FlowRelation_Condition_NoCalcFields): Validate calculates a
// FlowField that an if condition of the relation names; no CalcFields is needed first.
#[test]
fn validate_calculates_a_flowfield_named_by_an_arm_condition() {
    let r = run(
        "procedure F(): Text var C: Record \"Rel Flow Child\"; FP: Record \"Rel Flow Parent\"; PA: Record \"Rel Parent\"; PB: Record \"Rel Parent B\"; s: Text; begin
            FP.Id := 1; FP.Grp := 'G1'; FP.Insert();
            FP.Id := 2; FP.Grp := 'G2'; FP.Insert();
            PA.Code := 'ONLYA'; PA.Insert();
            PB.Code := 'ONLYB'; PB.Insert();
            C.Id := 1; C.ParentId := 1; C.Insert();
            C.Id := 2; C.ParentId := 2; C.Insert();
            Commit();
            C.Get(1);
            C.Validate(CondByFlow, 'ONLYA'); s := C.CondByFlow;
            C.Get(2);
            C.Validate(CondByFlow, 'ONLYB'); s += '|' + C.CondByFlow;
            exit(s);
        end;",
    );
    assert_eq!(text(r), "ONLYA|ONLYB");
}

// record/lt-sc-enum-const-relation/ScEnumConstRelation.al:52-53, :101-140: a condition `const("Enum"::Value)` on an Enum field
// of the field's own enum selects its arm as `const(Value)` does. An enum other than the field's is not pinned.
#[test]
fn an_enum_qualified_const_condition_selects_its_arm() {
    let src = r#"
enum 91240 "LT SC Std" { Extensible = true; value(0; A) { } value(1; B) { } value(2; C) { } }
enum 91245 "LT SC Other" { value(0; A) { } }
table 91241 "LT SC Rel A" { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { } } }
table 91242 "LT SC Rel B" { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { } } }
table 91243 "LT SC Rel Host"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Standard; Enum "LT SC Std") { }
        field(3; Kind; Enum "LT SC Std") { }
        field(4; Bic; Text[50]) { TableRelation = if (Standard = const("LT SC Std"::A)) "LT SC Rel A".Code else if (Kind = const("LT SC Std"::B)) "LT SC Rel B".Code; }
        field(5; Other; Text[50]) { TableRelation = if (Standard = const("LT SC Other"::A)) "LT SC Rel A".Code; }
    }
    keys { key(PK; Id) { } }
}
codeunit 50100 C
{
    [TryFunction]
    local procedure V(S: Enum "LT SC Std"; K: Enum "LT SC Std"; Value: Text)
    var H: Record "LT SC Rel Host";
    begin H.Standard := S; H.Kind := K; H.Validate(Bic, Value); end;
    local procedure T(S: Enum "LT SC Std"; K: Enum "LT SC Std"; Value: Text): Text
    begin if V(S, K, Value) then exit('ok'); exit('no'); end;
    procedure F(): Text
    var A: Record "LT SC Rel A"; B: Record "LT SC Rel B";
    begin
        A.Code := 'AA'; A.Insert(); B.Code := 'BB'; B.Insert();
        exit(T("LT SC Std"::A, "LT SC Std"::B, 'AA') + T("LT SC Std"::A, "LT SC Std"::B, 'BB') + T("LT SC Std"::C, "LT SC Std"::B, 'BB')
            + T("LT SC Std"::C, "LT SC Std"::B, 'AA') + T("LT SC Std"::B, "LT SC Std"::A, 'ZZ'));
    end;
    procedure G()
    var H: Record "LT SC Rel Host";
    begin H.Validate(Other, 'AA'); end;
}
"#;
    assert_eq!(text(al_testkit::run(src, "F")), "oknooknook");
    assert!(matches!(al_testkit::run(src, "G"), Err(Signal::Unsupported(_))));
}
