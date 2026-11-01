//! `Format` of a JsonObject, JsonArray, JsonValue or JsonToken (moved out of `format::format_value`).

use crate::format::FormatFail;
use crate::json::{Handle, JsonNode, Scalar};
use crate::value::{LibraryValue, Value};

/// A JSON value formats as its compact text. Pinned: an empty JsonObject is `{}` and a null token `null`
/// (types/ProbeTypeFormsAndFilters.al:Clear_JsonTypes_GiveFreshUnassigned, BC 27.5); an object or array
/// gives its compact text; any other value is not pinned.
pub(crate) fn format_json(name: &str, l: &Handle, standard: i32) -> Result<String, FormatFail> {
    if let LibraryValue::Json(c) = &*l.borrow() {
        if let Some(why) = c.unpinned {
            return Err(FormatFail::Unsupported(why.to_string()));
        }
    }
    let container = matches!(&*l.borrow(), LibraryValue::Json(c)
        if matches!(&c.node, JsonNode::Object(k) if !k.is_empty()) || matches!(&c.node, JsonNode::Array(_)));
    if container {
        return write(l);
    }
    if name == "JsonValue" && matches!(&*l.borrow(), LibraryValue::Json(c) if matches!(c.node, JsonNode::Scalar(_))) {
        return json_value_scalar(l, standard);
    }
    let out = match &*l.borrow() {
        LibraryValue::Json(c) => match &c.node {
            JsonNode::Object(k) if k.is_empty() => "{}".to_string(),
            // {"a":1} formats as `{"a":1}` (streams/ProbeJsonAndInStreamBytes.al:ReadFrom_InStream_WithBomAndWithout).
            JsonNode::Object(_) | JsonNode::Array(_) => String::new(),
            JsonNode::Scalar(Scalar::Null) => "null".to_string(),
            // A JsonToken holding a whole number formats as its digits (json/lt-ay-json/AyJsonTokenSelectTokens.al:
            // 64-66 `1@a[0].n`, :116 `7@`).
            JsonNode::Scalar(Scalar::Val(Value::Integer(n))) if name == "JsonToken" && standard == 0 => n.to_string(),
            JsonNode::Scalar(Scalar::Val(Value::BigInteger(n))) if name == "JsonToken" && standard == 0 => {
                n.to_string()
            }
            _ => {
                return Err(
                    format!("Format of {name} with content (standard format {standard}) is not pinned yet").into(),
                );
            }
        },
        LibraryValue::Empty if name == "JsonObject" => "{}".to_string(),
        // A default JsonArray is `[]` (misc/lt-probes-needed-5/PnE09Platform.al:163,172 JsonValue_NeverTouched); a cleared
        // one is a fresh default one (types/ProbeTypeFormsAndFilters.al:Clear_JsonTypes_GiveFreshUnassigned). al-runner
        // issue 5956.
        LibraryValue::Empty if name == "JsonArray" && standard == 0 => "[]".to_string(),
        // Standard format 9 of a JsonArray nobody touched raises (json/JsonWriteTo.al FormatOfEmptyJsonArrays
        // `untouched format 9`).
        LibraryValue::Empty if name == "JsonArray" && standard == 9 => {
            return Err(crate::format::no_such_format(9, "JsonArray"));
        }
        LibraryValue::Empty if matches!(name, "JsonValue" | "JsonToken") => "null".to_string(),
        _ => return Err(format!("Format of a {name} (standard format {standard}) is not pinned yet").into()),
    };
    Ok(out)
}

/// A JsonValue that holds a value (or was set to null) formats as its WriteTo text at standard formats 0, 1 and 2,
/// for every kind of value: numbers, Char and Option as numbers, text quoted and escaped, dates and times as JSON
/// strings; standard format 9 raises InvalidStandardFormat, also for null (json/lt-y44-format-jsonvalue/
/// Y44FormatJsonValue.al FormatJsonValue_Numbers :99-118, _TextLike :147-158, _DateTime :188-199,
/// _StandardFormats_Scalars :229-244, _StandardFormats_DatesAndNull :263-274). Formats 3 to 8 are not pinned.
fn json_value_scalar(l: &Handle, standard: i32) -> Result<String, FormatFail> {
    match standard {
        0..=2 => write(l),
        9 => Err(crate::format::no_such_format(9, "JsonValue")),
        _ => Err(format!("Format of JsonValue with content (standard format {standard}) is not pinned yet").into()),
    }
}

fn write(l: &Handle) -> Result<String, FormatFail> {
    let mut out = String::new();
    match crate::json::write_text(l, &mut out) {
        Ok(()) => Ok(out),
        Err(crate::native::Signal::Unsupported(why)) => Err(FormatFail::Unsupported(why)),
        Err(_) => Err("Format of a JSON value the writer refuses".to_string().into()),
    }
}
