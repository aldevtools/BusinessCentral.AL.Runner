//! The cells of corpus PR 1515 (`tests/al-language/json/JsonYamlUtf8.al`, codeunit 75621, merged on corpus master at b7fa8dd7, not yet pinned) that reach the measured
//! reader, each read as the probe reads it, and their neighbors. `~` is a line feed, `X` each measured character in turn
//! (`EachChar`, :195-205).

use super::super::super::{recv, tagged, write};
use super::super::read_from_yaml;
use crate::assumptions::{Assumption, clear_hits, take_hits};
use crate::native::tests::TestCtx;
use crate::native::{Ctx, Signal};
use crate::value::Value;

const UTF8: Assumption = Assumption::ReadFromYamlNonAsciiText;
const BLOCKS: Assumption = Assumption::ReadFromYamlBlankLinesAndBlocks;
const CHARS: [&str; 7] = [
    "\u{a7}",
    "\u{e9}",
    "\u{fc}",
    "\u{f8}",
    "\u{20ac}",
    "\u{1f600}",
    "e\u{301}",
];
const BAD_YAML: &str = "The data does not represent valid YAML.";

struct Read {
    answer: Result<Value, Signal>,
    json: String,
    last_error: String,
    marks: Vec<Assumption>,
}

fn read_into(text: &str, used: bool, start: &str) -> Read {
    let node = super::super::super::parse(start).expect("json").expect("node");
    let mut obj = tagged("JsonObject", node);
    let mut ctx = TestCtx::default();
    ctx.result_ignored = !used;
    clear_hits();
    let answer = read_from_yaml(&mut ctx, Some(&mut obj), &mut [Value::Text(text.into())]);
    let marks = take_hits();
    let mut json = String::new();
    write(&recv(Some(&mut obj)).expect("object"), &mut json).expect("write");
    Read {
        answer,
        json,
        last_error: ctx.last_error().message,
        marks,
    }
}

fn read(text: &str, used: bool) -> Read {
    read_into(text, used, "{}")
}

fn lf(src: &str) -> String {
    src.replace('~', "\n")
}

/// A cell: both forms read `src` to `json`, marked with both entries.
fn cell(src: &str, json: &str) {
    let text = lf(src);
    for used in [true, false] {
        let r = read(&text, used);
        assert!(r.answer.is_ok(), "{text:?} used {used}: {:?}", r.answer);
        assert_eq!(r.json, json, "{text:?}");
        assert!(
            r.marks.contains(&UTF8) && r.marks.contains(&BLOCKS),
            "{text:?} marks {:?}",
            r.marks
        );
    }
}

fn each_char(src: &str, json: &str) {
    for x in CHARS {
        cell(&src.replace('X', x), &json.replace('X', x));
    }
}

fn refused(text: &str) -> bool {
    [true, false]
        .iter()
        .all(|&used| matches!(read(text, used).answer, Err(Signal::Unsupported(_))))
}

