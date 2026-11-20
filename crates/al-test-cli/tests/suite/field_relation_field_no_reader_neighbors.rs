//! al-runner issue 8249, review round 2: the neighbors of the RelationFieldNo readers corpus PR 1477 measured. The assumption
//! relation-field-no-readers covers FieldRef.Value only after FindFirst, FindSet and Next, Get(RecordId) and GetTable, a RecordRef
//! Count, FindSet or Next filtered on RelationFieldNo only by FieldRef.SetRange(0), FieldRef.SetFilter('<>0') or SetView CONST(0), and Query.Open of one
//! Field data item without a trigger; every other way refuses, and a named field keeps main's clean result.
//!
//! Needs `System.app` of the pinned build in the artifact cache (`al_testfiles`).

use super::field_extension_package_ids::run;
use super::field_relation_field_no::{assert_clean_pass, assert_refused, outcome};
use al_builtins::assumptions::Assumption;
use al_sema::workspace::AppKind;
use al_test_cli::report::Outcome;

const APP_ID: &str = "8249c000-0000-4000-8000-000000008250";

const SRC: &str = r#"
table 50100 "Category"
{
    fields { field(1; Code; Code[20]) { } field(2; Name; Text[50]) { } }
    keys { key(PK; Code) { Clustered = true; } }
}

table 50110 "Item Like"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(10; "Category Code"; Code[20]) { TableRelation = Category; }
        field(17; Named; Code[20]) { TableRelation = Category.Code; }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}

query 50141 "Field Rows Before Open"
{
    elements
    {
        dataitem(FieldRow; Field)
        {
            column(No; "No.") { }
        }
    }
    trigger OnBeforeOpen() begin end;
}

query 50142 "Field Rows Two Items"
{
    elements
    {
        dataitem(Cat; Category)
        {
            column(Code; Code) { }
            dataitem(FieldRow; Field)
            {
                SqlJoinType = CrossJoin;
                column(No; "No.") { }
            }
        }
    }
}

