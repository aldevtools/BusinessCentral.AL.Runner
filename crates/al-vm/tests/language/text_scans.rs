//! Text functions on text of ASCII only work on the text where it lies; they must give what the UTF-16 path gives.

use al_builtins::value::Value;

fn text(body: &str) -> String {
    let src = format!("codeunit 50100 C {{ procedure F(): Text var T: Text; N: Text; I: Integer; begin {body} end; }}");
    match al_testkit::run_in(&src, "C", "F") {
        Ok(Value::Text(t) | Value::Code(t)) => t.to_string(),
        other => format!("{other:?}"),
    }
}

#[test]
fn index_of_substring_and_str_pos_agree_for_ascii_and_non_ascii_text() {
    // 'a' + e-acute + 'bcabc': the UTF-16 position of the second 'b' is 6 in both spellings of the same text.
    assert_eq!(
        text(
            "T := 'xabcabc'; exit(Format(T.IndexOf('b')) + ',' + Format(T.IndexOf('b', 4)) + ',' + Format(StrPos(T, 'c')));"
        ),
        "3,6,4"
    );
    assert_eq!(
        text(
            "T := 'x\u{e9}abcabc'; exit(Format(T.IndexOf('b')) + ',' + Format(T.IndexOf('b', 5)) + ',' + Format(StrPos(T, 'c')));"
        ),
        "4,7,5"
    );
    assert_eq!(
        text("T := 'hello world'; exit(T.Substring(7) + '|' + T.Substring(1, 5) + '|' + T.Substring(7, 99));"),
        "world|hello|world"
    );
    assert_eq!(
        text("T := 'h\u{e9}llo world'; exit(T.Substring(7) + '|' + T.Substring(1, 5) + '|' + T.Substring(7, 99));"),
        "world|h\u{e9}llo|world"
    );
    assert_eq!(
        text("T := 'abc'; exit(Format(T.IndexOf('')) + ',' + Format(T.IndexOf('', 3)));"),
        "1,3"
    );
}

#[test]
fn a_character_written_into_a_text_is_seen_by_the_next_read_and_not_by_a_copy() {
    assert_eq!(
        text("T := 'hello'; N := T; T[1] := 'J'; T[5] := 'y'; exit(T + ',' + N + ',' + Format(T[1]) );"),
        "Jelly,hello,J"
    );
    // After the write the text is still known to be ASCII; a write of a non-ASCII character goes the general way.
    assert_eq!(text("T := 'abc'; T[2] := 'x'; exit(T);"), "axc");
}

#[test]
fn last_index_of_agrees_for_ascii_and_non_ascii_text() {
    assert_eq!(
        text(
            "T := 'xabcabc'; exit(Format(T.LastIndexOf('b')) + ',' + Format(T.LastIndexOf('b', 5)) + ',' + Format(T.LastIndexOf('')));"
        ),
        "6,3,8"
    );
    assert_eq!(
        text(
            "T := 'x\u{e9}abcabc'; exit(Format(T.LastIndexOf('b')) + ',' + Format(T.LastIndexOf('b', 6)) + ',' + Format(T.LastIndexOf('')));"
        ),
        "7,4,9"
    );
}

#[test]
fn last_index_of_start_index_may_be_one_past_the_end() {
    // corpus text/text-methods/Search1.Codeunit.al:LastIndexOf_StartIndex
    assert_eq!(
        text(
            "T := 'abcabcabc'; exit(Format(T.LastIndexOf('a', 10)) + ',' + Format(T.LastIndexOf('', 5)) + ',' + Format(T.LastIndexOf('', 9)));"
        ),
        "7,6,10"
    );
    // On an empty text the empty text is found at 1 (Search1.Codeunit.al:351, LastIndexOf_StartIndex_Empty).
    assert_eq!(text("T := ''; exit(Format(T.LastIndexOf('', 1)));"), "1");
}
