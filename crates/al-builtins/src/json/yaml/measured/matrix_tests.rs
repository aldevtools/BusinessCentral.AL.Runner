//! The punctuation matrix of corpus PR 1519 (`tests/al-language/json/JsonYamlPunctMatrix.al`, codeunit 75623, merged on
//! corpus master at b3dd6fc9, not yet pinned): every item in every slot of every position, read as BC reads it or
//! refused where the reader does not read it, and the shapes the matrix lists as not measured. `~` is a line feed.

use super::super::super::{recv, tagged, write};
use super::super::read_from_yaml;
use crate::assumptions::{Assumption, clear_hits, take_hits};
use crate::native::tests::TestCtx;
use crate::native::{Ctx, Signal};
use crate::value::Value;

const UTF8: Assumption = Assumption::ReadFromYamlNonAsciiText;
const BAD_YAML: &str = "The data does not represent valid YAML.";

/// The outcome of both forms: `F` (used result false, the YAML error, the object empty; the statement refused),
/// `R:<reason>` (both refused, unmarked) or the JSON both forms give.
fn outcome(src: &str) -> String {
    outcome_text(&src.replace('~', "\n"))
}

/// `outcome` of a text whose line feeds are in place (a `~` item stays a `~`).
fn outcome_text(text: &str) -> String {
    let src = text;
    let mut got = Vec::new();
    for used in [true, false] {
        let node = super::super::super::parse("{}").expect("json").expect("node");
        let mut obj = tagged("JsonObject", node);
        let mut ctx = TestCtx::default();
        ctx.result_ignored = !used;
        clear_hits();
        let answer = read_from_yaml(&mut ctx, Some(&mut obj), &mut [Value::Text(text.into())]);
        let marks = take_hits();
        let mut json = String::new();
        write(&recv(Some(&mut obj)).expect("object"), &mut json).expect("write");
        got.push(match answer {
            Ok(Value::Boolean(false)) if used => {
                assert_eq!(ctx.last_error().message, BAD_YAML, "{src:?}");
                assert_eq!(json, "{}", "{src:?}");
                assert!(marks.contains(&UTF8), "{src:?}");
                "F".to_string()
            }
            Ok(_) => json,
            Err(Signal::Unsupported(r)) => {
                assert!(marks.is_empty(), "{src:?} marks {marks:?}");
                format!("R:{}", r.reason())
            }
            Err(e) => panic!("{src:?}: {e:?}"),
        });
    }
    match (&got[0][..], &got[1][..]) {
        ("F", s) if s.starts_with("R:") => "F".to_string(),
        (a, b) => {
            assert_eq!(a, b, "{src:?}: the two forms differ");
            got.remove(0)
        }
    }
}

const ITEMS: [&str; 37] = [
    "!", "\"", "#", "$", "%", "&", "'", "(", ")", "*", "+", ",", "-", ".", "/", ":", ";", "<", "=", ">", "?", "@", "[",
    "\\", "]", "^", "_", "`", "{", "|", "}", "~", "...", ": ", " #", "- ", "? ",
];

fn slots(k: &str) -> [String; 4] {
    [format!("{k}ab"), format!("a{k}b"), format!("ab{k}"), format!("a {k} b")]
}

fn js(v: &str) -> String {
    format!("\"{}\"", v.replace('\\', "\\\\").replace('"', "\\\""))
}

