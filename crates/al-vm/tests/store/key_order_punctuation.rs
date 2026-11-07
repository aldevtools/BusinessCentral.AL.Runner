//! The key order of Code values with a hyphen or a dot inside (al-runner issue 6980): a hyphen does not count first, so
//! `TTS.001` comes before `TTS-HD.001`; ranges and comparisons on the Code field follow the same order (record/keys-sorting/
//! KeyPunctuationOrder.al, staging r6980, measured on a normal record).

use al_testkit::text;

const SRC: &str = r#"
table 69930 K
{
    fields { field(1; K; Code[50]) { } }
    keys { key(PK; K) { Clustered = true; } }
}
codeunit 69930 C
{
    local procedure List(var R: Record K): Text
    var Out: Text;
    begin
        if R.FindSet() then
            repeat
                if Out <> '' then Out += '|';
                Out += R.K;
            until R.Next() = 0;
        exit(Out);
    end;

    local procedure Seed(var R: Record K)
    begin
        R.K := 'TTS.001'; R.Insert(); R.K := 'TTS-HD.001'; R.Insert(); R.K := 'TTSA'; R.Insert(); R.K := 'TTS-A'; R.Insert();
        R.K := 'A-B'; R.Insert(); R.K := 'AB'; R.Insert(); R.K := 'A.B'; R.Insert();
    end;

    procedure Run(): Text
    var R: Record K; T: Record K temporary; Out: Text;
    begin
        Seed(T);
        Out := List(T);
        Seed(R);
        Out += ' / ' + List(R);
        R.SetRange(K, 'TTS-A', 'TTS.001');
        Out += ' / ' + List(R);
        R.SetFilter(K, '<%1', 'AB');
        Out += ' / ' + List(R);
        R.SetFilter(K, '>%1', 'TTS.001');
        Out += ' / ' + List(R);
        exit(Out);
    end;
}
"#;

#[test]
fn a_hyphen_does_not_count_first_in_the_key_order() {
    assert_eq!(
        text(al_testkit::run(SRC, "Run")),
        "A.B|AB|A-B|TTS.001|TTSA|TTS-A|TTS-HD.001 / A.B|AB|A-B|TTS.001|TTSA|TTS-A|TTS-HD.001 /  / A.B / TTSA|TTS-A|TTS-HD.001"
    );
}

/// A temporary record uses the normal record's order, measured on Windows BC 27.5 and 29.0 only (the corpus CI on Linux sorts
/// otherwise and cannot assert it): a pass that relies on it is flagged with `temporary-key-order-windows`.
mod temporary_order_is_flagged {
    use al_builtins::assumptions::{Assumption, take_hits};

    const SRC: &str = r#"
table 69931 K
{
    fields { field(1; K; Code[50]) { } }
    keys { key(PK; K) { Clustered = true; } }
}
codeunit 69931 C
{
    local procedure Seed(var R: Record K; A: Text; B: Text; C: Text)
    begin
        R.K := A; R.Insert(); R.K := B; R.Insert(); R.K := C; R.Insert();
        R.FindSet();
    end;

    procedure TempPunctuation()
    var T: Record K temporary;
    begin
        Seed(T, 'TTS.001', 'TTS-HD.001', 'TTSA');
    end;

    procedure TempPlain()
    var T: Record K temporary;
    begin
        Seed(T, 'TTS001', 'TTSHD001', 'TTSA');
    end;

    procedure NormalPunctuation()
    var R: Record K;
    begin
        Seed(R, 'TTS.001', 'TTS-HD.001', 'TTSA');
    end;
}
"#;

    fn flagged(routine: &str) -> bool {
        take_hits();
        let _ = al_testkit::run(SRC, routine);
        take_hits().contains(&Assumption::TemporaryKeyOrderWindows)
    }

    #[test]
    fn a_temporary_key_with_punctuation_is_flagged() {
        assert!(flagged("TempPunctuation"));
    }

    #[test]
    fn a_temporary_key_without_punctuation_is_not_flagged() {
        assert!(!flagged("TempPlain"));
    }

    #[test]
    fn a_normal_key_with_punctuation_is_not_flagged() {
        assert!(!flagged("NormalPunctuation"));
    }
}
