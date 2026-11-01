//! `FieldRef.Length` of the field types the first probe left out (al-runner issue 6663). Real BC values from corpus writer
//! Y66 (tests/al-language/fieldref/lt-y66-fieldref-length-types/Y66FieldRefLengthTypes.al:61-141, not on corpus master yet).

use al_testkit::{run_in, text};

const SRC: &str = r#"
table 50100 Child { fields { field(1; Id; Integer) { } field(2; Amount; Decimal) { } field(3; Name; Text[40]) { } field(4; Due; Date) { } } keys { key(PK; Id) { } } }
table 50101 Kinds {
    fields {
        field(1; Id; Integer) { }
        field(2; Formula; DateFormula) { }
        field(3; RecId; RecordId) { }
        field(4; Long; Text[2048]) { }
        field(5; LongCode; Code[2048]) { }
        field(6; SumAmount; Decimal) { FieldClass = FlowField; CalcFormula = sum(Child.Amount where(Id = field(Id))); }
        field(7; Cnt; Integer) { FieldClass = FlowField; CalcFormula = count(Child where(Id = field(Id))); }
        field(8; Has; Boolean) { FieldClass = FlowField; CalcFormula = exist(Child where(Id = field(Id))); }
        field(9; LookName; Text[40]) { FieldClass = FlowField; CalcFormula = lookup(Child.Name where(Id = field(Id))); }
        field(10; LookDue; Date) { FieldClass = FlowField; CalcFormula = lookup(Child.Due where(Id = field(Id))); }
        field(11; DateFilter; Date) { FieldClass = FlowFilter; }
        field(12; CodeFilter; Code[20]) { FieldClass = FlowFilter; }
        field(13; Auto; BigInteger) { AutoIncrement = true; }
    }
    keys { key(PK; Id) { } }
}
codeunit 50102 C {
    local procedure L(var RR: RecordRef; No: Integer): Text begin exit(Format(RR.Field(No).Length()) + ';'); end;
    procedure Kinds(): Text
    var RR: RecordRef; I: Integer; Res: Text;
    begin
        RR.Open(Database::Kinds);
        for I := 2 to 13 do Res += L(RR, I);
        exit(Res);
    end;
    procedure SystemFields(): Text
    var RR: RecordRef; Res: Text;
    begin
        RR.Open(Database::Kinds);
        Res := L(RR, 2000000000) + L(RR, 2000000001) + L(RR, 2000000002) + L(RR, 2000000003) + L(RR, 2000000004);
        exit(Res);
    end;
    procedure TempDateFormula(): Text
    var RR: RecordRef;
    begin
        RR.Open(Database::Kinds, true);
        exit(L(RR, 2));
    end;
    procedure Helper(): Text
    var A: RecordRef; B: RecordRef;
    begin
        A.Open(Database::Kinds); B.Open(Database::Kinds);
        exit(Format((A.Field(2).Type() = B.FieldIndex(2).Type()) and (A.Field(2).Length() >= B.FieldIndex(2).Length())));
    end;
}
"#;

#[test]
fn dateformula_recordid_long_texts_flowfields_flowfilters_autoincrement() {
    assert_eq!(text(run_in(SRC, "C", "Kinds")), "32;448;2048;2048;12;4;4;40;4;4;20;8;");
}

#[test]
fn system_fields() {
    assert_eq!(text(run_in(SRC, "C", "SystemFields")), "16;8;16;8;16;");
}

#[test]
fn a_temporary_dateformula_field() {
    assert_eq!(text(run_in(SRC, "C", "TempDateFormula")), "32;");
}

#[test]
fn the_reporters_type_and_length_helper() {
    assert_eq!(text(run_in(SRC, "C", "Helper")), "Yes");
}
