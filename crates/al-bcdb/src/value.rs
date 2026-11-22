//! Decoded values. A `Value` carries the typed form (so a consumer that loads rows
//! into a store never re-parses text), and its `Display` is exactly the text bcdb
//! prints, which is what the differential tests compare.

use std::fmt::{self, Write as _};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    /// SQL `bit`.
    Bool(bool),
    /// tinyint, smallint, int, bigint.
    Int(i64),
    /// SQL `real`.
    Real(f32),
    /// SQL `float`.
    Float(f64),
    /// decimal/numeric: `(-1)^negative * magnitude / 10^scale`, exact for precision 38.
    Decimal {
        negative: bool,
        magnitude: u128,
        scale: u8,
    },
    /// Legacy `datetime`: days since 1900-01-01 (may be negative) and 1/300 s ticks.
    DateTime {
        days: i32,
        ticks: u32,
    },
    /// `date`: days since 0001-01-01.
    Date {
        days: i64,
    },
    /// `time(scale)`: units of 10^-scale seconds since midnight.
    Time {
        units: i64,
        scale: u8,
    },
    /// `datetime2(scale)`.
    DateTime2 {
        days: i64,
        units: i64,
        scale: u8,
    },
    /// `uniqueidentifier`, in SQL Server's storage byte order (.NET `Guid(byte[])`).
    Guid([u8; 16]),
    /// binary, varbinary, image, and every off-row binary value.
    Binary(Vec<u8>),
    /// `rowversion`/`timestamp`, big-endian number.
    RowVersion(u64),
    /// Any character type.
    Text(String),
}

impl Value {
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// The text bcdb's tsv output prints: `NULL` for null, `1`/`0` for bit.
    pub fn to_tsv(&self) -> String {
        match self {
            Value::Null => "NULL".into(),
            Value::Bool(b) => if *b { "1" } else { "0" }.into(),
            v => v.to_string(),
        }
    }

    /// Append the tsv text to `out`.
    pub fn write_tsv(&self, out: &mut String) {
        match self {
            Value::Null => out.push_str("NULL"),
            Value::Bool(b) => out.push(if *b { '1' } else { '0' }),
            v => v.append_text(out),
        }
    }

    /// Append the json text `read --format json` prints for a value: numbers for
    /// integers, `true`/`false` for bit, everything else a quoted string.
    pub fn write_json(&self, out: &mut String) {
        match self {
            Value::Null => out.push_str("null"),
            Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Value::Int(i) => {
                let _ = write!(out, "{i}");
            }
            Value::Real(f) => out.push_str(&format_real(*f)),
            Value::Float(f) => out.push_str(&format_float(*f)),
            Value::Text(s) => json_string(s, out),
            // the other typed values print as plain ASCII that needs no escaping
            v => {
                out.push('"');
                v.append_text(out);
                out.push('"');
            }
        }
    }
}

/// bcdb's `J()`: a JSON string with only the escapes it makes.
pub fn json_string(s: &str, out: &mut String) {
    out.push('"');
    if !s.bytes().any(|b| b < 0x20 || b == b'"' || b == b'\\') {
        out.push_str(s);
        out.push('"');
        return;
    }
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => f.write_str("NULL"),
            Value::Bool(b) => f.write_str(if *b { "True" } else { "False" }),
            Value::Text(s) => f.write_str(s),
            v => {
                let mut s = String::new();
                v.append_text(&mut s);
                f.write_str(&s)
            }
        }
    }
}

const HEX: &[u8; 16] = b"0123456789ABCDEF";

/// Both hex digits of every byte value.
const HEX2: [[u8; 2]; 256] = {
    let mut t = [[0u8; 2]; 256];
    let mut i = 0;
    while i < 256 {
        t[i] = [HEX[i >> 4], HEX[i & 15]];
        i += 1;
    }
    t
};

/// Upper-case hex of `b`. Done in one pass over a byte buffer: pushing a `char` per nibble
/// was most of the time of an export dominated by one large blob table.
fn push_hex(out: &mut String, b: &[u8]) {
    let mut buf = vec![0u8; b.len() * 2];
    for (pair, &x) in buf.chunks_exact_mut(2).zip(b) {
        pair.copy_from_slice(&HEX2[x as usize]);
    }
    // only ASCII hex digits were written
    out.push_str(std::str::from_utf8(&buf).expect("hex is ASCII"));
}

fn push_pad2(out: &mut String, v: u32) {
    out.push((b'0' + (v / 10 % 10) as u8) as char);
    out.push((b'0' + (v % 10) as u8) as char);
}

