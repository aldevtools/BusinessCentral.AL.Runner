//! `Format(Rec.Field, 0, '<format string>')` of a Decimal field that declares its places (DecimalPlaces or an AutoFormatType)
//! with format strings other than `<Precision,..>` and `<Standard Format,2>` (al-runner issue 5737; corpus
//! types/lt-at-partner-gaps/AtPrecisionStandardFormat2.al:805-1047, tests RO_*, tables 87356 and 88442).

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_sema::workspace::AppSource;
use al_version::{BcVersion, Target};
use al_vm::{Engine, Session};

const SRC: &str = r#"
table 87390 "FS Qs Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Amt; Decimal) { AutoFormatType = 1; }
        field(3; UnitAmt; Decimal) { AutoFormatType = 2; }
        field(4; Custom; Decimal) { AutoFormatType = 10; AutoFormatExpression = '<Precision,1:1><Standard Format,0>'; }
        field(5; Dec5; Decimal) { DecimalPlaces = 0 : 5; }
        field(6; Plain; Decimal) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}
table 87392 "FS Ro Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; DP22; Decimal) { DecimalPlaces = 2 : 2; }
        field(3; DP00; Decimal) { DecimalPlaces = 0 : 0; }
        field(4; DP35; Decimal) { DecimalPlaces = 3 : 5; }
        field(5; DP05; Decimal) { DecimalPlaces = 0 : 5; }
    }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 87391 C
{
    procedure Repro(): Text
    var
        Row: Record "FS Qs Row";
        Actual: Text;
    begin
        Row.Dec5 := 5;
        Actual := Format(Row.Dec5, 0, '<Integer>');
        Row.Dec5 := 1234;
        Actual += '|' + Format(Row.Dec5, 0, '<Integer>');
        exit(Actual);
    end;

    local procedure Values(var Values: List of [Decimal])
    begin
        Values.Add(5);
        Values.Add(1234);
        Values.Add(1234.5678);
        Values.Add(-0.5);
        Values.Add(0);
        Values.Add(-1234567.25);
    end;

    procedure Qs(Spec: Text): Text
    var
        Row: Record "FS Qs Row";
        Vs: List of [Decimal];
        D: Decimal;
        Line: Text;
    begin
        Values(Vs);
        foreach D in Vs do begin
            Row.Amt := D;
            Row.UnitAmt := D;
            Row.Custom := D;
            Row.Dec5 := D;
            Row.Plain := D;
            if Line <> '' then
                Line += ' ; ';
            Line += Format(Row.Amt, 0, Spec) + '|' + Format(Row.UnitAmt, 0, Spec) + '|' + Format(Row.Custom, 0, Spec) + '|' + Format(Row.Dec5, 0, Spec) + '|' + Format(Row.Plain, 0, Spec);
        end;
        exit(Line);
    end;

    procedure Ro(Spec: Text): Text
    var
        Row: Record "FS Ro Row";
        Vs: List of [Decimal];
        D: Decimal;
        Line: Text;
    begin
        Values(Vs);
        foreach D in Vs do begin
            Row.DP22 := D;
            Row.DP00 := D;
            Row.DP35 := D;
            Row.DP05 := D;
            if Line <> '' then
                Line += ' ; ';
            Line += Format(Row.DP22, 0, Spec) + '|' + Format(Row.DP00, 0, Spec) + '|' + Format(Row.DP35, 0, Spec) + '|' + Format(Row.DP05, 0, Spec);
        end;
        exit(Line);
    end;

    procedure RoDe(Spec: Text): Text
    var
        Orig: Integer;
        R: Text;
    begin
        Orig := GlobalLanguage();
        GlobalLanguage(1031);
        R := Ro(Spec);
        GlobalLanguage(Orig);
        exit(R);
    end;

    procedure Unpinned(Which: Integer): Text
    var
        Row: Record "FS Ro Row";
    begin
        Row.DP22 := 5;
        case Which of
            1:
                exit(Format(Row.DP22, 0, '<Integer,8>'));
            2:
                begin
                    Row.DP22 := -0.001;
                    exit(Format(Row.DP22, 0, '<Integer>'));
                end;
        end;
    end;
}
"#;

