//! `JsonObject.WriteToYaml` and `JsonObject.ReadFromYaml`. Citations are relative to `corpus/tests/al-language/json/`:
//! P88 `json/P88ObjectPathYaml.al`, PnA `PnAJsonYaml.al`, Dc `DcJsonLeads.al`, Probe `TestProbeJsonContracts.al`.

use super::{Handle, JsonNode, Reader, Scalar, invalid_json, read_from, read_text_into, recv, with_node, write};
use crate::assumptions::{Assumption, assume};
use crate::native::{Ctx, Signal, unsupported};
use crate::value::Value;

mod read;
use read::{Fail, Yaml};

/// A key WriteToYaml writes: letters, digits and `_` (every pinned key); any other key refuses.
fn plain_yaml_key(k: &str) -> bool {
    !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Where a text is written: the quoted forms are pinned for a member of the top object only.
#[derive(Clone, Copy, PartialEq)]
enum At {
    TopMember,
    Inner,
}

/// A character every pinned text holds: printable, no line break (a line feed is pinned as the folded `>-` form,
/// P88:187, which is not written yet).
fn printable(c: char) -> bool {
    !c.is_control() && !matches!(c, '\u{2028}' | '\u{2029}' | '\u{FEFF}' | '\u{FFFE}' | '\u{FFFF}')
}

/// A text written bare: it starts with a letter or a digit, or is `~`; it has no `: `, no ` #` and does not end with a
/// space or `:`. Pinned: P88:187 `yes`, `null`, `~`, `a:b`, `say "hi"`, `it's`, `ünï ✓`; PnA:121-139 `say "hi"`, `123`,
/// `true`, `é`; Probe:547 `text`, `one`, `123`, `true`; P88:164 `two`, `b` in an array and a nested object.
fn bare(t: &str) -> bool {
    let first_ok = t.starts_with(char::is_alphanumeric) || t == "~";
    first_ok && t.chars().all(printable) && !t.contains(": ") && !t.contains(" #") && !t.ends_with([' ', ':'])
}

/// A text written in single quotes, as a top member only: the empty text (P88:187, PnA:130, Probe:547), a leading space
/// (P88:187 ` `, PnA:118 ` x`), a trailing space (P88:187 `a `), a first `#` (P88:187 `#c`) or `- ` (P88:187 `- x`) or
/// `: ` (Probe:547 `: x`), an inner `: ` (PnA:112 `x: y`, Probe:547 `a: b # c`) or ` #` (PnA:115 `x # y`). A quote inside
/// (doubled in YAML) is not pinned.
fn quoted(t: &str) -> bool {
    let reason = t.is_empty()
        || t.starts_with([' ', '#'])
        || t.ends_with(' ')
        || t.starts_with("- ")
        || t.starts_with(": ")
        || t.contains(": ")
        || t.contains(" #");
    reason && t.chars().all(printable) && !t.contains('\'')
}

fn yaml_text(t: &str, at: At) -> Result<String, Signal> {
    if bare(t) {
        Ok(t.to_string())
    } else if at == At::TopMember && quoted(t) {
        Ok(format!("'{t}'"))
    } else {
        unsupported("WriteToYaml of a text whose YAML form at this place is not pinned")
    }
}

fn yaml_scalar(c: &Handle, at: At) -> Result<String, Signal> {
    let scalar = with_node(c, |n| match n {
        JsonNode::Scalar(Scalar::Val(v)) => Some(v.clone()),
        _ => None,
    });
    match scalar {
        Some(Value::Text(t) | Value::Code(t)) => yaml_text(&t, at),
        Some(Value::Integer(_) | Value::BigInteger(_) | Value::Boolean(_) | Value::Decimal(_)) => {
            let mut t = String::new();
            write(c, &mut t)?;
            Ok(t)
        }
        _ => unsupported(
            "WriteToYaml of a value that is not a number, a Boolean or a text (the YAML form is not pinned)",
        ),
    }
}

/// The members of an object at `indent`: `key: value`, a nested object as `key:` and its members indented by two, an
/// array as `key:` and its items at the key's own indent (P88:164; PnA:70-82; Probe:547). An empty object or array is
/// `{}` or `[]` after the key of a top member (P88:164); deeper it is not pinned.
fn yaml_members(kids: &[(String, Handle)], indent: usize, out: &mut String) -> Result<(), Signal> {
    let pad = " ".repeat(indent);
    let at = if indent == 0 { At::TopMember } else { At::Inner };
    for (k, c) in kids {
        if !plain_yaml_key(k) {
            return unsupported(format!("WriteToYaml with the key `{k}` (quoting is not pinned)"));
        }
        enum Kind {
            Obj(Vec<(String, Handle)>),
            Arr(Vec<Handle>),
            Other,
        }
        let kind = with_node(c, |n| match n {
            JsonNode::Object(k) => Kind::Obj(k.clone()),
            JsonNode::Array(k) => Kind::Arr(k.clone()),
            _ => Kind::Other,
        });
        match kind {
            Kind::Obj(inner) if inner.is_empty() && at == At::TopMember => out.push_str(&format!("{k}: {{}}\r\n")),
            Kind::Arr(items) if items.is_empty() && at == At::TopMember => out.push_str(&format!("{k}: []\r\n")),
            Kind::Obj(inner) if inner.is_empty() => {
                return unsupported("WriteToYaml of an empty nested object (not pinned)");
            }
            Kind::Arr(items) if items.is_empty() => {
                return unsupported("WriteToYaml of an empty nested array (not pinned)");
            }
            Kind::Obj(inner) => {
                out.push_str(&format!("{pad}{k}:\r\n"));
                yaml_members(&inner, indent + 2, out)?;
            }
            Kind::Arr(items) => {
                out.push_str(&format!("{pad}{k}:\r\n"));
                for item in items {
                    array_item(&item, &pad, out)?;
                }
            }
            Kind::Other => out.push_str(&format!("{pad}{k}: {}\r\n", yaml_scalar(c, at)?)),
        }
    }
    Ok(())
}

/// A word YAML may read as null or a Boolean: WriteToYaml writes it inside an array item nowhere a pin shows.
fn keyword(w: &str) -> bool {
    ["null", "true", "false", "yes", "no", "on", "off"]
        .iter()
        .any(|k| w.eq_ignore_ascii_case(k))
}

/// The texts measured inside an array item: P88:164 `two`, PnA:82 and Dc:208 `b`, `c`.
fn measured_word(t: &str) -> bool {
    t.starts_with(|c: char| c.is_ascii_alphabetic()) && t.chars().all(|c| c.is_ascii_alphanumeric()) && !keyword(t)
}

/// An array item: `- ` and a scalar, or an object of scalars whose first member follows the `- ` and whose other
/// members are indented by two more (P88:164 `- x: 1\r\n  y: two`; PnA:82 `- b: 1`; Dc:208 `  - name: b\r\n    run: c`).
/// The object form is pinned with the members measured there, in an array at indent 0 or 2: a whole number from 0 (an
/// Integer; a BigInteger is how a JSON text reads one) and a one-word text of ASCII letters and digits that starts with
/// a letter (`measured_word`). Every other member, an empty object, an array and deeper arrays are not pinned.
fn array_item(item: &Handle, pad: &str, out: &mut String) -> Result<(), Signal> {
    let members = with_node(item, |n| match n {
        JsonNode::Object(m) if !m.is_empty() => Some(Some(m.clone())),
        JsonNode::Scalar(_) => Some(None),
        _ => None,
    });
    match members {
        Some(None) => out.push_str(&format!("{pad}- {}\r\n", yaml_scalar(item, At::Inner)?)),
        Some(Some(members)) => {
            for (i, (k, c)) in members.iter().enumerate() {
                if !plain_yaml_key(k) {
                    return unsupported(format!("WriteToYaml with the key `{k}` (quoting is not pinned)"));
                }
                let pinned_member = with_node(c, |n| match n {
                    JsonNode::Scalar(Scalar::Val(Value::Integer(i))) => *i >= 0,
                    JsonNode::Scalar(Scalar::Val(Value::BigInteger(i))) => *i >= 0,
                    JsonNode::Scalar(Scalar::Val(Value::Text(t) | Value::Code(t))) => measured_word(t),
                    _ => false,
                });
                if !(pinned_member && matches!(pad.len(), 0 | 2)) {
                    return unsupported(
                        "WriteToYaml of an object inside an array with a member other than a whole number from 0 or a one-word text, or deeper than indent 2 (not pinned)",
                    );
                }
                let lead = if i == 0 { "- " } else { "  " };
                out.push_str(&format!("{pad}{lead}{k}: {}\r\n", yaml_scalar(c, At::Inner)?));
            }
        }
        None => return unsupported("WriteToYaml of an array or an empty object inside an array (not pinned)"),
    }
    Ok(())
}

pub(super) fn write_to_yaml(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let h = recv(r)?;
    let kids = with_node(&h, |n| match n {
        JsonNode::Object(k) => Ok(k.clone()),
        _ => unsupported("WriteToYaml of something that is not a JsonObject"),
    })?;
    let mut out = String::new();
    if kids.is_empty() {
        // The empty object (P88:140, PnA:73).
        out.push_str("{}\r\n");
    }
    yaml_members(&kids, 0, &mut out)?;
    a[0] = Value::Text(out.into());
    Ok(Value::Boolean(true))
}

/// The texts the pinned corpus reads, each with the statement form (`corpus/tests/al-language/json/`): P88:210-224,
/// PnA:150-155, Dc:189-199, Probe:573-574 (an AL `\n` there is a backslash and an `n`), and `TestJsonRuntimeContracts.al:256`.
/// Every other read runs through the assumption `read-from-yaml-probed-shapes`.
const PINNED: [&str; 24] = [
    "",
    "{a: 1, b: x}",
    "{\"a\":1}",
    "- 1\n- 2",
    "hello",
    "a: 1",
    "a: 1\n",
    "a: 1\nb: 2",
    "a:\n  b: 1",
    "a:\n- 1\n- 2",
    "a: '1'",
    "a: true",
    "a: null",
    "a: [1, 2",
    "a:\n<TAB>b: 1",
    "a: 1\nb: true\nc: txt",
    "a:\n  - 1\n  - 2",
    "a: \"x: y\"",
    ": : :",
    "steps:\n  - a\n  - b",
    "steps:\n- a\n- b",
    "job:\n  steps:\n    - a\n    - name: b\n      run: c",
    "a: 1\\nb: text",
    "a:\\n  b: 1",
];

/// The deepest JSON a read gives that the probes measured as read (`flowseq-depth50`, `flowmap-depth50`, `seq-depth50`,
/// `map-depth50`: 51 containers with the top mapping) and the range they measured as InvalidJsonData (`flowseq-depth100`
/// to `flowseq-depth300`, `map-depth100`, `map-depth200`: 101 to 301 containers).
const DEEPEST_READ: usize = 51;
const TOO_DEEP: std::ops::RangeInclusive<usize> = 101..=301;

/// `ReadFromYaml(Text)` and `ReadFromYaml(InStream)`: the text is read into a YAML tree (`read::document`), written as
/// JSON and read like `ReadFrom` into the object. A stream is read as UTF-8 text without a byte order mark (`bt1`,
/// `bt4`). A document that is no mapping fails with ReadFrom's InvalidJsonType error (P88:225, 228-229; PnA:173-176;
/// `tl2`-`tl19`), a text that is no YAML with the YAML error (P88:238, PnA:177-178), a mapping nested too deep with
/// InvalidJsonData (`flowseq-depth100`).
pub(super) fn read_from_yaml(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let (text, stream) = if matches!(&a[0], Value::Library(..)) {
        // Measured: a stream of the default encoding and of UTF-8 (JsonYamlUtf8.al, JsonYamlRemaining.al).
        if !matches!(
            crate::stream::in_stream_encoding(&mut a[0])?,
            crate::stream::Encoding::MsDos | crate::stream::Encoding::Utf8
        ) {
            return unsupported(
                "ReadFromYaml of an InStream that is not of the default or the UTF-8 encoding (not measured)",
            );
        }
        (crate::stream::read_serialised_text(&mut a[0])?, true)
    } else {
        let Some(text) = crate::native::text_of(&a[0]) else {
            return unsupported("ReadFromYaml with an argument that is not text");
        };
        (text.to_string(), false)
    };
    let doc = read::document(&text);
    if read::tag_or_directive(&text) {
        assume(Assumption::ReadFromYamlFinalShapes);
    }
    if matches!(doc, Err(Fail::Refuse(_))) {
        let Err(Fail::Refuse(why)) = doc else { unreachable!() };
        return unsupported(why);
    }
    if stream || c.ret_used() || !PINNED.contains(&text.as_str()) {
        assume(Assumption::ReadFromYamlProbedShapes);
    }
    let doc = match doc {
        Ok(doc) => doc,
        Err(_) => return failed(c, r, read::bad_yaml(), true),
    };
    let depth = doc.depth();
    if TOO_DEEP.contains(&depth) && matches!(doc, Yaml::Map(_)) {
        // The cells read with the result used; the statement form is not measured.
        return failed(c, r, invalid_json(), false);
    }
    if depth > DEEPEST_READ {
        return unsupported("ReadFromYaml of a document nested deeper than the probes measured");
    }
    let mut json = String::new();
    doc.write_json(&mut json);
    if matches!(doc, Yaml::Map(_)) {
        return read_from(c, r, &mut [Value::Text(json.into())], Some("object"));
    }
    let mut r = r;
    let empty = receiver_empty(r.as_deref_mut())?;
    match read_text_into(false, r.as_deref_mut(), &json, Some("object"), Reader::ReadFromText) {
        Err(e @ Signal::Error(_)) if empty || !c.ret_used() => failed(c, r, e, true),
        _ => unsupported("ReadFromYaml of a document that is no mapping, into an object with members (not measured)"),
    }
}

fn receiver_empty(r: Option<&mut Value>) -> Result<bool, Signal> {
    let h = recv(r)?;
    Ok(with_node(&h, |n| matches!(n, JsonNode::Object(k) if k.is_empty())))
}

/// A read that fails with `e`. With the result used, into an empty object (every cell `Clear`s it first): false, `e` left
/// as the last error, the object as it was. As a statement it raises `e` where a pin shows it (`statement`: the YAML
/// error, P88:223,238 and PnA:155,177; InvalidJsonType, P88:210,225 and PnA:154,173).
fn failed(c: &mut dyn Ctx, r: Option<&mut Value>, e: Signal, statement: bool) -> Result<Value, Signal> {
    if !c.ret_used() {
        return if statement {
            Err(e)
        } else {
            unsupported("ReadFromYaml of a document nested too deep, as a statement (not measured)")
        };
    }
    if !receiver_empty(r)? {
        return unsupported("ReadFromYaml that fails into an object with members (not measured)");
    }
    let Signal::Error(e) = e else { return Err(e) };
    c.set_last_error(*e);
    Ok(Value::Boolean(false))
}

#[cfg(test)]
mod tests {
    use super::super::{JsonNode, new_handle, parse, tagged};
    use super::*;
    use crate::native::tests::TestCtx;

    /// One read into a fresh object: the answer, the object as JSON, the last error code and whether it was marked.
    fn read(text: &str, used: bool) -> (Result<Value, Signal>, String, String, bool) {
        let mut obj = tagged("JsonObject", new_handle(JsonNode::Object(Vec::new())));
        let mut ctx = TestCtx::default();
        ctx.result_ignored = !used;
        crate::assumptions::clear_hits();
        let answer = read_from_yaml(&mut ctx, Some(&mut obj), &mut [Value::Text(text.into())]);
        let marked = crate::assumptions::take_hits().contains(&Assumption::ReadFromYamlProbedShapes);
        let mut json = String::new();
        write(&recv(Some(&mut obj)).expect("object"), &mut json).expect("write");
        (answer, json, ctx.last_error().code, marked)
    }

    fn raised(answer: &Result<Value, Signal>, code: &str) -> bool {
        matches!(answer, Err(Signal::Error(e)) if e.code == code)
    }

    /// With the result used, a failed read answers false into the empty object and leaves the error as the last error
    /// (`tl2`, `tl3`, `tl4`, `fs13` of corpus PR 1530); as a statement it raises it (P88:225-229,238; PnA:173-178).
    #[test]
    fn a_failed_read_answers_false_or_raises() {
        for (text, code) in [
            ("", "InvalidJsonType"),
            ("- 1\n- 2", "InvalidJsonType"),
            ("hello", "InvalidJsonType"),
            (": : :", "YamlBadFormat"),
            ("q: [a, b", "YamlBadFormat"),
        ] {
            let (answer, json, last, marked) = read(text, true);
            assert!(matches!(answer, Ok(Value::Boolean(false))), "{text:?}: {answer:?}");
            assert_eq!((json.as_str(), last.as_str()), ("{}", code), "{text:?}");
            assert!(marked, "{text:?}");
            assert!(raised(&read(text, false).0, code), "{text:?}");
        }
    }

    /// The last of equal keys wins at the place of the first (`dk1`, `dk2`, `dk3`, `dk7` of corpus PR 1530).
    #[test]
    fn the_last_duplicate_key_wins() {
        for (text, json) in [
            ("a: 1\na: 2", r#"{"a":"2"}"#),
            ("o:\n  a: 1\n  a: 2", r#"{"o":{"a":"2"}}"#),
            ("q: {a: 1, a: 2}", r#"{"q":{"a":"2"}}"#),
            ("a: 1\nb: 2\na: 3", r#"{"a":"3","b":"2"}"#),
        ] {
            let (answer, out, _, _) = read(text, false);
            assert!(answer.is_ok(), "{text:?}: {answer:?}");
            assert_eq!(out, json, "{text:?}");
        }
    }

    fn nested(depth: usize) -> String {
        format!("q: {}x{}", "[".repeat(depth), "]".repeat(depth))
    }

    /// 51 containers read and 101 to 301 answer InvalidJsonData with the result used (`flowseq-depth50`,
    /// `flowseq-depth100`, `flowseq-depth300` of corpus PR 1530); the statement form and the depths between are not measured.
    #[test]
    fn deep_documents() {
        assert!(matches!(read(&nested(50), true).0, Ok(Value::Boolean(true))));
        for depth in [100, 300] {
            let (answer, json, last, _) = read(&nested(depth), true);
            assert!(matches!(answer, Ok(Value::Boolean(false))), "{depth}: {answer:?}");
            assert_eq!((json.as_str(), last.as_str()), ("{}", "InvalidJsonData"));
            assert!(
                matches!(read(&nested(depth), false).0, Err(Signal::Unsupported(_))),
                "{depth}"
            );
        }
        for depth in [51, 99, 301] {
            assert!(
                matches!(read(&nested(depth), true).0, Err(Signal::Unsupported(_))),
                "{depth}"
            );
        }
    }

    /// A pinned text read as a statement is a clean read; with the result used, or any other text, it is marked.
    #[test]
    fn only_the_pinned_texts_read_unmarked() {
        for text in PINNED {
            let (_, _, _, marked) = read(text, false);
            assert!(!marked, "{text:?}");
            assert!(read(text, true).3, "{text:?}");
        }
        for text in ["a: 2", "a: 1\n\n", "a:\n   b: 1"] {
            assert!(read(text, false).3, "{text:?}");
        }
    }

    fn refused_read(text: &str) -> bool {
        matches!(read(text, true).0, Err(Signal::Unsupported(_)))
    }

    /// Every shape below is read by no cell of the corpus probes; each refuses (round 1 review of PR 8824).
    #[test]
    fn unmeasured_shapes_refuse() {
        let long_key = format!("{}: 1", "k".repeat(1500));
        let wide_key = format!("{}: 1", "\u{1F600}".repeat(600));
        let deep = format!("{}{}", "[".repeat(601), "]".repeat(601));
        let flow_key = format!("[{}b]: 1", "a, ".repeat(500));
        let mut laughs = String::from("a: &a0 [x, x, x, x, x, x, x, x, x, x]\n");
        for n in 1..9 {
            laughs.push_str(&format!(
                "a{n}: &a{n} [{}]\n",
                vec![format!("*a{}", n - 1); 10].join(", ")
            ));
        }
        let mut unrefused = Vec::new();
        for text in [
            // a core tag on the wrong kind of node
            "q: !!int [a]",
            "q: !!bool {a: 1}",
            "q: !!float [1]",
            "q: !!timestamp {a: 1}",
            "q: !!map x",
            "q: !!foo x",
            "q: !!omap [a]",
            "q: ! [a]",
            "q: !!map",
            "q: !custom [a]",
            "q: !!str \"\"",
            // other texts of a core tag
            "q: !!int 007",
            "q: !!int -1",
            "q: !!int 9999999999",
            "q: !!int yes",
            "q: !!float 1.5",
            "q: !!float -0",
            "q: !!float 100000000000000000000",
            "q: !!bool no",
            "q: !!bool true",
            "q: !!bool false",
            "q: !!timestamp 2001-12-15",
            "q: ! x",
            // tagged keys, tagged top scalars
            "!!int 1: a",
            "!!bool yes: a",
            "!!str : a",
            "!!str x",
            "!!int 1",
            // directives
            "%TAG !e! tag:yaml.org,2002:\n---\nq: !e!int \"1\"",
            "%TAG ! tag:x,2000:\n---\nq: a",
            "%TAG !! tag:x,2000:\n---\nq: a",
            "%TAG !e! tag:x,2000:\n---\nq: a",
            "%TAG !e! tag:yaml.org,200\n---\nq: !e!2:int \"1\"",
            "%TAG !e! tag:yaml.org,200\n---\nq: !e!2:str x",
            "%YAML 1.2\n%YAML 1.2\n---\nq: a",
            "%YAML 2.0\n---\nq: a",
            "%YAML 1\n---\nq: a",
            // tabs
            "  \tq: a",
            "q: [a,\n\tb]",
            "q: [a,\n  \tb]",
            // names
            "a: &a.b 1",
            "a: &\u{e9} 1",
            "a: *a.b",
            "a: !\u{e9} x",
            // key length
            long_key.as_str(),
            wide_key.as_str(),
            flow_key.as_str(),
            // escapes, nesting
            "q: \"\\U00110000\"",
            deep.as_str(),
            // expansion
            laughs.as_str(),
        ] {
            if !refused_read(text) {
                unrefused.push(format!(
                    "{:?}: {:?}",
                    text.chars().take(40).collect::<String>(),
                    read(text, true).0
                ));
            }
        }
        assert!(unrefused.is_empty(), "read, not refused:\n{}", unrefused.join("\n"));
    }

    /// The measured forms of the same families still read (tg1-tg24, ck28, dc7-dc9, dc17 of corpus PR 1530).
    #[test]
    fn measured_tag_and_directive_cells_still_read() {
        for (text, json) in [
            ("q: !!int \"1\"", r#"{"q":1}"#),
            ("q: !!float 1", r#"{"q":1.0}"#),
            ("q: !!bool yes", r#"{"q":true}"#),
            ("q: !!str 123", r#"{"q":"123"}"#),
            ("q: !!map {a: 1}", r#"{"q":{"a":"1"}}"#),
            ("q: !!str", r#"{"q":null}"#),
            ("!!str q: 1", r#"{"q":"1"}"#),
            ("q: !!timestamp 2001-12-14", r#"{"q":"2001-12-14T00:00:00"}"#),
            ("%YAML 1.1\n---\nq: a", r#"{"q":"a"}"#),
            ("%YAML 1.3\n---\nq: a", r#"{"q":"a"}"#),
            ("--- !!map\nq: a", r#"{"q":"a"}"#),
            ("a: &x 1\nb: *x", r#"{"a":"1","b":"1"}"#),
        ] {
            let (answer, out, _, _) = read(text, true);
            assert!(matches!(answer, Ok(Value::Boolean(true))), "{text:?}: {answer:?}");
            assert_eq!(out, json, "{text:?}");
        }
        for text in [
            "q: !!int abc",
            "q: !!null \"\"",
            "q: !!null",
            "q: !!seq [a]",
            "q: !!binary aGk=",
            "q: !!set {a, b}",
            "q: !custom x",
            "q: !",
            "%TAG !e! tag:x,2000:\n---\nq: !e!foo x",
        ] {
            let (answer, _, last, _) = read(text, true);
            assert!(matches!(answer, Ok(Value::Boolean(false))), "{text:?}: {answer:?}");
            assert_eq!(last, "YamlBadFormat", "{text:?}");
        }
    }

    /// A key of 3000 characters is the YAML error and a long value reads (`long-key`, `longvalue-100000`).
    #[test]
    fn long_keys_and_values() {
        let key = format!("{}: 1", "k".repeat(3000));
        assert_eq!(read(&key, true).2, "YamlBadFormat");
        let value = format!("q: {}", "v".repeat(100_000));
        assert!(matches!(read(&value, true).0, Ok(Value::Boolean(true))));
        assert!(matches!(
            read(&format!("q: {}", "v".repeat(2000)), true).0,
            Ok(Value::Boolean(true))
        ));
        assert!(matches!(read("q: x\n\ta: 1", true).0, Ok(Value::Boolean(false))));
    }

    /// The pinned results (P88:225-239, PnA:156-179, Dc:204-207, Probe:575-576).
    #[test]
    fn the_pinned_texts_read_as_pinned() {
        for (text, json) in [
            ("{a: 1, b: x}", r#"{"a":"1","b":"x"}"#),
            ("{\"a\":1}", r#"{"a":"1"}"#),
            ("a: 1\n", r#"{"a":"1"}"#),
            ("a:\n- 1\n- 2", r#"{"a":["1","2"]}"#),
            ("a: '1'", r#"{"a":"1"}"#),
            ("a: true", r#"{"a":"true"}"#),
            ("a: null", r#"{"a":null}"#),
            ("a:\n<TAB>b: 1", r#"{"a":null,"<TAB>b":"1"}"#),
            ("a: \"x: y\"", r#"{"a":"x: y"}"#),
            (
                "job:\n  steps:\n    - a\n    - name: b\n      run: c",
                r#"{"job":{"steps":["a",{"name":"b","run":"c"}]}}"#,
            ),
            ("a:\\n  b: 1", r#"{"a:\\n  b":"1"}"#),
        ] {
            let (answer, out, _, _) = read(text, false);
            assert!(answer.is_ok(), "{text:?}: {answer:?}");
            assert_eq!(out, json, "{text:?}");
        }
        assert!(raised(&read("a: 1\\nb: text", false).0, "YamlBadFormat"));
    }

    fn yaml(json: &str) -> Result<String, Signal> {
        let mut obj = tagged("JsonObject", parse(json)?.expect("json"));
        let mut out = [Value::Text("".into())];
        write_to_yaml(&mut TestCtx::default(), Some(&mut obj), &mut out)?;
        Ok(crate::native::text_of(&out[0]).expect("text").to_string())
    }

    /// An object of scalars inside an array is written; any other container inside an array refuses.
    #[test]
    fn an_object_of_scalars_inside_an_array_is_written() {
        assert_eq!(
            yaml(r#"{"l":[1,{"x":1,"y":"two"}]}"#).ok().as_deref(),
            Some("l:\r\n- 1\r\n- x: 1\r\n  y: two\r\n")
        );
        assert_eq!(
            yaml(r#"{"j":{"s":["a",{"name":"b","run":"c"}]}}"#).ok().as_deref(),
            Some("j:\r\n  s:\r\n  - a\r\n  - name: b\r\n    run: c\r\n")
        );
        for json in [
            r#"{"l":[[1]]}"#,
            r#"{"l":[{}]}"#,
            r#"{"l":[{"x":{"y":1}}]}"#,
            r#"{"l":[{"a b":1}]}"#,
            r#"{"l":[{"x":1.5}]}"#,
            r#"{"l":[{"x":true}]}"#,
            r#"{"j":{"k":{"l":[{"x":1}]}}}"#,
        ] {
            assert!(matches!(yaml(json), Err(Signal::Unsupported(_))), "{json}");
        }
    }

    fn refuses(json: &str) -> bool {
        matches!(yaml(json), Err(Signal::Unsupported(_)))
    }

    /// The quoted forms are pinned for a top member only (P88:187, PnA:112-118, Probe:547); inside an array or a nested
    /// object they refuse (W005, W006, W007, W016 of the review), as does a text with no pinned form.
    #[test]
    fn a_text_is_quoted_only_where_pinned() {
        assert_eq!(
            yaml(r##"{"a":"","b":" ","c":"a ","d":"#c","e":"- x","f":": x","g":"x: y","h":"x # y","i":" x","j":"a: b # c"}"##)
                .ok()
                .as_deref(),
            Some("a: ''\r\nb: ' '\r\nc: 'a '\r\nd: '#c'\r\ne: '- x'\r\nf: ': x'\r\ng: 'x: y'\r\nh: 'x # y'\r\ni: ' x'\r\nj: 'a: b # c'\r\n")
        );
        for json in [
            r#"{"l":[{"x":"a "}]}"#,
            r#"{"l":[{"x":"- x"}]}"#,
            r##"{"l":[{"x":"#c"}]}"##,
            r#"{"l":[{"x":"a\nb"}]}"#,
            r#"{"l":[""]}"#,
            r#"{"l":["x: y"]}"#,
            r#"{"a":{"b":"x: y"}}"#,
            r#"{"a":{"b":""}}"#,
        ] {
            assert!(refuses(json), "{json}");
        }
    }

    /// The bare forms: P88:187 and PnA:121-139, Probe:547; a text that starts with another character, ends with `:` or a
    /// space, or holds a line break or a control character has no pinned form and refuses.
    #[test]
    fn a_text_is_bare_only_in_the_pinned_forms() {
        assert_eq!(
            yaml(r#"{"a":"yes","b":"null","c":"~","d":"a:b","e":"say \"hi\"","f":"it's","g":"ünï ✓","h":"123","i":"true"}"#)
                .ok()
                .as_deref(),
            Some("a: yes\r\nb: null\r\nc: ~\r\nd: a:b\r\ne: say \"hi\"\r\nf: it's\r\ng: ünï ✓\r\nh: 123\r\ni: true\r\n")
        );
        assert_eq!(
            yaml(r#"{"a":{"b":"two"},"l":["b",{"x":"c"}]}"#).ok().as_deref(),
            Some("a:\r\n  b: two\r\nl:\r\n- b\r\n- x: c\r\n")
        );
        for json in [
            r#"{"a":"a\nb"}"#,
            r#"{"a":"a\rb"}"#,
            r#"{"a":"it's "}"#,
            r#"{"a":"-x"}"#,
            r#"{"a":":x"}"#,
            r#"{"a":"x:"}"#,
            r#"{"a":"[x]"}"#,
            r#"{"a":"\"q\""}"#,
            r#"{"a":".5"}"#,
            r#"{"a":"~x"}"#,
            r#"{"a":"a\u0001b"}"#,
            r#"{"a":"a\u2028b"}"#,
            r#"{"a":"a\u0085b"}"#,
            r#"{"a":"a\ufeffb"}"#,
        ] {
            assert!(refuses(json), "{json}");
        }
    }

    /// A member of an object inside an array is pinned only as a one-word text of letters and digits that starts with a
    /// letter (P88:164 `two`, Dc:208 `b`, `c`) and a whole number from 0 (PnA:82 `b: 1`); every other member refuses
    /// (W000-W017 of the round-2 review).
    #[test]
    fn an_array_item_member_is_written_only_in_the_measured_forms() {
        assert_eq!(
            yaml(r#"{"l":[{"x":0,"y":"two","z":"b2"}]}"#).ok().as_deref(),
            Some("l:\r\n- x: 0\r\n  y: two\r\n  z: b2\r\n")
        );
        for json in [
            r#"{"l":[{"x":"it's"}]}"#,
            r#"{"l":[{"x":"null"}]}"#,
            r#"{"l":[{"x":"~"}]}"#,
            r#"{"l":[{"x":"123"}]}"#,
            r#"{"l":[{"x":"true"}]}"#,
            r#"{"l":[{"x":"a,b"}]}"#,
            r#"{"l":[{"x":"a:b"}]}"#,
            r#"{"l":[{"x":"say \"hi\""}]}"#,
            r#"{"l":[{"x":"\u00e9"}]}"#,
            r#"{"l":[{"x":"a b"}]}"#,
            r#"{"l":[{"x":-1}]}"#,
        ] {
            assert!(refuses(json), "{json}");
        }
    }

    /// An empty object and an empty array as a top member, and the empty object itself (P88:140,164; PnA:73); deeper they
    /// refuse.
    #[test]
    fn empty_containers_where_pinned() {
        assert_eq!(yaml("{}").ok().as_deref(), Some("{}\r\n"));
        assert_eq!(yaml(r#"{"o":{},"l":[]}"#).ok().as_deref(), Some("o: {}\r\nl: []\r\n"));
        for json in [r#"{"a":{"o":{}}}"#, r#"{"a":{"l":[]}}"#] {
            assert!(refuses(json), "{json}");
        }
    }
}
