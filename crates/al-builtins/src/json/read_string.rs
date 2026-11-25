//! A JSON string literal as `ReadFrom` reads it: either quote, the escapes, and the characters of the text as UTF-16 units.

use super::{PResult, ParseError, Parser, Reader};

/// Where a string literal stands: the raw control characters were measured apart for a key, a member value, an array
/// element and a string that is the whole text.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum StringAt {
    Key,
    Member,
    Element,
    Root,
}

impl Parser<'_> {
    fn hex4(&mut self) -> PResult<u16> {
        let h = self.s.get(self.i..self.i + 4).ok_or(ParseError::Invalid)?;
        let h = std::str::from_utf8(h).map_err(|_| ParseError::Invalid)?;
        let v = u16::from_str_radix(h, 16).map_err(|_| ParseError::Invalid)?;
        self.i += 4;
        Ok(v)
    }

    /// Whether `ReadFrom(Text)` was measured to keep the raw control character `b` as part of a string (json/JsonReadFrom.al;
    /// corpus pull request 1327): TAB, LF, CR, 1, 8, 12, 27 and 31 in a value in double quotes, for JsonObject, JsonToken,
    /// JsonArray and JsonValue (:440-509 ObjectReadFromRawControlInString .. ValueReadFromRawControlInString); only TAB in a key
    /// (:549 `tok_tab_in_key`) and in a value in single quotes (:550 `tok_tab_single_quoted`). al-runner issue 7812: a raw TAB in
    /// Microsoft's Sales Lines Suggestions dataset.
    fn raw_control_pinned(&self, b: u8, quote: u8, at: StringAt) -> bool {
        if self.reader != Reader::ReadFromText {
            return false;
        }
        if at == StringAt::Key || quote == b'\'' {
            return b == 9;
        }
        matches!(b, 1 | 8 | 9 | 10 | 12 | 13 | 27 | 31)
    }

    /// Whether the raw control character `b` is kept as part of a string under the assumption `json-readfrom-raw-control`
    /// (al-runner issue 7812): exactly the places tests/al-language/json/JsonReadFromControlChars.al (corpus pull request
    /// 1582) measured. From a Text, codes 1 to 31 (a Text cannot hold code 0, `Text_CharCodeZero_WhatTextKeeps`) in a
    /// member value in either quote and a key in double quotes (JsonObject, JsonToken), an element in either quote
    /// (JsonArray), the whole text in either quote (JsonValue); from an InStream, codes 0 to 31 in double quotes in the
    /// same value places (`*_ReadFromStream_DoubleQuotedValue_*`). Each probe text holds one level: a key of the root object
    /// (depth 1), a member value of the root object or an element of the root array (depth 2), a whole-text string (depth 1).
    fn raw_control_assumed(&self, b: u8, quote: u8, at: StringAt) -> bool {
        use super::ReadInto::{Array, Object, Token, Value};
        use Reader::{ReadFromStream, ReadFromText};
        use StringAt::{Element, Key, Member, Root};
        matches!(
            (self.reader, self.into, at, self.depth, quote, b),
            (ReadFromText, Some(Object | Token), Member, 2, b'"' | b'\'', 1..=0x1f)
                | (ReadFromText, Some(Object | Token), Key, 1, b'"', 1..=0x1f)
                | (ReadFromText, Some(Array), Element, 2, b'"' | b'\'', 1..=0x1f)
                | (ReadFromText, Some(Value), Root, 1, b'"' | b'\'', 1..=0x1f)
                | (ReadFromStream, Some(Object | Token), Member, 2, b'"', 0..=0x1f)
                | (ReadFromStream, Some(Array), Element, 2, b'"', 0..=0x1f)
                | (ReadFromStream, Some(Value), Root, 1, b'"', 0..=0x1f)
        )
    }

    pub(super) fn string(&mut self, at: StringAt) -> PResult<String> {
        // Either quote opens a string and the same one closes it (`{'a':1}`, `single_quotes`).
        let quote = self.s[self.i];
        self.i += 1;
        let mut units: Vec<u16> = Vec::new();
        loop {
            let b = *self.s.get(self.i).ok_or(ParseError::Invalid)?;
            match b {
                q if q == quote => {
                    self.i += 1;
                    break;
                }
                b'\\' => {
                    self.i += 1;
                    let e = *self.s.get(self.i).ok_or(ParseError::Invalid)?;
                    self.i += 1;
                    let c = match e {
                        b'"' => '"' as u16,
                        b'\'' => '\'' as u16,
                        b'\\' => '\\' as u16,
                        b'/' => '/' as u16,
                        b'b' => 8,
                        b'f' => 12,
                        b'n' => 10,
                        b'r' => 13,
                        b't' => 9,
                        b'u' => self.hex4()?,
                        _ => return Err(ParseError::Invalid),
                    };
                    units.push(c);
                }
                0..=0x1f if self.raw_control_pinned(b, quote, at) => {
                    units.push(u16::from(b));
                    self.i += 1;
                }
                0..=0x1f if self.raw_control_assumed(b, quote, at) => {
                    crate::assumptions::assume(crate::assumptions::Assumption::JsonReadfromRawControl);
                    units.push(u16::from(b));
                    self.i += 1;
                }
                0..=0x1f => {
                    return Err(ParseError::Unsupported(
                        "a raw control character inside a JSON string where it was not measured (not pinned)",
                    ));
                }
                _ => {
                    // A UTF-8 sequence copied through as UTF-16.
                    let len = match b {
                        0..=0x7f => 1,
                        0xc0..=0xdf => 2,
                        0xe0..=0xef => 3,
                        _ => 4,
                    };
                    let seq = self.s.get(self.i..self.i + len).ok_or(ParseError::Invalid)?;
                    let ch = std::str::from_utf8(seq)
                        .map_err(|_| ParseError::Invalid)?
                        .chars()
                        .next()
                        .ok_or(ParseError::Invalid)?;
                    let mut buf = [0u16; 2];
                    units.extend_from_slice(ch.encode_utf16(&mut buf));
                    self.i += len;
                }
            }
        }
        // `"\uD800"` alone is half of a surrogate pair. Real BC keeps it; this runtime cannot hold it until ADR-068 is
        // built, and U+FFFD would be a guessed value (hard rule 4).
        String::from_utf16(&units).map_err(|_| {
            ParseError::Unsupported("a JSON \\u escape that ends up with half of a surrogate pair (not pinned)")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::{ReadInto, Reader, parse, parse_by, parse_read_from};
    use crate::native::Signal;

    fn assumed(text: &str, reader: Reader, into: ReadInto) -> bool {
        crate::assumptions::clear_hits();
        matches!(parse_by(text, reader, Some(into)), Ok(Some(_)))
            && crate::assumptions::hit(crate::assumptions::Assumption::JsonReadfromRawControl)
    }

    fn refused(text: &str, reader: Reader, into: ReadInto) -> bool {
        matches!(&parse_by(text, reader, Some(into)), Err(Signal::Unsupported(w)) if w.contains("raw control character"))
    }

    /// The pinned raw control characters read as part of the string; the other codes read under the assumption
    /// `json-readfrom-raw-control` in exactly the places corpus pull request 1582 measured; every other place, code 0 in a
    /// Text and the readers other than ReadFrom are refused, not read as false.
    #[test]
    fn a_raw_control_character_reads_only_where_measured() {
        for c in [1u8, 8, 9, 10, 12, 13, 27, 31] {
            let text = format!("{{\"a\":\"x{}y\"}}", c as char);
            assert!(matches!(parse_read_from(&text, Some("object")), Ok(Some(_))), "{c}");
            // Only ReadFrom was measured.
            assert!(matches!(parse(&text), Err(Signal::Unsupported(_))), "{c}");
        }
        use ReadInto::{Array, Object, Token, Value};
        use Reader::{ReadFromStream as S, ReadFromText as T};
        for c in [2u8, 7, 11, 14, 21, 26, 30] {
            let c = c as char;
            for (text, reader, into) in [
                (format!("{{\"a\":\"x{c}y\"}}"), T, Object),
                (format!("{{\"a\":\"x{c}y\"}}"), T, Token),
                (format!("[\"x{c}y\"]"), T, Array),
                (format!("\"x{c}y\""), T, Value),
                (format!("{{\"k{c}z\":\"v\"}}"), T, Object),
                (format!("{{\"k{c}z\":\"v\"}}"), T, Token),
                (format!("{{'a':'x{c}y'}}"), T, Object),
                (format!("{{'a':'x{c}y'}}"), T, Token),
                (format!("['x{c}y']"), T, Array),
                (format!("'x{c}y'"), T, Value),
                (format!("{{\"a\":\"x{c}y\"}}"), S, Object),
                (format!("{{\"a\":\"x{c}y\"}}"), S, Token),
                (format!("[\"x{c}y\"]"), S, Array),
                (format!("\"x{c}y\""), S, Value),
            ] {
                assert!(assumed(&text, reader, into), "{text:?} {reader:?} {into:?}");
                assert!(matches!(parse(&text), Err(Signal::Unsupported(_))), "{text:?}");
            }
            // Places the probe did not measure.
            for (text, reader, into) in [
                (format!("{{'k{c}z':'v'}}"), T, Object),
                (format!("[{{\"k{c}z\":1}}]"), T, Array),
                (format!("{{\"a\":[\"x{c}y\"]}}"), T, Object),
                (format!("{{\"o\":{{\"a\":\"x{c}y\"}}}}"), T, Object),
                (format!("{{\"o\":{{\"k{c}z\":1}}}}"), T, Token),
                (format!("[[\"x{c}y\"]]"), S, Array),
                (format!("[\"x{c}y\"]"), T, Token),
                (format!("\"x{c}y\""), T, Token),
                (format!("{{\"a\":\"x{c}y\"}}"), T, Array),
                (format!("{{\"k{c}z\":\"v\"}}"), S, Object),
                (format!("{{'a':'x{c}y'}}"), S, Object),
            ] {
                assert!(refused(&text, reader, into), "{text:?} {reader:?} {into:?}");
            }
        }
        // Code 0: kept from an InStream; a Text cannot hold it in BC (the probe's `text1_after_c1_0`), so it is refused.
        assert!(assumed("{\"a\":\"x\u{0}y\"}", S, Object));
        assert!(refused("{\"a\":\"x\u{0}y\"}", T, Object));
        // Between tokens a form feed is a blank and character 1 is not JSON (json/JsonReadFrom.al ReadFromRawControlOtherPlaces).
        assert!(matches!(parse_read_from("{\"a\":\u{c}\"v\"}", Some("object")), Ok(Some(_))));
        assert!(matches!(parse_read_from("{\"a\":\u{1}\"v\"}", Some("object")), Ok(None)));
    }
}
