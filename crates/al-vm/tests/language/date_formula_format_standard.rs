//! `Format(DateFormula, 0, N)` for the standard formats 0, 1, 2, 9 and 3 to 8 under the global languages 1033, 1030 and 1031,
//! and Evaluate of the Danish text format 1 prints. Every literal is what real BC answered in the corpus:
//! types/lt-y62-dateformula-format/Y62DateFormulaFormat.al (codeunit 93262, writer Y62).

use al_testkit::text;

const SRC: &str = r#"
codeunit 50197 C
{
    [TryFunction]
    local procedure TryFormat(F: DateFormula; Std: Integer; var Txt: Text)
    begin
        Txt := Format(F, 0, Std);
    end;

    local procedure Show(T: Text; Std: Integer): Text
    var
        F: DateFormula;
        Txt: Text;
    begin
        if T <> '' then
            Evaluate(F, T);
        if TryFormat(F, Std, Txt) then
            exit('[' + Txt + ']');
        exit('error:' + GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    local procedure EvalText(Txt: Text): Text
    var
        G: DateFormula;
    begin
        exit(Format(Evaluate(G, Txt)) + '|' + Format(G, 0, 9));
    end;

    procedure English(): Text
    begin
        exit(Show('<-1W>', 1) + Show('1M+CM', 1) + Show('', 1) + Show('<-1W>', 2) + Show('', 2) + Show('<CM+1D>', 9) + Show('<-1W>', 3));
    end;

    procedure Danish(): Text
    var
        Out: Text;
    begin
        GlobalLanguage(1030);
        Out := Show('<-1W>', 1) + Show('<+CY-24M>', 1) + Show('<CM+1D>', 0) + Show('<-1Y>', 2) + Show('<-1Y>', 9);
        Out += EvalText('-1U') + ';' + EvalText('LM+1D') + ';';
        GlobalLanguage(1033);
        Out += EvalText('-1U') + ';' + EvalText('-1M');
        exit(Out);
    end;

    procedure German(): Text
    var
        Out: Text;
    begin
        GlobalLanguage(1031);
        Out := Show('<-1W>', 1) + Show('<-1Y>', 1) + Show('<CM+1D>', 1) + Show('<CM+1D>', 2);
        GlobalLanguage(1033);
        exit(Out);
    end;
}
"#;

/// DateFormulaFormat_Std1_English, _Std2_English, _Std0And9_English, _OtherStandards_English.
#[test]
fn english_format_one_has_no_brackets_and_format_two_has_them() {
    assert_eq!(
        text(al_testkit::run(SRC, "English")),
        "[-1W][1M+CM][][<-1W>][<>][CM+1D]error:Standard format number 3 does not exist for the type 'Duration'.|InvalidStandardFormat"
    );
}

/// DateFormulaFormat_Std1And2_Danish and _Std1_Danish_EvaluateBack: formats 0 and 1 print the Danish letters, 2 and 9 the English
/// ones; the Danish text evaluates back under 1030 only.
#[test]
fn danish_formats_zero_and_one_print_danish_letters() {
    assert_eq!(
        text(al_testkit::run(SRC, "Danish")),
        "[-1U][+LÅ-24M][LM+1D][<-1Y>][-1Y]Ja|-1W;Ja|CM+1D;No|;Yes|-1M"
    );
}

/// DateFormulaFormat_Std1_German.
#[test]
fn german_format_one_prints_german_letters() {
    assert_eq!(text(al_testkit::run(SRC, "German")), "[-1W][-1J][LM+1T][<CM+1D>]");
}
