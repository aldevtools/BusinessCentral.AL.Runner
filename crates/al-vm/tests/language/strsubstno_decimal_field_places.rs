//! StrSubstNo, Message and Error `%n` of a Decimal table field that declares its format (DecimalPlaces, AutoFormatType or
//! AutoFormatExpression) substitute `Format(Rec.Field)` (al-runner issue 5791; corpus misc/lt-outside-format-json-list/
//! RwDecimalFieldPlaces.al:83 RW_StrSubstNo_DecimalField_TempRecord, :293 RW_MessageError_DecimalField). The places are lost
//! through any other carrier of the value (:242 RW_StrSubstNo_DecimalField_Carriers, `(Rec.F)` gives `70`). Field shapes the
//! corpus does not measure stay refused when they run.

use al_builtins::native::Signal;
use al_testkit::{run_in, text};

const SRC: &str = r#"
table 50760 R
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Amount; Decimal) { AutoFormatType = 1; }
        field(3; Plain; Decimal) { }
        field(4; Places; Decimal) { DecimalPlaces = 2 : 2; }
        field(5; Amt1; Decimal) { AutoFormatType = 1; AutoFormatExpression = ''; }
        field(6; Amt2; Decimal) { AutoFormatType = 2; AutoFormatExpression = ''; }
        field(7; Amt10; Decimal) { AutoFormatType = 10; AutoFormatExpression = '<Precision,3:3><Standard Format,0>'; }
        field(8; Dp33Amt1; Decimal) { DecimalPlaces = 3 : 3; AutoFormatType = 1; AutoFormatExpression = ''; }
        field(9; Dp00; Decimal) { DecimalPlaces = 0 : 0; }
        field(10; Amt2NoExpr; Decimal) { AutoFormatType = 2; }
        field(11; Dp33Amt2; Decimal) { DecimalPlaces = 3 : 3; AutoFormatType = 2; AutoFormatExpression = ''; }
        field(12; AmtCurrency; Decimal) { AutoFormatType = 1; AutoFormatExpression = 'EUR'; }
    }
    keys { key(PK; Id) { } }
}
codeunit 50761 C
{
    procedure SubstAmount(): Text
    var R: Record R;
    begin
        R.Amount := 70;
        exit(StrSubstNo('%1', R.Amount));
    end;

    procedure SubstPlaces(): Text
    var R: Record R;
    begin
        R.Places := 70;
        exit(StrSubstNo('%1', R.Places));
    end;

    procedure ErrorAmount(): Text
    var R: Record R;
    begin
        R.Amount := 70;
        asserterror Error('%1', R.Amount);
        exit(GetLastErrorText());
    end;

    procedure ErrorAmt2(): Text
    var R: Record R;
    begin
        R.Amt2 := 70;
        asserterror Error('x=%1', R.Amt2);
        exit(GetLastErrorText());
    end;

    procedure SubstTwo(): Text
    var R: Record R;
    begin
        R.Amt1 := -12.5;
        R.Plain := -12.5;
        exit(StrSubstNo('%1|%2 %1', R.Amt1, R.Plain));
    end;

    procedure SubstAmt2(): Text
    var R: Record R;
    begin
        R.Amt2 := 3.14159;
        exit(StrSubstNo('%1', R.Amt2));
    end;

    procedure SubstAmt10(): Text
    var R: Record R;
    begin
        R.Amt10 := 70;
        exit(StrSubstNo('%1', R.Amt10));
    end;

    procedure SubstDp33Amt1(): Text
    var R: Record R;
    begin
        R.Dp33Amt1 := 70;
        exit(StrSubstNo('%1', R.Dp33Amt1) + '/' + Format(R.Dp33Amt1));
    end;

    procedure SubstDp00(): Text
    var R: Record R;
    begin
        R.Dp00 := 1234.5;
        exit(StrSubstNo('%1', R.Dp00));
    end;

    procedure SubstParen(): Text
    var R: Record R;
    begin
        R.Amt1 := 70;
        exit(StrSubstNo('%1', (R.Amt1)) + '/' + StrSubstNo('%1', R.Amt1 + 0) + '/' + StrSubstNo('%1', Format(R.Amt1)));
    end;

    procedure SubstAmt2NoExpr(): Text
    var R: Record R;
    begin
        R.Amt2NoExpr := 70;
        exit(StrSubstNo('%1', R.Amt2NoExpr));
    end;

    procedure SubstDp33Amt2(): Text
    var R: Record R;
    begin
        R.Dp33Amt2 := 70;
        exit(StrSubstNo('%1', R.Dp33Amt2));
    end;

    procedure FormatDp33Amt2(): Text
    var R: Record R;
    begin
        R.Dp33Amt2 := 70;
        exit(Format(R.Dp33Amt2));
    end;

    procedure SubstAmtCurrency(): Text
    var R: Record R;
    begin
        R.AmtCurrency := 70;
        exit(StrSubstNo('%1', R.AmtCurrency));
    end;

    procedure SubstPlain(): Text
    var R: Record R;
    begin
        R.Plain := 70;
        exit(StrSubstNo('%1', R.Plain));
    end;

    procedure SubstNotReached(): Text
    var R: Record R;
    begin
        if R.Id <> 0 then
            exit(StrSubstNo('%1', R.Amt2NoExpr));
        exit('skipped');
    end;
}
"#;

fn refused(routine: &str) {
    let r = run_in(SRC, "C", routine);
    assert!(
        matches!(&r, Err(Signal::Unsupported(m)) if m.contains("not pinned")),
        "{routine}: {r:?}"
    );
}

#[test]
fn a_field_that_declares_its_format_is_substituted_as_format_of_the_field() {
    assert_eq!(text(run_in(SRC, "C", "SubstAmount")), "70.00");
    assert_eq!(text(run_in(SRC, "C", "SubstPlaces")), "70.00");
    assert_eq!(text(run_in(SRC, "C", "ErrorAmount")), "70.00");
    assert_eq!(text(run_in(SRC, "C", "ErrorAmt2")), "x=70.00");
    assert_eq!(text(run_in(SRC, "C", "SubstTwo")), "-12.50|-12.5 -12.50");
    assert_eq!(text(run_in(SRC, "C", "SubstAmt2")), "3.14159");
    assert_eq!(text(run_in(SRC, "C", "SubstAmt10")), "70.000");
    assert_eq!(text(run_in(SRC, "C", "SubstDp00")), "1,235");
}

#[test]
fn an_amount_autoformat_wins_over_decimal_places() {
    assert_eq!(text(run_in(SRC, "C", "SubstDp33Amt1")), "70.00/70.00");
}

#[test]
fn the_places_are_lost_through_an_expression() {
    assert_eq!(text(run_in(SRC, "C", "SubstParen")), "70/70/70.00");
}

#[test]
fn field_shapes_the_corpus_does_not_measure_are_refused() {
    refused("SubstAmt2NoExpr");
    refused("SubstDp33Amt2");
    refused("FormatDp33Amt2");
    refused("SubstAmtCurrency");
}

#[test]
fn a_plain_field_and_code_that_does_not_reach_the_call_still_run() {
    assert_eq!(text(run_in(SRC, "C", "SubstPlain")), "70");
    assert_eq!(text(run_in(SRC, "C", "SubstNotReached")), "skipped");
}
