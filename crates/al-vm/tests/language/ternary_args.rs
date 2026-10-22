//! Lane T14: the false branch of a ternary takes the type of the true branch, a by-value argument of a TryFunction is
//! converted inside the try, and a Char variable handed to an Integer parameter of a report procedure fails.

use al_builtins::native::Signal;
use al_testkit::{run_in, text};

const TERNARY: &str = r#"
codeunit 69900 TN
{
    procedure IntTrue(): Text
    var Q: Decimal;
    begin
        if TryIntTrue(false, 0.25, Q) then exit('ok|' + Format(Q));
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;
    procedure IntTrueWhole(): Text
    var Q: Decimal;
    begin
        if TryIntTrue(false, 3, Q) then exit('ok|' + Format(Q));
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;
    procedure DecTrue(): Text
    var Q: Decimal;
    begin
        if TryDecTrue(true, 0.25, Q) then exit('ok|' + Format(Q));
        exit(GetLastErrorText());
    end;
    [TryFunction]
    local procedure TryIntTrue(Boo: Boolean; OrigQty: Decimal; var Quantity: Decimal)
    begin
        Quantity := Boo ? 1 : OrigQty;
    end;
    [TryFunction]
    local procedure TryDecTrue(Boo: Boolean; OrigQty: Decimal; var Quantity: Decimal)
    begin
        Quantity := Boo ? OrigQty : 1;
    end;
    procedure CodeTrue(): Text
    var R: Text;
    begin
        if TryCodeTrue(false, R) then exit('ok|' + R);
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;
    procedure TextTrue(): Text
    var R: Text;
    begin
        if TryTextTrue(false, R) then exit('ok|' + R);
        exit(GetLastErrorText());
    end;
    [TryFunction]
    local procedure TryCodeTrue(Cond: Boolean; var Result: Text)
    var C20: Code[20]; LongText: Text;
    begin
        C20 := 'shortcode';
        LongText := 'abcdefghijklmnopqrstuvwxyz1234';
        Result := Cond ? C20 : LongText;
    end;
    [TryFunction]
    local procedure TryTextTrue(Cond: Boolean; var Result: Text)
    var C20: Code[20]; LongText: Text;
    begin
        C20 := 'shortcode';
        LongText := 'abcdefghijklmnopqrstuvwxyz1234';
        Result := Cond ? LongText : C20;
    end;
    procedure VariantFalse(): Text
    var V: Variant;
    begin
        if TryTextTrueVariant(false, V) then exit('ok|' + Format(V));
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;
    procedure VariantTrue(): Text
    var V: Variant;
    begin
        if TryVariantTrue(false, V) then
            if V.IsText() then exit('ok|' + Format(V) + '|Text') else exit('ok|' + Format(V) + '|other');
        exit(GetLastErrorText());
    end;
    local procedure GetVariant(): Variant
    var V: Variant; I: Integer;
    begin
        I := 5;
        V := I;
        exit(V);
    end;
    [TryFunction]
    local procedure TryTextTrueVariant(Cond: Boolean; var V: Variant)
    begin
        V := Cond ? 'Bar' : GetVariant();
    end;
    [TryFunction]
    local procedure TryVariantTrue(Cond: Boolean; var V: Variant)
    begin
        V := Cond ? GetVariant() : 'Bar';
    end;
    procedure LiteralTrue(): Text
    var Cond: Boolean;
    begin
        exit(Cond ? 'big' : 'smaller');
    end;
}
"#;

// corpus types/TestTernaryContracts.al:236-280 (BN_Ternary_IntegerAndDecimalBranches).
#[test]
fn an_integer_true_branch_converts_a_decimal_false_branch_to_integer() {
    assert_eq!(
        text(run_in(TERNARY, "TN", "IntTrue")),
        "Overflow under conversion of Microsoft.Dynamics.Nav.Runtime.Decimal18 value 0.25 to System.Int32.|ConversionCast"
    );
    assert_eq!(text(run_in(TERNARY, "TN", "IntTrueWhole")), "ok|3");
    assert_eq!(text(run_in(TERNARY, "TN", "DecTrue")), "ok|0.25");
}

// corpus types/TestTernaryContracts.al:289-337 (BN_Ternary_Code20TrueBranch).
#[test]
fn a_code20_true_branch_checks_the_length_of_a_text_false_branch() {
    assert_eq!(
        text(run_in(TERNARY, "TN", "CodeTrue")),
        "The length of the string is 30, but it must be less than or equal to 20 characters. Value: abcdefghijklmnopqrstuvwxyz1234|StringLengthExceeded"
    );
    assert_eq!(text(run_in(TERNARY, "TN", "TextTrue")), "ok|SHORTCODE");
}