fn push_int(out: &mut String, mut v: u128) {
    let mut tmp = [0u8; 40];
    let mut i = tmp.len();
    loop {
        i -= 1;
        tmp[i] = b'0' + (v % 10) as u8;
        v /= 10;
        if v == 0 {
            break;
        }
    }
    out.push_str(std::str::from_utf8(&tmp[i..]).unwrap());
}

fn push_padded(out: &mut String, v: u128, width: usize) {
    let mark = out.len();
    push_int(out, v);
    let digits = out.len() - mark;
    if digits < width {
        out.insert_str(mark, &"0".repeat(width - digits));
    }
}

impl Value {
    /// Append the text `Display` gives, without an intermediate string or `format!`.
    pub fn append_text(&self, out: &mut String) {
        match self {
            Value::Null => out.push_str("NULL"),
            Value::Bool(b) => out.push_str(if *b { "True" } else { "False" }),
            Value::Int(i) => {
                if *i < 0 {
                    out.push('-');
                }
                push_int(out, i.unsigned_abs() as u128);
            }
            Value::Real(x) => out.push_str(&format_real(*x)),
            Value::Float(x) => out.push_str(&format_float(*x)),
            Value::Decimal {
                negative,
                magnitude,
                scale,
            } => {
                if *negative && *magnitude != 0 {
                    out.push('-');
                }
                if *scale == 0 {
                    push_int(out, *magnitude);
                } else {
                    let p = 10u128.pow(*scale as u32);
                    push_int(out, magnitude / p);
                    out.push('.');
                    push_padded(out, magnitude % p, *scale as usize);
                }
            }
            Value::DateTime { days, ticks } => {
                let ms = ((ticks % 300) as f64 * 10.0 / 3.0).round() as u32;
                let total_secs = *days as i64 * SECS_PER_DAY + (ticks / 300) as i64;
                let day = total_secs.div_euclid(SECS_PER_DAY) + DAYS_0001_TO_1900;
                let sec = total_secs.rem_euclid(SECS_PER_DAY) as u32;
                push_date(out, day);
                out.push(' ');
                push_pad2(out, sec / 3600);
                out.push(':');
                push_pad2(out, sec / 60 % 60);
                out.push(':');
                push_pad2(out, sec % 60);
                out.push('.');
                push_padded(out, ms as u128, 3);
            }
            Value::Date { days } => push_date(out, *days),
            Value::Time { units, scale } => push_time(out, *units, *scale),
            Value::DateTime2 { days, units, scale } => {
                push_date(out, *days);
                out.push(' ');
                push_time(out, *units, *scale);
            }
            Value::Guid(b) => {
                for (i, &x) in [3usize, 2, 1, 0, 5, 4, 7, 6, 8, 9, 10, 11, 12, 13, 14, 15]
                    .iter()
                    .enumerate()
                {
                    if matches!(i, 4 | 6 | 8 | 10) {
                        out.push('-');
                    }
                    out.push(HEX[(b[x] >> 4) as usize] as char);
                    out.push(HEX[(b[x] & 15) as usize] as char);
                }
            }
            Value::Binary(b) => {
                out.push_str("0x");
                push_hex(out, b);
            }
            Value::RowVersion(v) => {
                out.push_str("0x");
                for sh in (0..16).rev() {
                    out.push(HEX[((v >> (sh * 4)) & 15) as usize] as char);
                }
            }
            Value::Text(s) => out.push_str(s),
        }
    }
}

fn push_date(out: &mut String, days_since_0001: i64) {
    let (y, m, d) = civil_from_days(days_since_0001);
    push_padded(out, y.max(0) as u128, 4);
    out.push('-');
    push_pad2(out, m);
    out.push('-');
    push_pad2(out, d);
}

fn push_time(out: &mut String, units: i64, scale: u8) {
    let per_second = 10i64.pow(scale as u32);
    let seconds = units / per_second;
    let frac = units % per_second;
    push_pad2(out, ((seconds / 3600) % 24) as u32);
    out.push(':');
    push_pad2(out, ((seconds / 60) % 60) as u32);
    out.push(':');
    push_pad2(out, (seconds % 60) as u32);
    if scale > 0 {
        out.push('.');
        push_padded(out, frac as u128, scale as usize);
    }
}

/// .NET `Guid(byte[16]).ToString().ToUpperInvariant()`: the first three groups are
/// little-endian in storage.
pub fn format_guid(b: &[u8; 16]) -> String {
    format!(
        "{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
        b[3], b[2], b[1], b[0], b[5], b[4], b[7], b[6], b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]
    )
}