fn call(bc: &str, routine: &str, args: Vec<Value>) -> Result<Value, Signal> {
    let target = Target::for_bc(BcVersion::parse(bc).unwrap());
    let apps = vec![AppSource {
        id: "a".into(),
        name: "a".into(),
        dependencies: vec![],
        files: vec![("f.al".into(), SRC.into())],
    }];
    let (ws, l) = al_sema::lower::compile_for(apps, &Default::default(), &target);
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked: Vec<Vec<Option<String>>> = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == routine)
        .unwrap();
    let engine = Engine::with_target(l.program, blocked, Some(target));
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    s.call(oi, ri, inst, args).map(|(v, _)| v)
}

fn fmt(bc: &str, routine: &str, spec: &str) -> String {
    match call(bc, routine, vec![Value::Text(spec.into())]) {
        Ok(Value::Text(t)) => t.to_string(),
        other => panic!("{routine} {spec}: expected a text, got {other:?}"),
    }
}

/// AtPrecisionStandardFormat2.al:805-820 (RO_IntegerOfDecimalPlacesField_Repro), the reporter's shape.
#[test]
fn integer_of_a_decimal_places_field() {
    assert!(matches!(call("29.0", "Repro", vec![]), Ok(Value::Text(t)) if t.as_str() == "5|1234"));
}

/// AtPrecisionStandardFormat2.al:883-967 (RO_FieldFormatStrings_AutoFormatFields_EnUs_1 to _4), BC 28 and later.
#[test]
fn format_strings_on_auto_format_fields() {
    let cases = [
        (
            "<Integer>",
            "5|5|5|5|5 ; 1234|1234|1234|1234|1234 ; 1234|1234|1234|1234|1234 ; 0|0|0|0|0 ; 0|0|0|0|0 ; 1234567|1234567|1234567|1234567|1234567",
        ),
        (
            "<Integer Thousand>",
            "5|5|5|5|5 ; 1,234|1,234|1,234|1,234|1,234 ; 1,234|1,234|1,234|1,234|1,234 ; 0|0|0|0|0 ; 0|0|0|0|0 ; 1,234,567|1,234,567|1,234,567|1,234,567|1,234,567",
        ),
        (
            "<Decimals>",
            ".00|.00|.0|| ; .00|.00|.0|| ; .57|.5678|.6|.5678|.5678 ; .50|.50|.5|.5|.5 ; .00|.00|.0|| ; .25|.25|.3|.25|.25",
        ),
        (
            "<Integer><Decimals>",
            "5.00|5.00|5.0|5|5 ; 1234.00|1234.00|1234.0|1234|1234 ; 1234.57|1234.5678|1234.6|1234.5678|1234.5678 ; 0.50|0.50|0.5|0.5|0.5 ; 0.00|0.00|0.0|0|0 ; 1234567.25|1234567.25|1234567.3|1234567.25|1234567.25",
        ),
        (
            "<Sign><Integer Thousand><Decimals>",
            "5.00|5.00|5.0|5|5 ; 1,234.00|1,234.00|1,234.0|1,234|1,234 ; 1,234.57|1,234.5678|1,234.6|1,234.5678|1,234.5678 ; -0.50|-0.50|-0.5|-0.5|-0.5 ; 0.00|0.00|0.0|0|0 ; -1,234,567.25|-1,234,567.25|-1,234,567.3|-1,234,567.25|-1,234,567.25",
        ),
        (
            "<Standard Format,0>",
            "5.00|5.00|5.0|5|5 ; 1,234.00|1,234.00|1,234.0|1,234|1,234 ; 1,234.57|1,234.5678|1,234.6|1,234.5678|1,234.5678 ; -0.50|-0.50|-0.5|-0.5|-0.5 ; 0.00|0.00|0.0|0|0 ; -1,234,567.25|-1,234,567.25|-1,234,567.3|-1,234,567.25|-1,234,567.25",
        ),
        (
            "<Standard Format,1>",
            "5.00|5.00|5.0|5|5 ; 1234.00|1234.00|1234.0|1234|1234 ; 1234.57|1234.5678|1234.6|1234.5678|1234.5678 ; -0.50|-0.50|-0.5|-0.5|-0.5 ; 0.00|0.00|0.0|0|0 ; -1234567.25|-1234567.25|-1234567.3|-1234567.25|-1234567.25",
        ),
        (
            "<Standard Format,9>",
            "5.00|5.00|5.0|5|5 ; 1234.00|1234.00|1234.0|1234|1234 ; 1234.57|1234.5678|1234.6|1234.5678|1234.5678 ; -0.50|-0.50|-0.5|-0.5|-0.5 ; 0.00|0.00|0.0|0|0 ; -1234567.25|-1234567.25|-1234567.3|-1234567.25|-1234567.25",
        ),
    ];
    for (spec, want) in cases {
        assert_eq!(fmt("29.0", "Qs", spec), want, "{spec}");
        assert_eq!(fmt("28.0", "Qs", spec), want, "{spec}");
    }
}

