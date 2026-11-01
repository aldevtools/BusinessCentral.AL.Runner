//! The false branch of a conditional expression whose branches have different types converts to the true branch's type
//! (corpus types/lt-y31-conversion-matrix/Y31ConvCd.al, LT Y31 Cd: `Byte_Char`, `Text10_Text`, `Option_Duration`).

use al_testkit::{run_in, text};

const SRC: &str = r#"
codeunit 69960 TM
{
    procedure ByteChar(): Text
    var B: Byte; C: Char; V: Variant;
    begin
        B := 65;
        C := 97;
        V := false ? B : C;
        exit(Format(V.IsByte()) + '|' + Format(V));
    end;
    procedure Text10Text(): Text
    var T: Text[10]; L: Text; R: Text;
    begin
        T := 'abc';
        L := 'abcdefghijk';
        if TryText(T, L, R) then exit('ok|' + R);
        exit(GetLastErrorCode() + '|' + GetLastErrorText());
    end;
    [TryFunction]
    local procedure TryText(T: Text[10]; L: Text; var R: Text)
    begin
        R := false ? T : L;
    end;
    procedure OptionDuration(): Text
    var O: Option A,B,C; D: Duration; V: Variant;
    begin
        O := 1;
        D := 2;
        V := false ? O : D;
        exit(Format(V.IsOption()) + '|' + Format(V));
    end;
}
"#;

#[test]
fn false_branch_converts_to_the_true_branch_type() {
    assert_eq!(text(run_in(SRC, "TM", "ByteChar")), "Yes|a");
    assert_eq!(
        text(run_in(SRC, "TM", "Text10Text")),
        "StringLengthExceeded|The length of the string is 11, but it must be less than or equal to 10 characters. Value: abcdefghijk"
    );
    assert_eq!(text(run_in(SRC, "TM", "OptionDuration")), "Yes|C");
}

const LABELS: &str = r#"
codeunit 69961 TL
{
    var
        Lbl: Label 'label';
    procedure TextLabel(): Text
    var T: Text;
    begin
        T := 'some text';
        exit(false ? T : Lbl);
    end;
    procedure LabelText(): Text
    var T: Text;
    begin
        T := 'a text longer than the label';
        exit(false ? Lbl : T);
    end;
}
"#;

// LT Y31 Cd Text `Text_Label` and LT Y31 Cd Label `Label_Text`: the value of the branch taken, no length limit.
#[test]
fn text_and_label_branches_give_the_value_taken() {
    assert_eq!(text(run_in(LABELS, "TL", "TextLabel")), "label");
    assert_eq!(text(run_in(LABELS, "TL", "LabelText")), "a text longer than the label");
}

// LT Y31 Cd Label `Label_Code10`: into a Variant the branch taken keeps its own kind, a Label answering no Is... test. That
// kind is not built: refused.
#[test]
fn a_label_branch_into_a_variant_is_refused() {
    let src = r#"
codeunit 69963 TV
{
    var
        Lbl: Label 'abc';
    procedure P(): Text
    var C: Code[10]; V: Variant;
    begin
        C := 'A';
        V := false ? Lbl : C;
        exit(Format(V));
    end;
}
"#;
    match run_in(src, "TV", "P") {
        Err(al_builtins::native::Signal::Unsupported(why)) => assert!(why.contains("into a Variant"), "{why}"),
        other => panic!("{other:?}"),
    }
}
