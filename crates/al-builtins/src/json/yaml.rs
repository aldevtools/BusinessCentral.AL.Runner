//! `JsonObject.WriteToYaml` and `JsonObject.ReadFromYaml`. Citations are relative to `corpus/tests/al-language/json/`:
//! P88 `json/P88ObjectPathYaml.al`, PnA `PnAJsonYaml.al`, Dc `DcJsonLeads.al`, Probe `TestProbeJsonContracts.al`.

use super::{Handle, JsonNode, Scalar, read_from, recv, with_node, write};
use crate::native::{Ctx, Signal, unsupported};
use crate::value::Value;

mod read;
use read::{Yaml, document};

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

/// An array item: `- ` and a scalar, or an object of scalars whose first member follows the `- ` and whose other
/// members are indented by two more (P88:164 `- x: 1\r\n  y: two`; PnA:82 `- b: 1`; Dc:208 `  - name: b\r\n    run: c`).
/// The object form is pinned with whole-number members (an Integer; a BigInteger is how a JSON text reads one) and
/// text members, in an array at indent 0 or 2. An empty object, an array, other members and deeper arrays are not.
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
                let pinned_member = with_node(c, |n| {
                    matches!(
                        n,
                        JsonNode::Scalar(Scalar::Val(
                            Value::Integer(_) | Value::BigInteger(_) | Value::Text(_) | Value::Code(_)
                        ))
                    )
                });
                if !(pinned_member && matches!(pad.len(), 0 | 2)) {
                    return unsupported(
                        "WriteToYaml of an object inside an array with a member other than an Integer or a text, or deeper than indent 2 (not pinned)",
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

/// `ReadFromYaml(Text)`: the text is read into a YAML tree (`read::document`, which owns every pinned shape and refuses
/// the rest), written as JSON and read like `ReadFrom` into an object. An empty text, a list and a single word are no
/// object and fail with ReadFrom's InvalidJsonType error, the receiver left as it was (P88:225, 228-229;
/// PnA:173-176).
pub(super) fn read_from_yaml(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let Some(text) = crate::native::text_of(&a[0]) else {
        return unsupported("ReadFromYaml with an argument that is not text");
    };
    let doc = document(&text);
    // Every pin calls ReadFromYaml as a statement; what a used result answers is not pinned.
    if c.ret_used()
        && matches!(
            doc,
            Ok(Yaml::Seq(_) | Yaml::Text(_) | Yaml::Null) | Err(Signal::Error(_))
        )
    {
        return unsupported("the result of ReadFromYaml of a text that is no YAML object (not pinned)");
    }
    let mut json = String::new();
    doc?.write_json(&mut json);
    read_from(c, r, &mut [Value::Text(json.into())], Some("object"))
}

#[cfg(test)]
mod tests {
    use super::super::{JsonNode, new_handle, parse, tagged};
    use super::*;
    use crate::native::tests::TestCtx;

    fn read(text: &str, used: bool) -> Result<Value, Signal> {
        let mut obj = tagged("JsonObject", new_handle(JsonNode::Object(Vec::new())));
        let mut ctx = TestCtx::default();
        ctx.result_ignored = !used;
        read_from_yaml(&mut ctx, Some(&mut obj), &mut [Value::Text(text.into())])
    }

    /// With the result used, an object reads as before; the error paths, pinned only as statements, refuse.
    #[test]
    fn a_used_result_refuses_where_only_the_statement_is_pinned() {
        assert!(matches!(read("a: 1", true), Ok(Value::Boolean(true))));
        for text in ["", "- 1\n- 2", "hello", ": : :"] {
            assert!(matches!(read(text, true), Err(Signal::Unsupported(_))), "{text:?}");
            assert!(matches!(read(text, false), Err(Signal::Error(_))), "{text:?}");
        }
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

    /// A duplicate key refuses through ReadFrom, also nested and in a flow mapping.
    #[test]
    fn a_duplicate_key_refuses() {
        for text in ["a: 1\na: 2", "a:\n  b: 1\n  b: 2", "{a: 1, a: 2}"] {
            assert!(matches!(read(text, false), Err(Signal::Unsupported(_))), "{text:?}");
        }
    }
}
