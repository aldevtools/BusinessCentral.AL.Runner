//! A Variant holding a field's value (or a literal) into a Text or Code field with `FieldRef.Validate(Variant)` or
//! `FieldRef.Value := Variant`: converted as Format does, Code upper-cased; Validate runs OnValidate, Value := does not
//! (recordref/lt-y47-validate-fieldref/Y47ValidateFieldRef.al:415-570, Y57_*).

use al_testkit::text;

const SRC: &str = r#"
enum 69980 "FVT Kind" { Extensible = true; value(0; First) { } value(1; Second) { } }
table 69981 "FVT Row"
{
    fields
    {
        field(1; "No."; Code[10]) { }
        field(2; Txt; Text[20]) { trigger OnValidate() begin Log += 'vTxt=' + Txt + ';'; end; }
        field(3; Cod; Code[10]) { trigger OnValidate() begin Log += 'vCod=' + Cod + ';'; end; }
        field(4; Int; Integer) { }
        field(5; Dec; Decimal) { }
        field(6; Dat; Date) { }
        field(7; Enm; Enum "FVT Kind") { }
        field(8; Opt; Option) { OptionMembers = A,B,C; }
        field(10; Bool; Boolean) { }
        field(12; NumTxt; Text[10]) { }
        field(13; Log; Text[250]) { }
    }
    keys { key(PK; "No.") { } }
}
codeunit 69982 C
{
    local procedure Put(V: Variant; TargetNo: Integer; UseValidate: Boolean): Text
    var
        T: Record "FVT Row";
        RR: RecordRef;
        F: FieldRef;
    begin
        if not T.Get('T') then begin T."No." := 'T'; T.Insert(); end;
        T.Log := ''; T.Modify();
        RR.GetTable(T);
        F := RR.Field(TargetNo);
        if UseValidate then F.Validate(V) else F.Value := V;
        RR.Modify();
        T.Get('T');
        exit('[' + Format(RR.Field(TargetNo).Value) + '] ' + T.Log);
    end;

    local procedure FromField(SourceNo: Integer; TargetNo: Integer; UseValidate: Boolean): Text
    var
        S: Record "FVT Row";
        RR: RecordRef;
        V: Variant;
    begin
        if not S.Get('S') then begin
            S."No." := 'S'; S.Txt := 'Hello'; S.Cod := 'ABC'; S.Int := 42; S.Dec := 12.5; S.Dat := DMY2Date(15, 1, 2024);
            S.Enm := S.Enm::Second; S.Opt := S.Opt::B; S.Bool := true; S.Insert();
        end;
        RR.GetTable(S);
        V := RR.Field(SourceNo).Value;
        exit(Put(V, TargetNo, UseValidate));
    end;

    procedure Validated(): Text
    begin
        exit(FromField(4, 2, true) + '|' + FromField(5, 2, true) + '|' + FromField(6, 2, true) + '|' + FromField(10, 2, true) + '|' +
            FromField(8, 2, true) + '|' + FromField(7, 2, true) + '|' + FromField(3, 2, true) + '|' + FromField(6, 3, true) + '|' +
            FromField(10, 3, true) + '|' + FromField(2, 3, true));
    end;

    procedure Assigned(): Text
    begin
        exit(FromField(6, 2, false) + '|' + FromField(10, 2, false) + '|' + FromField(8, 2, false) + '|' + FromField(5, 3, false));
    end;

    procedure Literals(): Text
    var
        V: Variant;
        D: Decimal;
        Big: BigInteger;
        B: Boolean;
    begin
        D := 1234.5;
        V := D;
        exit(Put(V, 2, true));
    end;

    procedure LiteralsMore(): Text
    var
        V: Variant;
        Big: BigInteger;
        B: Boolean;
    begin
        Big := 3000000000L;
        V := Big;
        exit(Put(V, 2, true) + '|' + LiteralFalse());
    end;

    local procedure LiteralFalse(): Text
    var
        V: Variant;
        B: Boolean;
    begin
        B := false;
        V := B;
        exit(Put(V, 2, true));
    end;

    procedure TooLong()
    var
        V: Variant;
        Big: BigInteger;
    begin
        Big := 12345678901L;
        V := Big;
        Put(V, 12, true);
    end;
}
"#;

#[test]
fn validate_of_a_variant_field_value_into_text_and_code_is_its_format_text() {
    // Y47ValidateFieldRef.al:461-492 (Y57_ValidateVariantValue_IntoText).
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "Validated")),
        "[42] vTxt=42;|[12.5] vTxt=12.5;|[01/15/24] vTxt=01/15/24;|[Yes] vTxt=Yes;|[B] vTxt=B;|[Second] vTxt=Second;|\
         [ABC] vTxt=ABC;|[01/15/24] vCod=01/15/24;|[YES] vCod=YES;|[HELLO] vCod=HELLO;"
    );
}

#[test]
fn value_of_a_variant_field_value_into_text_runs_no_onvalidate() {
    // Y47ValidateFieldRef.al:495-514 (Y57_ValueVariantValue_IntoText).
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "Assigned")),
        "[01/15/24] |[Yes] |[B] |[12.5] "
    );
}

#[test]
fn literal_variants_format_with_thousands_and_a_long_one_raises() {
    // Y47ValidateFieldRef.al:517-569 (Y57_ValidateVariantLiteral_IntoText): `1,234.5`, `3000000000`, `No`, and 11 digits
    // into a Text[10] raise StringLengthExceeded.
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "Literals")),
        "[1,234.5] vTxt=1,234.5;"
    );
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "LiteralsMore")),
        "[3000000000] vTxt=3000000000;|[No] vTxt=No;"
    );
    let err = al_testkit::error_message(al_testkit::run_in(SRC, "C", "TooLong"));
    assert_eq!(
        err,
        "The length of the string is 11, but it must be less than or equal to 10 characters. Value: 12345678901"
    );
}
