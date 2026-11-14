use super::*;

fn json(text: &str) -> String {
    let mut out = String::new();
    document(text).expect("read").write_json(&mut out);
    out
}

fn refused(text: &str) -> bool {
    matches!(document(text), Err(Signal::Unsupported(_)))
}

fn bad(text: &str) -> bool {
    matches!(document(text), Err(Signal::Error(e)) if e.code == "YamlBadFormat")
}

/// Main read every flat `key: value` line with a plain word or a whole number as a text; that stays.
#[test]
fn flat_lines_keep_mains_result() {
    assert_eq!(
        json("a: 1\n\nb: hello world\r\nc: é"),
        r#"{"a":"1","b":"hello world","c":"é"}"#
    );
    assert_eq!(json("a:\\n  b: 1"), r#"{"a:\\n  b":"1"}"#);
    assert!(bad("a: 1\\nb: text"));
}

#[test]
fn the_pinned_block_shapes_read() {
    assert_eq!(
        json("job:\n  steps:\n    - a\n    - name: b\n      run: c"),
        r#"{"job":{"steps":["a",{"name":"b","run":"c"}]}}"#
    );
    assert_eq!(json("a:\n- 1\n- 2"), r#"{"a":["1","2"]}"#);
    assert_eq!(json("a:\n  - 1\n  - 2"), r#"{"a":["1","2"]}"#);
    assert_eq!(json("a:\n  b: 1"), r#"{"a":{"b":"1"}}"#);
    assert_eq!(json("{a: 1, b: x}"), r#"{"a":"1","b":"x"}"#);
    assert_eq!(json("a: \"x: y\""), r#"{"a":"x: y"}"#);
    assert_eq!(json("a:\n<TAB>b: 1"), r#"{"a":null,"<TAB>b":"1"}"#);
    assert_eq!(json("{\"a\":1}"), r#"{"a":"1"}"#);
    assert!(bad(": : :"));
    assert!(bad("a: [1, 2"));
}

/// Block scalars, comments, anchors, tags, flow lists, other indents, escapes and other scalar forms refuse.
#[test]
fn unpinned_neighbors_refuse() {
    for text in [
        "a: |\n  x",
        "a: >-\n  x",
        "# c\na: 1",
        "a: 1 # c",
        "a: &x 1",
        "a: *x",
        "a: !t 1",
        "a: [1, 2]",
        "a: {b: 1}",
        "a:\n    b: 1",
        "a:\n b: 1",
        "a:",
        "a:\n  b:",
        "a: 'it''s'",
        "a: \"x\\ty\"",
        "a: ''",
        "a: ~",
        "a: NULL",
        "a: 1.5",
        "a: -5",
        "a: 007",
        "a: 1 ",
        "\"a\": 1",
        "null: 1",
        "a:\n  - b:\n      c: 1",
        "- a\nb: 1",
        "null",
        "{}",
        "{a: 'x'}",
        "  ",
        "a:\tb",
        "---\na: 1",
        "a: 1\n  b: 2",
    ] {
        assert!(refused(text), "{text:?} should refuse, got {:?}", document(text));
    }
}

/// An unclosed `[` that is not on the last line, and `: ` inside a nested value, refuse: only the top forms are pinned.
#[test]
fn bad_format_only_where_pinned() {
    assert!(refused("a: [1, 2\nb: 3"));
    assert!(refused("a:\n  b: 1: 2"));
    assert!(refused("a: -x: y"));
}

/// YamlBadFormat only for the pinned lines (Probe:573, PnA:155, P88:223): one line, the key checked first, no comment.
#[test]
fn bad_format_needs_every_pinned_fact() {
    assert!(bad("a: 1: c"));
    assert!(bad("é: 2\\nb: text"));
    assert!(bad("a: [1, 2"));
    for text in [
        "a: b # x: y",
        "# note: 1: 2\na: 1",
        "{a: 1, b: 2,\nc: 3}",
        "{\"a\": 1, \"b\": 2,\n\"c\": 3}",
        "a: 1\\nb: text\nc: 2",
        "c: 2\na: 1: 2",
        "a: [",
        "a: [1 # c",
        "-x: 1: 2",
        "x : : :",
        "a: b: c",
    ] {
        assert!(refused(text), "{text:?} should refuse, got {:?}", document(text));
    }
}

/// Characters YAML reads as a line break or forbids, and a byte order mark, refuse (every pin is printable text and LF).
#[test]
fn line_break_and_control_characters_refuse() {
    for text in [
        "a: 1\rb: 2",
        "a: x\ry",
        "a: x\u{85}y",
        "a: 1\u{2028}b: 2",
        "a: 1\u{2029}b: 2",
        "a: x\u{1}y",
        "\u{FEFF}a: 1",
        "a: x\u{7f}y",
    ] {
        assert!(refused(text), "{text:?} should refuse, got {:?}", document(text));
    }
}

/// Block, list and flow shapes are pinned with LF only and no blank line (a trailing LF, P88:216); flat lines keep main's
/// result.
#[test]
fn crlf_and_blank_lines_refuse_outside_flat_lines() {
    for text in [
        "a:\r\n  b: 1",
        "a:\n\n  b: 1",
        "\na:\n  b: 1",
        "a:\n  b: 1\n\n",
        "a:\n   \n  b: 1",
        "- 1\r\n- 2",
        "{a: 1}\r\n",
        "\n{a: 1}",
        "a:\n- 1\n\n- 2",
    ] {
        assert!(refused(text), "{text:?} should refuse, got {:?}", document(text));
    }
    assert_eq!(json("\na: 1\r\n\nb: 2\n\n"), r#"{"a":"1","b":"2"}"#);
}

/// A list at its key's own indent, a null key, nested mappings and a key after a block are read only where pinned.
#[test]
fn block_shapes_only_at_the_pinned_depth() {
    for text in [
        "a:\n  b:\n  - 1\n  - 2",
        "a:\n  b:\n  - 1\n  c: 2",
        "x:\n  a:\n  b: 1",
        "a:\nb:\nc: 1",
        "a:\n  b:\n    c: 1",
        "a:\n  b:\n    c:\n      d: 1",
        "a:\n- 1\nb: 2",
        "a:\n  - 1\nb: 2",
        "a:\n  b: 1\nc: 2",
        "a:\n- b: 1",
        "a:\n  - b: 1",
        "- b: 1",
        "  a: 1",
        "- 1\nb: 2",
        "a: 1\n  b: 2",
    ] {
        assert!(refused(text), "{text:?} should refuse, got {:?}", document(text));
    }
    assert_eq!(json("a:\nb: 1"), r#"{"a":null,"b":"1"}"#);
}

/// Null, quoted texts and keyword-like texts are read only where pinned; a flow mapping only in the two pinned styles.
#[test]
fn scalars_only_where_pinned() {
    for text in [
        "123",
        "true",
        "False",
        "a:\n- null",
        "a:\n  - x\n  - null",
        "job:\n  steps:\n    - name: null",
        "a:\n- 'x'",
        "job:\n  steps:\n    - k: 'q'",
        "a:\n  b: null",
        "a:\n  b: 'x'",
        "a: 'null'",
        "a: 'true'",
        "a: \"null\"",
        "a: \"a #b\"",
        "a: ' x'",
        "{\"a\":1,\"b\":2}",
        "{\"a\": 1}",
        "{\"a\":1, b: 2}",
        "{a: 1, \"b\":2}",
        "{ a: 1 }",
        "{a: x y}",
        "{a: 1,b: 2}",
        "{a: x:y}",
        "{a: null}",
        "{\"a\":true}",
        "{a: x, b: false}",
        "a:b",
        "say \"hi\"",
        "a,b]",
        "=x",
        "a:\u{a0}1",
        "hello  world",
    ] {
        assert!(refused(text), "{text:?} should refuse, got {:?}", document(text));
    }
    for text in ["hello\n", "{\"a\":1}\n", "- 1\n- 2\n"] {
        assert!(refused(text), "{text:?} should refuse, got {:?}", document(text));
    }
}

/// Round-2 review B2: a whole text reads as a word only for one word of ASCII letters (P88:214,229 `hello`).
#[test]
fn a_whole_text_is_one_ascii_word() {
    for text in ["hello world", "a b c", "é", "abc1", "a1"] {
        assert!(refused(text), "{text:?} should refuse, got {:?}", document(text));
    }
    assert!(matches!(document("hello"), Ok(Yaml::Text(_))));
}

/// Round-2 review B3: a block under a key holds one member and belongs to the first and only top line.
#[test]
fn a_block_holds_one_member_under_the_first_top_line() {
    for text in [
        "a:\n  b: 1\n  c: 2",
        "a:\n  c: 2\n  b:\n    - 1",
        "a: 1\nb:\n  c: 2",
        "x: 1\na:\n  b: 1\n  c: two",
        "x: 1\na:\n- 1",
        "x: 1\na:\nb: 2",
    ] {
        assert!(refused(text), "{text:?} should refuse, got {:?}", document(text));
    }
    assert_eq!(json("a:\n  b: 1"), r#"{"a":{"b":"1"}}"#);
}

/// Round-2 review B4: a flow mapping reads with two plain entries, or one quoted key and a whole number.
#[test]
fn a_flow_mapping_reads_only_the_pinned_forms() {
    for text in ["{a: 1}", "{a: 1, b: x, c: y}", "{\"a\":x}", "{\"a\":1}x"] {
        assert!(refused(text), "{text:?} should refuse, got {:?}", document(text));
    }
    assert_eq!(json("{a: 1, b: x}"), r#"{"a":"1","b":"x"}"#);
    assert_eq!(json("{\"a\":1}"), r#"{"a":"1"}"#);
}

/// Round-2 review B5: a value below the top mapping or in a list is one word of letters and digits.
#[test]
fn an_inner_value_is_one_word() {
    for text in ["a:\n  b: x y", "a:\n- it's", "a:\n- a,b", "a:\n- x:y", "a:\n  b: é"] {
        assert!(refused(text), "{text:?} should refuse, got {:?}", document(text));
    }
    assert_eq!(json("a:\n  b: x1"), r#"{"a":{"b":"x1"}}"#);
}

/// Round-2 follow-up F1: a trailing line feed is pinned after flat lines only (P88:216).
#[test]
fn a_trailing_line_feed_only_after_flat_lines() {
    for text in ["a:\n  b: 1\n", "a:\n- 1\n- 2\n"] {
        assert!(refused(text), "{text:?} should refuse, got {:?}", document(text));
    }
    assert_eq!(json("a: 1\n"), r#"{"a":"1"}"#);
}
