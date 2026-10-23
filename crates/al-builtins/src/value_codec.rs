//! Plain values as bytes, for the install baseline's snapshot on disk and between threads
//! (`al_vm::Baseline::to_bytes`). Only [`Value::is_plain`] values have a form: a handle (`Rc`)
//! cannot leave its thread. Little-endian, a tag byte per value, lengths as `u32`.

use crate::value::Value;
use rust_decimal::Decimal;
use std::sync::Arc;

pub fn put_u8(out: &mut Vec<u8>, v: u8) {
    out.push(v);
}
pub fn put_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}
pub fn put_u64(out: &mut Vec<u8>, v: u64) {
    out.extend_from_slice(&v.to_le_bytes());
}
pub fn put_i64(out: &mut Vec<u8>, v: i64) {
    out.extend_from_slice(&v.to_le_bytes());
}
pub fn put_bytes(out: &mut Vec<u8>, v: &[u8]) {
    put_u32(out, v.len() as u32);
    out.extend_from_slice(v);
}
pub fn put_str(out: &mut Vec<u8>, v: &str) {
    put_bytes(out, v.as_bytes());
}

/// Reads what the `put_*` functions and [`encode`] wrote; every read says `None` on short or damaged input.
pub struct Reader<'a> {
    b: &'a [u8],
}

impl<'a> Reader<'a> {
    pub fn new(b: &'a [u8]) -> Reader<'a> {
        Reader { b }
    }
    pub fn is_empty(&self) -> bool {
        self.b.is_empty()
    }
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        if self.b.len() < n {
            return None;
        }
        let (a, rest) = self.b.split_at(n);
        self.b = rest;
        Some(a)
    }
    pub fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|b| b[0])
    }
    pub fn u32(&mut self) -> Option<u32> {
        self.take(4).map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")))
    }
    pub fn u64(&mut self) -> Option<u64> {
        self.take(8).map(|b| u64::from_le_bytes(b.try_into().expect("8 bytes")))
    }
    pub fn i64(&mut self) -> Option<i64> {
        self.take(8).map(|b| i64::from_le_bytes(b.try_into().expect("8 bytes")))
    }
    pub fn bytes(&mut self) -> Option<&'a [u8]> {
        let n = self.u32()? as usize;
        self.take(n)
    }
    pub fn str(&mut self) -> Option<String> {
        String::from_utf8(self.bytes()?.to_vec()).ok()
    }
    pub fn rest(&self) -> &'a [u8] {
        self.b
    }
}

/// Appends `v`; false (and nothing appended) when it is not a plain value.
pub fn encode(v: &Value, out: &mut Vec<u8>) -> bool {
    let at = out.len();
    let ok = encode_in(v, out);
    if !ok {
        out.truncate(at);
    }
    ok
}

