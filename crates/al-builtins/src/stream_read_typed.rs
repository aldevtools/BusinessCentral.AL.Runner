//! `InStream.Read` into a variable of a fixed-width type (`stream::in_read`): how many bytes each type takes, the value a read at
//! the end gives, and the value the bytes give.

use crate::describe::Describe;
use crate::native::{Signal, unsupported};
use crate::value::{RtType, Value};

/// The bytes a `Read` into `v` takes; the other types are refused.
pub(crate) fn width_of(v: &Value) -> Result<usize, Signal> {
    Ok(match v {
        // An Option is its ordinal in 4 bytes (streams/instream-outstream/StreamLengthOthers.al:Option_WriteReadLength_1 `nb_rd`).
        Value::Integer(_) | Value::Boolean(_) | Value::Date(..) | Value::Option(..) => 4,
        Value::Byte(_) | Value::Char(_) => 1,
        Value::BigInteger(_) => 8,
        Value::Guid(_) => 16,
        // streams/TestStreamsBinaryDateDecimalContracts.al:117-123.
        Value::Decimal(_) => 12,
        _ => {
            return unsupported(format!(
                "InStream.Read into {} (only Integer, Byte and Decimal are pinned)",
                v.describe()
            ));
        }
    })
}

/// The value `v` takes from the first read at the end of a stream that holds data: the default of its type
/// (streams/media/P102ReadTyped.al: every `pastend_*`).
pub(crate) fn default_after_end(v: &Value) -> Value {
    Value::default_for(&match v {
        Value::Integer(_) => RtType::Integer,
        Value::Boolean(_) => RtType::Boolean,
        Value::Date(..) => RtType::Date,
        Value::Byte(_) => RtType::Byte,
        Value::Char(_) => RtType::Char,
        Value::BigInteger(_) => RtType::BigInteger,
        Value::Guid(_) => RtType::Guid,
        // The first member (StreamLengthOthers.al:Option_WriteReadLength_1 `nb_rd2` is A).
        Value::Option(_, names) => RtType::Option(names.clone()),
        _ => RtType::Decimal,
    })
}

/// The value of the type of `v` that `bytes` (as many as [`width_of`] gives) read as.
pub(crate) fn decode(v: &Value, bytes: &[u8]) -> Result<Value, Signal> {
    Ok(match v {
        Value::Byte(_) => Value::Byte(bytes[0]),
        Value::Char(_) => Value::Char(u16::from(bytes[0])),
        Value::Boolean(_) => Value::Boolean(bytes.iter().any(|b| *b != 0)),
        Value::BigInteger(_) => Value::BigInteger(i64::from_le_bytes(bytes[..8].try_into().expect("8 bytes"))),
        Value::Guid(_) => {
            let mut g: [u8; 16] = bytes[..16].try_into().expect("16 bytes");
            g[0..4].reverse();
            g[4..6].reverse();
            g[6..8].reverse();
            Value::Guid(g)
        }
        Value::Date(..) => {
            // The inverse of `date_bytes`: twice the day number plus 731, plus 1 for a closing date.
            let n = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            if n == 0 {
                Value::Date(0, false)
            } else {
                let closing = (n - 731) % 2 != 0;
                let day = (n - 731 - i32::from(closing)) / 2;
                // Bytes that are no date raise (streams/PnAStreamReadMisc.al:Read_Types_A `op5`).
                if !(1..=3_652_059).contains(&day) {
                    return Err(Signal::Error(Box::new(crate::native::AlError {
                        message: "The date is not valid.".into(),
                        code: "DateInvalid".into(),
                        call_stack: String::new(),
                        ..Default::default()
                    })));
                }
                Value::Date(day, closing)
            }
        }
        Value::Decimal(_) => Value::Decimal(decimal_from_bytes(bytes)?),
        Value::Option(_, names) => {
            let n = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            if usize::try_from(n).map_or(true, |i| i >= names.len()) {
                return unsupported("InStream.Read into an Option of an ordinal that is no member (not pinned)");
            }
            Value::Option(n, names.clone())
        }
        _ => Value::Integer(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])),
    })
}

/// The inverse of `stream::decimal_bytes`.
fn decimal_from_bytes(b: &[u8]) -> Result<rust_decimal::Decimal, Signal> {
    if b.iter().all(|&x| x == 0) {
        return Ok(rust_decimal::Decimal::ZERO);
    }
    let exponent = i32::from(b[0] & 0x7F) - 0x40;
    let mut mantissa: i128 = 0;
    for i in 0..18 {
        let byte = b[9 - i / 2];
        let v = if i % 2 == 0 { byte >> 4 } else { byte & 0xF };
        mantissa = mantissa * 10 + i128::from(v);
    }
    let scale = 17 - exponent;
    let bad = || Signal::Unsupported("InStream.Read of a Decimal out of range (not pinned)".into());
    let mut d = if scale >= 0 {
        rust_decimal::Decimal::try_from_i128_with_scale(mantissa, scale as u32).map_err(|_| bad())?
    } else {
        let m = mantissa.checked_mul(10i128.pow((-scale) as u32)).ok_or_else(bad)?;
        rust_decimal::Decimal::try_from_i128_with_scale(m, 0).map_err(|_| bad())?
    };
    d.normalize_assign();
    if b[0] & 0x80 != 0 {
        d.set_sign_negative(true);
    }
    Ok(d)
}
