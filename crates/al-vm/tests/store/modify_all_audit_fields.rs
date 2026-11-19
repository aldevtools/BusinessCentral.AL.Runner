//! `ModifyAll` of an audit field (record/system-fields/AssignedAuditFields.al:594-631, ModifyAll_CreatedAt): SystemCreatedAt,
//! SystemCreatedBy and SystemModifiedAt raise the "cannot change" error in the measured shape only (a table as table 61800
//! declares it that holds one row, the variable as Reset leaves it on the primary key ascending, no RunTrigger argument, a
//! value that is not blank and differs from the stored one, called from a codeunit). Each neighbor shape that changes one of
//! those facts refuses; another field keeps main's ModifyAll.

use al_builtins::native::Signal;
use al_testkit::{run_in, text};

const SRC: &str = r#"
table 50330 A
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
    }
    keys { key(PK; Id) { } }
}

table 50331 B
{
    Caption = 'Bee';
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { } }
}

table 50333 W
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { } }
    trigger OnModify() begin end;
}

table 50334 S
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { } }
}

table 50336 Ins
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { } }
    trigger OnInsert() begin end;
}

table 50337 Del
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { } }
    trigger OnDelete() begin end;
}

table 50338 Val
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { trigger OnValidate() begin end; } }
    keys { key(PK; Id) { } }
}

table 50339 Ext
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { } }
}

tableextension 50340 ExtX extends Ext
{
    fields { field(50000; Extra; Integer) { } }
}

table 50341 Shared
{
    DataPerCompany = false;
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { } }
}

table 50342 Proc
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { } }

    procedure ModifyAllOnRec()
    begin
        Rec.Id := 1; Rec.Insert();
        Rec.ModifyAll(SystemCreatedAt, CreateDateTime(20200102D, 120000T));
    end;
}

table 2000000006 Company
{
    DataPerCompany = false;
    fields
    {
        field(1; Name; Text[30]) { }
        field(2; "Evaluation Company"; Boolean) { }
        field(3; "Display Name"; Text[250]) { }
        field(8000; Id; Guid) { }
        field(8005; "Business Profile Id"; Text[250]) { }
    }
    keys { key(PK; Name) { } }
}

codeunit 50335 Sub
{
    [EventSubscriber(ObjectType::Table, Database::S, 'OnAfterModifyEvent', '', false, false)]
    local procedure AfterModify(var Rec: Record S) begin end;
}

