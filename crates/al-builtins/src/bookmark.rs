//! The bookmark of a record's primary key: the `bookmark=` part of `GetUrl` (escaped there) and `Format(RecordId, 0, 10)`
//! (unescaped), read back by `Evaluate(RecordId, Text, 10)`. The text is `<size>;<base64>`: the table number (4 bytes, little
//! endian), then per key field a Code as `02 7b <marker>` and its UTF-16 characters ended by two zero bytes, an Enum or Option
//! as `00 8b` and its number, an Integer as `00 87` and its number; zero bytes at the end are dropped. `<size>` counts 6, then
//! 5 plus 2 per character for a Code and 6 for a number.

use crate::value::{RtType, Value};

/// The key shapes whose bookmark GetUrl pins: one Code[10], Code[20] or Code[50] field, one Integer field, a Code[20] and an
/// Integer field, two Code[20] fields (platform/lt-t-geturl/TGetUrl.al:439-447 GetUrl_Web_BookmarkOfCodeKeys), an Enum or
/// Option field and a Code[20] field (platform/lt-system-database-members/GetUrlBookmarkEnumKey.al:80-170), or one Enum field.
pub fn url_shape_pinned(shape: &[&RtType]) -> bool {
    matches!(
        shape,
        [RtType::Code(Some(10 | 20 | 50))]
            | [RtType::Integer]
            | [RtType::Code(Some(20)), RtType::Integer]
            | [RtType::Code(Some(20)), RtType::Code(Some(20))]
            | [RtType::Enum(_) | RtType::Option(_), RtType::Code(Some(20))]
            | [RtType::Enum(_)]
    )
}

/// The key shapes `Format(RecordId, 0, 10)` and `Evaluate(RecordId, Text, 10)` take: those of GetUrl (the text is the same,
/// types/lt-y49-recordid-format10/Y49RecordIdFormat10.al:117-148 `A-1` and `A-1 url`) and an Option, a Code[20] and an
/// Integer field (:134 `B line`).
pub fn record_id_shape_pinned(shape: &[&RtType]) -> bool {
    url_shape_pinned(shape) || matches!(shape, [RtType::Option(_), RtType::Code(Some(20)), RtType::Integer])
}

/// The size and the bytes (zero bytes at the end dropped) of the bookmark of table `table_id` with the key `values` of the
/// types `shape`; `Err` names a key value whose form is not pinned.
pub fn encode(table_id: u32, shape: &[&RtType], values: &[&Value]) -> Result<(usize, Vec<u8>), String> {
    let mut data: Vec<u8> = table_id.to_le_bytes().to_vec();
    let mut size = 6usize;
    let two_codes = shape.len() == 2 && matches!(shape[1], RtType::Code(_));
    // An empty Code after an Enum or Option field is pinned (GetUrlBookmarkEnumKey.al:102, :151).
    let enum_code = shape.len() == 2 && matches!(shape[0], RtType::Enum(_) | RtType::Option(_));
    // A key of one Code[20] field: every Code that starts with a digit is pinned (GetUrlBookmarkDigitCode.al:117-163,
    // corpus master).
    let one_code20 = matches!(shape, [RtType::Code(Some(20))]);
    for v in values {
        match v {
            Value::Code(c) => {
                let s = c.as_str();
                if !s.is_ascii() {
                    return Err("GetUrl bookmark of a Code value with a non-ASCII character (not pinned)".into());
                }
                if s.is_empty() && shape.len() > 1 && !two_codes && !enum_code {
                    return Err("GetUrl bookmark of an empty Code in a two-field key (not pinned)".into());
                }
                // A non-empty Code is marked 0xff, an empty one 0x00 (TGetUrl.al:446-447 `two second empty`,
                // `two first empty`). A Code of six digits is marked 0x06 (GetUrlBookmarkEnumKey.al:149-150
                // `order 106001`); a Code that starts with a digit in another form is not pinned, except in a key of one
                // Code[20] field: there a Code of digits only is marked with its digit count (`1` 0x01, `007` 0x03, 20
                // digits 0x14) and digits followed by other characters (`1A`, `12345A`, `1-2`, `1 2`) 0xff
                // (platform/lt-system-database-members/GetUrlBookmarkDigitCode.al:117-163, corpus master).
                let marker = if s.is_empty() {
                    0x00
                } else if one_code20 && s.starts_with(|c: char| c.is_ascii_digit()) {
                    if s.bytes().all(|b| b.is_ascii_digit()) {
                        s.len() as u8
                    } else {
                        0xff
                    }
                } else if s.starts_with(|c: char| c.is_ascii_digit()) {
                    if s.len() != 6 || !s.bytes().all(|b| b.is_ascii_digit()) {
                        return Err(
                            "GetUrl bookmark of a Code value that starts with a digit (pinned only for six digits)"
                                .into(),
                        );
                    }
                    0x06
                } else {
                    0xff
                };
                data.extend_from_slice(&[0x02, 0x7b, marker]);
                for b in s.bytes() {
                    data.extend_from_slice(&[b, 0]);
                }
                data.extend_from_slice(&[0, 0]);
                size += 5 + 2 * s.len();
            }
            // An Enum field is marked 0x8b and carries the value's number, an Option field its ordinal
            // (GetUrlBookmarkEnumKey.al:99-104 `fifth A1` = 5, :168-169 `fifth A1` = 2). An Enum as the only key field: its
            // zero bytes at the end are dropped like any (`pWQBAACL` for 0, `...AQ==` for 1; GetUrlBookmarkDigitCode.al:166-184,
            // corpus master).
            Value::Enum(_, n) | Value::Option(n, _) => {
                data.extend_from_slice(&[0x00, 0x8b]);
                data.extend_from_slice(&n.to_le_bytes());
                size += 6;
            }
            Value::Integer(n) => {
                data.extend_from_slice(&[0x00, 0x87]);
                data.extend_from_slice(&n.to_le_bytes());
                size += 6;
            }
            other => {
                return Err(format!(
                    "GetUrl bookmark of a key value {}",
                    crate::describe::Describe::describe(*other)
                ));
            }
        }
    }
    while data.last() == Some(&0) {
        data.pop();
    }
    Ok((size, data))
}

