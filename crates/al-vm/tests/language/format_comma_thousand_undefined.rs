//! Format strings with `<1000Character,x>` and `<Comma,y>` (al-runner issue 7369) and on the undefined Date (issue 7378):
//! corpus master format-evaluate/FormatCommaMore.al Format_CommaAfterIntegerAndThousandChar_* and P144DateTextWidth.al
//! Format_UndefinedDate_* (corpus PR 1247); Date2DMY of 0D with part 5 (format-matrix/FMDateTime.al:4000, corpus PR 1250).

use al_builtins::value::Value;
use al_testkit::{Built, text};

const SRC: &str = r#"
codeunit 50990 F
{
    [TryFunction]
    local procedure TryFormat(V: Variant; Fmt: Text; var R: Text)
    begin
        R := Format(V, 0, Fmt);
    end;

    local procedure Cell(V: Variant; Fmt: Text; Lang: Integer): Text
    var
        R: Text;
    begin
        GlobalLanguage(Lang);
        ClearLastError();
        if TryFormat(V, Fmt, R) then
            R := 'ok:' + R
        else
            R := 'error:' + GetLastErrorText() + '|' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure Dec(Value: Text; Fmt: Text; Lang: Integer): Text
    var
        D: Decimal;
    begin
        Evaluate(D, Value, 9);
        exit(Cell(D, Fmt, Lang));
    end;

    procedure Int(Value: Integer; Fmt: Text; Lang: Integer): Text
    begin
        exit(Cell(Value, Fmt, Lang));
    end;

    procedure Undefined(Fmt: Text; Lang: Integer): Text
    begin
        exit(Cell(0D, Fmt, Lang));
    end;

    [TryFunction]
    local procedure TryDmy(Part: Integer; var R: Integer)
    begin
        R := Date2DMY(0D, Part);
    end;

    procedure Dmy0D(Part: Integer): Text
    var
        R: Integer;
    begin
        if TryDmy(Part, R) then
            exit('ok:' + Format(R));
        exit('err:' + GetLastErrorCode() + ':' + GetLastErrorText());
    end;
}
"#;

const INVALID_EN: &str = "An invalid field or attribute has been specified for the 'Format' property.\\\\";
const INVALID_DE: &str =
    "Ein ung\u{fc}ltiges Feld oder Attribut wurde f\u{fc}r die Eigenschaft 'Format' angegeben.\\\\";

fn call(b: &mut Built, routine: &str, args: Vec<Value>) -> String {
    let (oi, ri) = b.routine_in("F", routine);
    text(b.call(oi, ri, args).map(|(v, _)| v))
}

fn t(s: &str) -> Value {
    Value::Text(s.to_string().into())
}

/// The table of FormatCommaMore.al: 1234567.891, 1234.5, 0.5 and -1234.5 are Decimals, 0 and 1000 Integer literals.
fn table(b: &mut Built, fmt: &str, lang: i32, want: [&str; 6]) {
    let invalid = if lang == 1031 { INVALID_DE } else { INVALID_EN };
    let mut got: Vec<String> = Vec::new();
    for v in ["1234567.891", "1234.5", "0.5", "-1234.5"] {
        got.push(call(b, "Dec", vec![t(v), t(fmt), Value::Integer(lang)]));
    }
    for v in [0, 1000] {
        got.push(call(b, "Int", vec![Value::Integer(v), t(fmt), Value::Integer(lang)]));
    }
    let want: Vec<String> = want
        .iter()
        .map(|w| {
            if *w == "E" {
                format!("error:{invalid}{fmt}|InvalidFormatString")
            } else {
                format!("ok:{w}")
            }
        })
        .collect();
    assert_eq!(got, want, "{fmt} ({lang})");
}

#[test]
fn comma_after_integer_and_thousand_character() {
    let mut b = Built::from_source(SRC);
    for lang in [1033, 1031] {
        let a = "<Sign><Integer><1000Character,.><Decimal,3><Comma,,>";
        table(
            &mut b,
            a,
            lang,
            ["1.234.567***", "1.234,50", "0,50", "-1.234,50", "E", "E"],
        );
        let bb = "<Integer><1000Character,,><Decimal,3><Comma,.>";
        table(
            &mut b,
            bb,
            lang,
            ["1,234,567***", "1,234.50", "0.50", "1,234.50", "E", "E"],
        );
        let c = "<Sign><Integer><1000Character,.><Decimals><Comma,,>";
        table(
            &mut b,
            c,
            lang,
            ["1.234.567,891", "1.234,5", "0,5", "-1.234,5", "E", "E"],
        );
    }
    table(
        &mut b,
        "<Integer><1000Character,.><Comma,,>",
        1033,
        ["1.234.567", "1.234", "0", "1.234", "0", "1.000"],
    );
    table(
        &mut b,
        "<Integer><1000Character,.><Decimals,3><Comma,,>",
        1033,
        ["1.234.567***", "1.234,50", "0,50", "1.234,50", "E", "E"],
    );
    table(
        &mut b,
        "<Integer><Decimal,3><Comma,.>",
        1033,
        ["1234567***", "1234.50", "0.50", "1234.50", "E", "E"],
    );
    table(
        &mut b,
        "<Integer><Decimals,3><Comma,.>",
        1033,
        ["1234567***", "1234.50", "0.50", "1234.50", "E", "E"],
    );
    table(
        &mut b,
        "<Integer Thousand><Decimals><Comma,.>",
        1033,
        ["1,234,567.891", "1,234.5", "0.5", "1,234.5", "E", "E"],
    );
    table(
        &mut b,
        "<Integer><1000Character,.><Decimals,2><Comma,,>",
        1033,
        ["1.234.567**", "1.234,5", "0,5", "1.234,5", "E", "E"],
    );
    table(
        &mut b,
        "<Sign><Integer><1000Character,.>",
        1033,
        ["1.234.567", "1.234", "0", "-1.234", "0", "1.000"],
    );
    table(
        &mut b,
        "<Sign><Integer><Comma,,><Decimals,3>",
        1033,
        ["1234567***", "1234.50", "0.50", "-1234.50", "E", "E"],
    );
}

#[test]
fn the_undefined_date_formats_as_the_empty_text() {
    let mut b = Built::from_source(SRC);
    for fmt in [
        "<Month>/<Day>/<Year4>",
        "<Closing><Day> <Month Text>, <Year4>",
        "<Month Text> <Closing><Day>, <Year4>",
        "<Day>. <Month>. <Year4>",
        "<Day>",
        "<Day,2>",
        "<Month,2>",
        "<Year>",
        "<Year4,4>",
        "<Closing>",
        "<Weekday Text>",
        "<Week>",
        "<Standard Format,0>",
        "<Standard Format,4>",
        "literal",
        "Day: <Day>",
    ] {
        assert_eq!(
            call(&mut b, "Undefined", vec![t(fmt), Value::Integer(1033)]),
            "ok:",
            "{fmt}"
        );
    }
    assert_eq!(
        call(&mut b, "Undefined", vec![t("<Month Text>"), Value::Integer(1031)]),
        "ok:"
    );
    assert_eq!(
        call(&mut b, "Undefined", vec![t("<Day Text>"), Value::Integer(1033)]),
        format!("error:{INVALID_EN}<Day Text>|InvalidFormatString")
    );
}

#[test]
fn date2dmy_of_the_undefined_date_part_5_raises_date_invalid() {
    let mut b = Built::from_source(SRC);
    assert_eq!(
        call(&mut b, "Dmy0D", vec![Value::Integer(5)]),
        "err:DateInvalid:The date is not valid."
    );
}
