//! `JsonObject.WriteToYaml` and `JsonObject.ReadFromYaml`.

use super::{Handle, JsonNode, Scalar, read_from, recv, with_node, write};
use crate::native::{Ctx, Signal, unsupported};
use crate::value::Value;

mod read;
use read::{Yaml, document};

/// The YAML forms the corpus pins (json/TestJsonRuntimeContracts.al:JsonObject_Yaml_RoundTripsOnThisTier,
/// json/TestProbeJsonContracts.al:Yaml_WriteNested): `key: value` lines, a nested object as `key:` and its
/// members indented by two spaces, an array as `key:` and `- item` lines at the key's own indent. A text is quoted with
/// single quotes (doubled inside) when it is empty, starts with `:` or holds `: ` or ` #`; `123` and `true` as texts stay
/// bare. Keys other than letters, digits and `_`, and a container inside an array other than an object of scalars (`array_item`), are refused.
fn plain_yaml_key(k: &str) -> bool {
    !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn yaml_text(t: &str) -> String {
    if t.is_empty() || t.starts_with(':') || t.contains(": ") || t.contains(" #") {
        format!("'{}'", t.replace('\'', "''"))
    } else {
        t.to_string()
    }
}

fn yaml_scalar(c: &Handle) -> Result<String, Signal> {
    let scalar = with_node(c, |n| match n {
        JsonNode::Scalar(Scalar::Val(v)) => Some(v.clone()),
        _ => None,
    });
    match scalar {
        Some(Value::Text(t) | Value::Code(t)) => Ok(yaml_text(&t)),
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

fn yaml_members(kids: &[(String, Handle)], indent: usize, out: &mut String) -> Result<(), Signal> {
    let pad = " ".repeat(indent);
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
            Kind::Other => out.push_str(&format!("{pad}{k}: {}\r\n", yaml_scalar(c)?)),
        }
    }
    Ok(())
}

/// An array item: `- ` and a scalar, or an object of scalars whose first member follows the `- ` and whose other
/// members are indented by two more (json/json/P88ObjectPathYaml.al:164 `- x: 1\r\n  y: two`; json/DcJsonLeads.al:208
/// `  - name: b\r\n    run: c`). An empty object, an array, or an object holding a container inside an array is not pinned.
fn array_item(item: &Handle, pad: &str, out: &mut String) -> Result<(), Signal> {
    let members = with_node(item, |n| match n {
        JsonNode::Object(m) if !m.is_empty() => Some(Some(m.clone())),
        JsonNode::Scalar(_) => Some(None),
        _ => None,
    });
    match members {
        Some(None) => out.push_str(&format!("{pad}- {}\r\n", yaml_scalar(item)?)),
        Some(Some(members)) => {
            for (i, (k, c)) in members.iter().enumerate() {
                if !plain_yaml_key(k) {
                    return unsupported(format!("WriteToYaml with the key `{k}` (quoting is not pinned)"));
                }
                if with_node(c, |n| !matches!(n, JsonNode::Scalar(_))) {
                    return unsupported("WriteToYaml of a container in an object inside an array (not pinned)");
                }
                let lead = if i == 0 { "- " } else { "  " };
                out.push_str(&format!("{pad}{lead}{k}: {}\r\n", yaml_scalar(c)?));
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
    yaml_members(&kids, 0, &mut out)?;
    a[0] = Value::Text(out.into());
    Ok(Value::Boolean(true))
}

/// `ReadFromYaml(Text)`: the text is read into a YAML tree (`read::document`, which owns every pinned shape and refuses
/// the rest), written as JSON and read like `ReadFrom` into an object. An empty text, a list and a single word are no
/// object and fail with ReadFrom's InvalidJsonType error, the receiver left as it was (json/json/P88ObjectPathYaml.al:225,
/// 228-229; json/PnAJsonYaml.al:173-176).
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
        for json in [
            r#"{"l":[[1]]}"#,
            r#"{"l":[{}]}"#,
            r#"{"l":[{"x":{"y":1}}]}"#,
            r#"{"l":[{"a b":1}]}"#,
        ] {
            assert!(matches!(yaml(json), Err(Signal::Unsupported(_))), "{json}");
        }
    }
}
