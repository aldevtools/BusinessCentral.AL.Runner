//! `Evaluate` of a Boolean under language 1031 (types/lt-wc/WcUnpinnedSingles.al:79-113, al-runner issue 6322 item B): `Ja` is
//! true, `Nein` false, `Yes` and `vielleicht` raise the German evaluate error.

use al_testkit::text;

const SRC: &str = r#"
codeunit 50487 C
{
    [TryFunction] local procedure TryEval(T: Text; var B: Boolean) begin Evaluate(B, T); end;
    procedure Run(): Text
    var B: Boolean; Saved: Integer; Out: Text;
    begin
        Saved := GlobalLanguage();
        GlobalLanguage(1031);
        Out := Format(TryEval('vielleicht', B)) + '|' + GetLastErrorText() + '|' + GetLastErrorCode();
        Out += '/' + Format(TryEval('Ja', B)) + '|' + Format(B);
        Out += '/' + Format(TryEval('Nein', B)) + '|' + Format(B);
        Out += '/' + Format(TryEval('Yes', B)) + '|' + Format(B);
        GlobalLanguage(Saved);
        exit(Out);
    end;
}
"#;

/// WcUnpinnedSingles.al:79-113 WC_Evaluate_Boolean_German.
#[test]
fn german_boolean_words_and_the_german_error() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "Run")),
        "Nein|Der Wert „vielleicht“ kann nicht in Typ Boolean ausgewertet werden.|Evaluate/Ja|Ja/Ja|Nein/Nein|Nein"
    );
}
