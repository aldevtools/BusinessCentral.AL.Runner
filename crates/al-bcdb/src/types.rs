//! Storage-form cells to typed values (port of `Values.cs`).
//!
//! Entry points: `SysColumn`, `type_name`, `Cell`, `LobResolver`.

use crate::error::Result;
use crate::value::Value;

/// One column as the catalog (or `model.xml`) describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SysColumn {
    pub col_id: i32,
    pub name: String,
    pub xtype: u8,
    /// Bytes for string and binary types, -1 for MAX, as syscolpars carries it.
    pub max_length: i16,
    pub precision: u8,
    pub scale: u8,
    pub is_nullable: bool,
}

impl SysColumn {
    pub fn new(col_id: i32, name: &str, xtype: u8, max_length: i16, precision: u8, scale: u8) -> Self {
        SysColumn {
            col_id,
            name: name.to_string(),
            xtype,
            max_length,
            precision,
            scale,
            is_nullable: true,
        }
    }
    pub fn type_name(&self) -> String {
        type_name(self.xtype)
    }
}

pub fn type_name(xtype: u8) -> String {
    match xtype {
        36 => "uniqueidentifier",
        40 => "date",
        41 => "time",
        42 => "datetime2",
        43 => "datetimeoffset",
        48 => "tinyint",
        52 => "smallint",
        56 => "int",
        58 => "smalldatetime",
        59 => "real",
        60 => "money",
        61 => "datetime",
        62 => "float",
        98 => "sql_variant",
        104 => "bit",
        106 => "decimal",
        108 => "numeric",
        122 => "smallmoney",
        127 => "bigint",
        165 => "varbinary",
        167 => "varchar",
        173 => "binary",
        175 => "char",
        189 => "timestamp",
        231 => "nvarchar",
        239 => "nchar",
        34 => "image",
        35 => "text",
        99 => "ntext",
        241 => "xml",
        x => return format!("xtype{x}"),
    }
    .to_string()
}

