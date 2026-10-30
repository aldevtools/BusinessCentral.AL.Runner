//! Format(FieldRef, Length, FormatNumber) with a format number other than 0 formats the field's value as
//! Format(FieldRef.Value, Length, FormatNumber) does (corpus errors/lt-ms-refusals-129/P129FormatFieldRef.al).

use al_testkit::text;

const SRC: &str = r#"
enum 69801 K { Extensible = true; value(0; None) { } value(1; "Big Kind") { } }
table 69800 R
{
    fields
    {
        field(1; Id; Integer) { }
        field(3; Dec; Decimal) { }
        field(4; Dur; Duration) { }
        field(5; D; Date) { }
        field(8; Flag; Boolean) { }
        field(9; Opt; Option) { OptionMembers = Zero,One,Two; OptionCaption = 'Nil,Uno,Dos'; }
        field(10; Cd; Code[10]) { }
        field(12; Kind; Enum K) { }
        field(13; Dec22; Decimal) { DecimalPlaces = 2 : 2; }
    }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 69802 C
{
    local procedure F(var RR: RecordRef; No: Integer; Std: Integer): Text
    var FR: FieldRef;
    begin
        FR := RR.Field(No);
        exit(Format(FR, 0, Std) + '|');
    end;

    procedure Filled(): Text
    var Row: Record R; RR: RecordRef; T: Text;
    begin
        Row.Id := 1234;
        Row.Dec := 1234.5;
        Row.Dur := 90061000;
        Row.D := 20240305D;
        Row.Flag := true;
        Row.Opt := Row.Opt::Two;
        Row.Cd := 'abc';
        Row.Kind := Row.Kind::"Big Kind";
        Row.Dec22 := 1234.5;
        RR.GetTable(Row);
        T := F(RR, 1, 1) + F(RR, 1, 9) + F(RR, 3, 1) + F(RR, 3, 2) + F(RR, 3, 9) + F(RR, 4, 1) + F(RR, 4, 2) + F(RR, 4, 9);
        T += F(RR, 5, 1) + F(RR, 5, 9) + F(RR, 8, 2) + F(RR, 8, 9) + F(RR, 9, 1) + F(RR, 9, 2) + F(RR, 9, 9);
        T += F(RR, 10, 9) + F(RR, 12, 1) + F(RR, 12, 2) + F(RR, 12, 9) + F(RR, 13, 1) + F(RR, 13, 9);
        exit(T);
    end;

    local procedure L(var RR: RecordRef; No: Integer; Len: Integer): Text
    var FR: FieldRef;
    begin
        FR := RR.Field(No);
        exit(Format(FR, Len) + '|');
    end;

    procedure WithLength(): Text
    var Row: Record R; RR: RecordRef;
    begin
        Row.Id := 1234;
        Row.Dec := 1234.5;
        Row.D := 20240305D;
        Row.Opt := Row.Opt::Two;
        Row.Cd := 'abc';
        RR.GetTable(Row);
        exit(L(RR, 10, 5) + L(RR, 1, 2) + L(RR, 1, 8) + L(RR, 3, 4) + L(RR, 9, 2) + L(RR, 5, 5));
    end;
}
"#;

#[test]
fn a_field_ref_with_a_format_number_formats_its_value() {
    // P129FormatFieldRef.al:P129_FormatFieldRef_Numbers, _BooleanOptionCodeTextEnum, BT_FormatFieldRef_AgainstFormatValue(_BlobOptionEnum).
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "Filled")),
        "1234|1234|1234.5|1234.5|1234.5|1 day 1 hour 1 minute 1 second|1 0 1 2 1 4 1 6|P1DT1H1M1.0S|\
         03/05/24|2024-03-05|1|true|Dos|2|2|ABC|Big Kind|1|1|1234.5|1234.5|"
    );
}

#[test]
fn a_field_ref_with_a_length_is_cut_or_padded() {
    // P129FormatFieldRef.al:P129_FormatFieldRef_WithLength (:214-237).
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "WithLength")),
        "ABC  |12|    1234|1,23|Do|03/05|"
    );
}