/// A scale-shifted integer as a plain decimal string with exactly `scale` fraction digits.
pub fn format_scaled(negative: bool, magnitude: u128, scale: u8) -> String {
    let sign = if negative && magnitude != 0 { "-" } else { "" };
    if scale == 0 {
        return format!("{sign}{magnitude}");
    }
    let p = 10u128.pow(scale as u32);
    let (i, f) = (magnitude / p, magnitude % p);
    format!("{sign}{i}.{f:0width$}", width = scale as usize)
}

const SECS_PER_DAY: i64 = 86_400;

/// Days since 0001-01-01 to (year, month, day), proleptic Gregorian. Hinnant's algorithm.
fn civil_from_days(days_since_0001: i64) -> (i64, u32, u32) {
    // 0001-01-01 is day -719162 relative to 1970-01-01.
    let z = days_since_0001 - 719_162 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Days from 0001-01-01 to 1900-01-01.
pub const DAYS_0001_TO_1900: i64 = 693_595;

pub fn format_date(days_since_0001: i64) -> String {
    let (y, m, d) = civil_from_days(days_since_0001);
    format!("{y:04}-{m:02}-{d:02}")
}

/// `time` text: `HH:mm:ss` plus `.` and `scale` digits when scale > 0. Hours wrap at 24
/// like `TimeSpan.Hours`.
pub fn format_time(units: i64, scale: u8) -> String {
    let per_second = 10i64.pow(scale as u32);
    let seconds = units / per_second;
    let frac = units % per_second;
    let (h, m, s) = ((seconds / 3600) % 24, (seconds / 60) % 60, seconds % 60);
    if scale == 0 {
        format!("{h:02}:{m:02}:{s:02}")
    } else {
        format!("{h:02}:{m:02}:{s:02}.{frac:0width$}", width = scale as usize)
    }
}

/// Legacy `datetime` text, `yyyy-MM-dd HH:mm:ss.fff`. The milliseconds are
/// `round(ticks % 300 * 10 / 3)` half away from zero, as bcdb prints them.
pub fn format_datetime(days: i32, ticks: u32) -> String {
    let ms = ((ticks % 300) as f64 * 10.0 / 3.0).round() as i64;
    let total_secs = days as i64 * SECS_PER_DAY + (ticks / 300) as i64;
    let day = total_secs.div_euclid(SECS_PER_DAY) + DAYS_0001_TO_1900;
    let sec_of_day = total_secs.rem_euclid(SECS_PER_DAY);
    let (h, m, s) = (sec_of_day / 3600, (sec_of_day / 60) % 60, sec_of_day % 60);
    format!("{} {h:02}:{m:02}:{s:02}.{ms:03}", format_date(day))
}

/// .NET `float.ToString("R")` (shortest round-trip digits; scientific outside
/// 1e-5..1e15 for float, which is where .NET switches).
pub fn format_real(x: f32) -> String {
    format_shortest(x.is_nan(), x.is_infinite(), x < 0.0, &format!("{:e}", x.abs()), 7)
}

/// .NET `double.ToString("R")`.
pub fn format_float(x: f64) -> String {
    format_shortest(x.is_nan(), x.is_infinite(), x < 0.0, &format!("{:e}", x.abs()), 15)
}

/// `sci` is Rust's `{:e}` of the absolute value: `d.ddde[-]x`.
fn format_shortest(nan: bool, inf: bool, neg: bool, sci: &str, max_digits: i32) -> String {
    if nan {
        return "NaN".into();
    }
    if inf {
        return if neg { "-Infinity" } else { "Infinity" }.into();
    }
    let (mant, exp) = sci.split_once('e').expect("rust {:e}");
    let exp: i32 = exp.parse().expect("exponent");
    let digits: String = mant.chars().filter(|c| *c != '.').collect();
    let sign = if neg { "-" } else { "" };
    if digits == "0" {
        return if neg { "-0".into() } else { "0".into() };
    }
    let dig_pos = exp + 1; // position of the decimal point relative to the first digit
    if dig_pos > max_digits || dig_pos < -3 {
        // scientific: d[.ddd]E+xx
        let m = if digits.len() > 1 {
            format!("{}.{}", &digits[..1], &digits[1..])
        } else {
            digits
        };
        let e = if exp < 0 {
            format!("-{:02}", -exp)
        } else {
            format!("+{:02}", exp)
        };
        return format!("{sign}{m}E{e}");
    }
    if dig_pos <= 0 {
        return format!("{sign}0.{}{}", "0".repeat((-dig_pos) as usize), digits);
    }
    let dp = dig_pos as usize;
    if digits.len() <= dp {
        format!("{sign}{digits}{}", "0".repeat(dp - digits.len()))
    } else {
        format!("{sign}{}.{}", &digits[..dp], &digits[dp..])
    }
}