/// ReadFromYaml_Utf8FoldedStrip_Read :218, ..Folded_Read :246, ..Literal_Read :274, ..LiteralStrip_Read :302.
#[test]
fn block_scalars() {
    each_char("q: >-~  aXb~n: 1", r#"{"q":"aXb","n":"1"}"#);
    each_char("q: >~  aXb~n: 1", r#"{"q":"aXb\n","n":"1"}"#);
    each_char("q: |~  aXb~n: 1", r#"{"q":"aXb\n","n":"1"}"#);
    each_char("q: |-~  aXb~n: 1", r#"{"q":"aXb","n":"1"}"#);
}

/// ..FoldedLineEdges_Read :498, ..FoldedKeepLineEdges_Read :526, ..LiteralLineEdges_Read :554, ..FoldedCharOnly_Read :582.
#[test]
fn block_scalar_line_edges() {
    each_char(
        "q: >-~  Xone twoX~  XthreeX~n: 1",
        r#"{"q":"Xone twoX XthreeX","n":"1"}"#,
    );
    each_char("q: >~  XoneX~~  XtwoX~n: 1", r#"{"q":"XoneX\nXtwoX\n","n":"1"}"#);
    each_char(
        "q: |~  Xone twoX~  XthreeX~n: 1",
        r#"{"q":"Xone twoX\nXthreeX\n","n":"1"}"#,
    );
    each_char("q: >-~  X~n: 1", r#"{"q":"X","n":"1"}"#);
}

/// ..SingleQuoted_Read :386, ..DoubleQuoted_Read :414, ..ListItem_Read :470 (the plain value and key cells :330-385, :442-469 are
/// flat lines, which main's pinned reader reads, see `flat_lines_keep_mains_reader`).
#[test]
fn quoted_values_and_list_items() {
    each_char("q: 'aXb'~n: 1", r#"{"q":"aXb","n":"1"}"#);
    each_char("q: \"aXb\"~n: 1", r#"{"q":"aXb","n":"1"}"#);
    each_char("k:~  - aXb~  - c", r#"{"k":["aXb","c"]}"#);
}

/// ..FlowAndComments_Read :678-705.
#[test]
fn flow_forms_and_comments() {
    cell(
        "k: [a\u{a7}b, \"c\u{20ac}d\", 'e\u{1f600}f']",
        "{\"k\":[\"a\u{a7}b\",\"c\u{20ac}d\",\"e\u{1f600}f\"]}",
    );
    cell("q: {x\u{e9}: y\u{1f600}}", "{\"q\":{\"x\u{e9}\":\"y\u{1f600}\"}}");
    cell("# c\u{a7}\u{20ac}\u{1f600}~q: v", r#"{"q":"v"}"#);
    cell("q: v\u{fc} # c\u{a7}\u{1f600}", "{\"q\":\"v\u{fc}\"}");
    cell(
        "q: >-~  \u{a7}\u{e9}\u{fc}\u{f8}\u{20ac}\u{1f600}e\u{301}~n: 1",
        "{\"q\":\"\u{a7}\u{e9}\u{fc}\u{f8}\u{20ac}\u{1f600}e\u{301}\",\"n\":\"1\"}",
    );
}

const ISSUE: &str = "name: x~description: >-~  T1 \u{a7} 19.11 ask-order test, two turns. Turn 1: a clear item reference but no~  shipment/invoice number anywhere - the agent must ask~tests:~  - name: t~    description: Ask-order primary question.~";
const ISSUE_JSON: &str = "{\"name\":\"x\",\"description\":\"T1 \u{a7} 19.11 ask-order test, two turns. Turn 1: a clear item reference but no shipment/invoice number anywhere - the agent must ask\",\"tests\":[{\"name\":\"t\",\"description\":\"Ask-order primary question.\"}]}";

/// ..IssueDocument_Read :707-723 (LF, CRLF, a `>-` block in a list item mapping), ..IssueDescriptionLen_Read :725-733.
#[test]
fn issue_document() {
    cell(ISSUE, ISSUE_JSON);
    let crlf = ISSUE.replace('~', "\r\n");
    let r = read(&crlf, true);
    assert!(matches!(r.answer, Ok(Value::Boolean(true))), "{:?}", r.answer);
    assert_eq!(r.json, ISSUE_JSON);
    cell(
        "k:~  - name: t~    description: >-~      T1 \u{a7} 19.11 ask~      next \u{20ac} line~    n: 1~  - name: u",
        "{\"k\":[{\"name\":\"t\",\"description\":\"T1 \u{a7} 19.11 ask next \u{20ac} line\",\"n\":\"1\"},{\"name\":\"u\"}]}",
    );
    let desc = ISSUE_JSON
        .split("\"description\":\"")
        .nth(1)
        .and_then(|d| d.split('"').next())
        .expect("desc");
    assert_eq!(desc.encode_utf16().count(), 129);
}

/// ..SpecialPlain_Read :735, ..SpecialFolded_Read :757, ..SpecialDoubleQuoted_Read :779: a raw U+0085, U+2028 or U+2029
/// makes the Boolean form answer false with the YAML error, the object empty.
#[test]
fn bad_breaks_answer_false_with_the_yaml_error() {
    for x in ["\u{85}", "\u{2028}", "\u{2029}"] {
        for src in ["q: aXb~n: 1", "q: >-~  aXb~n: 1", "q: \"aXb\"~n: 1"] {
            let text = lf(&src.replace('X', x));
            let r = read(&text, true);
            assert!(
                matches!(r.answer, Ok(Value::Boolean(false))),
                "{text:?}: {:?}",
                r.answer
            );
            assert_eq!(r.json, "{}");
            assert_eq!(r.last_error, BAD_YAML);
            assert!(r.marks.contains(&UTF8), "{text:?}");
            // The statement form and a receiver with members are not measured: refused, with no mark.
            for refused in [read(&text, false), read_into(&text, true, r#"{"z":1}"#)] {
                assert!(matches!(refused.answer, Err(Signal::Unsupported(_))), "{text:?}");
                assert!(refused.marks.is_empty(), "{text:?} marks {:?}", refused.marks);
            }
        }
    }
}

/// Shapes next to the cells that no cell measures: each refuses in both forms.
#[test]
fn unmeasured_neighbors_refuse() {
    for src in [
        // Other characters outside printable ASCII, U+0301 after another letter, a byte order mark.
        "q: >-~  a\u{e4}b~n: 1",
        "q: >-~  a\u{a0}b~n: 1",
        "q: >-~  a\u{80}b~n: 1",
        "q: >-~  a\u{feff}b~n: 1",
        "q: >-~  a\u{fffd}b~n: 1",
        "q: >-~  ao\u{301}b~n: 1",
        "\u{feff}q: >-~  a\u{a7}b~n: 1",
        // A measured character first in a plain value, a key or a list item word, or in a quoted key.
        "q: \u{a7}b~k:~  - c",
        "\u{a7}b: 1~k:~  - c",
        "k:~  - \u{a7}b~  - c",
        "\"a\u{a7}b\": 1~k:~  - c",
        // Punctuation in block scalars below the places of the matrix of PR 1519 (`matrix_tests.rs` reads the others).
        "o:~  q: |~    a, b~n: 1",
        "o:~  q: >-~    a b.~n: 1",
        "k:~  - name: t~    description: >~      a; b",
        // A raw U+0085 at another place: nested, in a literal or kept block, in a comment, a key, a list item, a flow
        // form, a single-quoted value, a tail comment, at a word edge.
        "k:~  q: a\u{85}b",
        "q: |~  a\u{85}b~n: 1",
        "q: >~  a\u{85}b~n: 1",
        "# a\u{85}b~q: >-~  a~n: 1",
        "a\u{85}b: 1~k:~  - c",
        "k:~  - a\u{85}b",
        "q: [a\u{85}b]~k:~  - c",
        "q: 'a\u{85}b'~n: 1",
        "q: ab # c\u{85}d~k:~  - c",
        "q: a\u{85}~n: 1",
        "q: >-~  \u{85}a~n: 1",
    ] {
        let text = lf(src);
        assert!(refused(&text), "{text:?}");
    }
}

/// Flat lines of measured characters are read by main's pinned reader, unmarked (:330-385, :442-469).
#[test]
fn flat_lines_keep_mains_reader() {
    for x in CHARS {
        for src in ["q: aXb~n: 1", "q: X~n: 1", "aXb: 1~n: 2"] {
            let text = lf(&src.replace('X', x));
            let r = read(&text, false);
            assert!(r.answer.is_ok(), "{text:?}: {:?}", r.answer);
            assert!(r.marks.is_empty(), "{text:?} marks {:?}", r.marks);
        }
    }
}
