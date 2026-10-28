//! al-runner issue 6047: a conditional expression with a Label branch against a Code[N], a Text[N], a Label or a text
//! literal branch. The branch before the colon decides the result type: a Code[N] true branch gives a Code (upper case,
//! limited to N), a Label or Text true branch gives a Text. Writer TI's pins (corpus branch al-runtime-rs-staging-ti,
//! types/TestTernaryContracts.al, the TI_Ternary_* tests), registered as the assumption `ternary-label-branch` until the
//! file reaches the pinned corpus. Each pair is read into a Text, a Code[20], a Text[10], a Code[10] parameter, StrLen and
//! Format, once with the condition true and once false, as the corpus tests do.

use al_testkit::{run_in, text};

/// One codeunit with a reader per pair: `P<i>` returns the six readings with the condition true, `#`, and the six with it
/// false (the format of TestTernaryContracts.al: `[value|length]`, `[length]`, `[value]`, or `[E code Llength/max V:value]`).
fn source(pairs: &[(&str, &str, &str)]) -> String {
    let mut s = String::from("codeunit 69950 TL\n{\n");
    for (i, (decls, yes, no)) in pairs.iter().enumerate() {
        s += &format!(
            r#"
    procedure P{i}(): Text
    var R: Text; S: Text; Kind: Integer; T: Boolean; All: Text;
    begin
        for T := true downto false do begin
            S := '';
            for Kind := 1 to 6 do begin
                if Try{i}(T, Kind, R) then
                    S += '[' + R + ']'
                else
                    S += '[E ' + GetLastErrorCode() + ' ' + GetLastErrorText().Replace('The length of the string is ', 'L').Replace(', but it must be less than or equal to ', '/').Replace(' characters. Value: ', ' V:') + ']';
                ClearLastError();
            end;
            if T then All := S + '#' else All += S;
        end;
        exit(All);
    end;
    [TryFunction]
    local procedure Try{i}(Cond: Boolean; Kind: Integer; var R: Text)
    var C20: Code[20]; T10: Text[10]; {decls} RT: Text; RC20: Code[20]; RT10: Text[10];
    begin
        C20 := 'mycode';
        T10 := 'short abc';
        case Kind of
            1: begin RT := Cond ? {yes} : {no}; R := RT + '|' + Format(StrLen(RT)); end;
            2: begin RC20 := Cond ? {yes} : {no}; R := RC20 + '|' + Format(StrLen(RC20)); end;
            3: begin RT10 := Cond ? {yes} : {no}; R := RT10 + '|' + Format(StrLen(RT10)); end;
            4: R := TakeCode10(Cond ? {yes} : {no});
            5: R := Format(StrLen(Cond ? {yes} : {no}));
            6: R := Format(Cond ? {yes} : {no});
        end;
    end;
"#
        );
    }
    s += r#"
    local procedure TakeCode10(C: Code[10]): Text
    begin
        exit(C + '|' + Format(StrLen(C)));
    end;
}
"#;
    s
}

const L7: &str = "Lbl: Label 'abc def';";
const L10: &str = "Lbl: Label 'abcdefghij';";
const L15: &str = "Lbl: Label 'abcde fghij klm';";
const L20: &str = "Lbl: Label 'abcdefghij klmnopqrs';";
const L30: &str = "Lbl: Label 'abcdefghij klmnopqrst uvwxyz12';";
const L7_L30: &str = "Lbl: Label 'abc def'; Lbl2: Label 'abcdefghij klmnopqrst uvwxyz12';";

fn check(cases: &[((&str, &str, &str), &str, &str)]) {
    let pairs: Vec<(&str, &str, &str)> = cases.iter().map(|c| c.0).collect();
    let src = source(&pairs);
    for (i, (pair, yes, no)) in cases.iter().enumerate() {
        assert_eq!(
            text(run_in(&src, "TL", &format!("P{i}"))),
            format!("{yes}#{no}"),
            "{} ? {} : {}",
            pair.0,
            pair.1,
            pair.2
        );
    }
}

const MYCODE: &str = "[MYCODE|6][MYCODE|6][MYCODE|6][MYCODE|6][6][MYCODE]";
const SHORT: &str = "[short abc|9][SHORT ABC|9][short abc|9][SHORT ABC|9][9][short abc]";
const ABC7: &str = "[abc def|7][ABC DEF|7][abc def|7][ABC DEF|7][7][abc def]";
const LONG30: &str = "[abcdefghij klmnopqrst uvwxyz12|30][E StringLengthExceeded L30/20 V:abcdefghij klmnopqrst uvwxyz12][E StringLengthExceeded L30/10 V:abcdefghij klmnopqrst uvwxyz12][E StringLengthExceeded L30/10 V:abcdefghij klmnopqrst uvwxyz12][30][abcdefghij klmnopqrst uvwxyz12]";
const LIT15: &str = "[xyz uvw 1234567|15][XYZ UVW 1234567|15][E StringLengthExceeded L15/10 V:xyz uvw 1234567][E StringLengthExceeded L15/10 V:xyz uvw 1234567][15][xyz uvw 1234567]";