// corpus types/TestTernaryContracts.al:173-233 (BN_Ternary_VariantBranch).
#[test]
fn a_text_true_branch_converts_a_variant_false_branch_and_a_variant_true_branch_keeps_text() {
    assert_eq!(
        text(run_in(TERNARY, "TN", "VariantFalse")),
        "The expression Microsoft.Dynamics.Nav.Runtime.NavIndirectValue cannot be type-converted to a Microsoft.Dynamics.Nav.Runtime.NavText value.|TypeConversion"
    );
    assert_eq!(text(run_in(TERNARY, "TN", "VariantTrue")), "ok|Bar|Text");
}

// corpus types/TestTernaryContracts.al:13-26: a text literal true branch does not bound the length of the false branch.
#[test]
fn a_text_literal_true_branch_does_not_cut_the_false_branch() {
    assert_eq!(text(run_in(TERNARY, "TN", "LiteralTrue")), "smaller");
}

const TRY_ARG: &str = r#"
codeunit 69901 TA
{
    procedure F(): Text
    var T100: Text[100]; R: Text;
    begin
        T100 := 'abcdefghijklmnopqrstu';
        if TryTakeCode20(T100, R) then exit('ok|' + R);
        exit('error:' + GetLastErrorText() + '|' + GetLastErrorCode());
    end;
    procedure Fits(): Text
    var T100: Text[100]; R: Text;
    begin
        T100 := 'abcdefghijklmnopqrst';
        if TryTakeCode20(T100, R) then exit('ok|' + R);
        exit('error:' + GetLastErrorText());
    end;
    [TryFunction]
    local procedure TryTakeCode20(C: Code[20]; var R: Text)
    begin
        R := C + '|' + Format(StrLen(C));
    end;
}
"#;

// corpus text/lt-text-functions/TextAssign.Codeunit.al:240-281 (LT_BU_TextAssign_LongTextToCode20Parameter): the conversion of a
// by-value argument runs inside the TryFunction, so its error is caught.
#[test]
fn a_by_value_argument_of_a_try_function_is_converted_inside_the_try() {
    assert_eq!(
        text(run_in(TRY_ARG, "TA", "F")),
        "error:The length of the string is 21, but it must be less than or equal to 20 characters. Value: abcdefghijklmnopqrstu|StringLengthExceeded"
    );
    assert_eq!(text(run_in(TRY_ARG, "TA", "Fits")), "ok|ABCDEFGHIJKLMNOPQRST|20");
}

const CHAR_ARG: &str = r#"
table 69902 CRow { fields { field(1; Id; Integer) { } } }
report 69903 CRep
{
    ProcessingOnly = true;
    dataset { dataitem(Row; CRow) { } }
    procedure Bump(A: Integer): Text
    begin
        exit('report integer ' + Format(A));
    end;
}
codeunit 69904 CTarget
{
    procedure Bump(A: Integer): Text
    begin
        exit('codeunit integer ' + Format(A));
    end;
}
codeunit 69905 CA
{
    procedure ReportCall(): Text
    var R: Report CRep; Ch: Char; Result: Text;
    begin
        Ch := 'q';
        Result := '-';
        if TryReport(R, Ch, Result) then exit('ok|' + Result);
        exit(Result + '|' + GetLastErrorText() + '|' + GetLastErrorCode());
    end;
    [TryFunction]
    local procedure TryReport(var R: Report CRep; Ch: Char; var Result: Text)
    begin
        Result := R.Bump(Ch);
    end;
    procedure CodeunitCall(): Text
    var T: Codeunit CTarget; Ch: Char;
    begin
        Ch := 'q';
        exit(T.Bump(Ch));
    end;
    procedure LocalCall(): Text
    var Ch: Char;
    begin
        Ch := 'q';
        exit(LocalBump(Ch));
    end;
    local procedure LocalBump(A: Integer): Text
    begin
        exit('local integer ' + Format(A));
    end;
}
"#;

// corpus report/lt-bv-report-char-arg/BvReportCharArg.al:108-131 (PlainReport_CharVariableToInteger) and :160-170 (the local control).
#[test]
fn a_char_variable_into_an_integer_parameter_of_a_report_fails_with_conversion() {
    assert_eq!(
        text(run_in(CHAR_ARG, "CA", "ReportCall")),
        "-|Unable to convert from System.Char to System.Int32.|Conversion"
    );
    assert_eq!(text(run_in(CHAR_ARG, "CA", "LocalCall")), "local integer 113");
}