codeunit 50332 T
{
    local procedure Seed(N: Integer)
    var
        P: Record A;
        I: Integer;
    begin
        P.DeleteAll();
        for I := 1 to N do begin
            P.Init(); P.Id := I; P.Insert();
        end;
    end;

    procedure CreatedAt(): Text
    var
        P: Record A;
    begin
        Seed(1);
        asserterror P.ModifyAll(SystemCreatedAt, CreateDateTime(20200102D, 120000T));
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure CreatedBy(): Text
    var
        P: Record A;
    begin
        Seed(1);
        asserterror P.ModifyAll(SystemCreatedBy, CreateGuid());
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure ModifiedAt(): Text
    var
        P: Record A;
    begin
        Seed(1);
        asserterror P.ModifyAll(SystemModifiedAt, CreateDateTime(20200102D, 120000T));
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure ModifiedBy()
    var
        P: Record A;
    begin
        Seed(1);
        P.ModifyAll(SystemModifiedBy, CreateGuid());
    end;

    procedure TwoRows()
    var
        P: Record A;
    begin
        Seed(2);
        P.ModifyAll(SystemCreatedAt, CreateDateTime(20200102D, 120000T));
    end;

    procedure NoRows()
    var
        P: Record A;
    begin
        Seed(0);
        P.ModifyAll(SystemCreatedAt, CreateDateTime(20200102D, 120000T));
    end;

    procedure RunTrigger()
    var
        P: Record A;
    begin
        Seed(1);
        P.ModifyAll(SystemCreatedAt, CreateDateTime(20200102D, 120000T), false);
    end;

    procedure Temporary()
    var
        P: Record A temporary;
    begin
        P.Id := 1; P.Insert();
        P.ModifyAll(SystemCreatedAt, CreateDateTime(20200102D, 120000T));
    end;

    procedure TableCaption()
    var
        P: Record B;
    begin
        P.Id := 1; P.Insert();
        P.ModifyAll(SystemCreatedAt, CreateDateTime(20200102D, 120000T));
    end;

    procedure TableWithOnModify()
    var
        P: Record W;
    begin
        P.Id := 1; P.Insert();
        P.ModifyAll(SystemCreatedAt, CreateDateTime(20200102D, 120000T));
    end;

    procedure TableWithSubscriber()
    var
        P: Record S;
    begin
        P.Id := 1; P.Insert();
        P.ModifyAll(SystemCreatedAt, CreateDateTime(20200102D, 120000T));
    end;

    procedure FilteredOneRow()
    var
        P: Record A;
    begin
        Seed(1);
        P.SetRange(Id, 1);
        P.ModifyAll(SystemCreatedAt, CreateDateTime(20200102D, 120000T));
    end;

    procedure FilteredTwoRows()
    var
        P: Record A;
    begin
        Seed(2);
        P.SetRange(Id, 1);
        P.ModifyAll(SystemCreatedAt, CreateDateTime(20200102D, 120000T));
    end;

    procedure FilterInGroup2()
    var
        P: Record A;
    begin
        Seed(2);
        P.FilterGroup(2);
        P.SetRange(Id, 1);
        P.FilterGroup(0);
        P.ModifyAll(SystemCreatedAt, CreateDateTime(20200102D, 120000T));
    end;

    procedure OtherFilterGroup()
    var
        P: Record A;
    begin
        Seed(1);
        P.FilterGroup(2);
        P.ModifyAll(SystemCreatedAt, CreateDateTime(20200102D, 120000T));
    end;

    procedure MarkedOnly()
    var
        P: Record A;
    begin
        Seed(2);
        P.Get(1);
        P.Mark(true);
        P.MarkedOnly(true);
        P.ModifyAll(SystemCreatedAt, CreateDateTime(20200102D, 120000T));
    end;

    procedure Marks()
    var
        P: Record A;
    begin
        Seed(1);
        P.Get(1);
        P.Mark(true);
        P.ModifyAll(SystemCreatedAt, CreateDateTime(20200102D, 120000T));
    end;

    procedure OtherKey()
    var
        P: Record A;
    begin
        Seed(1);
        P.SetCurrentKey(Name);
        P.ModifyAll(SystemCreatedAt, CreateDateTime(20200102D, 120000T));
    end;

    procedure Descending()
    var
        P: Record A;
    begin
        Seed(1);
        P.Ascending(false);
        P.ModifyAll(SystemCreatedAt, CreateDateTime(20200102D, 120000T));
    end;

    procedure ChangeCompany()
    var
        P: Record A;
        C: Record Company;
    begin
        C.Name := 'Other';
        C.Insert();
        P.ChangeCompany('Other');
        P.Id := 1;
        P.Insert();
        P.ModifyAll(SystemCreatedAt, CreateDateTime(20200102D, 120000T));
    end;

    procedure LoadFields()
    var
        P: Record A;
    begin
        Seed(1);
        P.SetLoadFields(Name);
        P.ModifyAll(SystemCreatedAt, CreateDateTime(20200102D, 120000T));
    end;

    procedure ReadIsolation()
    var
        P: Record A;
    begin
        Seed(1);
        P.ReadIsolation := IsolationLevel::ReadUncommitted;
        P.ModifyAll(SystemCreatedAt, CreateDateTime(20200102D, 120000T));
    end;

    procedure FilterOnTheField()
    var
        P: Record A;
    begin
        Seed(1);
        P.SetFilter(SystemCreatedAt, '<>%1', 0DT);
        P.ModifyAll(SystemCreatedAt, CreateDateTime(20200102D, 120000T));
    end;

    procedure BlankValue()
    var
        P: Record A;
    begin
        Seed(1);
        P.ModifyAll(SystemCreatedAt, 0DT);
    end;

    procedure StoredValue()
    var
        P: Record A;
    begin
        Seed(1);
        P.Get(1);
        P.Reset();
        P.ModifyAll(SystemCreatedAt, P.SystemCreatedAt);
    end;

    procedure TableWithOnInsert()
    var
        P: Record Ins;
    begin
        P.DeleteAll();
        P.Id := 1; P.Insert();
        P.ModifyAll(SystemCreatedAt, CreateDateTime(20200102D, 120000T));
    end;

    procedure TableWithOnDelete()
    var
        P: Record Del;
    begin
        P.DeleteAll();
        P.Id := 1; P.Insert();
        P.ModifyAll(SystemCreatedAt, CreateDateTime(20200102D, 120000T));
    end;

    procedure FieldWithOnValidate()
    var
        P: Record Val;
    begin
        P.DeleteAll();
        P.Id := 1; P.Insert();
        P.ModifyAll(SystemCreatedAt, CreateDateTime(20200102D, 120000T));
    end;

    procedure TableWithExtension()
    var
        P: Record Ext;
    begin
        P.DeleteAll();
        P.Id := 1; P.Insert();
        P.ModifyAll(SystemCreatedAt, CreateDateTime(20200102D, 120000T));
    end;

    procedure SharedTable()
    var
        P: Record Shared;
    begin
        P.DeleteAll();
        P.Id := 1; P.Insert();
        P.ModifyAll(SystemCreatedAt, CreateDateTime(20200102D, 120000T));
    end;

    procedure OnRecInATableProcedure()
    var
        P: Record Proc;
    begin
        P.DeleteAll();
        P.ModifyAllOnRec();
    end;

    procedure OtherField(): Text
    var
        P: Record A;
    begin
        Seed(1);
        P.ModifyAll(Name, 'x');
        P.Get(1);
        exit(P.Name);
    end;
}
"#;

