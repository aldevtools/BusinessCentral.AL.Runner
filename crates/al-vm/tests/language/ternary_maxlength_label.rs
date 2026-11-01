//! A conditional expression with a Label that has MaxLength against a Text or Code branch (al-runner issue 6659): the true
//! branch decides the type, as without MaxLength. Real BC values from corpus writer Y66
//! (types/lt-y66-ternary-label/Y66TernaryLabel.al:27-63, :65-140, :293-368, not on corpus master yet).

use al_testkit::{run_in, text};

const SRC: &str = r#"
table 50100 Row {
    fields { field(1; Id; Integer) { } field(2; "My Text Field"; Text[100]) { } field(3; "My Code"; Code[10]) { } }
    keys { key(PK; Id) { } }
    procedure MyProc() Result: Text[100]
    var DefaultLbl: Label 'Default text', MaxLength = 100; BaseText: Text[100];
    begin
        BaseText := (Rec."My Text Field" <> '') ? Rec."My Text Field" : DefaultLbl;
        exit(BaseText);
    end;
}
codeunit 50101 C {
    var DefaultLbl: Label 'Default text', MaxLength = 100;
    procedure Reporter(): Text
    var R: Record Row; Res: Text;
    begin
        Res := R.MyProc() + '|' + Format(StrLen(R.MyProc()));
        R."My Text Field" := 'Field text';
        exit(Res + ';' + R.MyProc());
    end;
    procedure LabelFirst(): Text
    var R: Record Row; T: Text; C: Code[20];
    begin
        R."My Text Field" := 'Field text value';
        T := false ? DefaultLbl : R."My Text Field";
        C := true ? DefaultLbl : R."My Text Field";
        exit(T + '|' + C);
    end;
    [TryFunction]
    local procedure TryCode(var Res: Text)
    var R: Record Row;
    begin
        Res := false ? R."My Code" : DefaultLbl;
    end;
    procedure CodeTooShort(): Text
    var Res: Text;
    begin
        if TryCode(Res) then exit(Res);
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;
}
"#;

#[test]
fn the_reporters_table_procedure() {
    assert_eq!(text(run_in(SRC, "C", "Reporter")), "Default text|12;Field text");
}

#[test]
fn a_maxlength_label_true_branch_takes_the_field() {
    assert_eq!(text(run_in(SRC, "C", "LabelFirst")), "Field text value|DEFAULT TEXT");
}

/// Y66TernaryLabel.al:293-368 `FCode10 : LblMax No`.
#[test]
fn a_code10_true_branch_makes_the_label_too_long() {
    assert_eq!(
        text(run_in(SRC, "C", "CodeTooShort")),
        "The length of the string is 12, but it must be less than or equal to 10 characters. Value: Default text|StringLengthExceeded"
    );
}
