//! AsBigInteger, AsByte and AsOption of a JSON string of digits (json/JsonValueGetters.al Sq_DigitText_BigIntegerByteOption): the
//! number, or the conversion error where the number does not fit the type or is no whole number.

fn row(json: &str) -> String {
    let src = format!(
        "codeunit 50101 C {{
        [TryFunction] local procedure B(V: JsonValue; var R: Text) begin R := Format(V.AsBigInteger()); end;
        [TryFunction] local procedure Y(V: JsonValue; var R: Text) var I: Integer; begin I := V.AsByte(); R := Format(I); end;
        [TryFunction] local procedure O(V: JsonValue; var R: Text) begin R := Format(V.AsOption()); end;
        local procedure Show(Ok: Boolean; R: Text): Text begin if Ok then exit('ok:' + R); exit('error:' + GetLastErrorText() + '|' + GetLastErrorCode()); end;
        procedure F(): Text var T: JsonToken; R: Text; A: Text; begin T.ReadFrom('{}'); 
            A := 'big=' + Show(B(T.AsValue(), R), R); A += ' ; byte=' + Show(Y(T.AsValue(), R), R); A += ' ; option=' + Show(O(T.AsValue(), R), R);
            exit(A); end; }}",
        json.replace('\'', "''")
    );
    al_testkit::text(al_testkit::run_in(&src, "C", "F"))
}

fn err(to: &str) -> String {
    format!("error:Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavJsonValue to {to}.|Conversion")
}

#[test]
fn digit_texts_read_as_numbers_where_they_fit() {
    assert_eq!(row("\"42\""), "big=ok:42 ; byte=ok:42 ; option=ok:42");
    assert_eq!(row("\" 42\""), "big=ok:42 ; byte=ok:42 ; option=ok:42");
    assert_eq!(
        row("\"-5\""),
        format!("big=ok:-5 ; byte={} ; option=ok:-5", err("System.Byte"))
    );
    assert_eq!(
        row("\"300\""),
        format!("big=ok:300 ; byte={} ; option=ok:300", err("System.Byte"))
    );
    assert_eq!(
        row("\"3000000000\""),
        format!(
            "big=ok:3000000000 ; byte={} ; option={}",
            err("System.Byte"),
            err("Microsoft.Dynamics.Nav.Runtime.NavOption")
        )
    );
    assert_eq!(
        row("\"4.5\""),
        format!(
            "big={} ; byte={} ; option={}",
            err("Microsoft.Dynamics.Nav.Runtime.NavBigInteger"),
            err("System.Byte"),
            err("Microsoft.Dynamics.Nav.Runtime.NavOption")
        )
    );
}