/// What `Evaluate(RecordId, Text, 10)` reads from a text.
pub enum Read {
    /// The text has no `;`: it is no bookmark (Y49RecordIdFormat10.al:223-262 `garbage`, `format0 text`).
    NotABookmark,
    /// The table number and the bytes after it, padded with zero bytes as far as a key could reach.
    Bookmark { table_id: u32, rest: Vec<u8> },
    /// A form that is not pinned.
    Refused(String),
}

/// Splits a bookmark text. The size before the `;` is not checked (`17;D2sBAAJ7`, a size that does not fit, reads; :236).
pub fn read(text: &str) -> Read {
    let Some((size, b64)) = text.split_once(';') else {
        return Read::NotABookmark;
    };
    if size.is_empty() || !size.bytes().all(|b| b.is_ascii_digit()) {
        return Read::Refused("Evaluate of a bookmark text whose size is no number (not pinned)".into());
    }
    let Ok(mut bytes) = crate::platform_native::b64_decode(b64) else {
        return Read::Refused("Evaluate of a bookmark text that is no base64 (not pinned)".into());
    };
    if bytes.len() < 4 {
        return Read::Refused("Evaluate of a bookmark text without a table number (not pinned)".into());
    }
    let rest = bytes.split_off(4);
    Read::Bookmark {
        table_id: u32::from_le_bytes(bytes.try_into().expect("4 bytes")),
        rest,
    }
}

/// The key values of `shape` the bytes after the table number hold (`Some`), or `None` when they do not match the key.
/// Missing bytes are zero: a Code whose characters are cut off is empty (Y49RecordIdFormat10.al:236 `bookmark with wrong
/// length` reads `LT Y49 Bm Code Row: ""`).
pub fn decode_key(shape: &[&RtType], rest: &[u8]) -> Option<Vec<Value>> {
    let mut at = 0usize;
    let byte = |i: usize| rest.get(i).copied().unwrap_or(0);
    let mut key = Vec::new();
    for ty in shape {
        match ty {
            RtType::Code(_) => {
                if (byte(at), byte(at + 1)) != (0x02, 0x7b) {
                    return None;
                }
                at += 3;
                let mut s = String::new();
                loop {
                    let (lo, hi) = (byte(at), byte(at + 1));
                    at += 2;
                    if (lo, hi) == (0, 0) {
                        break;
                    }
                    if hi != 0 || !lo.is_ascii() {
                        return None;
                    }
                    s.push(lo as char);
                }
                key.push(Value::Code(s.into()));
            }
            RtType::Option(_) | RtType::Enum(_) | RtType::Integer => {
                let tag = if matches!(ty, RtType::Integer) { 0x87 } else { 0x8b };
                if (byte(at), byte(at + 1)) != (0x00, tag) {
                    return None;
                }
                let n = i32::from_le_bytes([byte(at + 2), byte(at + 3), byte(at + 4), byte(at + 5)]);
                at += 6;
                key.push(match ty {
                    RtType::Option(names) => Value::Option(n, std::sync::Arc::from(names.as_slice())),
                    RtType::Enum(id) => Value::Enum(*id, n),
                    _ => Value::Integer(n),
                });
            }
            _ => return None,
        }
    }
    (at >= rest.len()).then_some(key)
}
