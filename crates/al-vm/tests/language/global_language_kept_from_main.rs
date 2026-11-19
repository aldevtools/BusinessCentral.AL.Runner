//! Three answers main gave that corpus PR 1428 (commit 04e1fa0a) measured otherwise: a closing date plus or minus 1, a Danish
//! letter in `Evaluate(DateFormula, text, 9)` under 1030, and the Option filter `-1`. Each measured cell runs through its
//! assumption entry; each neighbor keeps main's answer or refuses.

use al_builtins::assumptions::{Assumption, clear_hits, take_hits};
use al_builtins::native::Signal;
use al_builtins::value::Value;

const SRC: &str = r#"
table 69980 "NM Row"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; Kind; Option) { OptionMembers = Alpha,Beta,Gamma; }
        field(3; Size; Option) { OptionMembers = S,M,L; OptionCaption = 'Small,Medium,Large'; }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 69981 C
{
    [TryFunction]
    local procedure TryOp(Op: Integer; var Txt: Text)
    var
        CD: Date;
        D: Date;
        B: BigInteger;
    begin
        CD := ClosingDate(DMY2Date(31, 12, 2023));
        D := DMY2Date(31, 12, 2023);
        B := 1;
        case Op of
            1: Txt := Format(CD + 1, 0, 9);
            2: Txt := Format(CD - 1, 0, 9);
            3: Txt := Format(CD + 2, 0, 9);
            4: Txt := Format(CD + B, 0, 9);
            5: Txt := Format(D + 1, 0, 9);
            6: Txt := Format(CD + Kind1(), 0, 9);
        end;
    end;

    local procedure Kind1(): Option Alpha,Beta,Gamma
    begin
        exit(1);
    end;

    local procedure Op(Lang: Integer; N: Integer): Text
    var
        Txt: Text;
        Ok: Boolean;
    begin
        GlobalLanguage(Lang);
        Ok := TryOp(N, Txt);
        GlobalLanguage(1033);
        if Ok then
            exit('[' + Txt + ']');
        exit('error:' + GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure PlusOne(): Text begin exit(Op(1033, 1)); end;
    procedure MinusOne(): Text begin exit(Op(1033, 2)); end;
    procedure PlusTwo(): Text begin exit(Op(1033, 3)); end;
    procedure PlusBigOne(): Text begin exit(Op(1033, 4)); end;
    procedure DanishPlusOne(): Text begin exit(Op(1030, 1)); end;
    procedure NormalPlusOne(): Text begin exit(Op(1033, 5)); end;
    procedure PlusOptionOne(): Text begin exit(Op(1033, 6)); end;

    local procedure Eval9(Txt: Text; Number: Integer): Text
    var
        DF: DateFormula;
        Ok: Boolean;
    begin
        Evaluate(DF, '<1Y>');
        GlobalLanguage(1030);
        if Number = 9 then
            Ok := Evaluate(DF, Txt, 9)
        else
            Ok := Evaluate(DF, Txt);
        GlobalLanguage(1033);
        exit(Format(Ok, 0, 9) + '|' + Format(DF, 0, 9));
    end;

    procedure Nine1U(): Text begin exit(Eval9('1U', 9)); end;
    procedure Nine1Aa(): Text begin exit(Eval9('1Å', 9)); end;
    procedure NineLower1u(): Text begin exit(Eval9('1u', 9)); end;
    procedure NineMinus1U(): Text begin exit(Eval9('-1U', 9)); end;
    procedure Nine1D(): Text begin exit(Eval9('1D', 9)); end;
    procedure NoNumber1U(): Text begin exit(Eval9('1U', 0)); end;
    procedure NineStatement(): Text
    var
        DF: DateFormula;
    begin
        GlobalLanguage(1030);
        Evaluate(DF, '1U', 9);
        GlobalLanguage(1033);
        exit(Format(DF, 0, 9));
    end;

    local procedure Seed(var R: Record "NM Row")
    begin
        R.DeleteAll();
        R."Code" := 'A'; R.Kind := R.Kind::Alpha; R.Insert();
        R."Code" := 'B'; R.Kind := R.Kind::Beta; R.Insert();
    end;

    [TryFunction]
    local procedure TryFilter(Shape: Integer; Filter: Text; var N: Integer; var R: Record "NM Row"; var TempR: Record "NM Row" temporary)
    var
        RR: RecordRef;
        FR: FieldRef;
    begin
        case Shape of
            1: begin R.SetFilter(Kind, Filter); N := R.Count(); end;
            2: begin RR.Open(Database::"NM Row"); FR := RR.Field(2); FR.SetFilter(Filter); N := RR.Count(); end;
            3: begin TempR.SetFilter(Kind, Filter); N := TempR.Count(); end;
            4: begin R.FilterGroup(2); R.SetFilter(Kind, Filter); N := R.Count(); end;
            5: begin R.SetFilter(Size, Filter); N := R.Count(); end;
        end;
    end;

    local procedure Filter(Lang: Integer; Shape: Integer; F: Text): Text
    var
        R: Record "NM Row";
        TempR: Record "NM Row" temporary;
        N: Integer;
        Ok: Boolean;
    begin
        Seed(R);
        Seed(TempR);
        GlobalLanguage(Lang);
        Ok := TryFilter(Shape, F, N, R, TempR);
        GlobalLanguage(1033);
        if Ok then
            exit('count=' + Format(N));
        exit('error:' + GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure MinusOneFilter(): Text begin exit(Filter(1033, 1, '-1')); end;
    procedure MinusTwoFilter(): Text begin exit(Filter(1033, 1, '-2')); end;
    procedure DanishMinusOneFilter(): Text begin exit(Filter(1030, 1, '-1')); end;
    procedure FieldRefMinusOneFilter(): Text begin exit(Filter(1033, 2, '-1')); end;
    procedure TemporaryMinusOneFilter(): Text begin exit(Filter(1033, 3, '-1')); end;
    procedure FilterGroupMinusOneFilter(): Text begin exit(Filter(1033, 4, '-1')); end;
    procedure CaptionMinusOneFilter(): Text begin exit(Filter(1033, 5, '-1')); end;
    procedure AlternativeMinusOneFilter(): Text begin exit(Filter(1033, 1, 'Alpha|-1')); end;
    procedure SevenFilter(): Text begin exit(Filter(1033, 1, '7')); end;
}
"#;

/// The result of procedure `r` of codeunit C and the assumptions it ran through.
fn call(r: &str) -> (Result<Value, Signal>, Vec<Assumption>) {
    clear_hits();
    let out = al_testkit::run_in(SRC, "C", r);
    (out, take_hits())
}

/// Procedure `r` gives `text` and runs through `flag` (or through no entry).
fn gives(r: &str, text: &str, flag: Option<Assumption>) {
    let (out, hits) = call(r);
    assert_eq!(al_testkit::text(out), text, "{r}");
    assert_eq!(hits, flag.into_iter().collect::<Vec<_>>(), "{r}");
}

/// Procedure `r` is refused by the runtime.
fn refused(r: &str) {
    let (out, _) = call(r);
    assert!(matches!(out, Err(Signal::Unsupported(_))), "{r}: {out:?}");
}

const CLOSING_ERROR: &str = "error:The operation cannot be performed on a closing date value.|ClosingDateOperation";

#[test]
fn a_closing_date_plus_or_minus_one_raises() {
    gives(
        "PlusOne",
        CLOSING_ERROR,
        Some(Assumption::ClosingDatePlusMinusOneRaises),
    );
    gives(
        "MinusOne",
        CLOSING_ERROR,
        Some(Assumption::ClosingDatePlusMinusOneRaises),
    );
    refused("PlusTwo");
    refused("PlusBigOne");
    refused("DanishPlusOne");
    // An Option value 1 is read as the number 1: not measured.
    refused("PlusOptionOne");
    gives("NormalPlusOne", "[2024-01-01]", None);
}

#[test]
fn a_danish_letter_with_format_number_nine_is_rejected_under_1030() {
    gives(
        "Nine1U",
        "false|1Y",
        Some(Assumption::DateFormulaDanishLetterNumberNine),
    );
    gives(
        "Nine1Aa",
        "false|1Y",
        Some(Assumption::DateFormulaDanishLetterNumberNine),
    );
    refused("NineLower1u");
    refused("NineMinus1U");
    refused("NineStatement");
    gives("Nine1D", "true|1D", None);
    gives("NoNumber1U", "true|1W", None);
}

#[test]
fn the_option_filter_minus_one_raises_the_not_an_option_error() {
    gives(
        "MinusOneFilter",
        "error:The filter \"-1\" is not valid for the Kind field on the NM Row table. '-1' is not an option. The existing options are: Alpha,Beta,Gamma|Filter:CSide",
        Some(Assumption::OptionFilterNegativeOneError),
    );
    for r in [
        "MinusTwoFilter",
        "DanishMinusOneFilter",
        "FieldRefMinusOneFilter",
        "TemporaryMinusOneFilter",
        "FilterGroupMinusOneFilter",
        "CaptionMinusOneFilter",
        "AlternativeMinusOneFilter",
    ] {
        refused(r);
    }
    gives("SevenFilter", "count=0", None);
}
