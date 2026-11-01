//! `FieldRef.Value()` of a number assigned to a Text or Code *variable*: the number's Format text, checked against the
//! variable's length (misc/lt-y13-triage/Y13FieldRefValueToText.al:96-153, corpus master: Integer, Decimal, BigInteger and
//! the field 0 timestamp into Text, Text[30] and Code[20]; Code[3] raises StringLengthExceeded). An Enum is its caption, and a
//! number passed straight as a Text argument its Format text (text/lt-y49-fieldref-into-text/Y49FieldRefIntoText.al). A Date into a Code variable is its Format text (report/lt-requestpage-savevalues-date-text-setvalue/
//! P147DateToText.al QO_*FieldRef_EachConversion).

const SRC: &str = r#"
enum 50715 NumKind { value(0; A) { } value(1; B) { } }
table 50714 NumRow { fields { field(1; K; Integer) { } field(2; B; BigInteger) { } field(3; D; Date) { } field(4; Dec; Decimal) { } field(5; E; Enum NumKind) { } } keys { key(PK; K) { } } }
codeunit 50767 C {
    procedure IntoCode(): Text
    var RR: RecordRef; C: Code[20];
    begin
        RR.Open(Database::NumRow); RR.Field(1).Value := 1234567;
        C := RR.Field(1).Value();
        exit(C);
    end;
    procedure DecimalIntoText30(): Text
    var RR: RecordRef; T: Text[30];
    begin
        RR.Open(Database::NumRow); RR.Field(4).Value := 12.5;
        T := RR.Field(4).Value();
        exit(T);
    end;
    procedure BigIntoText(): Text
    var RR: RecordRef; T: Text;
    begin
        RR.Open(Database::NumRow); RR.Field(2).Value := 9876543210L;
        T := RR.Field(2).Value();
        exit(T);
    end;
    procedure IntoCode3(): Text
    var RR: RecordRef; C: Code[3];
    begin
        RR.Open(Database::NumRow); RR.Field(1).Value := 1234567;
        C := RR.Field(1).Value();
        exit(C);
    end;
    procedure TimestampIntoNamedReturn() DocNo: Code[20]
    var Row: Record NumRow; RR: RecordRef; FR: FieldRef;
    begin
        Row.K := 1; Row.Insert();
        RR.Open(Database::NumRow); RR.FindFirst();
        FR := RR.Field(0);
        DocNo := FR.Value();
        if DocNo <> Format(FR.Value()) then Error('differs');
    end;
    procedure EnumIntoText(): Text
    var RR: RecordRef; T: Text;
    begin
        RR.Open(Database::NumRow);
        T := RR.Field(5).Value();
        exit(T);
    end;
    local procedure Echo(T: Text): Text begin exit(T); end;
    procedure IntoTextParameter(): Text
    var RR: RecordRef;
    begin
        RR.Open(Database::NumRow); RR.Field(1).Value := 7;
        exit(Echo(RR.Field(1).Value()));
    end;
    procedure DateIntoCode(): Text
    var RR: RecordRef; C: Code[20];
    begin
        RR.Open(Database::NumRow); RR.Field(3).Value := 20260116D;
        C := RR.Field(3).Value();
        exit(C);
    end;
}
"#;

fn run(routine: &str) -> String {
    al_testkit::text(al_testkit::run_in(SRC, "C", routine))
}

#[test]
fn a_number_from_fieldref_value_into_a_text_or_code_variable_is_its_format_text() {
    assert_eq!(run("IntoCode"), "1234567");
    assert_eq!(run("DecimalIntoText30"), "12.5");
    assert_eq!(run("BigIntoText"), "9876543210");
    assert!(!run("TimestampIntoNamedReturn").is_empty());
}

#[test]
fn a_number_from_fieldref_value_longer_than_the_code_variable_raises_string_length_exceeded() {
    let e = al_testkit::error_message(al_testkit::run_in(SRC, "C", "IntoCode3"));
    assert_eq!(
        e,
        "The length of the string is 7, but it must be less than or equal to 3 characters. Value: 1234567"
    );
}

#[test]
fn an_enum_from_fieldref_value_into_a_text_variable_is_its_caption() {
    assert_eq!(run("EnumIntoText"), "A");
}

/// text/lt-y49-fieldref-into-text/Y49FieldRefIntoText.al:290-327 (corpus master).
#[test]
fn a_number_from_fieldref_value_passed_as_a_text_argument_is_its_format_text() {
    assert_eq!(run("IntoTextParameter"), "7");
}

#[test]
fn a_date_from_fieldref_value_into_a_code_variable_is_its_format_text() {
    assert_eq!(run("DateIntoCode"), "01/16/26");
}
