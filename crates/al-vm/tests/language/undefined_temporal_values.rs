//! Undefined temporal values: `FieldRef.Value()` passed to a procedure overloaded on Integer and Variant takes the Variant
//! overload, `0DT - 0DT` is 0, and a Variant holding 0D, 0T or 0DT assigned to a number. Every literal is what real BC answered
//! in the corpus: types/lt-y62-undefined-date/Y62UndefinedDate.al (codeunit 93263, writer Y62).

use al_testkit::text;

const SRC: &str = r#"
table 50198 U
{
    fields { field(1; Id; Integer) { } field(2; D; Date) { } field(3; Dec; Decimal) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

codeunit 50198 C
{
    local procedure Num(Val: Integer): Text
    begin
        exit('int:' + Format(Val, 0, 9));
    end;

    local procedure Num(V: Variant): Text
    begin
        exit('var:' + Format(V, 0, 9));
    end;

    local procedure NumDec(Val: Decimal): Text
    begin
        exit('dec:' + Format(Val, 0, 9));
    end;

    local procedure NumDec(V: Variant): Text
    begin
        exit('var:' + Format(V, 0, 9));
    end;

    procedure Overloads(): Text
    var
        Row: Record U;
        RecRef: RecordRef;
    begin
        Row.Id := 1;
        Row.Insert();
        RecRef.Open(Database::U);
        RecRef.FindFirst();
        exit(Num(RecRef.Field(2).Value()) + ';' + NumDec(RecRef.Field(2).Value()) + ';' + Num(RecRef.Field(3).Value()) + ';' + NumDec(RecRef.Field(3).Value()));
    end;

    procedure Subtraction(): Text
    var
        Row: Record U;
        Dur: Duration;
        OneHour: Duration;
    begin
        OneHour := 60 * 60 * 1000;
        Dur := 0DT - 0DT;
        exit('[' + Format(Dur) + ']|' + Format(Dur, 0, 9) + ';' + Format(Row.SystemModifiedAt - Row.SystemCreatedAt < OneHour)
            + ';' + Format(0DT - 0DT < 0));
    end;

    [TryFunction]
    local procedure TryInteger(V: Variant; var Txt: Text)
    var
        I: Integer;
    begin
        I := V;
        Txt := Format(I, 0, 9);
    end;

    local procedure IntoInteger(V: Variant): Text
    var
        Txt: Text;
    begin
        if TryInteger(V, Txt) then
            exit(Txt);
        exit('error:' + GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure Variants(): Text
    var
        V: Variant;
        Dec: Decimal;
        Big: BigInteger;
        Dur: Duration;
        Out: Text;
    begin
        V := 0D;
        Dec := V;
        Big := V;
        Dur := V;
        Out := IntoInteger(V) + ' ' + Format(Dec, 0, 9) + ' ' + Format(Big, 0, 9) + ' ' + Format(Dur, 0, 9);
        V := 0T;
        Dur := V;
        Out += ';' + IntoInteger(V) + ' ' + Format(Dur, 0, 9);
        V := 0DT;
        Dec := V;
        Out += ';' + IntoInteger(V) + ' ' + Format(Dec, 0, 9);
        exit(Out);
    end;
}
"#;

/// FieldRefValue_OverloadPick_EmptyAndSet: `FieldRef.Value()` always takes the Variant overload.
#[test]
fn fieldref_value_takes_the_variant_overload() {
    assert_eq!(text(al_testkit::run(SRC, "Overloads")), "var:;var:;var:0;var:0");
}

/// DateTimeSub_BothUndefined: `0DT - 0DT` and the system fields of a record never inserted subtract to 0.
#[test]
fn two_undefined_datetimes_subtract_to_zero() {
    assert_eq!(text(al_testkit::run(SRC, "Subtraction")), "[]|P0DT0H0M0.0S;Yes;No");
}

/// VariantHoldingUndefined_IntoNumbers: 0D and 0T are 0 as numbers and as a Duration; 0DT raises into Integer.
#[test]
fn a_variant_holding_an_undefined_value_converts_to_zero() {
    assert_eq!(
        text(al_testkit::run(SRC, "Variants")),
        "0 0 0 P0DT0H0M0.0S;0 P0DT0H0M0.0S;error:Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDateTime to System.Int32.|Conversion 0"
    );
}