#[test]
fn modifyall_of_a_measured_audit_field_cannot_change_it() {
    for (routine, field) in [
        ("CreatedAt", "SystemCreatedAt"),
        ("CreatedBy", "SystemCreatedBy"),
        ("ModifiedAt", "SystemModifiedAt"),
    ] {
        assert_eq!(
            text(run_in(SRC, "T", routine)),
            format!("You cannot change the value of the {field} field.\\\\Table: A|DB:Ok")
        );
    }
}

#[test]
fn modifyall_of_an_audit_field_outside_the_measured_shape_refuses() {
    let mut wrong = Vec::new();
    for routine in [
        "ModifiedBy",
        "TwoRows",
        "NoRows",
        "RunTrigger",
        "Temporary",
        "TableCaption",
        "TableWithOnModify",
        "TableWithSubscriber",
        "FilteredOneRow",
        "FilteredTwoRows",
        "FilterInGroup2",
        "OtherFilterGroup",
        "MarkedOnly",
        "Marks",
        "OtherKey",
        "Descending",
        "ChangeCompany",
        "LoadFields",
        "ReadIsolation",
        "FilterOnTheField",
        "BlankValue",
        "StoredValue",
        "TableWithOnInsert",
        "TableWithOnDelete",
        "FieldWithOnValidate",
        "TableWithExtension",
        "SharedTable",
        "OnRecInATableProcedure",
    ] {
        match run_in(SRC, "T", routine) {
            Err(Signal::Unsupported(why)) if why.contains("outside the measured shape") => {}
            other => wrong.push(format!("{routine}: {other:?}")),
        }
    }
    assert!(wrong.is_empty(), "expected a refusal:\n{}", wrong.join("\n"));
}

#[test]
fn modifyall_of_another_field_keeps_writing() {
    assert_eq!(text(run_in(SRC, "T", "OtherField")), "x");
}
