//! al-runner issue 8249: Field.RelationFieldNo of a TableRelation that names a table and no field is 0, not the target's first key
//! field (metadata/platform-tables/FieldRelationFieldNo.al). The shapes the probe did not measure refuse a read of RelationFieldNo
//! while the row's other columns read as on main.
//!
//! Needs `System.app` of the pinned build in the artifact cache (`al_testfiles`).

use super::field_extension_package_ids::run;
use al_builtins::assumptions::Assumption;
use al_sema::workspace::AppKind;
use al_test_cli::report::Outcome;

const APP_ID: &str = "8249a000-0000-4000-8000-000000008249";
const OTHER_ID: &str = "8249b000-0000-4000-8000-000000008249";

const TABLES: &str = r#"
table 50100 "Category"
{
    fields { field(1; Code; Code[20]) { } field(2; Name; Text[50]) { } }
    keys { key(PK; Code) { Clustered = true; } }
}

table 50101 "Key Five"
{
    fields { field(1; Desc; Text[30]) { } field(5; KeyNo; Integer) { } }
    keys { key(PK; KeyNo) { Clustered = true; } }
}

table 50102 "Two Keys"
{
    fields { field(1; First; Code[10]) { } field(2; Second; Code[10]) { } }
    keys { key(PK; First, Second) { Clustered = true; } }
}

table 50103 "Option Key"
{
    fields { field(1; Kind; Option) { OptionMembers = A,B; } }
    keys { key(PK; Kind) { Clustered = true; } }
}

enum 50104 "Kind Enum" { value(0; A) { } value(1; B) { } }

table 50105 "Enum Key"
{
    fields { field(1; Kind; Enum "Kind Enum") { } }
    keys { key(PK; Kind) { Clustered = true; } }
}

table 50110 "Item Like"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Kind; Option) { OptionMembers = A,B; }
        field(10; "Category Code"; Code[20]) { TableRelation = Category; }
        field(11; KeyFive; Integer) { TableRelation = "Key Five"; }
        field(12; TwoKeys; Code[10]) { TableRelation = "Two Keys"; }
        field(13; WhereConst; Code[20]) { TableRelation = Category where(Name = const('X')); }
        field(14; NoValidate; Code[20]) { TableRelation = Category; ValidateTableRelation = false; }
        field(15; FilterField; Code[20]) { FieldClass = FlowFilter; TableRelation = Category; }
        field(16; UserRef; Guid) { TableRelation = User; }
        field(17; Named; Code[20]) { TableRelation = Category.Code; }
        field(18; NamedOther; Text[50]) { TableRelation = Category.Name; }
        field(19; Conditional; Code[20]) { TableRelation = if (Kind = const(A)) Category else "Two Keys"; }
        field(20; OptionTarget; Option) { OptionMembers = A,B; TableRelation = "Option Key"; }
        field(21; EnumTarget; Enum "Kind Enum") { TableRelation = "Enum Key"; }
        field(22; WhereFilter; Code[20]) { TableRelation = Category where(Name = filter('X*')); }
        field(23; WhereField; Code[20]) { TableRelation = Category where(Name = field("No.")); }
        field(24; TextSource; Text[20]) { TableRelation = Category; }
        field(25; CodeOntoInteger; Code[20]) { TableRelation = "Key Five"; }
        field(26; GuidOntoCategory; Guid) { TableRelation = Category; }
        field(27; TwoWheres; Code[20]) { TableRelation = Category where(Name = const('X'), Code = const('Y')); }
        field(28; RowVersion; Code[20]) { TableRelation = Category.SystemRowVersion; }
        field(29; Flow; Code[20]) { FieldClass = FlowField; CalcFormula = lookup(Category.Code); TableRelation = Category; }
        field(30; ShortCode; Code[10]) { TableRelation = Category; }
        field(31; LongCode; Code[30]) { TableRelation = Category; }
        field(32; TempTarget; Code[20]) { TableRelation = "Temp Category"; }
        field(33; Pending; Code[20]) { TableRelation = Category; ObsoleteState = Pending; ObsoleteReason = 'x'; }
        field(34; Disabled; Code[20]) { TableRelation = Category; Enabled = false; }
        field(35; NoTest; Code[20]) { TableRelation = Category; TestTableRelation = false; }
        field(36; SelfRef; Code[20]) { TableRelation = "Item Like"; }
        field(37; UserWhere; Guid) { TableRelation = User where("User Name" = const('X')); }
        field(38; KeyFiveWhere; Integer) { TableRelation = "Key Five" where(Desc = const('X')); }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}

