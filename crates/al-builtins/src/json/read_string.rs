//! A JSON string literal as `ReadFrom` reads it: either quote, the escapes, and the characters of the text as UTF-16 units.

use super::{PResult, ParseError, Parser};

impl Parser<'_> {
    fn hex4(&mut self) -> PResult<u16> {
        let h = self.s.get(self.i..self.i + 4).ok_or(ParseError::Invalid)?;
        let h = std::str::from_utf8(h).map_err(|_| ParseError::Invalid)?;
        let v = u16::from_str_radix(h, 16).map_err(|_| ParseError::Invalid)?;
        self.i += 4;
        Ok(v)
    }

    pub(super) fn string(&mut self) -> PResult<String> {
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
                // A raw control character is part of the string, in every ReadFrom, in a key and in a single-quoted string:
                // json/JsonReadFrom.al:440-555 (TAB, LF, CR, 1, 8, 12, 27, 31; corpus pull request 1327, needs the bump).
                // al-runner issue 7812: a raw TAB in Microsoft's Sales Lines Suggestions dataset.
                1 | 8 | 9 | 10 | 12 | 13 | 27 | 31 => {
                    units.push(u16::from(b));
                    self.i += 1;
                }
                // The other control characters are not measured.
                0..=0x1f => {
                    return Err(ParseError::Unsupported(
                        "a raw control character inside a JSON string other than 1, 8, 9, 10, 12, 13, 27 and 31 (not pinned)",
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
    use super::super::parse;
    use crate::native::Signal;

    /// The measured raw control characters read as part of the string; the others are refused, not read as false.
    #[test]
    fn a_raw_control_character_reads_only_where_measured() {
        for c in [1u8, 8, 9, 10, 12, 13, 27, 31] {
            let text = format!("{{\"a\":\"x{}y\"}}", c as char);
            assert!(matches!(parse(&text), Ok(Some(_))), "{c}");
        }
        for c in [2u8, 7, 11, 14, 26, 30] {
            let text = format!("{{\"a\":\"x{}y\"}}", c as char);
            assert!(
                matches!(&parse(&text), Err(Signal::Unsupported(w)) if w.contains("raw control character")),
                "{c}"
            );
        }
        // Between tokens a form feed is a blank and character 1 is not JSON (json/JsonReadFrom.al ReadFromRawControlOtherPlaces).
        assert!(matches!(parse("{\"a\":\u{c}\"v\"}"), Ok(Some(_))));
        assert!(matches!(parse("{\"a\":\u{1}\"v\"}"), Ok(None)));
    }
}