// types/TestTernaryContracts.al TI_Ternary_CodeTrueLabelFalse_1..3 (writer TI's branch).
#[test]
fn a_code20_true_branch_makes_a_label_false_branch_a_code() {
    let e30 = "[E StringLengthExceeded L30/20 V:abcdefghij klmnopqrst uvwxyz12]".repeat(6);
    check(&[
        (
            (L7, "C20", "Lbl"),
            MYCODE,
            "[ABC DEF|7][ABC DEF|7][ABC DEF|7][ABC DEF|7][7][ABC DEF]",
        ),
        (
            (L20, "C20", "Lbl"),
            MYCODE,
            "[ABCDEFGHIJ KLMNOPQRS|20][ABCDEFGHIJ KLMNOPQRS|20][E StringLengthExceeded L20/10 V:ABCDEFGHIJ KLMNOPQRS][E StringLengthExceeded L20/10 V:ABCDEFGHIJ KLMNOPQRS][20][ABCDEFGHIJ KLMNOPQRS]",
        ),
        ((L30, "C20", "Lbl"), MYCODE, &e30),
    ]);
}

// types/TestTernaryContracts.al TI_Ternary_LabelTrueCodeFalse_1..3.
#[test]
fn a_label_true_branch_gives_a_text_against_a_code20_false_branch() {
    check(&[
        ((L7, "Lbl", "C20"), ABC7, MYCODE),
        (
            (L20, "Lbl", "C20"),
            "[abcdefghij klmnopqrs|20][ABCDEFGHIJ KLMNOPQRS|20][E StringLengthExceeded L20/10 V:abcdefghij klmnopqrs][E StringLengthExceeded L20/10 V:abcdefghij klmnopqrs][20][abcdefghij klmnopqrs]",
            MYCODE,
        ),
        ((L30, "Lbl", "C20"), LONG30, MYCODE),
    ]);
}

// types/TestTernaryContracts.al TI_Ternary_TextNLabel_1..4.
#[test]
fn a_text10_true_branch_limits_a_label_false_branch_and_a_label_true_branch_does_not_limit_text10() {
    let e15 = "[E StringLengthExceeded L15/10 V:abcde fghij klm]".repeat(6);
    check(&[
        ((L7, "T10", "Lbl"), SHORT, ABC7),
        (
            (L10, "T10", "Lbl"),
            SHORT,
            "[abcdefghij|10][ABCDEFGHIJ|10][abcdefghij|10][ABCDEFGHIJ|10][10][abcdefghij]",
        ),
        ((L15, "T10", "Lbl"), SHORT, &e15),
        (
            (L15, "Lbl", "T10"),
            "[abcde fghij klm|15][ABCDE FGHIJ KLM|15][E StringLengthExceeded L15/10 V:abcde fghij klm][E StringLengthExceeded L15/10 V:abcde fghij klm][15][abcde fghij klm]",
            SHORT,
        ),
    ]);
}

// types/TestTernaryContracts.al TI_Ternary_LabelLabelAndConst_1..4.
#[test]
fn a_label_against_a_label_or_a_literal_keeps_the_taken_value() {
    check(&[
        ((L7_L30, "Lbl", "Lbl2"), ABC7, LONG30),
        ((L7_L30, "Lbl2", "Lbl"), LONG30, ABC7),
        ((L7, "Lbl", "'xyz uvw 1234567'"), ABC7, LIT15),
        ((L7, "'xyz uvw 1234567'", "Lbl"), LIT15, ABC7),
    ]);
}

/// The pairs no corpus test pins are refused: a TextConst branch (BC does not publish it, types/TestTernaryContracts.al:518-521),
/// a MaxLength label against a Code[N], and an unbounded Text variable against a Label.
#[test]
fn unpinned_label_pairs_are_refused() {
    for (decls, yes, no) in [
        ("Lbl: TextConst ENU = 'abc';", "C20", "Lbl"),
        ("Lbl: Label 'abc', MaxLength = 5;", "C20", "Lbl"),
        ("Lbl: Label 'abc', MaxLength = 5;", "Lbl", "C20"),
        ("Lbl: Label 'abc'; TU: Text;", "TU", "Lbl"),
        ("Lbl: Label 'abc'; TU: Text;", "Lbl", "TU"),
    ] {
        let src = source(&[(decls, yes, no)]);
        match run_in(&src, "TL", "P0") {
            Err(al_builtins::native::Signal::Unsupported(why)) => {
                assert!(
                    why.contains("conditional expression"),
                    "{yes} : {no} with {decls}: {why}"
                )
            }
            other => panic!("{yes} : {no} with {decls}: {other:?}"),
        }
    }
}

/// The new pairs run on the registered assumption; a Label against a literal stays clean (pinned on master, :464-466).
#[test]
fn the_label_pairs_mark_their_assumption() {
    use al_builtins::assumptions::{Assumption, take_hits};
    let marked = |decls, yes, no| {
        let src = source(&[(decls, yes, no)]);
        take_hits();
        text(run_in(&src, "TL", "P0"));
        take_hits().contains(&Assumption::TernaryLabelBranch)
    };
    assert!(marked(L7, "C20", "Lbl"));
    assert!(marked(L7, "Lbl", "T10"));
    assert!(marked(L7, "'xyz'", "Lbl"));
    assert!(!marked(L7, "Lbl", "'xyz'"));
    assert!(!marked(L7, "''", "Lbl"));
}
