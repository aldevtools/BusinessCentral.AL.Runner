//! One test per test of corpus PR 1493 (`tests/al-language/json/JsonYamlBlankLines.al`, codeunit 75620, open), each
//! cell read in both call forms as the probe reads it: the Boolean form, `WriteTo` of the result, the statement form.
//! `~` stands for the line break, as in the probe's `Doc`. Then the neighbors: every shape no cell measures refuses, and
//! a text the pinned reader reads keeps main's result without the mark.

use super::super::super::{JsonNode, new_handle, recv, tagged, write};
use super::super::read_from_yaml;
use crate::assumptions::{Assumption, clear_hits, take_hits};
use crate::native::tests::TestCtx;
use crate::native::{Ctx, Signal};
use crate::value::Value;

const MARK: Assumption = Assumption::ReadFromYamlBlankLinesAndBlocks;
const ARRAY_ERROR: &str = "JSON tokens of type 'Array' are not supported by this implementation.";

/// One read of `text` into a fresh object: the answer, the object written as JSON, the last error and whether the
/// read marked the assumption.
fn read(text: &str, used: bool) -> (Result<Value, Signal>, String, (String, String), bool) {
    let mut obj = tagged("JsonObject", new_handle(JsonNode::Object(Vec::new())));
    let mut ctx = TestCtx::default();
    ctx.result_ignored = !used;
    clear_hits();
    let answer = read_from_yaml(&mut ctx, Some(&mut obj), &mut [Value::Text(text.into())]);
    let marked = take_hits().contains(&MARK);
    let mut json = String::new();
    write(&recv(Some(&mut obj)).expect("object"), &mut json).expect("write");
    let e = ctx.last_error();
    (answer, json, (e.message, e.code), marked)
}

fn doc(src: &str, crlf: bool) -> String {
    src.replace('~', if crlf { "\r\n" } else { "\n" })
}

/// A cell whose document is an object: both forms read it, the result is `json`, and the read is marked.
fn object(src: &str, crlf: bool, json: &str) {
    let text = doc(src, crlf);
    let (ret, out, _, marked) = read(&text, true);
    assert!(matches!(ret, Ok(Value::Boolean(true))), "{text:?} used: {ret:?}");
    assert_eq!(out, json, "{text:?}");
    assert!(marked, "{text:?} not marked");
    let (stmt, out, _, marked) = read(&text, false);
    assert!(stmt.is_ok(), "{text:?} statement: {stmt:?}");
    assert_eq!(out, json, "{text:?}");
    assert!(marked, "{text:?} not marked");
}

/// A top-level list: the Boolean form answers false without raising and leaves InvalidJsonType as the last error; the
/// statement form raises it; the object stays empty.
fn top_list(src: &str, crlf: bool) {
    let text = doc(src, crlf);
    let (ret, out, last, marked) = read(&text, true);
    assert!(matches!(ret, Ok(Value::Boolean(false))), "{text:?} used: {ret:?}");
    assert_eq!(out, "{}");
    assert_eq!(last, (ARRAY_ERROR.to_string(), "InvalidJsonType".to_string()));
    assert!(marked);
    let (stmt, out, _, _) = read(&text, false);
    assert!(
        matches!(&stmt, Err(Signal::Error(e)) if e.message == ARRAY_ERROR && e.code == "InvalidJsonType"),
        "{text:?} statement: {stmt:?}"
    );
    assert_eq!(out, "{}");
}

fn refused(text: &str) -> bool {
    let used = matches!(read(text, true).0, Err(Signal::Unsupported(_)));
    let stmt = matches!(read(text, false).0, Err(Signal::Unsupported(_)));
    used && stmt
}