// corpus report/lt-bv-report-char-arg/BvReportCharArg.al:140-158 (OtherCodeunit_CharVariableToInteger) was measured on the Linux
// tiers only (a Linux suspect): the call is refused until the Windows check.
#[test]
fn a_char_variable_into_an_integer_parameter_of_another_codeunit_is_refused() {
    let r = run_in(CHAR_ARG, "CA", "CodeunitCall");
    assert!(matches!(&r, Err(Signal::Unsupported(m)) if m.contains("Char")), "{r:?}");
}

const MS_SHAPES: &str = r#"
enum 69906 TE { value(0; Default) { } value(1; Other) { } }
table 69907 TT
{
    fields
    {
        field(1; "Row No."; Code[10]) { }
        field(2; Description; Text[100]) { }
        field(3; Name; Text[30]) { }
        field(4; Kind; Enum TE) { }
    }
}
codeunit 69908 TM
{
    procedure RowNo(): Text
    var R: Record TT;
    begin
        R."Row No." := R."Row No." = '' ? CopyStr('abc', 1, MaxStrLen(R."Row No.")) : R."Row No.";
        exit(R."Row No.");
    end;
    procedure Shorter(): Text
    var R: Record TT; D: Text[250];
    begin
        R.Name := 'short';
        D := R.Description <> '' ? R.Description : R.Name;
        exit(D);
    end;
    procedure EnumExit(): Integer
    var R: Record TT; E: Enum TE;
    begin
        R.Kind := R.Kind::Other;
        E := R.Kind = R.Kind::Other ? R.Kind : TE::Default;
        exit(E.AsInteger());
    end;
}
"#;

// Shapes of Base Application (Acc. Schedule Line, Company Information Mgt.) that must still run: the false branch fits the
// true branch or has its type.
#[test]
fn ternaries_whose_false_branch_fits_still_run() {
    assert_eq!(text(run_in(MS_SHAPES, "TM", "RowNo")), "ABC");
    assert_eq!(text(run_in(MS_SHAPES, "TM", "Shorter")), "short");
    assert_eq!(al_testkit::int(run_in(MS_SHAPES, "TM", "EnumExit")), 1);
}

const SIZED: &str = r#"
codeunit 69901 TS
{
    procedure Second(Cond: Boolean; Len: Integer): Text
    var T80: Text[80]; T100: Text[100];
    begin
        T80 := 'short80';
        T100 := PadStr('', Len, 'x');
        T80 := Cond ? T80 : T100;
        exit('ok|' + Format(StrLen(T80)));
    end;

    procedure BiggerTarget(Len: Integer): Text
    var T80: Text[80]; T100: Text[100]; Big: Text[100];
    begin
        T100 := PadStr('', Len, 'x');
        Big := false ? T80 : T100;
        exit(Format(StrLen(Big)));
    end;
}
"#;

// types/lt-qx-ternary-sized/QxTernarySized.al:142-144, :152-155: `T80 := Cond ? T80 : T100` takes the false branch as it is when
// it fits and fails StringLengthExceeded when it does not.
#[test]
fn a_longer_text_false_branch_into_the_true_branch_type_fits_or_fails_its_length() {
    let run = |cond: bool, len: i32| {
        al_testkit::run_with(
            SIZED,
            "TS",
            "Second",
            vec![
                al_builtins::value::Value::Boolean(cond),
                al_builtins::value::Value::Integer(len),
            ],
        )
        .map(|(v, _)| v)
    };
    assert_eq!(text(run(false, 50)), "ok|50");
    assert_eq!(text(run(true, 90)), "ok|7");
    let e = al_testkit::error_message(run(false, 90));
    assert!(
        e.starts_with("The length of the string is 90, but it must be less than or equal to 80 characters."),
        "{e}"
    );
}

// Into a target longer than the true branch (Base App Acc. Schedule Line, `Description := Description = '' ? GLAccCategory.Description
// : Description`): whether BC converts the false branch to Text[80] is not pinned. A value that fits Text[80] is the same either
// way; a longer one is refused when it is taken.
#[test]
fn a_longer_text_false_branch_into_another_target_passes_what_fits_and_refuses_the_rest() {
    let run = |len: i32| {
        al_testkit::run_with(
            SIZED,
            "TS",
            "BiggerTarget",
            vec![al_builtins::value::Value::Integer(len)],
        )
        .map(|(v, _)| v)
    };
    assert_eq!(text(run(80)), "80");
    let r = run(81);
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}
