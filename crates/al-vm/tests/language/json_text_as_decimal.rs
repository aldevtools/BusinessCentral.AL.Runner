//! JsonValue.AsDecimal of a JSON string (al-runner issue 5690; types/lt-aj-probes/AjScalars.al:150-209).

use al_testkit::{run, text};

const SRC: &str = r#"
codeunit 50100 C
{
    procedure Read(): Text
    var Out: Text; T: Text; i: Integer;
    begin
        for i := 1 to 6 do begin
            case i of
                1: T := '100.00';
                2: T := '-3.5';
                3: T := ' 7 ';
                4: T := '';
                5: T := '1e2';
                6: T := 'abc';
            end;
            Out += Try(T) + '|';
        end;
        exit(Out);
    end;
    local procedure Try(T: Text): Text
    var V: JsonValue; D: Decimal;
    begin
        V.SetValue(T);
        if TryDecimal(V, D) then
            exit('ok ' + Format(D));
        exit(GetLastErrorCode());
    end;
    [TryFunction]
    local procedure TryDecimal(V: JsonValue; var D: Decimal)
    begin
        D := V.AsDecimal();
    end;
}
"#;

#[test]
fn json_string_as_decimal() {
    assert_eq!(text(run(SRC, "Read")), "ok 100|ok -3.5|ok 7|Conversion|Conversion|Conversion|");
}
