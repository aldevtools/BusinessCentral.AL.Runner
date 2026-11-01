//! An element of `in [...]` or a case label of another type than the selector converts to the selector's type before the
//! comparison (corpus types/lt-y31-conversion-matrix/Y31ConvCmp.al, LT Y31 Cmp: `Integer_Decimal`, `Code10_Text`,
//! `Text10_Text`).

use al_testkit::{run_in, text};

const SRC: &str = r#"
codeunit 69962 IC
{
    procedure IntInDecimal(): Text
    var I: Integer; D: Decimal; R: Boolean;
    begin
        I := 1;
        D := 1.5;
        if TryIn(I, D, R) then exit(Format(R));
        exit(GetLastErrorCode() + '|' + GetLastErrorText());
    end;
    [TryFunction]
    local procedure TryIn(I: Integer; D: Decimal; var R: Boolean)
    begin
        R := I in [D];
    end;
    procedure CodeInText(): Text
    var C: Code[10]; T: Text;
    begin
        C := 'ABC';
        T := 'abc';
        exit(Format(C in [T]));
    end;
    procedure CaseCodeChar(): Text
    var C: Code[10]; Ch: Char;
    begin
        C := 'A';
        Ch := 'a';
        case C of
            Ch:
                exit('matched');
        end;
        exit('no');
    end;
    procedure Text10InLong(): Text
    var T: Text[10]; L: Text; R: Boolean;
    begin
        T := 'abc';
        L := '5c4e1f0a-9b3d-4e2f-8a1b-0c2d3e4f5a6b';
        if TryLong(T, L, R) then exit(Format(R));
        exit(GetLastErrorCode());
    end;
    [TryFunction]
    local procedure TryLong(T: Text[10]; L: Text; var R: Boolean)
    begin
        R := T in [L];
    end;
}
"#;

#[test]
fn the_element_converts_to_the_selector_type() {
    assert_eq!(
        text(run_in(SRC, "IC", "IntInDecimal")),
        "ConversionCast|Overflow under conversion of Microsoft.Dynamics.Nav.Runtime.Decimal18 value 1.5 to System.Int32."
    );
    assert_eq!(text(run_in(SRC, "IC", "CodeInText")), "Yes");
    assert_eq!(text(run_in(SRC, "IC", "CaseCodeChar")), "matched");
    assert_eq!(text(run_in(SRC, "IC", "Text10InLong")), "StringLengthExceeded");
}

// LT Y31 Ar Text `Text_Guid`, Code10 `Code10_Guid`: `G += T` leaves the Guid as it was, whatever the text.
#[test]
fn a_text_added_to_a_guid_leaves_it() {
    let src = r#"
codeunit 69964 GA
{
    procedure P(): Text
    var G: Guid; T: Text; C: Code[10];
    begin
        G := '{5C4E1F0A-9B3D-4E2F-8A1B-0C2D3E4F5A6B}';
        T := 'abc';
        C := 'ABC';
        G += T;
        G += C;
        exit(Format(G));
    end;
}
"#;
    assert_eq!(text(run_in(src, "GA", "P")), "{5C4E1F0A-9B3D-4E2F-8A1B-0C2D3E4F5A6B}");
}

// LT Y31 Cv Literals `LitChar1_To_Integer`, `_Decimal`, `_Option`, `_Duration`: a one-character text literal is a Char where a
// number is wanted, 'a' is 97.
#[test]
fn a_one_character_literal_into_a_number_is_its_code() {
    let src = r#"
codeunit 69965 LC
{
    procedure P(): Text
    var I: Integer; D: Decimal; O: Option A,B; Du: Duration;
    begin
        I := 'a';
        D := 'a';
        O := 'a';
        Du := 'a';
        exit(Format(I) + '|' + Format(D) + '|' + Format(O) + '|' + Format(Du, 0, 9));
    end;
}
"#;
    assert_eq!(text(run_in(src, "LC", "P")), "97|97|97|P0DT0H0M0.097S");
}