codeunit 50120 T
{
    Subtype = Test;

    local procedure Filtered(var RecRef: RecordRef; FieldNo: Integer)
    begin
        RecRef.Open(Database::Field);
        RecRef.Field(1).SetRange(Database::"Item Like");
        RecRef.Field(2).SetRange(FieldNo);
    end;

    local procedure Check(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('got %1, expected %2', Actual, Expected);
    end;

    local procedure RecId(FieldNo: Integer): RecordId
    var
        F: Record Field;
    begin
        F.Get(Database::"Item Like", FieldNo);
        exit(F.RecordId);
    end;

    // Measured: FieldRelationFieldNoReaders.al:235 (FindSet and Next).
    [Test]
    procedure ScanValue()
    var
        RecRef: RecordRef;
        Rows: Text;
    begin
        RecRef.Open(Database::Field);
        RecRef.Field(1).SetRange(Database::"Item Like");
        if RecRef.FindSet() then
            repeat
                Rows += Format(RecRef.Field(22).Value) + ',';
            until RecRef.Next() = 0;
        Check(CopyStr(Rows, 1, 6), '0,0,1,');
    end;

    // Measured: :172 and :174.
    [Test]
    procedure GetTableAndRecordIdValue()
    var
        F: Record Field;
        RecRef: RecordRef;
        ById: RecordRef;
    begin
        F.Get(Database::"Item Like", 10);
        RecRef.GetTable(F);
        Check(Format(RecRef.Field(22).Value), '0');
        ById.Open(Database::Field);
        ById.Get(RecId(10));
        Check(Format(ById.Field(22).Value), '0');
    end;

    // Measured: :315 (SetView WHERE, then FindSet).
    [Test]
    procedure SetViewCount()
    var
        RecRef: RecordRef;
    begin
        RecRef.Open(Database::Field);
        RecRef.SetView('WHERE(TableNo=CONST(50110),RelationTableNo=CONST(50100),RelationFieldNo=CONST(0))');
        Check(Format(RecRef.Count()), '1');
    end;

    [Test]
    procedure FindLastValue()
    var
        RecRef: RecordRef;
    begin
        Filtered(RecRef, 10);
        RecRef.FindLast();
        Check(Format(RecRef.Field(22).Value), '0');
    end;

    [Test]
    procedure FindEqualsValue()
    var
        RecRef: RecordRef;
    begin
        RecRef.Open(Database::Field);
        RecRef.Field(1).Value := Database::"Item Like";
        RecRef.Field(2).Value := 10;
        RecRef.Find('=');
        Check(Format(RecRef.Field(22).Value), '0');
    end;

    [Test]
    procedure FindMinusValue()
    var
        RecRef: RecordRef;
    begin
        Filtered(RecRef, 10);
        RecRef.Find('-');
        Check(Format(RecRef.Field(22).Value), '0');
    end;

    [Test]
    procedure NextAfterFindFirstValue()
    var
        RecRef: RecordRef;
    begin
        RecRef.Open(Database::Field);
        RecRef.Field(1).SetRange(Database::"Item Like");
        RecRef.Field(2).SetFilter('1|10');
        RecRef.FindFirst();
        RecRef.Next();
        Check(Format(RecRef.Field(22).Value), '0');
    end;

    [Test]
    procedure WrittenAfterFindFirstValue()
    var
        RecRef: RecordRef;
    begin
        Filtered(RecRef, 10);
        RecRef.FindFirst();
        RecRef.Field(21).Value := 5;
        Check(Format(RecRef.Field(22).Value), '0');
    end;

    [Test]
    procedure OpenWithCompanyValue()
    var
        RecRef: RecordRef;
    begin
        RecRef.Open(Database::Field, false, CompanyName());
        RecRef.Field(1).SetRange(Database::"Item Like");
        RecRef.Field(2).SetRange(10);
        RecRef.FindFirst();
        Check(Format(RecRef.Field(22).Value), '0');
    end;

    [Test]
    procedure ChangeCompanyGetTableValue()
    var
        F: Record Field;
        RecRef: RecordRef;
    begin
        F.ChangeCompany(CompanyName());
        F.Get(Database::"Item Like", 10);
        RecRef.GetTable(F);
        Check(Format(RecRef.Field(22).Value), '0');
    end;

    [Test]
    procedure RecordFilterGetTableCount()
    var
        F: Record Field;
        RecRef: RecordRef;
    begin
        F.SetRange(TableNo, Database::"Item Like");
        F.SetRange(RelationFieldNo, 0);
        RecRef.GetTable(F);
        Check(Format(RecRef.Count()), '11');
    end;

    [Test]
    procedure FilterGroupTwoCount()
    var
        RecRef: RecordRef;
    begin
        RecRef.Open(Database::Field);
        RecRef.Field(1).SetRange(Database::"Item Like");
        RecRef.FilterGroup(2);
        RecRef.Field(22).SetRange(0);
        RecRef.FilterGroup(0);
        Check(Format(RecRef.Count()), '11');
    end;

    [Test]
    procedure SetFilterOrCount()
    var
        RecRef: RecordRef;
    begin
        RecRef.Open(Database::Field);
        RecRef.Field(1).SetRange(Database::"Item Like");
        RecRef.Field(22).SetFilter('0|5');
        Check(Format(RecRef.Count()), '11');
    end;

    [Test]
    procedure FilteredIsEmpty()
    var
        RecRef: RecordRef;
    begin
        Filtered(RecRef, 10);
        RecRef.Field(22).SetRange(0);
        Check(Format(RecRef.IsEmpty()), 'No');
    end;

    [Test]
    procedure FilteredFindFirst()
    var
        RecRef: RecordRef;
    begin
        RecRef.Open(Database::Field);
        RecRef.Field(1).SetRange(Database::"Item Like");
        RecRef.Field(22).SetRange(0);
        RecRef.Field(21).SetFilter('<>0');
        RecRef.FindFirst();
        Check(Format(RecRef.Field(2).Value), '10');
    end;

    [Test]
    procedure FilteredFindMinus()
    var
        RecRef: RecordRef;
    begin
        RecRef.Open(Database::Field);
        RecRef.Field(1).SetRange(Database::"Item Like");
        RecRef.Field(22).SetRange(0);
        RecRef.Field(21).SetFilter('<>0');
        RecRef.Find('-');
        Check(Format(RecRef.Field(2).Value), '10');
    end;

    [Test]
    procedure FilteredCountApprox()
    var
        RecRef: RecordRef;
    begin
        RecRef.Open(Database::Field);
        RecRef.Field(1).SetRange(Database::"Item Like");
        RecRef.Field(22).SetRange(0);
        if RecRef.CountApprox() < 0 then
            Error('negative');
    end;

    [Test]
    procedure GetTableIntoOpenRefValue()
    var
        F: Record Field;
        RecRef: RecordRef;
    begin
        RecRef.Open(Database::Field);
        F.Get(Database::"Item Like", 10);
        RecRef.GetTable(F);
        Check(Format(RecRef.Field(22).Value), '0');
    end;

    [Test]
    procedure FormatFieldRefNine()
    var
        RecRef: RecordRef;
    begin
        Filtered(RecRef, 10);
        RecRef.FindFirst();
        Check(Format(RecRef.Field(22), 0, 9), '0');
    end;

    [Test]
    procedure QueryBeforeOpenOpen()
    var
        Q: Query "Field Rows Before Open";
    begin
        Q.Open();
    end;

    [Test]
    procedure QueryTwoItemsOpen()
    var
        Q: Query "Field Rows Two Items";
    begin
        Q.Open();
    end;

    // Main's result, unmarked: a named field after FindLast, Format 9 of another column.
    [Test]
    procedure FindLastNamedField()
    var
        RecRef: RecordRef;
    begin
        Filtered(RecRef, 17);
        RecRef.FindLast();
        Check(Format(RecRef.Field(22).Value), '1');
    end;

    [Test]
    procedure FormatOtherColumnNine()
    var
        RecRef: RecordRef;
    begin
        Filtered(RecRef, 10);
        RecRef.FindFirst();
        Check(Format(RecRef.Field(21), 0, 9), '50100');
    end;
}
"#;

fn assert_flagged_pass(o: &[(String, Outcome, Vec<Assumption>)], names: &[&str]) {
    for n in names {
        let (_, r, hits) = outcome(o, n);
        assert_eq!(*r, Outcome::Pass, "{n}: {r:?}");
        assert_eq!(hits, &[Assumption::RelationFieldNoReaders], "{n}");
    }
}

/// The measured positionings and filter sources pass flagged; every other positioning, filter source, scan, Format 9 and query
/// shape refuses; a named field and another column keep main's clean result.
#[test]
fn issue_8249_relation_field_no_reader_neighbors() {
    let o = run(&[(APP_ID, AppKind::Workspace, SRC)]);
    assert_flagged_pass(&o, &["ScanValue", "GetTableAndRecordIdValue", "SetViewCount"]);
    assert_refused(
        &o,
        &[
            "FindLastValue",
            "FindEqualsValue",
            "FindMinusValue",
            "NextAfterFindFirstValue",
            "WrittenAfterFindFirstValue",
            "OpenWithCompanyValue",
            "ChangeCompanyGetTableValue",
            "RecordFilterGetTableCount",
            "FilterGroupTwoCount",
            "SetFilterOrCount",
            "FilteredIsEmpty",
            "FilteredFindFirst",
            "FilteredFindMinus",
            "FilteredCountApprox",
            "GetTableIntoOpenRefValue",
            "FormatFieldRefNine",
        ],
    );
    for n in ["QueryBeforeOpenOpen", "QueryTwoItemsOpen"] {
        let (_, r, _) = outcome(&o, n);
        assert!(
            matches!(r, Outcome::NotRun(why) if why.contains("a query over the Field table other than Query.Open")),
            "{n}: {r:?}"
        );
    }
    assert_clean_pass(&o, &["FindLastNamedField", "FormatOtherColumnNine"]);
}
