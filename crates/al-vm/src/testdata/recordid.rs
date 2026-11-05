//! A `RecordId` field of a restored row: BC stores it in a varbinary column as the table number and the primary key
//! values. The layout:
//!
//! - the table number, 4 bytes little endian;
//! - per key field a two-byte tag and the value: `00 87` and 4 bytes little endian for an Integer, `00 8b` and 4 bytes
//!   for an Option or Enum (its ordinal), `00 91` and 16 bytes for a Guid (.NET byte order), `02 7b` and one marker byte
//!   for a Code, `01 7b` for a Text, each followed by its UTF-16 characters and two zero bytes;
//! - `00 00` at the end (and only zero bytes after it).
//!
//! The Code marker is 0xff, the digit count of a Code of digits only, or 0x00 for an empty Code; it carries no part of the
//! value.
//!
//! Where it comes from: the bytes after the table number are the bytes of `Format(RecordId, 0, 10)` (corpus-pinned for
//! Code, Integer, Option and Enum keys, `al_builtins::bookmark`: format-evaluate/Y49RecordIdFormat10.al,
//! metadata/platform-tables/GetUrlBookmarkEnumKey.al and GetUrlBookmarkDigitCode.al). The stored form, the Text tag and
//! the Guid tag were measured on a restored BC 24 database (owner's local copy, not in the repository): all 3 123 075 rows
//! of its Change Log Entry table decode to the table number and the key values the row's own `Table No.` and
//! `Primary Key` fields hold (Code, Option, Integer, Text and Guid keys, tables 17, 36 to 39, 402, 403, 9001, 9002 and
//! platform tables), and all 7 472 non-blank `Record ID` and `Context Record ID` cells of its Error Message table decode
//! to the table its `Table Number` and `Context Table Number` name. No corpus test can read SQL bytes
//! (docs/probes-needed.md, "Test data").
//!
//! Refused (the table is not loaded, and the summary says why): a table number no single compiled table has, a key whose
//! fields do not match the stored ones in number or type, a tag this decoder does not know (a Date, Decimal, BigInteger,
//! Boolean or other key field: never measured), a non-ASCII character (never measured), a Code longer than its field, and
//! an Option key field with an OptionCaption (the runtime's own RecordId of such a record shows the captions, which
//! depend on the session's language).

use al_builtins::value::{LibraryValue, RtType, Value};
use std::cell::RefCell;
use std::rc::Rc;

/// The compiled table a stored RecordId names.
pub(super) struct Target<'a> {
    /// The table's object index.
    pub object: u32,
    pub name: &'a str,
    /// The types of the primary key fields, in key order.
    pub key: Vec<&'a RtType>,
    /// Whether the key field at the same position is an Option with an OptionCaption.
    pub option_caption: Vec<bool>,
}

/// The RecordId value the stored bytes hold. `target` gives the compiled table of a table number, or why there is none.
pub(super) fn decode<'a>(
    stored: &[u8],
    target: impl FnOnce(u32) -> Result<Target<'a>, String>,
) -> Result<Value, String> {
    let Some((head, rest)) = stored.split_first_chunk::<4>() else {
        return Err(format!("{} bytes hold no table number", stored.len()));
    };
    let table_no = u32::from_le_bytes(*head);
    let t = target(table_no)?;
    let mut r = Reader { b: rest, at: 0 };
    let mut key = Vec::with_capacity(t.key.len());
    for (i, ty) in t.key.iter().enumerate() {
        let tag = r.take(2).ok_or("the bytes end inside the key")?;
        let v = match (tag, ty) {
            ([0x00, 0x87], RtType::Integer) => Value::Integer(r.int()?),
            ([0x00, 0x8b], RtType::Option(_)) if t.option_caption[i] => {
                return Err(format!(
                    "key field {} of table {table_no} is an Option with an OptionCaption (its RecordId text is not pinned)",
                    i + 1
                ));
            }
            ([0x00, 0x8b], RtType::Option(names)) => Value::Option(r.int()?, names.clone()),
            ([0x00, 0x8b], RtType::Enum(id)) => Value::Enum(*id, r.int()?),
            ([0x00, 0x91], RtType::Guid) => {
                let b: [u8; 16] = r
                    .take(16)
                    .ok_or("the bytes end inside a Guid")?
                    .try_into()
                    .expect("16 bytes");
                Value::Guid(super::guid(&b))
            }
            ([0x02, 0x7b], RtType::Code(len)) => {
                r.take(1).ok_or("the bytes end inside a Code")?;
                let s = r.text()?;
                if len.is_some_and(|n| s.len() > n as usize) {
                    return Err(format!(
                        "the Code '{s}' is longer than key field {} of table {table_no}",
                        i + 1
                    ));
                }
                Value::Code(s.into())
            }
            ([0x01, 0x7b], RtType::Text(_)) => Value::Text(r.text()?.into()),
            (tag, ty) => {
                return Err(format!(
                    "key field {} of table {table_no} ({}) is stored with the tag {:02x} {:02x}, which this decoder does not read for it",
                    i + 1,
                    al_builtins::describe::Describe::describe(*ty),
                    tag[0],
                    tag[1]
                ));
            }
        };
        key.push(v);
    }
    if r.take(2) != Some(&[0, 0]) || r.b[r.at..].iter().any(|&x| x != 0) {
        return Err(format!(
            "the stored key of table {table_no} has more fields than its compiled key ({})",
            t.key.len()
        ));
    }
    Ok(Value::Library(
        "RecordId".into(),
        Rc::new(RefCell::new(LibraryValue::RecordId {
            table: t.object,
            table_no,
            name: t.name.to_string(),
            key,
        })),
    ))
}