fn encode_in(v: &Value, out: &mut Vec<u8>) -> bool {
    match v {
        Value::Null => put_u8(out, 0),
        Value::Boolean(b) => {
            put_u8(out, 1);
            put_u8(out, *b as u8);
        }
        Value::Integer(i) => {
            put_u8(out, 2);
            out.extend_from_slice(&i.to_le_bytes());
        }
        Value::BigInteger(i) => {
            put_u8(out, 3);
            put_i64(out, *i);
        }
        Value::Decimal(d) => {
            put_u8(out, 4);
            out.extend_from_slice(&d.serialize());
        }
        Value::Byte(b) => {
            put_u8(out, 5);
            put_u8(out, *b);
        }
        Value::Char(c) => {
            put_u8(out, 6);
            out.extend_from_slice(&c.to_le_bytes());
        }
        Value::Text(t) => {
            put_u8(out, 7);
            put_str(out, t.as_str());
        }
        Value::Code(t) => {
            put_u8(out, 8);
            put_str(out, t.as_str());
        }
        Value::Date(d, closing) => {
            put_u8(out, 9);
            out.extend_from_slice(&d.to_le_bytes());
            put_u8(out, *closing as u8);
        }
        Value::Time(t) => {
            put_u8(out, 10);
            put_i64(out, *t);
        }
        Value::DateTime(t) => {
            put_u8(out, 11);
            put_i64(out, *t);
        }
        Value::Duration(t) => {
            put_u8(out, 12);
            put_i64(out, *t);
        }
        Value::Guid(g) => {
            put_u8(out, 13);
            out.extend_from_slice(g);
        }
        Value::DateFormula(f) => {
            put_u8(out, 14);
            put_str(out, f);
        }
        Value::Option(ord, names) => {
            put_u8(out, 15);
            out.extend_from_slice(&ord.to_le_bytes());
            put_u32(out, names.len() as u32);
            for n in names.iter() {
                put_str(out, n);
            }
        }
        Value::Enum(id, ord) => {
            put_u8(out, 16);
            put_u32(out, *id);
            out.extend_from_slice(&ord.to_le_bytes());
        }
        Value::Blob(b) => {
            put_u8(out, 17);
            let b = b.borrow();
            put_bytes(out, &b.bytes);
            put_u64(out, b.write_pos as u64);
            put_u8(
                out,
                (b.loaded as u8)
                    | ((b.refetch as u8) << 1)
                    | ((b.persisted as u8) << 2)
                    | ((b.fetched as u8) << 3)
                    | ((b.written as u8) << 4)
                    | ((b.temp_row as u8) << 5)
                    | ((b.db_row as u8) << 6)
                    | ((b.first_write_truncates as u8) << 7),
            );
        }
        Value::Unknown(why) => {
            put_u8(out, 18);
            put_str(out, why);
        }
        // Handles (`Rc`) have no form: they cannot leave their thread. Every variant is named, so a new one is a
        // compile error here and has to be placed (plain, with a tag, or in this list), never a silent fallback.
        // A counter value is tied to the session whose reads it compares with (`sqlcount`).
        Value::SqlCount(_)
        | Value::List(_)
        | Value::Dictionary(_)
        | Value::Array(_)
        | Value::Codeunit(_)
        | Value::Library(..)
        | Value::Record(_)
        | Value::TestPage(_)
        | Value::TestControl(_)
        | Value::Query(_)
        | Value::DotNet(_) => return false,
    }
    true
}

/// Reads one value.
pub fn decode(r: &mut Reader) -> Option<Value> {
    Some(match r.u8()? {
        0 => Value::Null,
        1 => Value::Boolean(r.u8()? != 0),
        2 => Value::Integer(i32::from_le_bytes(r.take(4)?.try_into().ok()?)),
        3 => Value::BigInteger(r.i64()?),
        4 => Value::Decimal(Decimal::deserialize(r.take(16)?.try_into().ok()?)),
        5 => Value::Byte(r.u8()?),
        6 => Value::Char(u16::from_le_bytes(r.take(2)?.try_into().ok()?)),
        7 => Value::Text(r.str()?.into()),
        8 => Value::Code(r.str()?.into()),
        9 => {
            let d = i32::from_le_bytes(r.take(4)?.try_into().ok()?);
            Value::Date(d, r.u8()? != 0)
        }
        10 => Value::Time(r.i64()?),
        11 => Value::DateTime(r.i64()?),
        12 => Value::Duration(r.i64()?),
        13 => Value::Guid(r.take(16)?.try_into().ok()?),
        14 => Value::DateFormula(r.str()?),
        18 => Value::Unknown(r.str()?.into()),
        15 => {
            let ord = i32::from_le_bytes(r.take(4)?.try_into().ok()?);
            let n = r.u32()? as usize;
            let mut names = Vec::with_capacity(n.min(1024));
            for _ in 0..n {
                names.push(r.str()?);
            }
            Value::Option(ord, Arc::from(names))
        }
        16 => {
            let id = r.u32()?;
            Value::Enum(id, i32::from_le_bytes(r.take(4)?.try_into().ok()?))
        }
        17 => {
            let bytes = r.bytes()?.to_vec();
            let write_pos = r.u64()? as usize;
            let flags = r.u8()?;
            let blob = crate::stream::new_blob();
            {
                let mut b = blob.borrow_mut();
                b.bytes = bytes;
                b.write_pos = write_pos;
                b.loaded = flags & 1 != 0;
                b.refetch = flags & 2 != 0;
                b.persisted = flags & 4 != 0;
                b.fetched = flags & 8 != 0;
                b.written = flags & 16 != 0;
                b.temp_row = flags & 32 != 0;
                b.db_row = flags & 64 != 0;
                b.first_write_truncates = flags & 128 != 0;
            }
            Value::Blob(blob)
        }
        _ => return None,
    })
}

/// Appends a row (a count and the values); false (and nothing appended) when a value is not plain.
pub fn encode_row(row: &[Value], out: &mut Vec<u8>) -> bool {
    let at = out.len();
    put_u32(out, row.len() as u32);
    for v in row {
        if !encode(v, out) {
            out.truncate(at);
            return false;
        }
    }
    true
}