table 50106 "Temp Category"
{
    TableType = Temporary;
    fields { field(1; Code; Code[20]) { } }
    keys { key(PK; Code) { Clustered = true; } }
}

table 50107 "Temp Source"
{
    TableType = Temporary;
    fields { field(1; Id; Integer) { } field(2; CategoryRef; Code[20]) { TableRelation = Category; } }
    keys { key(PK; Id) { Clustered = true; } }
}

table 50108 "Key Source"
{
    fields { field(1; CategoryRef; Code[20]) { TableRelation = Category; } }
    keys { key(PK; CategoryRef) { Clustered = true; } }
}

tableextension 50111 "Item Like Ext" extends "Item Like"
{
    fields { field(50111; ExtNoField; Code[20]) { TableRelation = Category; } }
}
"#;

const TESTS: &str = r#"
codeunit 50120 T
{
    Subtype = Test;

    local procedure Row(TableNo: Integer; FieldNo: Integer): Text
    var
        F: Record Field;
    begin
        F.Get(TableNo, FieldNo);
        exit(Format(F.RelationTableNo) + '/' + Format(F.RelationFieldNo));
    end;

    local procedure Expect(FieldNo: Integer; Expected: Text)
    begin
        if Row(Database::"Item Like", FieldNo) <> Expected then
            Error('field %1: %2, expected %3', FieldNo, Row(Database::"Item Like", FieldNo), Expected);
    end;

    // The reporter's shape (Item."Item Category Code", TableRelation = "Item Category"): FieldRef.Relation() gives the target,
    // the Field row 5722/0 (FieldRelationFieldNo.al:217, :263).
    [Test]
    procedure ReporterShape()
    var
        RecRef: RecordRef;
    begin
        RecRef.Open(Database::"Item Like");
        if RecRef.Field(10).Relation() <> Database::Category then
            Error('relation %1', RecRef.Field(10).Relation());
        Expect(10, '50100/0');
    end;

    // FieldRelationFieldNo.al:161, :217.
    [Test]
    procedure KeyOnFieldFive() begin Expect(11, '50101/0'); end;
    [Test]
    procedure TwoFieldKey() begin Expect(12, '50102/0'); end;
    [Test]
    procedure ConstWhere() begin Expect(13, '50100/0'); end;
    [Test]
    procedure NoValidate() begin Expect(14, '50100/0'); end;
    [Test]
    procedure FlowFilter() begin Expect(15, '50100/0'); end;
    [Test]
    procedure GuidToUser() begin Expect(16, '2000000120/0'); end;
    [Test]
    procedure ExtensionField() begin Expect(50111, '50100/0'); end;

    // FieldRelationFieldNo.al:236: RelationFieldNo = 0 with RelationTableNo <> 0 lists the no-field relations.
    [Test]
    procedure FilterOnRelationFieldNo()
    var
        F: Record Field;
        Numbers: Text;
    begin
        F.SetRange(TableNo, Database::"Item Like");
        F.SetRange("No.", 10, 16);
        F.SetFilter(RelationTableNo, '<>0');
        F.SetRange(RelationFieldNo, 0);
        if F.FindSet() then
            repeat
                Numbers += Format(F."No.") + ',';
            until F.Next() = 0;
        if Numbers <> '10,11,12,13,14,15,16,' then
            Error('numbers %1', Numbers);
    end;

    // Main's answer, pinned and unmarked: a named field (TestFieldDataClassificationVirtualTable.al:70), a conditional relation
    // (TestPlatformMetadataDeclarationContracts.al:260-261).
    [Test]
    procedure NamedKeyField() begin Expect(17, '50100/1'); end;
    [Test]
    procedure NamedOtherField() begin Expect(18, '50100/2'); end;
    [Test]
    procedure ConditionalRelation() begin Expect(19, '0/0'); end;

    // Not measured: a read of RelationFieldNo refuses, RelationTableNo reads as on main.
    local procedure FieldNoOf(FieldNo: Integer): Integer
    var
        F: Record Field;
    begin
        F.Get(Database::"Item Like", FieldNo);
        exit(F.RelationFieldNo);
    end;

    local procedure TableNoOf(FieldNo: Integer): Integer
    var
        F: Record Field;
    begin
        F.Get(Database::"Item Like", FieldNo);
        exit(F.RelationTableNo);
    end;

    [Test]
    procedure OptionTargetKey() begin if FieldNoOf(20) = 0 then; end;
    [Test]
    procedure EnumTargetKey() begin if FieldNoOf(21) = 0 then; end;
    [Test]
    procedure FilterWhere() begin if FieldNoOf(22) = 0 then; end;
    [Test]
    procedure FieldWhere() begin if FieldNoOf(23) = 0 then; end;
    [Test]
    procedure TextSourceField() begin if FieldNoOf(24) = 0 then; end;
    [Test]
    procedure CodeOntoIntegerKey() begin if FieldNoOf(25) = 0 then; end;
    [Test]
    procedure GuidOntoOtherTable() begin if FieldNoOf(26) = 0 then; end;
    [Test]
    procedure TwoConstWheres() begin if FieldNoOf(27) = 0 then; end;
    [Test]
    procedure RowVersionRelation() begin if FieldNoOf(28) = 0 then; end;
    [Test]
    procedure FlowFieldSource() begin if FieldNoOf(29) = 0 then; end;
    [Test]
    procedure ShorterCodeField() begin if FieldNoOf(30) = 0 then; end;
    [Test]
    procedure LongerCodeField() begin if FieldNoOf(31) = 0 then; end;
    [Test]
    procedure TemporaryTarget() begin if FieldNoOf(32) = 0 then; end;
    [Test]
    procedure ObsoletePendingField() begin if FieldNoOf(33) = 0 then; end;
    [Test]
    procedure DisabledField() begin if FieldNoOf(34) = 0 then; end;
    [Test]
    procedure TestTableRelationFalse() begin if FieldNoOf(35) = 0 then; end;
    [Test]
    procedure RelationOntoOwnTable() begin if FieldNoOf(36) = 0 then; end;
    [Test]
    procedure GuidToUserWithWhere() begin if FieldNoOf(37) = 0 then; end;
    [Test]
    procedure IntegerKeyWithWhere() begin if FieldNoOf(38) = 0 then; end;
    [Test]
    procedure TemporarySource()
    var
        F: Record Field;
    begin
        F.Get(Database::"Temp Source", 2);
        if F.RelationFieldNo = 0 then;
    end;
    [Test]
    procedure FieldIsPrimaryKey()
    var
        F: Record Field;
    begin
        F.Get(Database::"Key Source", 1);
        if F.RelationFieldNo = 0 then;
    end;
    // A read of the row that does not read RelationFieldNo, of a field outside the measured shapes: main's answer.
    [Test]
    procedure UnmeasuredRowOtherColumn()
    var
        F: Record Field;
    begin
        F.Get(Database::"Item Like", 30);
        if F.FieldName <> 'ShortCode' then
            Error('name %1', F.FieldName);
    end;
    [Test]
    procedure OptionTargetTableNo() begin if TableNoOf(20) <> Database::"Option Key" then Error('table %1', TableNoOf(20)); end;
}
"#;