/// One storage cell: the raw storage-form bytes before type interpretation, or NULL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell<'a> {
    Null,
    Value(&'a [u8]),
    /// An off-row pointer (text pointer or inline root) rather than the value itself.
    Complex(&'a [u8]),
}

/// Resolves an off-row pointer to the full value; `None` is a type-8 NULL root.
pub trait LobResolver {
    fn resolve(&self, pointer: &[u8], column: &str) -> Result<Option<Vec<u8>>>;
}

fn utf16le(b: &[u8]) -> String {
    let mut units: Vec<u16> = b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    if b.len() % 2 == 1 {
        units.push(0xFFFD); // .NET's Encoding.Unicode.GetString replaces a dangling byte
    }
    String::from_utf16_lossy(&units)
}

fn latin1(b: &[u8]) -> String {
    b.iter().map(|&x| x as char).collect()
}

fn pad_right(mut s: String, width: usize) -> String {
    let n = s.encode_utf16().count();
    if n < width {
        s.extend(std::iter::repeat_n(' ', width - n));
    }
    s
}

/// Interprets storage-form cell bytes as a typed value.
///
/// Uncompressed (FixedVar) encodings are the classic little-endian forms; compressed
/// (row/page, CD record) encodings differ per type. Every rule was derived from probe
/// tables on a scratch SQL Server and validated against SELECT (bcdb PROVENANCE.md
/// "Type encodings"). Types this reader cannot decode throw, naming the column; it never
/// substitutes a default.
///
/// `text_is_utf16`: the bytes of char/varchar/text hold UTF-16 rather than the column's
/// single-byte collation form. True only for `.bacpac` streams.
pub fn decode(
    cell: Cell<'_>,
    col: &SysColumn,
    compressed: bool,
    lob: Option<&dyn LobResolver>,
    text_is_utf16: bool,
) -> Result<Value> {
    let b = match cell {
        Cell::Null => return Ok(Value::Null),
        Cell::Value(b) => b,
        Cell::Complex(pointer) => {
            let Some(lob) = lob else {
                return Err(crate::unsupported!(
                    "column {}: off-row value but no LOB reader available",
                    col.name
                ));
            };
            let Some(resolved) = lob.resolve(pointer, &col.name)? else {
                return Ok(Value::Null); // type-8 NULL root: the value is SQL NULL
            };
            // Off-row bytes are the plain value; MAX/LOB types are never SCSU-compressed.
            return match col.xtype {
                231 | 99 => Ok(Value::Text(utf16le(&resolved))),
                167 | 35 => Ok(Value::Text(if text_is_utf16 {
                    utf16le(&resolved)
                } else {
                    latin1(&resolved)
                })),
                165 | 34 => Ok(Value::Binary(resolved)),
                _ => Err(crate::unsupported!(
                    "column {}: off-row value for type {} — not derived, refusing to guess",
                    col.name,
                    type_name(col.xtype)
                )),
            };
        }
    };
    match col.xtype {
        231 | 239 => {
            let mut s = if !compressed || col.max_length < 0 || b.len() % 2 == 0 {
                utf16le(b) // plain UTF-16LE
            } else {
                crate::scsu::decode(b)? // odd length under compression = SCSU
            };
            if col.xtype == 239 && col.max_length > 0 {
                s = pad_right(s, col.max_length as usize / 2);
            }
            Ok(Value::Text(s))
        }
        167 | 175 => {
            let mut s = if text_is_utf16 { utf16le(b) } else { latin1(b) };
            if col.xtype == 175 && col.max_length > 0 {
                s = pad_right(s, col.max_length as usize);
            }
            Ok(Value::Text(s))
        }
        34 | 165 => Ok(Value::Binary(b.to_vec())),
        35 => Ok(Value::Text(if text_is_utf16 { utf16le(b) } else { latin1(b) })),
        99 => Ok(Value::Text(utf16le(b))),
        173 => {
            // binary(n): fixed width; compression trims trailing zero bytes
            let mut v = b.to_vec();
            if compressed && col.max_length > 0 && v.len() < col.max_length as usize {
                v.resize(col.max_length as usize, 0);
            }
            Ok(Value::Binary(v))
        }
        48 | 52 | 56 | 127 => {
            if compressed {
                return Ok(Value::Int(decode_compressed_int(b, col.xtype == 48, col)?));
            }
            let need = match col.xtype {
                48 => 1,
                52 => 2,
                56 => 4,
                _ => 8,
            };
            if b.len() < need {
                return Err(crate::invalid!(
                    "{} cell of {} bytes in {}",
                    type_name(col.xtype),
                    b.len(),
                    col.name
                ));
            }
            Ok(Value::Int(match col.xtype {
                48 => b[0] as i64,
                52 => i16::from_le_bytes([b[0], b[1]]) as i64,
                56 => i32::from_le_bytes([b[0], b[1], b[2], b[3]]) as i64,
                _ => i64::from_le_bytes(b[..8].try_into().unwrap()),
            }))
        }
        104 => Ok(Value::Bool(!b.is_empty() && b[0] != 0)),
        36 => {
            let g: [u8; 16] = b
                .try_into()
                .map_err(|_| crate::invalid!("GUID cell of {} bytes in {}", b.len(), col.name))?;
            Ok(Value::Guid(g))
        }
        189 => {
            if compressed {
                if b.len() > 8 {
                    return Err(crate::invalid!("rowversion cell of {} bytes in {}", b.len(), col.name));
                }
                Ok(Value::RowVersion(b.iter().fold(0u64, |t, &x| (t << 8) | x as u64)))
            } else {
                let a: [u8; 8] = b
                    .try_into()
                    .map_err(|_| crate::invalid!("rowversion cell of {} bytes in {}", b.len(), col.name))?;
                Ok(Value::RowVersion(u64::from_be_bytes(a)))
            }
        }
        106 | 108 => {
            if col.scale > 38 {
                return Err(crate::invalid!(
                    "decimal scale {} in {} is beyond 38",
                    col.scale,
                    col.name
                ));
            }
            if compressed {
                decode_vardecimal(b, col)
            } else {
                decode_decimal(b, col)
            }
        }
        61 => {
            if compressed {
                let v = decode_compressed_int(b, false, col)?;
                datetime((v >> 32) as i32, v as u32, col)
            } else {
                if b.len() != 8 {
                    return Err(crate::invalid!("datetime cell of {} bytes in {}", b.len(), col.name));
                }
                let ticks = i32::from_le_bytes(b[..4].try_into().unwrap());
                let days = i32::from_le_bytes(b[4..8].try_into().unwrap());
                datetime(days, ticks as u32, col)
            }
        }
        40 => {
            let days = read_le_uint(b, 3, col, "date")?;
            check_days(days, col)?;
            Ok(Value::Date { days })
        }
        41 => Ok(Value::Time {
            units: read_le_uint(b, time_width(col.scale), col, "time")?,
            scale: col.scale,
        }),
        42 => {
            let tw = time_width(col.scale);
            if b.is_empty() {
                return Ok(Value::DateTime2 {
                    days: 0,
                    units: 0,
                    scale: col.scale,
                });
            }
            if b.len() != tw + 3 {
                return Err(crate::invalid!(
                    "datetime2({}) cell of {} bytes in {} (expected {})",
                    col.scale,
                    b.len(),
                    col.name,
                    tw + 3
                ));
            }
            let units = b[..tw].iter().rev().fold(0i64, |u, &x| (u << 8) | x as i64);
            let days = b[tw] as i64 | (b[tw + 1] as i64) << 8 | (b[tw + 2] as i64) << 16;
            check_days(days, col)?;
            Ok(Value::DateTime2 {
                days,
                units,
                scale: col.scale,
            })
        }
        59 => Ok(Value::Real(f32::from_le_bytes(pad_float::<4>(b, compressed, col)?))),
        62 => Ok(Value::Float(f64::from_le_bytes(pad_float::<8>(b, compressed, col)?))),
        _ => Err(crate::unsupported!(
            "column {}: type {} (xtype {}) is not supported by this reader",
            col.name,
            type_name(col.xtype),
            col.xtype
        )),
    }
}

pub(crate) fn time_width(scale: u8) -> usize {
    if scale <= 2 {
        3
    } else if scale <= 4 {
        4
    } else {
        5
    }
}

/// Last day number .NET's DateTime can hold (9999-12-31).
const MAX_DAYS: i64 = 3_652_058;

pub(crate) fn check_days(days: i64, col: &SysColumn) -> Result<()> {
    if !(0..=MAX_DAYS).contains(&days) {
        return Err(crate::invalid!(
            "date value {} in {} is outside 0001-01-01..9999-12-31",
            days,
            col.name
        ));
    }
    Ok(())
}

fn datetime(days: i32, ticks: u32, col: &SysColumn) -> Result<Value> {
    let total = days as i64 * 86_400 + (ticks / 300) as i64;
    let day = total.div_euclid(86_400) + crate::value::DAYS_0001_TO_1900;
    check_days(day, col)?;
    Ok(Value::DateTime { days, ticks })
}

/// Compressed integers: big-endian, trimmed; signed values biased by 2^(8·len−1). Zero is stored empty.
fn decode_compressed_int(b: &[u8], unsigned: bool, col: &SysColumn) -> Result<i64> {
    if b.is_empty() {
        return Ok(0);
    }
    if b.len() > 8 {
        return Err(crate::invalid!("integer cell of {} bytes in {}", b.len(), col.name));
    }
    let u = b.iter().fold(0u64, |u, &x| (u << 8) | x as u64);
    if unsigned {
        return Ok(u as i64);
    }
    Ok(u.wrapping_sub(1u64 << (8 * b.len() - 1)) as i64)
}

/// Uncompressed decimal: `[u8 sign (1 = positive)][magnitude, little-endian, 4-byte units]`.
fn decode_decimal(b: &[u8], col: &SysColumn) -> Result<Value> {
    if !matches!(b.len(), 5 | 9 | 13 | 17) {
        return Err(crate::invalid!("decimal cell of {} bytes in {}", b.len(), col.name));
    }
    let mut mag = [0u8; 16];
    mag[..b.len() - 1].copy_from_slice(&b[1..]);
    Ok(Value::Decimal {
        negative: b[0] == 0,
        magnitude: u128::from_le_bytes(mag),
        scale: col.scale,
    })
}

/// Row-compression decimal (the vardecimal form): `[u8: bit 0x80 = positive, low 7 bits =
/// biased exponent (value − 64 + 1)]` then the mantissa as 10-bit base-1000 digit groups
/// packed MSB-first, trailing zero bytes trimmed. value = 0.digits × 10^exponent. Zero is
/// stored empty.
fn decode_vardecimal(b: &[u8], col: &SysColumn) -> Result<Value> {
    let scale = col.scale;
    if b.is_empty() {
        return Ok(Value::Decimal {
            negative: false,
            magnitude: 0,
            scale,
        });
    }
    let negative = (b[0] & 0x80) == 0;
    let exp = (b[0] & 0x7f) as i32 - 64 + 1;
    let bits = 8 * (b.len() - 1);
    let groups = bits.div_ceil(10);
    // The mantissa has up to 3 * groups decimal digits, which can exceed 128 bits before
    // the exponent scales it down, so it is kept as digits and converted at the end.
    let mut digits: Vec<u8> = Vec::with_capacity(groups * 3);
    for g in 0..groups {
        let mut val = 0u32;
        for bit in 0..10 {
            let idx = g * 10 + bit;
            let bv = if idx < bits {
                (b[1 + idx / 8] >> (7 - idx % 8)) & 1
            } else {
                0
            };
            val = (val << 1) | bv as u32;
        }
        if val > 999 {
            return Err(crate::invalid!("vardecimal digit group {} > 999 in {}", val, col.name));
        }
        digits.extend_from_slice(&[(val / 100) as u8, (val / 10 % 10) as u8, (val % 10) as u8]);
    }
    // value = digits × 10^(exp − 3·groups); scale to the column's declared scale
    let pow = exp - 3 * groups as i32 + scale as i32;
    if pow >= 0 {
        digits.extend(std::iter::repeat_n(0u8, pow as usize));
    } else {
        let drop = (-pow) as usize;
        if drop > digits.len() {
            if digits.iter().any(|&d| d != 0) {
                return Err(crate::invalid!(
                    "vardecimal value in {} does not fit scale {}",
                    col.name,
                    scale
                ));
            }
            digits.clear();
        } else {
            let keep = digits.len() - drop;
            if digits[keep..].iter().any(|&d| d != 0) {
                return Err(crate::invalid!(
                    "vardecimal value in {} does not fit scale {}",
                    col.name,
                    scale
                ));
            }
            digits.truncate(keep);
        }
    }
    let mut scaled: u128 = 0;
    for d in digits {
        scaled = scaled
            .checked_mul(10)
            .and_then(|v| v.checked_add(d as u128))
            .ok_or_else(|| crate::invalid!("vardecimal value in {} does not fit 128 bits", col.name))?;
    }
    Ok(Value::Decimal {
        negative,
        magnitude: scaled,
        scale,
    })
}

fn read_le_uint(b: &[u8], width: usize, col: &SysColumn, what: &str) -> Result<i64> {
    if b.is_empty() {
        return Ok(0);
    }
    if b.len() != width {
        return Err(crate::invalid!(
            "{} cell of {} bytes in {} (expected {})",
            what,
            b.len(),
            col.name,
            width
        ));
    }
    Ok(b.iter().rev().fold(0i64, |v, &x| (v << 8) | x as i64))
}

/// Compressed real/float: little-endian with low-order zero bytes trimmed; the stored bytes are the high end.
fn pad_float<const N: usize>(b: &[u8], compressed: bool, col: &SysColumn) -> Result<[u8; N]> {
    let mut full = [0u8; N];
    if !compressed {
        if b.len() != N {
            return Err(crate::invalid!(
                "{} cell of {} bytes in {}",
                type_name(col.xtype),
                b.len(),
                col.name
            ));
        }
        full.copy_from_slice(b);
        return Ok(full);
    }
    if b.len() > N {
        return Err(crate::invalid!(
            "{} cell of {} bytes in {}",
            type_name(col.xtype),
            b.len(),
            col.name
        ));
    }
    full[N - b.len()..].copy_from_slice(b);
    Ok(full)
}
