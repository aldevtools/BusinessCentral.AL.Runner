//! `Code = field(VarCode)` in a where() names the field being validated, and `Blocked = filter(<> Output)` filters an enum field
//! (Prod. Order Line, Assembly Header
//! and Assembly Line "Variant Code": `Item Variant.Code where("Item No." = field("Item No."), Code = field("Variant Code"),
//! "Production Blocked" = filter(<> Output))`; Tests-SCM SCM Order Tracking OrderTrackingOnSalesWithVariant).

use al_testkit::text;

const SRC: &str = r#"
enum 69980 "RWE Blocked" { value(0; " ") { } value(1; Output) { } value(2; Consumption) { } }
table 69981 "RWE Variant" { fields { field(1; ItemNo; Code[20]) { } field(2; Code; Code[10]) { } field(3; Blocked; Enum "RWE Blocked") { } } keys { key(PK; ItemNo, Code) { } } }
table 69982 "RWE Line" { fields { field(1; Id; Integer) { } field(2; ItemNo; Code[20]) { }
  field(3; VarCode; Code[10]) { TableRelation = "RWE Variant".Code where(ItemNo = field(ItemNo), Code = field(VarCode), Blocked = filter(<> Output)); } } keys { key(PK; Id) { } } }
codeunit 69983 C
{
    procedure F(): Text
    var V: Record "RWE Variant"; L: Record "RWE Line";
    begin
        V.ItemNo := 'I1'; V.Code := 'V1'; V.Insert();
        L.Id := 1; L.ItemNo := 'I1';
        L.Validate(VarCode, 'V1');
        exit(L.VarCode);
    end;
    procedure G(): Text
    var V: Record "RWE Variant"; L: Record "RWE Line";
    begin
        V.ItemNo := 'I1'; V.Code := 'V2'; V.Blocked := V.Blocked::Output; V.Insert();
        L.Id := 1; L.ItemNo := 'I1';
        asserterror L.Validate(VarCode, 'V2');
        exit(GetLastErrorCode());
    end;
}
"#;

#[test]
fn a_relation_naming_the_validated_field_and_an_enum_filter_finds_the_row() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "F")), "V1");
}

// Conditions on key fields and one on another field: DB:NothingInsideFilter, as with a key and a non-key condition
// (record/table-relation/RelationWhereMulti.al:214-236, :241-262).
#[test]
fn a_row_the_enum_filter_excludes_raises_nothing_inside_filter() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "G")), "DB:NothingInsideFilter");
}
