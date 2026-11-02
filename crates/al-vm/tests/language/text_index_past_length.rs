//! `Text[i]` past the end of the text reads the character 0, whatever the position
//! (text/lt-text-instance/Index1.Codeunit.al Index_Read_Variable: `T[7]` and `T[100]` of a 6-character text, `E[1]` of an empty
//! one); a position of 0 or below raises.

use al_testkit::{int, run_in, text};

const SRC: &str = r#"
codeunit 50995 C
{
    procedure Far(): Integer
    var T: Text; Wide: Text; C: Char;
    begin
        T := 'abcdef';
        C := T[100];
        exit(C);
    end;

    procedure FarWide(): Integer
    var Wide: Text; C: Char;
    begin
        Wide := 'ä€x';
        C := Wide[50];
        exit(C);
    end;

    procedure Literal(Which: Integer): Text
    var C: Char;
    begin
        case Which of
            1: asserterror C := 'hello'[6];
            2: asserterror C := 'hello'[0];
            3: asserterror C := ''[1];
        end;
        exit(GetLastErrorCode() + '|' + GetLastErrorText());
    end;

    procedure LiteralOk(): Integer
    var C: Char;
    begin
        C := 'hello'[5];
        exit(C);
    end;

    procedure HalfPair(): Integer
    var T: Text; P: Text; C: Char;
    begin
        T := '😀a';
        P := CopyStr(T, 1, 1);
        C := P[1];
        exit(C);
    end;

    procedure WholePair(): Integer
    var T: Text;
    begin
        T := '😀a';
        exit(StrLen(CopyStr(T, 1, 2)));
    end;

    procedure BlankLastOfCode(): Text
    var C: Code[10];
    begin
        C := 'abc';
        C[3] := ' ';
        exit('[' + C + ']' + Format(StrLen(C)));
    end;

    procedure Empty(): Integer
    var E: Text; C: Char;
    begin
        C := E[1];
        exit(C);
    end;
}
"#;

#[test]
fn a_position_past_the_end_reads_char_zero() {
    assert_eq!(int(run_in(SRC, "C", "Far")), 0);
    assert_eq!(int(run_in(SRC, "C", "FarWide")), 0);
    assert_eq!(int(run_in(SRC, "C", "Empty")), 0);
}

/// Index_Read_Literal: a position outside a text literal (`'hello'[6]`, `'hello'[0]`, `''[1]`) raises the .NET error.
#[test]
fn a_position_outside_a_text_literal_raises() {
    let src = SRC.replacen(
        "codeunit 50995 C\n{",
        "codeunit 50995 C\n{\n    procedure L1(): Text begin exit(Literal(1)); end;\n    procedure L2(): Text begin exit(Literal(2)); end;\n    procedure L3(): Text begin exit(Literal(3)); end;",
        1,
    );
    for r in ["L1", "L2", "L3"] {
        assert_eq!(
            text(run_in(&src, "C", r)),
            "AL:IndexOutOfRange|Index was outside the bounds of the array.",
            "{r}"
        );
    }
    assert_eq!(int(run_in(&src, "C", "LiteralOk")), 111);
}

/// LT_BU_Index_SurrogatePairs: half of a surrogate pair cannot be held, so the split is refused instead of read as U+FFFD; a whole
/// pair is fine. LT_BU_Index_Write_Code `code blank last`: a blank written at the end of a Code is cut off.
#[test]
fn half_a_surrogate_pair_is_refused_and_a_trailing_blank_of_a_code_is_cut() {
    let r = run_in(SRC, "C", "HalfPair");
    assert!(matches!(r, Err(al_builtins::native::Signal::Unsupported(_))), "{r:?}");
    assert_eq!(int(run_in(SRC, "C", "WholePair")), 2);
    assert_eq!(text(run_in(SRC, "C", "BlankLastOfCode")), "[AB]2");
}
