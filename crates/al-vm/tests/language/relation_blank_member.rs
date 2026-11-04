//! `TableRelation = "Service Item" where(Blocked = filter(" "))` (Service Contract Line): the filter names the enum member
//! whose name is one space. The relation text lost its quotes and was an empty filter term, which refused 265 Tests-SCM-Service
//! tests ("where() of a relation: empty filter term").

use al_testkit::{int, run_in};

const SRC: &str = r#"
enum 69850 "RB Blk" { Extensible = true; value(0; " ") { } value(1; Yes) { } }
table 69851 "RB Item" { fields { field(1; No; Code[10]) { } field(2; Blocked; Enum "RB Blk") { } } keys { key(PK; No) { } } }
table 69852 "RB Line" { fields { field(1; Id; Integer) { } field(2; ItemNo; Code[10]) { TableRelation = "RB Item" where(Blocked = filter(" ")); } } keys { key(PK; Id) { } } }
codeunit 69853 "RB C"
{
    procedure F(): Integer
    var I: Record "RB Item"; L: Record "RB Line"; Hits: Integer;
    begin
        I.No := 'OPEN'; I.Insert();
        I.No := 'BLOCKED'; I.Blocked := I.Blocked::Yes; I.Insert();
        L.Id := 1;
        L.Validate(ItemNo, 'OPEN');
        Hits += 1;
        asserterror L.Validate(ItemNo, 'BLOCKED');
        if GetLastErrorText() <> '' then Hits += 10;
        exit(Hits);
    end;
}
"#;

#[test]
fn a_relation_filter_naming_the_blank_enum_member_selects_the_unblocked_rows() {
    assert_eq!(int(run_in(SRC, "RB C", "F")), 11);
}

// An empty `const('')` or `filter('')` term of a where() selects the rows whose field is blank (corpus
// record/table-relation/RelationWhereEmptyTerm.al:Relation_ErrorTextOfFailedValidate, CalcFormula_CountWhereEmptyTerms:
// `empty_const` counts the 2 blank-zone rows).
const EMPTY: &str = r#"
table 69861 "RE Item" { fields { field(1; No; Code[10]) { } field(2; Zone; Code[10]) { } } keys { key(PK; No) { } } }
table 69862 "RE Line" { fields { field(1; Id; Integer) { } field(2; ItemNo; Code[10]) { TableRelation = "RE Item" where(Zone = const('')); } } keys { key(PK; Id) { } } }
codeunit 69863 "RE C"
{
    procedure F(): Integer
    var I: Record "RE Item"; L: Record "RE Line"; Hits: Integer;
    begin
        I.No := 'A'; I.Insert();
        I.No := 'B'; I.Zone := 'Z1'; I.Insert();
        L.Id := 1;
        L.Validate(ItemNo, 'A');
        Hits += 1;
        asserterror L.Validate(ItemNo, 'B');
        if GetLastErrorText() <> '' then Hits += 10;
        exit(Hits);
    end;
}
"#;

#[test]
fn an_empty_const_term_of_a_relation_selects_the_blank_rows() {
    assert_eq!(int(run_in(EMPTY, "RE C", "F")), 11);
}

// A where() on a field of the related table's primary key answers DB:RecordNotFound, not DB:NothingInsideFilter
// (corpus record/table-relation/PnF13RelationFilterConditions.al:122-126, 144-149, 166-169: primary key (Code, Zone),
// where(Zone = ...)). A where() on a field outside the key answers NothingInsideFilter
// (record/table-relation/RelationWhereEmptyTerm.al:173-177).
const KEYED: &str = r#"
table 69871 "RK Bin" { fields { field(1; Code; Code[10]) { } field(2; Zone; Code[10]) { } } keys { key(PK; Code, Zone) { } } }
table 69872 "RK Line" { fields { field(1; Id; Integer) { } field(2; BinCode; Code[10]) { TableRelation = "RK Bin".Code where(Zone = const('Z1')); } } keys { key(PK; Id) { } } }
codeunit 69873 "RK C"
{
    procedure F(): Text
    var B: Record "RK Bin"; L: Record "RK Line";
    begin
        B.Code := 'X'; B.Zone := 'Z2'; B.Insert();
        L.Id := 1;
        asserterror L.Validate(BinCode, 'X');
        exit(GetLastErrorCode());
    end;
}
"#;

#[test]
fn a_where_on_a_primary_key_field_answers_record_not_found() {
    assert_eq!(al_testkit::text(run_in(KEYED, "RK C", "F")), "DB:RecordNotFound");
}
