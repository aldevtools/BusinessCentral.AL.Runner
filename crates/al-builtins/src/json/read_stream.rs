//! `ReadFrom(InStream)` of the four JSON types: the text the stream holds, and what each type does with a stream whose
//! first token is a string.

use super::{JsonNode, ReadInto, Reader, Scalar, error, parse_at, with_node};
use crate::native::{Ctx, Signal, unsupported};
use crate::value::Value;

/// The rest of the text of `stream`, for `ReadFrom(InStream)` of the type `want` names (`read_from`). A stream whose first
/// token is a string reads only where it is pinned:
/// - JsonObject, statement form: the error "JSON tokens of type 'String' ..." (what is left after a partial read of
///   `{"a":2}`, streams/ProbeJsonAndInStreamBytes.al:146-170, `ReadFrom_InStream_AfterPartialRead_ReadsTheRest`);
/// - JsonValue, result used, the stream holds one whole string: read as that value (json/json/P101Streams.al:71,
///   :114-116, `read_ok` Yes, `read_back` "hello");
/// - JsonToken, result used, a string followed by more text: read as that value (json/json/P101Streams.al:162-167,
///   `text_ok` Yes, `text_isvalue` Yes; `"text"` is written over `{"a":[1,2,{"b":null}]}`, and CreateOutStream does not
///   empty a Blob, streams/TestStreamsRuntimeContracts.al:BlobField_CreateOutStream_DoesNotEmptyExistingData, so the stream
///   holds `"text"1,2,{"b":null}]}`).
///
/// Every other shape is refused: a JsonArray, the JsonObject result used, the JsonValue and JsonToken statement forms, a
/// JsonToken string alone, and a JsonValue string followed by more text.
pub(super) fn stream_text(c: &mut dyn Ctx, stream: &mut Value, want: Option<&str>) -> Result<String, Signal> {
    let text = crate::stream::read_serialised_text(stream)?;
    if !text.trim_start().starts_with('"') {
        return Ok(text);
    }
    let into = ReadInto::of(want);
    match (into, c.ret_used()) {
        (ReadInto::Object, false) => Err(error(
            "JSON tokens of type 'String' are not supported by this implementation.",
            "InvalidJsonType",
        )),
        (ReadInto::Value, true) if first_string(&text, into)? == Some(true) => Ok(text),
        (ReadInto::Token, true) if first_string(&text, into)? == Some(false) => Ok(text),
        _ => unsupported(
            "ReadFrom(InStream) on a stream that starts with a string token, for this type, result use or rest of the \
             text (not pinned)",
        ),
    }
}

/// For a `text` whose first value is a JSON string, as `ReadFrom(InStream)` of `into` reads it: whether only blanks follow
/// it (`Some(true)`) or more text (`Some(false)`); `None` when the first value is no string.
fn first_string(text: &str, into: ReadInto) -> Result<Option<bool>, Signal> {
    Ok(match parse_at(text, Reader::ReadFromStream, Some(into))? {
        Some((h, whole)) if with_node(&h, |n| matches!(n, JsonNode::Scalar(Scalar::Val(Value::Text(_))))) => Some(whole),
        _ => None,
    })
}
