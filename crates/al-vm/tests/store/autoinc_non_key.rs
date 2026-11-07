//! An AutoIncrement field outside the primary key (record/TestAutoIncrementEdges.al:552-600).

use al_testkit::{run_in, text};

const SRC: &str = r#"
table 50320 A
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
        field(3; Seq; BigInteger) { AutoIncrement = true; }
    }
    keys { key(PK; Id) { } }
}

codeunit 50321 T
{
    local procedure Seed()
    var
        P: Record A;
    begin
        P.Id := 1; P.Insert();
        P.Init(); P.Id := 2; P.Insert();
    end;

    procedure ChangedValue(): Text
    var
        P: Record A;
    begin
        Seed();
        P.Get(1);
        P.Seq := P.Seq + 1000;
        asserterror P.Modify();
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure OtherField(): Text
    var
        P: Record A;
    begin
        Seed();
        P.Get(1);
        P.Name := 'x';
        P.Modify();
        exit('ok');
    end;

    procedure ModifyAllSeq(): Text
    var
        P: Record A;
    begin
        Seed();
        asserterror P.ModifyAll(Seq, 99999);
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;
}
"#;

#[test]
fn a_modify_cannot_change_a_non_key_autoincrement_field() {
    assert_eq!(
        text(run_in(SRC, "T", "ChangedValue")),
        "You cannot change the value of an AutoIncrement field.\\\\Field: Seq\\Table: A\\\\Identification fields and values:\\Id='1'|DB:ModifyOnIdentityField"
    );
    assert_eq!(text(run_in(SRC, "T", "OtherField")), "ok");
}

#[test]
fn modifyall_cannot_change_a_non_key_autoincrement_field() {
    assert_eq!(
        text(run_in(SRC, "T", "ModifyAllSeq")),
        "You cannot change the value of the Seq field.\\\\Table: A|DB:Ok"
    );
}

const TRUNC: &str = r#"
table 50322 TT { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }

codeunit 50323 T
{
    procedure F(): Text
    var
        Temp: Record TT temporary;
        Ok: Boolean;
    begin
        Temp.Id := 1;
        Temp.Insert();
        Ok := Temp.Truncate();
        exit(Format(Ok) + '|' + GetLastErrorText() + '|' + Format(Temp.Count()));
    end;
}
"#;

// record/crud/TruncateGuards.al:P163_Truncate_Temporary_ReturnValueUsed_BQ.
#[test]
fn truncate_of_a_temporary_record_with_the_result_used_answers_false() {
    assert_eq!(
        text(run_in(TRUNC, "T", "F")),
        "No|Temporary tables does not support truncation.|1"
    );
}