pub fn decode_row(r: &mut Reader) -> Option<Vec<Value>> {
    let n = r.u32()? as usize;
    let mut row = Vec::with_capacity(n.min(4096));
    for _ in 0..n {
        row.push(decode(r)?);
    }
    Some(row)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;
    use rust_decimal::Decimal;

    fn plain_samples() -> Vec<Value> {
        let blob = crate::stream::new_blob();
        {
            let mut b = blob.borrow_mut();
            b.bytes = vec![1, 2, 3];
            b.write_pos = 2;
            b.loaded = false;
            b.refetch = true;
            b.persisted = true;
        }
        vec![
            Value::Null,
            Value::Boolean(true),
            Value::Integer(-7),
            Value::BigInteger(i64::MIN),
            Value::Decimal(Decimal::new(-123456789, 4)),
            Value::Byte(200),
            Value::Char(0xFFFE),
            Value::Text("häh".into()),
            Value::Code("A-1".into()),
            Value::Date(738000, true),
            Value::Time(12345),
            Value::DateTime(639277389274520),
            Value::Duration(-5),
            Value::Guid([9; 16]),
            Value::DateFormula("1M+CM".into()),
            Value::Option(2, std::sync::Arc::from(vec!["A".to_string(), "B".into(), "C".into()])),
            Value::Enum(50100, -3),
            Value::Blob(blob),
        ]
    }

    #[test]
    fn every_plain_value_round_trips() {
        for v in plain_samples() {
            let mut out = Vec::new();
            assert!(encode(&v, &mut out), "{v:?} has a form");
            let mut r = Reader::new(&out);
            let back = decode(&mut r).unwrap_or_else(|| panic!("{v:?} decodes"));
            assert!(r.is_empty(), "{v:?}: all bytes used");
            assert_eq!(format!("{v:?}"), format!("{back:?}"));
        }
    }

    #[test]
    fn a_blob_keeps_its_buffer_state() {
        let v = plain_samples().pop().unwrap();
        let mut out = Vec::new();
        assert!(encode(&v, &mut out));
        let Some(Value::Blob(b)) = decode(&mut Reader::new(&out)) else {
            panic!("a blob")
        };
        let b = b.borrow();
        assert_eq!(
            (&b.bytes[..], b.write_pos, b.loaded, b.refetch, b.persisted),
            (&[1u8, 2, 3][..], 2, false, true, true)
        );
    }

    #[test]
    fn a_handle_has_no_form_and_leaves_the_output_alone() {
        let list = Value::List(std::rc::Rc::default());
        let mut out = vec![42];
        assert!(!encode(&list, &mut out));
        assert_eq!(out, vec![42]);
    }

    #[test]
    fn a_value_has_a_form_exactly_when_it_is_plain() {
        let handles = vec![
            Value::List(std::rc::Rc::default()),
            Value::Dictionary(std::rc::Rc::new(std::cell::RefCell::new(Vec::new()))),
            Value::Array(std::rc::Rc::new(std::cell::RefCell::new(Vec::new()))),
        ];
        for v in plain_samples().into_iter().chain(handles) {
            assert_eq!(encode(&v, &mut Vec::new()), v.is_plain(), "{v:?}");
        }
    }

    #[test]
    fn damaged_input_decodes_to_nothing() {
        let mut out = Vec::new();
        assert!(encode(&Value::Text("hello".into()), &mut out));
        for cut in 0..out.len() {
            assert!(decode(&mut Reader::new(&out[..cut])).is_none(), "cut at {cut}");
        }
        assert!(decode(&mut Reader::new(&[250])).is_none(), "unknown tag");
        // A length that runs past the end.
        assert!(decode(&mut Reader::new(&[7, 255, 255, 255, 255])).is_none());
    }

    #[test]
    fn sequences_of_values_and_strings_round_trip() {
        let mut out = Vec::new();
        put_str(&mut out, "ünï");
        put_u64(&mut out, 1 << 40);
        assert!(encode_row(&[Value::Integer(1), Value::Text("x".into())], &mut out));
        let mut r = Reader::new(&out);
        assert_eq!(r.str().as_deref(), Some("ünï"));
        assert_eq!(r.u64(), Some(1 << 40));
        let row = decode_row(&mut r).unwrap();
        assert_eq!(format!("{row:?}"), "[Integer(1), Text(\"x\")]");
        assert!(r.is_empty());
    }
}