pub(super) fn outcome<'a>(
    o: &'a [(String, Outcome, Vec<Assumption>)],
    n: &str,
) -> &'a (String, Outcome, Vec<Assumption>) {
    o.iter()
        .find(|(x, ..)| x == n)
        .unwrap_or_else(|| panic!("no test {n}: {o:?}"))
}

pub(super) fn assert_clean_pass(o: &[(String, Outcome, Vec<Assumption>)], names: &[&str]) {
    for n in names {
        let (_, r, hits) = outcome(o, n);
        assert_eq!(*r, Outcome::Pass, "{n}: {r:?}");
        assert!(hits.is_empty(), "{n} flagged: {hits:?}");
    }
}

pub(super) fn assert_refused(o: &[(String, Outcome, Vec<Assumption>)], names: &[&str]) {
    for n in names {
        let r = &outcome(o, n).1;
        assert!(
            matches!(r, Outcome::NotRun(why) if why.contains("RelationFieldNo")),
            "{n}: {r:?}"
        );
    }
}

/// The measured shapes read 0, clean passes; every other shape refuses a read of RelationFieldNo; named fields and conditional relations keep main's pinned answer.
#[test]
fn issue_8249_relation_without_field_reads_field_no_zero() {
    let src = format!("{TABLES}{TESTS}");
    let o = run(&[(APP_ID, AppKind::Workspace, &src)]);
    assert_refused(
        &o,
        &[
            "OptionTargetKey",
            "EnumTargetKey",
            "FilterWhere",
            "FieldWhere",
            "TextSourceField",
            "CodeOntoIntegerKey",
            "GuidOntoOtherTable",
            "TwoConstWheres",
            "RowVersionRelation",
            "FlowFieldSource",
            "ShorterCodeField",
            "LongerCodeField",
            "TemporaryTarget",
            "ObsoletePendingField",
            "DisabledField",
            "TestTableRelationFalse",
            "RelationOntoOwnTable",
            "GuidToUserWithWhere",
            "IntegerKeyWithWhere",
            "TemporarySource",
            "FieldIsPrimaryKey",
        ],
    );
    assert_clean_pass(
        &o,
        &[
            "ReporterShape",
            "KeyOnFieldFive",
            "TwoFieldKey",
            "ConstWhere",
            "NoValidate",
            "FlowFilter",
            "GuidToUser",
            "ExtensionField",
            "FilterOnRelationFieldNo",
        ],
    );
    assert_clean_pass(
        &o,
        &[
            "NamedKeyField",
            "NamedOtherField",
            "ConditionalRelation",
            "OptionTargetTableNo",
            "UnmeasuredRowOtherColumn",
        ],
    );
}

