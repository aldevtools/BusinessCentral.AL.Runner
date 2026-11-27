//! `FieldRef.Value()` of a Date, Time or DateTime field converted straight into a number (issue 9265). BC 28.4 raises
//! `Unable to convert from ...NavDate to System.Int32.` for `I := FR.Value()` on a Date field; the corpus does not pin it yet,
//! so the conversion is refused instead of giving 0. The same value through a Variant converts as pinned
//! (types/date-time/Y62UndefinedDate.al:292,295: a Variant holding 0D is 0, one holding 20260115D is 1479993).

use al_builtins::native::Signal;

const SRC: &str = r#"
table 50716 TemporalRow { fields { field(1; K; Integer) { } field(2; D; Date) { } field(3; T; Time) { } field(4; Dt; DateTime) { } } keys { key(PK; K) { } } }
codeunit 50768 C {
    procedure EmptyDateIntoInteger(): Integer
    var RR: RecordRef; FR: FieldRef; I: Integer;
    begin
        RR.Open(Database::TemporalRow);
        FR := RR.Field(2);
        I := FR.Value();
        exit(I);
    end;
    procedure SetDateIntoInteger(): Integer
    var RR: RecordRef; I: Integer;
    begin
        RR.Open(Database::TemporalRow); RR.Field(2).Value := 20260115D;
        I := RR.Field(2).Value();
        exit(I);
    end;
    procedure TimeIntoBigInteger(): BigInteger
    var RR: RecordRef; B: BigInteger;
    begin
        RR.Open(Database::TemporalRow); RR.Field(3).Value := 120000T;
        B := RR.Field(3).Value();
        exit(B);
    end;
    procedure DateTimeIntoDecimal(): Decimal
    var RR: RecordRef; Dec: Decimal;
    begin
        RR.Open(Database::TemporalRow);
        Dec := RR.Field(4).Value();
        exit(Dec);
    end;
    procedure EmptyDateThroughVariant(): Integer
    var RR: RecordRef; V: Variant; I: Integer;
    begin
        RR.Open(Database::TemporalRow);
        V := RR.Field(2).Value();
        I := V;
        exit(I);
    end;
    procedure SetDateThroughVariant(): Integer
    var RR: RecordRef; V: Variant; I: Integer;
    begin
        RR.Open(Database::TemporalRow); RR.Field(2).Value := 20260115D;
        V := RR.Field(2).Value();
        I := V;
        exit(I);
    end;
}
"#;

fn refused(routine: &str) -> String {
    match al_testkit::run_in(SRC, "C", routine) {
        Err(Signal::Unsupported(why)) => why.to_string(),
        other => panic!("{routine}: expected a refusal, got {other:?}"),
    }
}

#[test]
fn a_date_time_or_datetime_from_fieldref_value_into_a_number_is_refused() {
    for routine in [
        "EmptyDateIntoInteger",
        "SetDateIntoInteger",
        "TimeIntoBigInteger",
        "DateTimeIntoDecimal",
    ] {
        assert!(refused(routine).contains("issue 9265"), "{routine}");
    }
}

#[test]
fn a_date_from_fieldref_value_through_a_variant_into_an_integer_converts_as_pinned() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "EmptyDateThroughVariant")),
        "0"
    );
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "SetDateThroughVariant")),
        "1479993"
    );
}