/// A plain position: its template, its JSON with `X` for the value, whether it is a list item.
const PLAIN: [(&str, &str, bool); 6] = [
    ("q: V~n: 1", r#"{"q":X,"n":"1"}"#, false),
    ("o:~  q: V~  r: 2~n: 1", r#"{"o":{"q":X,"r":"2"},"n":"1"}"#, false),
    ("k:~  - V~  - c", r#"{"k":[X,"c"]}"#, true),
    ("k:~  - m: V~    r: 2~  - c", r#"{"k":[{"m":X,"r":"2"},"c"]}"#, false),
    ("k:~  - r: 2~    m: V~  - c", r#"{"k":[{"r":"2","m":X},"c"]}"#, false),
    ("o:~  k:~    - V~    - c~n: 1", r#"{"o":{"k":[X,"c"]},"n":"1"}"#, true),
];

/// What BC answers for item `k` in slot `s` (0 S, 1 M, 2 E, 3 W) of a plain position (the rule summary and the
/// deviation rows, :176-442): `None` = the value as written, `Some("F")`, `Some("null")`, `Some(text)`, or
/// `Some("map")` for a list item that turns into a mapping or a nested list (not read: refused).
fn plain_expect(k: &str, s: usize, list: bool) -> Option<&'static str> {
    match (k, s, list) {
        (k, 0, _) if "!\"%'*,>@[]`{|}".contains(k) && k.len() == 1 => Some("F"),
        (": ", 0, _) => Some("F"),
        ("#" | "&" | " #", 0, _) => Some("null"),
        ("#", 3, _) => Some("a"),
        (" #", 1 | 3, _) => Some("a"),
        (" #", 2, _) => Some("ab"),
        ("- ", 2, _) => Some("ab-"),
        ("? ", 2, _) => Some("ab?"),
        (":", 2 | 3, false) | (": ", _, false) | ("- " | "? ", 0, false) => Some("F"),
        (": ", 1, true) => Some("pair"),
        (":", 2 | 3, true) | (": ", _, true) | ("- " | "? ", 0, true) => Some("map"),
        _ => None,
    }
}

#[test]
fn plain_positions_read_as_the_matrix() {
    for (tpl, wrap, list) in PLAIN {
        for k in ITEMS {
            for (s, v) in slots(k).iter().enumerate() {
                let src = tpl.replace('~', "\n").replace('V', v);
                let got = outcome_text(&src);
                let value = |t: &str| wrap.replace('X', t);
                match plain_expect(k, s, list) {
                    None => assert_eq!(got, value(&js(v)), "{src:?}"),
                    Some("F") => assert_eq!(got, "F", "{src:?}"),
                    Some("null") => assert_eq!(got, value("null"), "{src:?}"),
                    // `- a: b`: the item mapping main's measured reader reads ({"a":"b"}, :299 M).
                    Some("pair") => assert_eq!(got, value(r#"{"a":"b"}"#), "{src:?}"),
                    Some("map") => assert!(got.starts_with("R:"), "{src:?}: {got}"),
                    Some(t) => assert_eq!(got, value(&js(t)), "{src:?}"),
                }
            }
        }
    }
}

/// `>-`, `>`, `|`, `|-` at the top and `>-` in a list item mapping (:443-667): every item in every slot as written. S of
/// ` #` indents the lines by three, which is not read (refused).
#[test]
fn block_lines_keep_every_mark() {
    let blocks = [
        ("q: >-~  V~  V~n: 1", "V V", r#"{"q":X,"n":"1"}"#),
        ("q: >~  V~  V~n: 1", "V V\n", r#"{"q":X,"n":"1"}"#),
        ("q: |~  V~  V~n: 1", "V\nV\n", r#"{"q":X,"n":"1"}"#),
        ("q: |-~  V~  V~n: 1", "V\nV", r#"{"q":X,"n":"1"}"#),
        (
            "k:~  - m: >-~      V~      V~    r: 2",
            "V V",
            r#"{"k":[{"m":X,"r":"2"}]}"#,
        ),
    ];
    for (tpl, exp, wrap) in blocks {
        for k in ITEMS {
            for (s, v) in slots(k).iter().enumerate() {
                let src = tpl.replace('~', "\n").replace('V', v);
                let got = outcome_text(&src);
                if k == " #" && s == 0 {
                    assert!(got.starts_with("R:"), "{src:?}: {got}");
                    continue;
                }
                let text = js(&exp.replace('V', v)).replace('\n', "\\n");
                assert_eq!(got, wrap.replace('X', &text), "{src:?}");
            }
        }
    }
}

/// `q: "V"` and `q: 'V'` (:668-756): every mark kept; a lone quote of the kind answers false; in double quotes `\a` is
/// U+0007, `\ ` a space and `\` before the closing quote answers false; `a\bb` (M) is not read.
#[test]
fn quoted_top_values_read_as_the_matrix() {
    for (q, tpl) in [('"', "q: \"V\"~n: 1"), ('\'', "q: 'V'~n: 1")] {
        for k in ITEMS {
            for (s, v) in slots(k).iter().enumerate() {
                let src = tpl.replace('~', "\n").replace('V', v);
                let got = outcome_text(&src);
                let value = |t: &str| format!(r#"{{"q":{t},"n":"1"}}"#);
                match (k, s) {
                    (k, _) if k == q.to_string() => assert_eq!(got, "F", "{src:?}"),
                    ("\\", 0) if q == '"' => assert_eq!(got, value(r#""\u0007b""#), "{src:?}"),
                    ("\\", 1) if q == '"' => assert!(got.starts_with("R:"), "{src:?}: {got}"),
                    ("\\", 2) if q == '"' => assert_eq!(got, "F", "{src:?}"),
                    ("\\", 3) if q == '"' => assert_eq!(got, value(r#""a  b""#), "{src:?}"),
                    _ => assert_eq!(got, value(&js(v)), "{src:?}"),
                }
            }
        }
    }
}

/// The reporter's two remaining shapes (al-runner issue 8580): a `>-` line that ends in `...` and a comma in a value of a
/// list item mapping now read, marked.
#[test]
fn the_reporters_dataset_shapes_read() {
    let src = "name: x~description: >-~  T1 \u{a7} 19.11 ask-order test ...~  next line~tests:~  - name: t~    description: Ask-order primary question, then fallthrough ...~";
    assert_eq!(
        outcome(src),
        r#"{"name":"x","description":"T1 § 19.11 ask-order test ... next line","tests":[{"name":"t","description":"Ask-order primary question, then fallthrough ..."}]}"#
    );
}

/// Review round 1: a raw U+2028 in a `>-` line with other characters than letters, and a value before ` #` that is not
/// ASCII or starts with a digit, refuse.
#[test]
fn review_round_one_shapes_refuse() {
    for (src, reason) in [
        ("q: >-~  a\u{2028}b: c~n: 1", "in a block line with other characters"),
        ("q: >-~  a\u{2028}b c: d~n: 1", "in a block line with other characters"),
        ("o:~  q: a\u{e9} #b~  r: 2~n: 1", ""),
        ("o:~  q: 1 #b~  r: 2~n: 1", ""),
    ] {
        let got = outcome(src);
        assert!(got.starts_with("R:") && got.contains(reason), "{src:?}: {got}");
    }
}

/// What the matrix lists as not measured refuses: punctuation in a key, in a flow form, a value that is only the mark, a
/// document marker, CRLF line ends with a matrix-only shape, a place below the measured ones (with its own text), and
/// a block line that starts with `#` below the measured places.
#[test]
fn unmeasured_shapes_refuse() {
    for (src, reason) in [
        ("o:~  a,b: 1~n: 2", ""),
        ("o:~  q: [a, b!]~n: 1", ""),
        ("o:~  q: {x: a!}~n: 1", ""),
        ("o:~  q: !~n: 1", ""),
        ("o:~  q: ...~n: 1", ""),
        ("o:~  q: #~n: 1", ""),
        ("k:~  - ,~  - c", ""),
        ("---~o:~  q: a!~n: 1", ""),
        ("o:~  q: a!~n: 1~...", ""),
        ("o:\r\n  q: a!\r\nn: 1", "CRLF"),
        ("o:\r\n  q: !ab\r\nn: 1", "CRLF"),
        ("o:~  p:~    q: a, b~n: 1", "punctuation in a plain value at this place"),
        ("k:~  - m:~      - a, b", ""),
        (
            "o:~  k:~    - m: a, b~n: 1",
            "punctuation in a plain value at this place",
        ),
        ("o:~  q: |~    #ab~n: 1", "starts with `#` at this place"),
        ("q: |~  a, b~r: x\u{2028}y~n: 1", ""),
        ("q: !ab~r: a\u{2028}b~n: 1", ""),
        ("q: \"ab~r: \"c\"~n: 1", ""),
        ("q: *ab~r: &ab~n: 1", ""),
        ("o:~  q: .inf~n: 1", ""),
        ("o:~  q: a  , b~n: 1", ""),
        ("o:~  q: a, \u{e9}~n: 1", ""),
    ] {
        let got = outcome(src);
        assert!(got.starts_with("R:") && got.contains(reason), "{src:?}: {got}");
    }
}

/// Block lines below the measured places keep the texts of the rules that refuse them.
#[test]
fn block_line_refusals_name_their_reason() {
    for (src, reason) in [
        ("o:~  q: >-~    a b...~n: 1", "starts or ends with punctuation"),
        ("o:~  q: |~    a, b~n: 1", "punctuation in a `|`, `|-` or `>` line"),
        ("o:~  q: >-~    a; b~n: 1", "a block scalar line with other characters"),
        ("q: >-~  ;;;~n: 1", "a block scalar line with other characters"),
    ] {
        let got = outcome(src);
        assert!(got.starts_with("R:") && got.contains(reason), "{src:?}: {got}");
    }
}
