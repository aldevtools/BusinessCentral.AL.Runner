//! `InStream.Read(Text, Length)`: what one read takes from the stream, per encoding (al-runner issue 7113).

use crate::native::Signal;
use crate::stream::{Encoding, InState};

/// What one `Read(Text, Length)` gave: the text and the count the call answers.
pub(crate) struct LenRead {
    pub text: String,
    pub count: usize,
}

fn unsupported<T>(what: &str) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

fn invalid_data() -> Signal {
    Signal::Error(Box::new(crate::native::AlError {
        message: "Invalid data encountered in stream, unable to read text.".into(),
        code: "InvalidStreamData".into(),
        call_stack: String::new(),
        ..Default::default()
    }))
}

/// How many bytes later reads of the rest of the stream are checked for being UTF-8 (see [`utf8`]).
const RESTCHECK: usize = 1024;

/// A UTF-8 stream: Length counts characters, a surrogate pair once (streams/W7StreamRead.al:268-277 ReadTextWithLength_Utf8Sequences;
/// streams/instream-outstream/StreamUtf8CharsRead.al:242-258 Utf8_Emoji_ReadVariants `Read(Text,1)`, :277-282
/// Utf8_CjkRun_ThreeChars_LengthArguments). The count is the bytes of the text plus one, also for Length 0 and at the end
/// (W7StreamRead.al:296-303, :358-370; StreamReadMatrix.al:149 `u8 len 0`, :150 `u8 len 3`). The position moves by the bytes of
/// the characters read (W7StreamRead.al:358-370 ReadTextWithLength_Utf8_WhatIsConsumed).
/// - A zero byte stops the read and is consumed (W7StreamRead.al:296-297; StreamReadMatrix.al:415 Matrix_Utf16Ab `u8 len 3`: the rest
///   reads `b`); one after the last character is left (:180 Matrix_AbcZero `u8 len 3`, `eos No rest 1 []`).
/// - A byte order mark at the start is skipped and not counted (:535 Matrix_Utf8Bom `u8 len 3`); with Length 0 the stream answers
///   '' and 1 and the next read skips it (:534 `u8 len 0`), so only the Position in between is not pinned.
/// - A lead byte left alone at the end is dropped (:355 Matrix_TruncatedUtf8 `u8 len 3`, :296 Matrix_EacuteLatin1); the byte FF
///   raises (:325-327 Matrix_ByteFF `u8 len 3`).
///
/// Not pinned, refused: a negative Length, other bytes that are not UTF-8, a cut sequence with continuation bytes, and invalid bytes
/// in the rest of the stream after what was read (the tier decodes ahead, so they might raise in this read).
pub(crate) fn utf8(s: &mut InState, n: i32) -> Result<LenRead, Signal> {
    let Ok(n) = usize::try_from(n) else {
        return unsupported("InStream.Read(Text, Length) with a negative length on a UTF-8 stream (not pinned)");
    };
    let rest = s.rest();
    let mark = s.pos == 0 && rest.starts_with(&[0xEF, 0xBB, 0xBF]);
    if mark && n == 0 {
        s.len0_over_mark = true;
        return Ok(LenRead {
            text: String::new(),
            count: 1,
        });
    }
    let start = if mark { 3 } else { 0 };
    let body = &rest[start..];
    let (mut i, mut chars, mut extra) = (0, 0, 0);
    while chars < n && i < body.len() {
        let need = match body[i] {
            0 => {
                extra = 1;
                break;
            }
            0x01..=0x7F => 1,
            0xC2..=0xDF => 2,
            0xE0..=0xEF => 3,
            0xF0..=0xF4 => 4,
            0xFF => return Err(invalid_data()),
            _ => {
                return unsupported(
                    "InStream.Read(Text, Length) of a byte that cannot start a UTF-8 character (not pinned)",
                );
            }
        };
        if i + need > body.len() {
            if i + 1 == body.len() {
                extra = 1;
                break;
            }
            return unsupported(
                "InStream.Read(Text, Length) of a UTF-8 character cut at the end of the stream (not pinned)",
            );
        }
        if std::str::from_utf8(&body[i..i + need]).is_err() {
            return unsupported("InStream.Read(Text, Length) of bytes that are not UTF-8 (not pinned)");
        }
        i += need;
        chars += 1;
    }
    let after = &body[i + extra..];
    let window = &after[..after.len().min(RESTCHECK)];
    // With Length 0 bytes after the read do not raise (StreamReadMatrix.al:324 Matrix_ByteFF `u8 len 0`, `1 [] eos No rest error`).
    if n > 0
        && let Err(e) = std::str::from_utf8(window)
        // A window that ends inside a character is fine; anything else is not pinned.
        && (e.error_len().is_some() || window.len() == after.len())
    {
        return unsupported(
            "InStream.Read(Text, Length) on a UTF-8 stream whose rest holds bytes that are not UTF-8 (not pinned)",
        );
    }
    let text = String::from_utf8_lossy(&body[..i]).into_owned();
    s.pos += start + i + extra;
    Ok(LenRead { text, count: i + 1 })
}

/// A Windows or MS-DOS stream: Length bytes, one character each (xml/TestXmlDocumentWriteToOutStreamContracts.al:LeadingBytes), the
/// count the bytes consumed (W7StreamRead.al:237-249 ReadTextWithLength_BytesFrom128To159, also past the end: `Windows 100 bytes` 8;
/// :278-281). A zero byte where the read stops, met before Length bytes or right after them, is consumed and counted
/// (W7StreamRead.al:298-299 `Windows 5 bytes with a zero byte` 3 and 97 98; StreamReadMatrix.al:189 Matrix_AbcZero `win len 3`
/// `4 ... eos Yes`, :222 Matrix_AbcZeroDef `win len 3` with the rest `def`, :249-250 Matrix_TwoZeros `win len 0` and `win len 3`,
/// :186, :219, :247 the same on MS-DOS). Bytes 128 to 159 read through code page 1252 on Windows (W7StreamRead.al:237-249).
pub(crate) fn single_byte(s: &mut InState, n: i32) -> Result<LenRead, Signal> {
    let Ok(n) = usize::try_from(n) else {
        return unsupported("InStream.Read(Text, Length) with a negative length (not pinned)");
    };
    let rest = s.rest();
    let i = rest.iter().take(n).position(|b| *b == 0).unwrap_or(n.min(rest.len()));
    let taken = &rest[..i];
    if !s.enc.decode_pinned(taken) {
        return unsupported("InStream.Read(Text, Length) of MS-DOS bytes whose character is not pinned");
    }
    let consumed = i + usize::from(rest.get(i) == Some(&0));
    let text = s.enc.decode(taken);
    s.pos += consumed;
    debug_assert!(matches!(s.enc, Encoding::Windows | Encoding::MsDos));
    Ok(LenRead { text, count: consumed })
}