struct Reader<'b> {
    b: &'b [u8],
    at: usize,
}

impl<'b> Reader<'b> {
    fn take(&mut self, n: usize) -> Option<&'b [u8]> {
        let s = self.b.get(self.at..self.at + n)?;
        self.at += n;
        Some(s)
    }

    fn int(&mut self) -> Result<i32, String> {
        let b = self.take(4).ok_or("the bytes end inside a number")?;
        Ok(i32::from_le_bytes(b.try_into().expect("4 bytes")))
    }

    /// UTF-16 characters up to two zero bytes.
    fn text(&mut self) -> Result<String, String> {
        let mut s = String::new();
        loop {
            let u = self.take(2).ok_or("the bytes end inside a text")?;
            match (u[0], u[1]) {
                (0, 0) => return Ok(s),
                (lo, 0) if lo.is_ascii() => s.push(lo as char),
                _ => return Err("a key text holds a non-ASCII character (its stored form is not measured)".into()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rid(v: &Value) -> (u32, String) {
        let Value::Library(_, l) = v else { panic!("a RecordId") };
        let LibraryValue::RecordId { table_no, key, .. } = &*l.borrow() else {
            panic!("a RecordId")
        };
        (*table_no, format!("{key:?}"))
    }

    fn utf16(s: &str) -> Vec<u8> {
        let mut b: Vec<u8> = s.bytes().flat_map(|c| [c, 0]).collect();
        b.extend([0, 0]);
        b
    }

    fn target<'a>(key: Vec<&'a RtType>) -> impl FnOnce(u32) -> Result<Target<'a>, String> {
        move |_| {
            let option_caption = vec![false; key.len()];
            Ok(Target {
                object: 7,
                name: "Sales Line",
                key,
                option_caption,
            })
        }
    }

    #[test]
    fn an_option_code_and_integer_key_is_read_as_measured() {
        // Sales Line (37), Document Type 1, Document No. "SQ1", Line No. 10000: the shape of the measured rows
        let mut b = vec![0x25, 0, 0, 0, 0x00, 0x8b, 1, 0, 0, 0, 0x02, 0x7b, 0xff];
        b.extend(utf16("SQ1"));
        b.extend([0x00, 0x87, 0x10, 0x27, 0, 0, 0, 0]);
        let opt = RtType::Option(vec!["Quote".to_string(), "Order".to_string()].into());
        let (code, int) = (RtType::Code(Some(20)), RtType::Integer);
        let v = decode(&b, target(vec![&opt, &code, &int])).unwrap();
        let (n, key) = rid(&v);
        assert_eq!(n, 37);
        assert!(
            key.contains("Option(1") && key.contains("\"SQ1\"") && key.contains("Integer(10000)"),
            "{key}"
        );
    }

    #[test]
    fn a_guid_and_a_text_key_field_are_read_as_measured() {
        // Access Control (2000000053): a Guid and a Text field, as in the measured rows
        let mut b = 2_000_000_053u32.to_le_bytes().to_vec();
        b.extend([
            0x00, 0x91, 0x78, 0xBD, 0x3C, 0x07, 0x67, 0x47, 0x7A, 0x4C, 0xAA, 0xA3, 0x11, 0xF0, 0xB8, 0xCD, 0x36, 0xC8,
        ]);
        b.extend([0x01, 0x7b]);
        b.extend(utf16("CRONUS"));
        b.extend([0, 0]);
        let (g, t) = (RtType::Guid, RtType::Text(Some(30)));
        let v = decode(&b, target(vec![&g, &t])).unwrap();
        let (_, key) = rid(&v);
        let want = format!(
            "{:?}",
            Value::Guid([
                0x07, 0x3C, 0xBD, 0x78, 0x47, 0x67, 0x4C, 0x7A, 0xAA, 0xA3, 0x11, 0xF0, 0xB8, 0xCD, 0x36, 0xC8
            ])
        );
        assert!(key.contains(&want) && key.contains("\"CRONUS\""), "{key}");
    }

    #[test]
    fn an_empty_code_is_read() {
        // table 402 with an empty Code key: 92 01 00 00 02 7b 00 00 00 00 00 (measured)
        let b = [0x92, 0x01, 0, 0, 0x02, 0x7b, 0, 0, 0, 0, 0];
        let code = RtType::Code(Some(10));
        let (n, key) = rid(&decode(&b, target(vec![&code])).unwrap());
        assert_eq!(n, 402);
        assert!(key.contains("\"\""), "{key}");
    }

    #[test]
    fn what_does_not_match_the_compiled_key_is_refused() {
        let int = RtType::Integer;
        let code = RtType::Code(Some(20));
        // an Integer stored, a Code compiled
        let b = [0x11, 0, 0, 0, 0x00, 0x87, 1, 0, 0, 0, 0, 0];
        assert!(decode(&b, target(vec![&code])).unwrap_err().contains("tag 00 87"));
        // more stored fields than the compiled key
        let b = [0x11, 0, 0, 0, 0x00, 0x87, 1, 0, 0, 0, 0x00, 0x87, 2, 0, 0, 0, 0, 0];
        assert!(decode(&b, target(vec![&int])).unwrap_err().contains("more fields"));
        // fewer
        assert!(
            decode(&[0x11, 0, 0, 0, 0, 0], target(vec![&int, &int]))
                .unwrap_err()
                .contains("tag 00 00")
        );
        // a tag never measured (a Date key field)
        let date = RtType::Date;
        let b = [0x11, 0, 0, 0, 0x00, 0x85, 1, 0, 0, 0, 0, 0];
        assert!(decode(&b, target(vec![&date])).unwrap_err().contains("does not read"));
        // cut off
        assert!(
            decode(&[0x11, 0, 0, 0, 0x00, 0x87, 1], target(vec![&int]))
                .unwrap_err()
                .contains("end inside")
        );
        assert!(
            decode(&[1, 2, 3], target(vec![&int]))
                .unwrap_err()
                .contains("no table number")
        );
        // a Code longer than its field
        let short = RtType::Code(Some(2));
        let mut b = vec![0x11, 0, 0, 0, 0x02, 0x7b, 0xff];
        b.extend(utf16("ABC"));
        b.extend([0, 0]);
        assert!(decode(&b, target(vec![&short])).unwrap_err().contains("longer"));
        // a table the run does not have
        let err = decode(&[0x11, 0, 0, 0, 0, 0], |n| Err(format!("no table {n}"))).unwrap_err();
        assert_eq!(err, "no table 17");
    }

    #[test]
    fn an_option_key_field_with_captions_is_refused() {
        let opt = RtType::Option(vec!["A".to_string()].into());
        let b = [0x11, 0, 0, 0, 0x00, 0x8b, 0, 0, 0, 0, 0, 0];
        let err = decode(&b, |_| {
            Ok(Target {
                object: 1,
                name: "T",
                key: vec![&opt],
                option_caption: vec![true],
            })
        })
        .unwrap_err();
        assert!(err.contains("OptionCaption"), "{err}");
    }
}
