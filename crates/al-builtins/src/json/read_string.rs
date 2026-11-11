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
                // A raw TAB is part of the string: assumed from Microsoft's Sales Lines Suggestions test app, whose install
                // codeunit reads JSONL lines with a raw TAB in a string through JsonToken.ReadFrom (al-runner issue 7812). The
                // probe is json/JsonReadFrom.al *ReadFromRawControlInString (corpus pull request 1327); it wins when pinned.
                b'\t' => {
                    crate::assumptions::assume(crate::assumptions::Assumption::JsonRawTabInString);
                    units.push(9);
                    self.i += 1;
                }
                // The other raw control characters: not valid here (main's rule; the same probe measures them).
                0..=0x1f => return Err(ParseError::Invalid),
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
