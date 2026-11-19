//! `ModifyAll` of an audit field (record/system-fields/AssignedAuditFields.al:594-631, ModifyAll_CreatedAt): over one row of a
//! table without a Caption, with no RunTrigger argument, SystemCreatedAt, SystemCreatedBy and SystemModifiedAt raise the
//! "cannot change" error. Every other shape of the four audit fields is not measured and refuses; another field keeps main's
//! ModifyAll.

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
    for routine in [
        "ModifiedBy",
        "TwoRows",
        "NoRows",
        "RunTrigger",
        "Temporary",
        "TableCaption",
    ] {
        match run_in(SRC, "T", routine) {
            Err(Signal::Unsupported(why)) => assert!(why.contains("outside the measured shape"), "{routine}: {why}"),
            other => panic!("{routine}: expected a refusal, got {other:?}"),
        }
    }
}

#[test]
fn modifyall_of_another_field_keeps_writing() {
    assert_eq!(text(run_in(SRC, "T", "OtherField")), "x");
}
