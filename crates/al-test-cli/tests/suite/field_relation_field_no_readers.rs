//! al-runner issue 8249: the readers of Field.RelationFieldNo other than a Record variable for a TableRelation without a field.
//! The ones corpus PR 1477 measured (FieldRelationFieldNoReaders.al) run under the assumption relation-field-no-readers, every
//! other one refuses. A temporary Field record keeps what AL wrote into it, another column and a named field keep main's result,
//! and a Record variable's reads are not touched by the assumption (pinned by FieldRelationFieldNo.al:161-260).
//!
//! Needs `System.app` of the pinned build in the artifact cache (`al_testfiles`).

use super::field_extension_package_ids::run;
use super::field_relation_field_no::{assert_clean_pass, assert_refused, outcome};
use al_builtins::assumptions::Assumption;
use al_sema::workspace::AppKind;
use al_test_cli::report::Outcome;

const APP_ID: &str = "8249c000-0000-4000-8000-000000008249";

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

page 50130 "Field List"
{
    PageType = List;
    SourceTable = Field;
    Editable = false;
    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(TableNo; Rec.TableNo) { }
                field("No."; Rec."No.") { }
                field(RelationTableNo; Rec.RelationTableNo) { }
                field(RelationFieldNo; Rec.RelationFieldNo) { }
            }
        }
    }
}

page 50131 "Field Card"
{
    PageType = Card;
    SourceTable = Field;
    Editable = false;
    layout
    {
        area(Content)
        {
            group(General)
            {
                field(TableNo; Rec.TableNo) { }
                field("No."; Rec."No.") { }
                field(RelationTableNo; Rec.RelationTableNo) { }
                field(RelationFieldNo; Rec.RelationFieldNo) { }
            }
        }
    }
}

page 50132 "Field List Zero View"
{
    PageType = List;
    SourceTable = Field;
    SourceTableView = where(RelationFieldNo = const(0));
    Editable = false;
    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("No."; Rec."No.") { }
                field(RelationFieldNo; Rec.RelationFieldNo) { }
            }
        }
    }
}