/// A relation onto a table of another app, and a field a table extension of another app declares: not measured, refused.
#[test]
fn issue_8249_other_app_target_and_extension_are_refused() {
    let test = r#"
tableextension 50130 "Other Ext" extends "Item Like"
{
    fields { field(50130; OtherExtNoField; Code[20]) { TableRelation = Category; } }
}

table 50131 "Own Source"
{
    fields { field(1; Id; Integer) { } field(2; CategoryRef; Code[20]) { TableRelation = Category; } }
    keys { key(PK; Id) { Clustered = true; } }
}

codeunit 50120 T
{
    Subtype = Test;

    [Test]
    procedure OtherAppExtension()
    var
        F: Record Field;
    begin
        F.Get(Database::"Item Like", 50130);
        if F.RelationFieldNo = 0 then;
    end;

    [Test]
    procedure OtherAppTarget()
    var
        F: Record Field;
    begin
        F.Get(Database::"Own Source", 2);
        if F.RelationFieldNo = 0 then;
    end;

    [Test]
    procedure OtherAppTargetTableNo()
    var
        F: Record Field;
    begin
        F.Get(Database::"Own Source", 2);
        if F.RelationTableNo <> Database::Category then
            Error('table %1', F.RelationTableNo);
    end;
}"#;
    let o = run(&[
        (OTHER_ID, AppKind::Workspace, TABLES),
        (APP_ID, AppKind::Workspace, test),
    ]);
    assert_refused(&o, &["OtherAppExtension", "OtherAppTarget"]);
    assert_clean_pass(&o, &["OtherAppTargetTableNo"]);
}

/// A target named after its namespace reads as the same relation named alone (FieldRelationFieldNo.al:88, :161-180).
#[test]
fn issue_8249_namespace_qualified_target_reads_field_no_zero() {
    let src = r#"namespace Review.Rfn;

table 50140 "N Category"
{
    fields { field(1; Code; Code[20]) { } }
    keys { key(PK; Code) { Clustered = true; } }
}

table 50141 "N Source"
{
    fields { field(1; Id; Integer) { } field(2; Qualified; Code[20]) { TableRelation = Review.Rfn."N Category"; } }
    keys { key(PK; Id) { Clustered = true; } }
}

codeunit 50142 T
{
    Subtype = Test;

    [Test]
    procedure QualifiedTarget()
    var
        F: Record Field;
    begin
        F.Get(Database::"N Source", 2);
        if Format(F.RelationTableNo) + '/' + Format(F.RelationFieldNo) <> '50140/0' then
            Error('row %1/%2', F.RelationTableNo, F.RelationFieldNo);
    end;
}"#;
    let o = run(&[(APP_ID, AppKind::Workspace, src)]);
    assert_clean_pass(&o, &["QualifiedTarget"]);
}
