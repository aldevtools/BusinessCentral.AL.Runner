//! The plain scalar punctuation cells of corpus PR 1515 (`tests/al-language/json/JsonYamlUtf8.al`, codeunit 75621,
//! merged on corpus master at b7fa8dd7, not yet pinned), each read in the four places of `PlainAll` (:122-129), and the
//! places and forms next to them. `~` is a line feed.

use super::super::super::{recv, tagged, write};
use super::super::read_from_yaml;
use crate::assumptions::{Assumption, clear_hits, take_hits};
use crate::native::Signal;
use crate::native::tests::TestCtx;
use crate::value::Value;

/// Both forms of `ReadFromYaml` of `src`: the JSON, or `None` for a refusal, and whether the read is marked
/// `read-from-yaml-non-ascii-text`.
fn read(src: &str) -> Vec<(Option<String>, bool)> {
    let text = src.replace('~', "\n");
    [true, false]
        .iter()
        .map(|&used| {
            let node = super::super::super::parse("{}").expect("json").expect("node");
            let mut obj = tagged("JsonObject", node);
            let mut ctx = TestCtx::default();
            ctx.result_ignored = !used;
            clear_hits();
            let answer = read_from_yaml(&mut ctx, Some(&mut obj), &mut [Value::Text(text.as_str().into())]);
            let marked = take_hits().contains(&Assumption::ReadFromYamlNonAsciiText);
            let mut json = String::new();
            write(&recv(Some(&mut obj)).expect("object"), &mut json).expect("write");
            match answer {
                Ok(_) => (Some(json), marked),
                Err(Signal::Unsupported(_)) => (None, marked),
                Err(e) => panic!("{src:?}: {e:?}"),
            }
        })
        .collect()
}

/// The four places of `PlainAll` (:122-129) of value `v`, as JSON text `j`.
fn plain_all(v: &str, j: &str) {
    let places = [
        (format!("q: {v}~n: 1"), format!(r#"{{"q":"{j}","n":"1"}}"#)),
        (
            format!("o:~  q: {v}~  r: 2~n: 1"),
            format!(r#"{{"o":{{"q":"{j}","r":"2"}},"n":"1"}}"#),
        ),
        (format!("k:~  - {v}~  - c"), format!(r#"{{"k":["{j}","c"]}}"#)),
        (
            format!("o:~  k:~    - {v}~    - c"),
            format!(r#"{{"o":{{"k":["{j}","c"]}}}}"#),
        ),
    ];
    // Main's pinned reader reads most flat top values itself, unmarked; the other places reach the measured reader.
    for (i, (src, json)) in places.iter().enumerate() {
        for (got, marked) in read(src) {
            assert_eq!(got.as_deref(), Some(json.as_str()), "{src:?}");
            assert!(marked || i == 0, "{src:?}");
        }
    }
}

/// ReadFromYaml_PlainComma_Read :856, ..PlainCommaDots_Read :890, ..PlainHashNoSpace_Read :971, ..PlainParens_Read :988,
/// ..PlainApostrophe_Read :1022, ..PlainDoubleQuoteMid_Read :1039, ..PlainHyphenLeading_Read :1056,
/// ..PlainQuestionMid_Read :1107, ..PlainBrackets_Read :1124, ..PlainPercent_Read :1141, ..PlainAtAmp_Read :1158: the
/// text is kept as written in all four places.
#[test]
fn punctuation_in_plain_scalars_is_kept() {
    for (v, j) in [
        ("a, b", "a, b"),
        (
            "Ask-order primary question, then fallthrough ...",
            "Ask-order primary question, then fallthrough ...",
        ),
        ("a#b", "a#b"),
        ("a (b) c", "a (b) c"),
        ("it's ok", "it's ok"),
        ("a \"b\" c", "a \\\"b\\\" c"),
        ("-a b", "-a b"),
        ("a ? b", "a ? b"),
        ("a [b] c", "a [b] c"),
        ("a % b", "a % b"),
        ("a @ b & c", "a @ b & c"),
    ] {
        plain_all(v, j);
    }
}

/// ..PlainSpaceHash_Read :954: ` #` starts a comment, the value is `a`.
#[test]
fn space_hash_starts_a_comment() {
    plain_all("a #b", "a");
}

/// ..PlainQuotedColon_Read :907, ..PlainQuotedColonSingle_Read :922 (`: ` inside quotes: the YAML error as a mapping value,
/// a one-pair mapping as a list item) are not read; places no cell measures (a value of a list item mapping, a mapping
/// value two levels down, a flow item) and forms next to the cells refuse, unmarked.
#[test]
fn unmeasured_neighbors_refuse() {
    for src in [
        "o:~  q: say \"a: b\" end~n: 1",
        "k:~  - say 'a: b' end~  - c",
        "k:~  - name: t~    description: a, b~",
        "o:~  p:~    q: a, b~n: 1",
        "o:~  q: [a (b) c]~n: 1",
        "o:~  q: a,~n: 1",
        "o:~  q: a (b)~n: 1",
        "o:~  q: a \"b\"~n: 1",
        "o:~  q: ,a~n: 1",
        "o:~  q: a  , b~n: 1",
        "o:~  q: a, \u{e9}~n: 1",
        "o:~  q: a ^ b~n: 1",
        "o:~  q: a #b c~n: 1",
        "k:~  - a, b #c d~  - c",
        "q: a\u{2028}b, c~n: 1",
        "o:~  q: a\u{e9}b-c~n: 1",
        "k:~  - a\u{a7}b.~  - c",
    ] {
        for (got, marked) in read(src) {
            assert_eq!(got, None, "{src:?}");
            assert!(!marked, "{src:?}");
        }
    }
}
