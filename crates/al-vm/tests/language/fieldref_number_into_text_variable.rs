//! `FieldRef.Value()` of a number assigned to a Text or Code *variable*. Into a record's Text or Code field it is the number's
//! Format text (types/lt-variant-to-text-fields/VariantFieldRefBigIntToText.al), and a Variant *variable* holding a number raises
//! TypeConversion into a Text variable (types/lt-variant/VaTCompare.al:Concat_After_Conversion). The direct assignment of
//! `FieldRef.Value()` into a variable is pinned only for a Date, a Time and a DateTime (Format text, report/
//! lt-requestpage-savevalues-date-text-setvalue/P147DateToText.al QO_*FieldRef_EachConversion); for a number it is refused.

const SRC: &str = r#"
table 50714 NumRow { fields { field(1; K; Integer) { } field(2; B; BigInteger) { } field(3; D; Date) { } } keys { key(PK; K) { } } }
codeunit 50767 C {
    procedure IntoCode(): Text
    var RR: RecordRef; C: Code[20];
    begin
        RR.Open(Database::NumRow); RR.Field(1).Value := 7;
        C := RR.Field(1).Value();
        exit(C);
    end;
    procedure TimestampIntoNamedReturn() DocNo: Code[20]
    var RR: RecordRef; FR: FieldRef;
    begin
        RR.Open(Database::NumRow);
        FR := RR.Field(0);
        DocNo := FR.Value();
    end;
    procedure BigIntoText(): Text
    var RR: RecordRef; T: Text;
    begin
        RR.Open(Database::NumRow);
        T := RR.Field(2).Value();
        exit(T);
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

fn refused(routine: &str) -> String {
    match al_testkit::run_in(SRC, "C", routine) {
        Err(al_builtins::native::Signal::Unsupported(r)) => r,
        other => panic!("{routine}: {other:?}"),
    }
}

#[test]
fn a_number_from_fieldref_value_into_a_text_or_code_variable_is_refused() {
    for routine in ["IntoCode", "TimestampIntoNamedReturn", "BigIntoText"] {
        let r = refused(routine);
        assert!(
            r.contains("FieldRef.Value") && r.contains("not pinned"),
            "{routine}: {r}"
        );
    }
}

#[test]
fn a_date_from_fieldref_value_into_a_code_variable_is_its_format_text() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "DateIntoCode")),
        "01/16/26"
    );
}