query 50140 "Field Rows"
{
    elements
    {
        dataitem(FieldRow; Field)
        {
            column(TableNo; TableNo) { }
            column(No; "No.") { }
            column(RelationFieldNo; RelationFieldNo) { }
        }
    }
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

codeunit 50120 T
{
    Subtype = Test;

    local procedure OpenRow(var RecRef: RecordRef; FieldNo: Integer)
    begin
        RecRef.Open(Database::Field);
        RecRef.Field(1).SetRange(Database::"Item Like");
        RecRef.Field(2).SetRange(FieldNo);
        RecRef.FindFirst();
    end;

    local procedure Check(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('got %1, expected %2', Actual, Expected);
    end;

    // Measured: FieldRelationFieldNoReaders.al:148.
    [Test]
    procedure RefValue()
    var
        RecRef: RecordRef;
    begin
        OpenRow(RecRef, 10);
        Check(Format(RecRef.Field(21).Value) + '/' + Format(RecRef.Field(22).Value), '50100/0');
    end;

    // Measured: :283.
    [Test]
    procedure RefFilter()
    var
        RecRef: RecordRef;
    begin
        RecRef.Open(Database::Field);
        RecRef.Field(1).SetRange(Database::"Item Like");
        RecRef.Field(21).SetFilter('<>0');
        RecRef.Field(22).SetRange(0);
        // Field 10 and SystemCreatedBy and SystemModifiedBy (User/0).
        Check(Format(RecRef.Count()), '3');
    end;

    // Measured: :380.
    [Test]
    procedure ListValueAndAsInteger()
    var
        ListPage: TestPage "Field List";
    begin
        ListPage.OpenView();
        ListPage.GoToKey(Database::"Item Like", 10);
        Check(ListPage.RelationFieldNo.Value, '0');
        Check(Format(ListPage.RelationFieldNo.AsInteger()), '0');
        ListPage.Close();
    end;

    // Measured: :412.
    [Test]
    procedure ListFilter()
    var
        ListPage: TestPage "Field List";
    begin
        ListPage.OpenView();
        ListPage.Filter.SetFilter(TableNo, Format(Database::"Item Like"));
        ListPage.Filter.SetFilter(RelationFieldNo, '0');
        ListPage.First();
        Check(Format(ListPage."No.".AsInteger()), '1');
        ListPage.Close();
    end;

    // Measured: :434-441.
    [Test]
    procedure CardAsInteger()
    var
        CardPage: TestPage "Field Card";
    begin
        CardPage.OpenView();
        CardPage.GoToKey(Database::"Item Like", 10);
        Check(Format(CardPage.RelationFieldNo.AsInteger()), '0');
        CardPage.Close();
    end;

    // Measured: :446-450.
    [Test]
    procedure CardTrap()
    var
        F: Record Field;
        Trapped: TestPage "Field Card";
    begin
        F.Get(Database::"Item Like", 10);
        Trapped.Trap();
        Page.Run(Page::"Field Card", F);
        Check(Format(Trapped.RelationFieldNo.AsInteger()), '0');
        Trapped.Close();
    end;

    // Measured: :327-353.
    [Test]
    procedure QueryOpen()
    var
        Q: Query "Field Rows";
    begin
        Q.SetRange(TableNo, Database::"Item Like");
        asserterror Q.Open();
        Check(GetLastErrorText(), 'A query cannot be based on a virtual table. Please contact your system administrator.');
        Check(GetLastErrorCode(), 'DB:Ok');
    end;

    [Test]
    procedure RefOrder()
    var
        RecRef: RecordRef;
    begin
        RecRef.Open(Database::Field);
        RecRef.SetView('SORTING(RelationFieldNo) WHERE(TableNo=CONST(50110))');
        RecRef.FindFirst();
    end;

    [Test]
    procedure FormatRecRef()
    var
        RecRef: RecordRef;
    begin
        OpenRow(RecRef, 10);
        if Format(RecRef) = '' then
            Error('empty');
    end;

    [Test]
    procedure FormatFieldRef()
    var
        RecRef: RecordRef;
    begin
        OpenRow(RecRef, 10);
        Check(Format(RecRef.Field(22)), '0');
    end;

    [Test]
    procedure RefTestField()
    var
        RecRef: RecordRef;
    begin
        OpenRow(RecRef, 10);
        RecRef.Field(22).TestField(0);
    end;

    [Test]
    procedure RecordTestField()
    var
        F: Record Field;
    begin
        F.Get(Database::"Item Like", 10);
        F.TestField(RelationFieldNo, 0);
    end;

    [Test]
    procedure RecordCalcSums()
    var
        F: Record Field;
    begin
        F.SetRange(TableNo, Database::"Item Like");
        F.CalcSums(RelationFieldNo);
    end;

    [Test]
    procedure CardValue()
    var
        CardPage: TestPage "Field Card";
    begin
        CardPage.OpenView();
        CardPage.GoToKey(Database::"Item Like", 10);
        Check(CardPage.RelationFieldNo.Value, '0');
    end;

    [Test]
    procedure ListOpenEdit()
    var
        ListPage: TestPage "Field List";
    begin
        ListPage.OpenEdit();
        ListPage.GoToKey(Database::"Item Like", 10);
        Check(Format(ListPage.RelationFieldNo.AsInteger()), '0');
    end;

    [Test]
    procedure ListAssertEquals()
    var
        ListPage: TestPage "Field List";
    begin
        ListPage.OpenView();
        ListPage.GoToKey(Database::"Item Like", 10);
        ListPage.RelationFieldNo.AssertEquals(0);
    end;

    [Test]
    procedure CardFilter()
    var
        CardPage: TestPage "Field Card";
    begin
        CardPage.OpenView();
        CardPage.Filter.SetFilter(RelationFieldNo, '0');
    end;

    [Test]
    procedure PageViewFilter()
    var
        ListPage: TestPage "Field List Zero View";
    begin
        ListPage.OpenView();
    end;

    [Test]
    procedure QueryBeforeOpen()
    var
        Q: Query "Field Rows Before Open";
    begin
        Q.Open();
        Q.Read();
    end;

    // Main's result, unmarked.
    [Test]
    procedure RefOtherColumn()
    var
        RecRef: RecordRef;
    begin
        OpenRow(RecRef, 10);
        Check(Format(RecRef.Field(21).Value), '50100');
    end;

    [Test]
    procedure RefNamedField()
    var
        RecRef: RecordRef;
    begin
        OpenRow(RecRef, 17);
        Check(Format(RecRef.Field(22).Value), '1');
    end;

    [Test]
    procedure TempRef()
    var
        TempF: Record Field temporary;
        RecRef: RecordRef;
    begin
        TempF.TableNo := 50110;
        TempF."No." := 10;
        TempF.RelationTableNo := 50100;
        TempF.RelationFieldNo := 0;
        TempF.Insert();
        RecRef.GetTable(TempF);
        RecRef.FindFirst();
        Check(Format(RecRef.Field(22).Value), '0');
        Check(Format(RecRef), Format(RecRef));
    end;

    [Test]
    procedure RecordRead()
    var
        F: Record Field;
    begin
        F.Get(Database::"Item Like", 10);
        Check(Format(F.RelationFieldNo), '0');
    end;

    [Test]
    procedure ListOtherColumn()
    var
        ListPage: TestPage "Field List";
    begin
        ListPage.OpenView();
        ListPage.GoToKey(Database::"Item Like", 10);
        Check(ListPage.RelationTableNo.Value, '50100');
        ListPage.Close();
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

/// The measured readers pass flagged; the unmeasured ones refuse; another column, a named field and a temporary Field record
/// keep main's clean result, and a Record read passes clean under the pin (FieldRelationFieldNo.al:161-260).
#[test]
fn issue_8249_relation_field_no_readers() {
    let o = run(&[(APP_ID, AppKind::Workspace, SRC)]);
    assert_flagged_pass(
        &o,
        &[
            "RefValue",
            "RefFilter",
            "ListValueAndAsInteger",
            "ListFilter",
            "CardAsInteger",
            "CardTrap",
            "QueryOpen",
        ],
    );
    assert_refused(
        &o,
        &[
            "RefOrder",
            "FormatRecRef",
            "FormatFieldRef",
            "RefTestField",
            "RecordTestField",
            "RecordCalcSums",
            "CardValue",
            "ListOpenEdit",
            "ListAssertEquals",
            "CardFilter",
            "PageViewFilter",
        ],
    );
    let (_, r, _) = outcome(&o, "QueryBeforeOpen");
    assert!(
        matches!(r, Outcome::NotRun(why) if why.contains("a query over the Field table other than Query.Open")),
        "QueryBeforeOpen: {r:?}"
    );
    assert_clean_pass(
        &o,
        &[
            "RefOtherColumn",
            "RefNamedField",
            "TempRef",
            "RecordRead",
            "ListOtherColumn",
        ],
    );
}