/// The same tests, `#else` side: BC 27 does not round the amount field (AutoFormatType 1) to 2 places.
#[test]
fn format_strings_on_auto_format_fields_bc_27() {
    let cases = [
        (
            "<Integer>",
            "5|5|5|5|5 ; 1234|1234|1234|1234|1234 ; 1234|1234|1234|1234|1234 ; 0|0|0|0|0 ; 0|0|0|0|0 ; 1234567|1234567|1234567|1234567|1234567",
        ),
        (
            "<Decimals>",
            "|.00|.0|| ; |.00|.0|| ; .5678|.5678|.6|.5678|.5678 ; .5|.50|.5|.5|.5 ; |.00|.0|| ; .25|.25|.3|.25|.25",
        ),
        (
            "<Integer><Decimals>",
            "5|5.00|5.0|5|5 ; 1234|1234.00|1234.0|1234|1234 ; 1234.5678|1234.5678|1234.6|1234.5678|1234.5678 ; 0.5|0.50|0.5|0.5|0.5 ; 0|0.00|0.0|0|0 ; 1234567.25|1234567.25|1234567.3|1234567.25|1234567.25",
        ),
        (
            "<Standard Format,0>",
            "5|5.00|5.0|5|5 ; 1,234|1,234.00|1,234.0|1,234|1,234 ; 1,234.5678|1,234.5678|1,234.6|1,234.5678|1,234.5678 ; -0.5|-0.50|-0.5|-0.5|-0.5 ; 0|0.00|0.0|0|0 ; -1,234,567.25|-1,234,567.25|-1,234,567.3|-1,234,567.25|-1,234,567.25",
        ),
        (
            "<Standard Format,9>",
            "5|5.00|5.0|5|5 ; 1234|1234.00|1234.0|1234|1234 ; 1234.5678|1234.5678|1234.6|1234.5678|1234.5678 ; -0.5|-0.50|-0.5|-0.5|-0.5 ; 0|0.00|0.0|0|0 ; -1234567.25|-1234567.25|-1234567.3|-1234567.25|-1234567.25",
        ),
    ];
    for (spec, want) in cases {
        assert_eq!(fmt("27.0", "Qs", spec), want, "{spec}");
    }
}

