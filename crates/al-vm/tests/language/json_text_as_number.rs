//! JsonValue getters on a JSON string that is not a plain number (al-runner issue 6676; json/JsonValueGetters.al
//! Y65_NotANumberText_EveryGetter, Y65_DecimalShapedText_AsDecimal, _2 on the corpus branch of writer Y65).

use al_testkit::text;

const SRC: &str = r#"
codeunit 69796 C
{
    local procedure Val(Json: Text) V: JsonValue
    begin
        V.ReadFrom(Json);
    end;

    [TryFunction]
    local procedure TryGet(V: JsonValue; Getter: Text; var Res: Text)
    begin
        case Getter of
            'dec': Res := Format(V.AsDecimal(), 0, 9);
            'int': Res := Format(V.AsInteger());
            'big': Res := Format(V.AsBigInteger());
            'byte': Res := Format(V.AsByte());
            'opt': Res := Format(V.AsOption());
            'char': Res := Format(V.AsChar());
            'date': Res := Format(V.AsDate());
            'dt': Res := Format(V.AsDateTime());
            'time': Res := Format(V.AsTime());
            'dur': Res := Format(V.AsDuration());
            'code': Res := V.AsCode();
        end;
    end;

    local procedure Get(Json: Text; Getter: Text): Text
    var Res: Text; E: Text;
    begin
        if TryGet(Val(Json), Getter, Res) then
            exit(Res);
        E := GetLastErrorText();
        if StrPos(E, 'NavJsonValue to ') > 0 then
            exit('E:' + CopyStr(E, StrPos(E, 'NavJsonValue to ') + 16));
        exit('E:' + CopyStr(E, StrPos(E, 'from ') + 5));
    end;

    procedure NotANumber(): Text
    var G: List of [Text]; S: Text; R: Text;
    begin
        G.AddRange('dec', 'int', 'big', 'byte', 'opt', 'char', 'date', 'dt', 'time', 'dur', 'code');
        foreach S in G do
            R += Get('"n/a"', S) + ';';
        exit(R);
    end;

    procedure Shapes(): Text
    var G: List of [Text]; S: Text; R: Text;
    begin
        G.AddRange('"4,5"', '"1,000.5"', '"+3"', '".5"', '" 4.5 "', '"-0"', '"1e3"', '"1.5.1"', '"NaN"', '"Infinity"', '"0x10"', '"1 000"', '""',
            '"99999999999999999999"');
        foreach S in G do
            R += Get(S, 'dec') + ';';
        exit(R);
    end;
}
"#;

#[test]
fn a_text_that_is_no_number_raises_the_conversion_error_of_every_getter() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "NotANumber")),
        "E:Microsoft.Dynamics.Nav.Runtime.Decimal18.;E:System.Int32.;E:Microsoft.Dynamics.Nav.Runtime.NavBigInteger.;\
         E:System.Byte.;E:Microsoft.Dynamics.Nav.Runtime.NavOption.;E:System.Char.;E:Microsoft.Dynamics.Nav.Runtime.NavDate.;\
         E:Microsoft.Dynamics.Nav.Runtime.NavDateTime.;E:Microsoft.Dynamics.Nav.Runtime.NavTime.;\
         E:Microsoft.Dynamics.Nav.Runtime.NavDuration.;N/A;"
    );
}

#[test]
fn decimal_shaped_texts_read_as_measured() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "Shapes")),
        "45;1000.5;3;0.5;4.5;0;E:Microsoft.Dynamics.Nav.Runtime.Decimal18.;E:Microsoft.Dynamics.Nav.Runtime.Decimal18.;\
         E:Microsoft.Dynamics.Nav.Runtime.Decimal18.;E:Microsoft.Dynamics.Nav.Runtime.Decimal18.;\
         E:Microsoft.Dynamics.Nav.Runtime.Decimal18.;E:Microsoft.Dynamics.Nav.Runtime.Decimal18.;\
         E:Microsoft.Dynamics.Nav.Runtime.Decimal18.;E:-999,999,999,999,999.99 to 999,999,999,999,999.99.;"
    );
}