#[test]
fn blank_line_top_list() {
    top_list("- a~~- b", false);
    object("k:~- a~~- b", false, r#"{"k":["a","b"]}"#);
    object("k:~- a~~- b~~- c", false, r#"{"k":["a","b","c"]}"#);
}

#[test]
fn blank_line_nested_list() {
    object("k:~  - a~~  - b", false, r#"{"k":["a","b"]}"#);
    object("k:~  - a~~  - b", true, r#"{"k":["a","b"]}"#);
    object("k:~  - a~  - b", true, r#"{"k":["a","b"]}"#);
}

#[test]
fn blank_line_nested_keys() {
    object("a:~  b: 1~~  c: 2", false, r#"{"a":{"b":"1","c":"2"}}"#);
    object("a:~  b: 1~~  c: 2", true, r#"{"a":{"b":"1","c":"2"}}"#);
    object("a:~  b: 1~~c: 2", false, r#"{"a":{"b":"1"},"c":"2"}"#);
}

#[test]
fn blank_line_list_item_mapping() {
    object("k:~  - x: 1~~    y: 2", false, r#"{"k":[{"x":"1","y":"2"}]}"#);
    let between = "k:~  - x: 1~    y: 2~~  - x: 3~    y: 4";
    let json = r#"{"k":[{"x":"1","y":"2"},{"x":"3","y":"4"}]}"#;
    object(between, false, json);
    object(between, true, json);
}

#[test]
fn crlf_no_blank() {
    object("a:~  b: 1", true, r#"{"a":{"b":"1"}}"#);
    top_list("- a~- b", true);
    object("k:~  - [a, b]~  - {x: 1}", true, r#"{"k":[["a","b"],{"x":"1"}]}"#);
}

#[test]
fn trailing_lines() {
    object("a:~  b: 1~", false, r#"{"a":{"b":"1"}}"#);
    object("a:~  b: 1~~~", false, r#"{"a":{"b":"1"}}"#);
    object("k:~  - a~  - b~", false, r#"{"k":["a","b"]}"#);
}

#[test]
fn trailing_lines_crlf() {
    object("a:~  b: 1~", true, r#"{"a":{"b":"1"}}"#);
    object("k:~  - a~  - b~~", true, r#"{"k":["a","b"]}"#);
}

/// `flat-lf-crlf` (`a: 1~b: 2~` with CRLF): the pinned reader reads flat lines with any line end (main's result, the
/// same JSON), so the read is not marked.
#[test]
fn trailing_crlf_after_flat_lines_keeps_mains_reader() {
    let (ret, out, _, marked) = read(&doc("a: 1~b: 2~", true), true);
    assert!(matches!(ret, Ok(Value::Boolean(true))));
    assert_eq!(out, r#"{"a":"1","b":"2"}"#);
    assert!(!marked);
}

#[test]
fn spaces_only_line() {
    object("k:~  - a~   ~  - b", false, r#"{"k":["a","b"]}"#);
    object("a:~  b: 1~  ~  c: 2", false, r#"{"a":{"b":"1","c":"2"}}"#);
    // `flat` (`a: 1~  ~b: 2`): flat lines, main's reader, the same JSON.
    assert_eq!(read(&doc("a: 1~  ~b: 2", false), true).1, r#"{"a":"1","b":"2"}"#);
}

#[test]
fn spaces_only_line_more() {
    object("k:~  - a~   ~  - b", true, r#"{"k":["a","b"]}"#);
    object("a:~  b: 1~    ", false, r#"{"a":{"b":"1"}}"#);
    object(
        "a:~  b:~    c: 1~      ~    d: 2",
        false,
        r#"{"a":{"b":{"c":"1","d":"2"}}}"#,
    );
}

#[test]
fn flow_in_block_list() {
    object("k:~  - [a, b]~  - {x: 1}", false, r#"{"k":[["a","b"],{"x":"1"}]}"#);
    object("k:~  - [a, b]~~  - {x: 1}", false, r#"{"k":[["a","b"],{"x":"1"}]}"#);
    object(
        "k: [a, b]~m: {x: 1, y: two words}",
        false,
        r#"{"k":["a","b"],"m":{"x":"1","y":"two words"}}"#,
    );
}

#[test]
fn flow_in_list_item_mapping() {
    let src = "k:~  - tags: [a, \"b c\"]~    opts: {x: 1}~~  - tags: []";
    let json = r#"{"k":[{"tags":["a","b c"],"opts":{"x":"1"}},{"tags":[]}]}"#;
    object(src, false, json);
    object(src, true, json);
    object("k: [\"x, y\", 'z']", false, r#"{"k":["x, y","z"]}"#);
}

#[test]
fn literal_block_scalar() {
    object(
        "q: |~  line one~  line two~n: 1",
        false,
        r#"{"q":"line one\nline two\n","n":"1"}"#,
    );
    object(
        "q: |-~  line one~  line two~n: 1",
        false,
        r#"{"q":"line one\nline two","n":"1"}"#,
    );
    object(
        "q: |~  line one~~  line three~n: 1",
        false,
        r#"{"q":"line one\n\nline three\n","n":"1"}"#,
    );
}

#[test]
fn folded_block_scalar() {
    object(
        "q: >~  line one~  line two~n: 1",
        false,
        r#"{"q":"line one line two\n","n":"1"}"#,
    );
    object(
        "q: >-~  line one~  line two~n: 1",
        false,
        r#"{"q":"line one line two","n":"1"}"#,
    );
    object(
        "q: >~  line one~~  line three~n: 1",
        false,
        r#"{"q":"line one\nline three\n","n":"1"}"#,
    );
}

#[test]
fn block_scalar_end() {
    object("q: |~  line one~  line two", false, r#"{"q":"line one\nline two"}"#);
    object("q: |~  line one~  line two~", false, r#"{"q":"line one\nline two\n"}"#);
    object(
        "q: |~  line one~  line two~n: 1",
        true,
        r#"{"q":"line one\nline two\n","n":"1"}"#,
    );
}

#[test]
fn block_scalar_in_list_item() {
    let src = "k:~  - q: |~      one~      two~    n: 1~~  - q: >-~      three~      four";
    let json = r#"{"k":[{"q":"one\ntwo\n","n":"1"},{"q":"three four"}]}"#;
    object(src, false, json);
    object(src, true, json);
}

#[test]
fn comments_and_quotes() {
    object("# head~a: 1 # tail~~# between~b: 2", false, r#"{"a":"1","b":"2"}"#);
    object(
        "\"Some Key\": two words~'Other Key': \"x\"",
        false,
        r#"{"Some Key":"two words","Other Key":"x"}"#,
    );
    object(
        r#"a: "say \"hi\"\nnext"~b: 'it''s'"#,
        false,
        r#"{"a":"say \"hi\"\nnext","b":"it's"}"#,
    );
}

#[test]
fn scalar_forms() {
    object(
        "a: 1.5~b: -3~c: 0010~d: 1e3",
        false,
        r#"{"a":"1.5","b":"-3","c":"0010","d":"1e3"}"#,
    );
    object(
        "a: two words here~b: yes~c: ~~d: Null",
        false,
        r#"{"a":"two words here","b":"yes","c":null,"d":null}"#,
    );
    object(
        "a: http://x/y~b: x:y~c: \"$Var-<CW+1M>$\"",
        false,
        r#"{"a":"http://x/y","b":"x:y","c":"$Var-<CW+1M>$"}"#,
    );
}

/// The probe's `DatasetDoc`, own data in the shape the AI Test Toolkit reads.
const DATASET: &str = concat!(
    "# Demo evaluation dataset~",
    "name: Demo dataset~",
    "language: en-US~",
    "tests:~",
    "  - name: \"First case\"~",
    "    description: >-~",
    "      Folded text~",
    "      on two lines~",
    "    question: |~",
    "      Line one~",
    "      Line two~",
    "    \"Expected Amount\": 10~",
    "    expected_data:~",
    "      lines:~",
    "        - \"Item No.\": \"1000\"~",
    "          quantity: 2~",
    "        - \"Item No.\": '2000'~",
    "          quantity: 5~",
    "~",
    "  - name: Second case~",
    "    description: plain multi word value # a comment~",
    "    question: |-~",
    "      Only line~",
    "    \"Expected Amount\": 0~",
    "    tags: [alpha, beta]~",
    "    options: {mode: fast, retries: 2}~",
);

const DATASET_JSON: &str = r#"{"name":"Demo dataset","language":"en-US","tests":[{"name":"First case","description":"Folded text on two lines","question":"Line one\nLine two\n","Expected Amount":"10","expected_data":{"lines":[{"Item No.":"1000","quantity":"2"},{"Item No.":"2000","quantity":"5"}]}},{"name":"Second case","description":"plain multi word value","question":"Only line","Expected Amount":"0","tags":["alpha","beta"],"options":{"mode":"fast","retries":"2"}}]}"#;

#[test]
fn dataset_document() {
    object(DATASET, false, DATASET_JSON);
}

#[test]
fn dataset_document_crlf() {
    object(DATASET, true, DATASET_JSON);
}

/// Shapes next to the cells that no cell measures: each refuses in both forms.
#[test]
fn unmeasured_neighbors_refuse() {
    for src in [
        // Tabs, a byte order mark, a lone CR, LF and CRLF mixed, a character outside ASCII.
        "k:~\t- a~~- b",
        "\u{feff}k:~- a~~- b",
        "k:\r- a\n\n- b",
        "k:\r\n  - a\n\n  - b",
        "k:~  - é~~  - b",
        // Leading blank line, a blank line right after `key:`, a trailing comment, an indented comment.
        "~k:~  - a~~  - b",
        "k:~~  - a~  - b",
        "k:~  - a~~  - b~# end",
        "k:~  # c~  - a~~  - b",
        // Keep and indentation indicators, a blank line at the end of a block scalar, more indented content, a
        // spaces-only line inside one, a blank line in a stripped one, punctuation in one.
        "q: |+~  x~~n: 1",
        "q: >+~  x~~n: 1",
        "q: |2~  x~~n: 1",
        "q: |~  x~~n: 1",
        "q: |~  x~    y~~n: 1",
        "q: |~  x~  ~  y~n: 1",
        "q: |-~  x~~  y~~n: 1",
        "q: |~  x, y.~~n: 1",
        // A block scalar at the end other than `|` with at most one line feed and `>-` with none.
        "q: >~  x~~a:~  - b",
        "q: |~  x~~",
        // Anchors, aliases, tags, several documents, a key and a value YAML may read otherwise.
        "a: &x 1~~b: *x",
        "a: !t 1~~b: 2",
        "---~a: 1~~b: 2",
        "a: NULL~~b: 2",
        "a:~  b: true~~  c: 2",
        "a:~  b: ~~  c: 2",
        "a: 0x1F~~b: 2",
        "a: 2024-03-15~~b: 2",
        "a: .5~~b: 2",
        // A list at its key's own indent below a nested key, followed by a top key, a nested list item, an item
        // mapping that opens a block on its dash line, a quoted list item, an empty flow mapping.
        "a:~  k:~  - x~~  m: 1",
        "k:~- a~~- b~m: 1",
        "k:~  - - a~~  - b",
        "k:~  - x:~      y: 1~~  - z",
        "k:~  - 'a'~~  - b",
        "k:~  - {}~~  - b",
        // A tail comment after a quoted value and after a flow value, an escape other than `\"` and `\n`.
        "a: \"x\" # c~~b: 2",
        "a: [x] # c~~b: 2",
        "a: \"x\\ty\"~~b: 2",
        // A top list of other items than plain words, a duplicate key.
        "- a: 1~~- b",
        "a:~  b: 1~~  b: 2",
    ] {
        for crlf in [false, true] {
            let text = doc(src, crlf);
            assert!(refused(&text), "{text:?} was not refused: {:?}", read(&text, true).0);
        }
    }
}

/// Shapes the pinned reader refuses on purpose and no cell measures (review of PR 8631): a quoted keyword, a key that
/// is a keyword or does not start with a letter, `null` below the top or in an item mapping, `key:` null after another
/// top line, an item mapping or flow item in a list at a top key's own indent.
#[test]
fn pinned_refusals_are_kept() {
    for src in [
        "a: 'null'~~b:~  c: 1",
        "a: \"true\"~~b:~  c: 1",
        "a: 'null'",
        "null: a~~b:~  c: 1",
        "true: a~~b:~  c: 1",
        "yes: a~~b:~  c: 1",
        "1: a~~b:~  c: 1",
        "_a: 1~~b:~  c: 1",
        "a:~  2024: x~~  b: 1",
        "\"null\": a~~b:~  c: 1",
        "a:~  b: null~~  c: 1",
        "a:~  b: Null~~  c: 1",
        "k:~  - x: Null~~  - x: 1",
        "x: 1~a:~b: 2~~c: 3",
        "k:~- x: 1~~- x: 2",
        "k:~- [a]~~- b",
    ] {
        for crlf in [false, true] {
            let text = doc(src, crlf);
            assert!(refused(&text), "{text:?} was not refused: {:?}", read(&text, true).0);
        }
    }
}

/// Shapes the pinned reader reads keep main's result and are not marked.
#[test]
fn pinned_shapes_keep_mains_result() {
    for (text, json) in [
        ("a: 1\n\nb: hello world", r#"{"a":"1","b":"hello world"}"#),
        ("a:\n  - 1\n  - 2", r#"{"a":["1","2"]}"#),
        (
            "job:\n  steps:\n    - a\n    - name: b\n      run: c",
            r#"{"job":{"steps":["a",{"name":"b","run":"c"}]}}"#,
        ),
    ] {
        let (ret, out, _, marked) = read(text, false);
        assert!(ret.is_ok(), "{text:?}");
        assert_eq!(out, json);
        assert!(!marked, "{text:?} marked");
    }
    // A top list in LF form without a blank line is main's: its used result refuses, unmarked.
    let (ret, _, _, marked) = read("- a\n- b", true);
    assert!(matches!(ret, Err(Signal::Unsupported(_))));
    assert!(!marked);
}