// LT Y31 Cv Variant `Variant_To_Integer` big: a Variant holding 3000000000L into an Integer raises TypeConversion:Overflow as a
// by-value argument and from `exit` too, not only in `:=`.
#[test]
fn a_variant_beyond_integer_into_an_argument_or_exit_raises_type_conversion() {
    let src = r#"
codeunit 69966 VI
{
    procedure Arg(): Text
    var V: Variant;
    begin
        V := 3000000000L;
        if TryArg(V) then exit('ok');
        exit(GetLastErrorCode() + '|' + GetLastErrorText());
    end;
    [TryFunction]
    local procedure TryArg(V: Variant)
    begin
        Take(V);
    end;
    local procedure Take(I: Integer)
    begin
    end;
    procedure Ret(): Text
    var I: Integer;
    begin
        if TryRet(I) then exit('ok');
        exit(GetLastErrorCode());
    end;
    [TryFunction]
    local procedure TryRet(var I: Integer)
    begin
        I := Give();
    end;
    local procedure Give(): Integer
    var V: Variant;
    begin
        V := 3000000000L;
        exit(V);
    end;
}
"#;
    assert_eq!(
        text(run_in(src, "VI", "Arg")),
        "TypeConversion:Overflow|The expression Variant cannot be type-converted to a Integer value."
    );
    assert_eq!(text(run_in(src, "VI", "Ret")), "TypeConversion:Overflow");
}

// LT Y31 Ar Char `Char_Text`: a Char variable set from a number joins a Text as its character, `C + T` and `T += C`.
#[test]
fn a_char_set_from_a_number_joins_a_text_as_a_character() {
    let src = r#"
codeunit 69967 CT
{
    procedure P(): Text
    var C: Char; T: Text; V: Variant;
    begin
        C := 65;
        T := 'a';
        V := C + T;
        T += C;
        exit(Format(V) + '|' + T);
    end;
}
"#;
    assert_eq!(text(run_in(src, "CT", "P")), "Aa|aA");
}

// LT Y31 Cv Literals `LitChar1_To_Integer` exit: `exit('a')` from an Integer procedure is 97.
#[test]
fn a_one_character_literal_returned_as_a_number_is_its_code() {
    let src = r#"
codeunit 69968 LE
{
    procedure P(): Text
    begin
        exit(Format(Give()));
    end;
    local procedure Give(): Integer
    begin
        exit('a');
    end;
}
"#;
    assert_eq!(text(run_in(src, "LE", "P")), "97");
}

// types/lt-scalars/ScT70061.al:Char_Arithmetic: a one-character literal compared with a Char is a Char ('a' > 'A' is true), while
// a Text variable against a Char compares as texts (LT Y31 Cmp Text `Text_Char`: 'a' < 'A').
#[test]
fn a_char_against_a_literal_compares_as_chars() {
    let src = r#"
codeunit 69969 CL
{
    procedure P(): Text
    var C: Char; T: Text;
    begin
        C := 'a';
        T := 'A';
        exit(Format(C > 'A') + '|' + Format(C < 'b') + '|' + Format(T > C));
    end;
}
"#;
    assert_eq!(text(run_in(src, "CL", "P")), "Yes|Yes|Yes");
}

// LT Y31 Cv Variant `Variant_To_BigInteger_2` exit: a Variant holding true returned as a BigInteger is 1, as with `:=`.
#[test]
fn a_variant_returned_meets_the_variant_cells() {
    let src = r#"
codeunit 69970 VR
{
    procedure P(): Text
    var B: BigInteger;
    begin
        B := Give();
        exit(Format(B));
    end;
    local procedure Give(): BigInteger
    var V: Variant;
    begin
        V := true;
        exit(V);
    end;
}
"#;
    assert_eq!(text(run_in(src, "VR", "P")), "1");
}

// LT Y31 Cv Variant `Variant_To_Text10`: a Variant holding the Byte 65 into a Text is 'A'; holding a Duration it raises.
#[test]
fn a_variant_byte_into_a_text_is_its_character() {
    let src = r#"
codeunit 69971 VB
{
    procedure P(): Text
    var B: Byte; V: Variant; T: Text[10];
    begin
        B := 65;
        V := B;
        T := V;
        exit(T);
    end;
    procedure D(): Text
    var Du: Duration; V: Variant; T: Text[10];
    begin
        Du := 65;
        V := Du;
        if TryD(V, T) then exit('ok');
        exit(GetLastErrorCode());
    end;
    [TryFunction]
    local procedure TryD(V: Variant; var T: Text[10])
    begin
        T := V;
    end;
}
"#;
    assert_eq!(text(run_in(src, "VB", "P")), "A");
    assert_eq!(text(run_in(src, "VB", "D")), "TypeConversion");
}

// LT Y31 Cv Variant `Variant_To_Text10` arg_byvalue: the Byte 65 a Variant holds goes into a Text parameter as 'A'.
#[test]
fn a_variant_byte_into_a_text_parameter_is_its_character() {
    let src = r#"
codeunit 69972 VP
{
    procedure P(): Text
    var B: Byte; V: Variant;
    begin
        B := 65;
        V := B;
        exit(Take(V));
    end;
    local procedure Take(T: Text[10]): Text
    begin
        exit(T);
    end;
}
"#;
    assert_eq!(text(run_in(src, "VP", "P")), "A");
}
