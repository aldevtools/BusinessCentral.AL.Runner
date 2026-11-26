//! A text function whose result would hold half of a surrogate pair refuses (hard rule 4) until ADR-068 audits it.
//! DelStr is audited (`text_lone_half.rs`); the others still refuse.
//! Before, each turned the half into U+FFFD silently. The corpus pins that real BC keeps the half
//! (text/lt-cr-text-leads/CrTextLeads.al:414-439, types/lt-text-char-index/TextCharIndexAssignEdges.al:88-116), so
//! U+FFFD is a guessed value.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::{run_in, run_with, text};

const SRC: &str = r#"
codeunit 50996 C
{
    procedure Cut(Which: Integer): Text
    var
        T: Text; Ab: Text; Hi: Char; L: List of [Char]; Parts: List of [Text];
        TB: TextBuilder; BT: BigText; S: Text;
    begin
        T := '😀';
        case Which of
            3: begin Hi := 55357; L.Add(Hi); Parts := T.Split(L); exit(Parts.Get(2)); end;
            4: begin Ab := 'ba'; exit(ConvertStr(Ab, 'ab', T)); end;
            5: exit(PadStr(T, 1));
            6: exit(DelStr(T, 1, 1));
            7: exit(InsStr(T, 'x', 2));
            8: begin TB.Append(T); exit(TB.ToText(1, 1)); end;
            9: begin TB.Append(T); TB.Length(1); exit(TB.ToText()); end;
            10: begin TB.Append(T + 'x'); TB.Replace('x', 'y', 2, 2); exit(TB.ToText()); end;
            11: begin TB.Append(T); TB.Insert(2, 'x'); exit(TB.ToText()); end;
            12: begin TB.Append(T); TB.Remove(2, 1); exit(TB.ToText()); end;
            13: begin BT.AddText(T); BT.AddText('x', 2); BT.GetSubText(S, 1); exit(S); end;
            14: begin BT.AddText(T); BT.GetSubText(S, 1, 1); exit(S); end;
        end;
    end;

    procedure Whole(): Text
    var T: Text; TB: TextBuilder; BT: BigText; S: Text;
    begin
        T := 'a😀b';
        TB.Append(T);
        BT.AddText(T);
        BT.GetSubText(S, 2, 2);
        exit(T.Substring(2, 2) + DelStr(T, 1, 1) + TB.ToText(2, 2) + S + PadStr(T, 3));
    end;
}
"#;

#[test]
fn a_cut_through_a_pair_refuses() {
    for which in (3..=14).filter(|w| *w != 6) {
        let r = run_with(SRC, "C", "Cut", vec![Value::Integer(which)]).map(|(v, _)| v);
        match &r {
            Err(Signal::Unsupported(why)) => assert!(why.contains("half of a surrogate pair"), "{which}: {why}"),
            other => panic!("{which}: {other:?}"),
        }
    }
}

/// `Text.Substring` and `Text.Remove` still give U+FFFD: the corpus reads only the length
/// (text/text-methods/Index1.Codeunit.al:189, :192), which U+FFFD keeps. ADR-068 replaces this.
#[test]
fn substring_and_remove_keep_the_pinned_length() {
    let src = r#"
codeunit 50997 D
{
    procedure Lengths(): Text
    var T: Text;
    begin
        T := '😀a';
        exit(Format(StrLen(T.Substring(1, 1))) + Format(StrLen(T.Remove(1, 1))));
    end;
}
"#;
    assert_eq!(text(run_in(src, "D", "Lengths")), "12");
}

#[test]
fn a_cut_beside_a_pair_still_runs() {
    assert_eq!(text(run_in(SRC, "C", "Whole")), "😀😀b😀😀a😀");
}
