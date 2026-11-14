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
    assert_eq!(json("a:\n- 1\n- 2\nb: x"), r#"{"a":["1","2"],"b":"x"}"#);
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
