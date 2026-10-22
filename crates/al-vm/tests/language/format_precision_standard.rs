//! `Format` of a Decimal with `<Precision,..>` and `<Standard Format,N>` (corpus types/lt-format-precision-standard/
//! FormatPrecStdEn.al, FormatPrecStdDe.al, codeunits 71576 and 71577; types/lt-at-partner-gaps/AtPrecisionStandardFormat2.al
//! :63-660, tests QS_*), and of Decimal fields with an AutoFormatType (AtPrecisionStandardFormat2.al:663-790,
//! QS_AutoFormatFields_EnUs/DeDe), whose amount field rounds from BC 28 only.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_sema::workspace::AppSource;
use al_version::{BcVersion, Target};
use al_vm::{Engine, Session};

const SRC: &str = r#"
table 87390 "FP Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Amt; Decimal) { AutoFormatType = 1; }
        field(3; UnitAmt; Decimal) { AutoFormatType = 2; }
        field(4; Custom; Decimal) { AutoFormatType = 10; AutoFormatExpression = '<Precision,1:1><Standard Format,0>'; }
        field(5; Dec5; Decimal) { DecimalPlaces = 0 : 5; }
    }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 87391 C
{
    [TryFunction]
    local procedure TryFmt(Value: Decimal; Spec: Text; var Result: Text)
    begin
        Result := Format(Value, 0, Spec);
    end;

    local procedure Fmt(Value: Decimal; Spec: Text): Text
    var Result: Text;
    begin
        if TryFmt(Value, Spec, Result) then
            exit('[' + Result + ']');
        exit('err|' + GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    local procedure Std(Value: Decimal): Text
    var N: Integer; T: Text;
    begin
        for N := 0 to 9 do
            T += Fmt(Value, '<Precision,2:2><Standard Format,' + Format(N) + '>');
        exit(T);
    end;

    procedure StdEn(): Text
    begin
        GlobalLanguage(1033);
        exit(Std(1234.5) + '#' + Std(-0.5));
    end;

    procedure StdDe5(): Text
    begin
        GlobalLanguage(1031);
        exit(Fmt(1234.5, '<Precision,2:2><Standard Format,5>') + Fmt(1234.5, '<Precision,2:2><Standard Format,1>'));
    end;

    procedure Others(): Text
    begin
        exit(Fmt(1234.5, '<Precision,0:0><Standard Format,9>') + Fmt(1234.5, '<Precision,2:5><Standard Format,9>')
            + Fmt(1234.5, '<Precision,3><Standard Format,9>') + Fmt(1234.5, '<Precision,0:3><Standard Format,9>')
            + Fmt(1234.5, '<Standard Format,9><Precision,2:2>') + Fmt(-0.5, '<Precision,0:0><Standard Format,9>')
            + Fmt(0.0000001, '<Precision,7:7><Standard Format,9>') + Fmt(0, '<Precision,3:3><Standard Format,9>'));
    end;

    procedure Single(): Text
    begin
        exit(Fmt(10, '<Precision,2><Standard Format,0>') + Fmt(0, '<Precision,2><Standard Format,1>')
            + Fmt(0.1, '<Precision,2><Standard Format,2>') + Fmt(-1, '<Precision,2><Standard Format,9>')
            + Fmt(999999.995, '<Precision,2><Standard Format,0>'));
    end;

    procedure Fields(): Text
    var Row: Record "FP Row";
    begin
        Row.Amt := 1234.5678;
        Row.UnitAmt := 1234.5678;
        Row.Custom := 1234.5678;
        Row.Dec5 := 1234.5678;
        exit(Format(Row.Amt) + '|' + Format(Row.Amt, 0, '<Standard Format,2>') + '|' + Format(Row.Amt, 0, 2) + '|'
            + Format(Row.Amt, 0, 9) + '|' + Format(Row.Amt, 0, '<Precision,2:2><Standard Format,2>') + '|'
            + Format(Row.UnitAmt) + '|' + Format(Row.Custom) + '|' + Format(Row.Custom, 0, '<Standard Format,2>') + '|'
            + Format(Row.Custom, 0, 2) + '|' + Format(Row.Custom, 0, '<Precision,2><Standard Format,2>') + '|'
            + Format(Row.Dec5, 0, 2));
    end;

    procedure NegFields(): Text
    var Row: Record "FP Row";
    begin
        Row.Amt := -0.005;
        Row.Custom := -0.005;
        exit(Format(Row.Amt) + '|' + Format(Row.Custom) + '|' + Format(Row.Custom, 0, 9) + '|' + Format(Row.Custom, 0, '<Standard Format,2>'));
    end;
}
"#;

fn run(bc: &str, routine: &str) -> Result<Value, Signal> {
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
    if let Some(Some(r)) = blocked.get(oi).and_then(|o| o.get(ri)) {
        panic!("{routine} is blocked: {r}");
    }
    let engine = Engine::with_target(l.program, blocked, Some(target));
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    s.call(oi, ri, inst, vec![]).map(|(v, _)| v)
}

fn text(r: Result<Value, Signal>) -> String {
    match r {
        Ok(Value::Text(t)) => t.to_string(),
        other => panic!("expected a text, got {other:?}"),
    }
}

const E: &str = "Standard format number";

#[test]
fn precision_with_each_standard_format() {
    // FormatPrecStdEn.al:43-58 (1234.5) and :62-81 (-0.5).
    let none = |n: u32| format!("err|{E} {n} does not exist for the type 'Decimal'.|InvalidStandardFormat");
    let errs: String = (5..=8).map(none).collect();
    let want = format!(
        "[1,234.50][1234.50][1234.50][1,234.50 ][1234.50 ]{errs}[1234.50]#[-0.50][-0.50][-0.50][0.50-][0.50-]{errs}[-0.50]"
    );
    assert_eq!(text(run("29.0", "StdEn")), want);
    // FormatPrecStdDe.al:52, the text in German under GlobalLanguage 1031; the number reads as in en-US.
    assert_eq!(
        text(run("29.0", "StdDe5")),
        "err|Die Standardformatnummer 5 ist für den Typ 'Decimal' nicht vorhanden.|InvalidStandardFormat[1234.50]"
    );
}

#[test]
fn other_precisions_with_standard_format_9() {
    // FormatPrecStdEn.al:154-191 P175_PrecStd9_OtherPrecisions.
    assert_eq!(
        text(run("29.0", "Others")),
        "[1235][1234.50][1234.500][1234.5][1234.5][-1][0.0000001][0.000]"
    );
}

#[test]
fn a_single_precision_pads_like_n_n() {
    // AtPrecisionStandardFormat2.al:138-... QS_P2S*_EnUs.
    assert_eq!(text(run("29.0", "Single")), "[10.00][0.00][0.10][-1.00][1,000,000.00]");
}

#[test]
fn auto_format_fields_from_bc_28_and_on_bc_27() {
    // AtPrecisionStandardFormat2.al:684-723 QS_AutoFormatFields_EnUs.
    let from28 = "1,234.57|1234.57|1234.57|1234.57|1234.57|1,234.5678|1,234.6|1234.6|1234.6|1234.57|1234.5678";
    assert_eq!(text(run("28.0", "Fields")), from28);
    assert_eq!(text(run("29.0", "Fields")), from28);
    assert_eq!(
        text(run("27.0", "Fields")),
        "1,234.5678|1234.5678|1234.5678|1234.5678|1234.57|1,234.5678|1,234.6|1234.6|1234.6|1234.57|1234.5678"
    );
    assert_eq!(text(run("29.0", "NegFields")), "-0.01|-0.0|-0.0|-0.0");
    assert_eq!(text(run("27.0", "NegFields")), "-0.005|-0.0|-0.0|-0.0");
}