/// AtPrecisionStandardFormat2.al:969-1047 (RO_FieldFormatStrings_DecimalPlacesFields_EnUs_1 to _4, _DeDe), every BC version.
#[test]
fn format_strings_on_decimal_places_fields() {
    let cases = [
        (
            "<Integer>",
            "5|5|5|5 ; 1234|1234|1234|1234 ; 1234|1235|1234|1234 ; 0|1|0|0 ; 0|0|0|0 ; 1234567|1234567|1234567|1234567",
        ),
        (
            "<Integer Thousand>",
            "5|5|5|5 ; 1,234|1,234|1,234|1,234 ; 1,234|1,235|1,234|1,234 ; 0|1|0|0 ; 0|0|0|0 ; 1,234,567|1,234,567|1,234,567|1,234,567",
        ),
        (
            "<Decimals>",
            ".00||.000| ; .00||.000| ; .57||.5678|.5678 ; .50||.500|.5 ; .00||.000| ; .25||.250|.25",
        ),
        (
            "<Integer><Decimals>",
            "5.00|5|5.000|5 ; 1234.00|1234|1234.000|1234 ; 1234.57|1235|1234.5678|1234.5678 ; 0.50|1|0.500|0.5 ; 0.00|0|0.000|0 ; 1234567.25|1234567|1234567.250|1234567.25",
        ),
        (
            "<Sign><Integer Thousand><Decimals>",
            "5.00|5|5.000|5 ; 1,234.00|1,234|1,234.000|1,234 ; 1,234.57|1,235|1,234.5678|1,234.5678 ; -0.50|-1|-0.500|-0.5 ; 0.00|0|0.000|0 ; -1,234,567.25|-1,234,567|-1,234,567.250|-1,234,567.25",
        ),
        (
            "<Standard Format,0>",
            "5.00|5|5.000|5 ; 1,234.00|1,234|1,234.000|1,234 ; 1,234.57|1,235|1,234.5678|1,234.5678 ; -0.50|-1|-0.500|-0.5 ; 0.00|0|0.000|0 ; -1,234,567.25|-1,234,567|-1,234,567.250|-1,234,567.25",
        ),
        (
            "<Standard Format,1>",
            "5.00|5|5.000|5 ; 1234.00|1234|1234.000|1234 ; 1234.57|1235|1234.5678|1234.5678 ; -0.50|-1|-0.500|-0.5 ; 0.00|0|0.000|0 ; -1234567.25|-1234567|-1234567.250|-1234567.25",
        ),
        (
            "<Standard Format,9>",
            "5.00|5|5.000|5 ; 1234.00|1234|1234.000|1234 ; 1234.57|1235|1234.5678|1234.5678 ; -0.50|-1|-0.500|-0.5 ; 0.00|0|0.000|0 ; -1234567.25|-1234567|-1234567.250|-1234567.25",
        ),
        (
            "<Integer Thousand><Decimals,3>",
            "5.00|5.00|5.00|5.00 ; 1,234.00|1,234.00|1,234.00|1,234.00 ; 1,234.57|1,235.00|1,234***|1,234*** ; 0.50|1.00|0.50|0.50 ; 0.00|0.00|0.00|0.00 ; 1,234,567.25|1,234,567.00|1,234,567.25|1,234,567.25",
        ),
        (
            "<Integer>.<Decimals>",
            "5..00|5.|5..000|5. ; 1234..00|1234.|1234..000|1234. ; 1234..57|1235.|1234..5678|1234..5678 ; 0..50|1.|0..500|0..5 ; 0..00|0.|0..000|0. ; 1234567..25|1234567.|1234567..250|1234567..25",
        ),
    ];
    for bc in ["27.0", "29.0"] {
        for (spec, want) in cases {
            assert_eq!(fmt(bc, "Ro", spec), want, "{bc} {spec}");
        }
    }
    for spec in ["<Integer Thousand>", "<Integer><Decimals>", "<Standard Format,0>"] {
        let want = cases.iter().find(|c| c.0 == spec).unwrap().1;
        assert_eq!(fmt("29.0", "RoDe", spec), want, "de {spec}");
    }
}

/// A format string RO did not measure, and a negative value that rounds to zero, stay refused.
#[test]
fn unpinned_field_format_strings_are_refused() {
    for which in [1, 2] {
        let r = call("29.0", "Unpinned", vec![Value::Integer(which)]);
        assert!(matches!(r, Err(Signal::Unsupported(_))), "{which}: {r:?}");
    }
}
