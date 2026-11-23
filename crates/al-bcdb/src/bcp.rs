//! Reads rows out of a bacpac's native-BCP data stream (port of `Bcp.cs`).
//!
//! The stream is a bare sequence of rows with no header, trailer or row delimiter. Each
//! row is its columns in model.xml order, each column a length prefix (of a width the
//! type and nullability decide) followed by that many value bytes; a prefix of all
//! one-bits is SQL NULL. The widths were derived from probe tables whose every value is
//! chosen and re-checked against 567 tables of a production export (bcdb PROVENANCE.md
//! "bacpac: native BCP row framing"):
//!
//! ```text
//!   0 bytes  the fixed-length numeric and temporal types, and char(n), when NOT NULL
//!   1 byte   those same types when nullable; bit, uniqueidentifier and decimal/numeric always
//!   2 bytes  char(n) when nullable, and always nchar/nvarchar/varchar/binary/varbinary/rowversion
//!   4 bytes  text / ntext / image
//!   8 bytes  any (max) type
//! ```
//!
//! Two encodings genuinely differ from the storage form and are converted here: datetime
//! (the halves are the other way round) and time/datetime2 (always written at full
//! precision). DacFx also writes char/varchar/text as UTF-16.
//!
//! Entry points: `BacpacColumn`, `decimal_storage_bytes`, `BcpRowReader`.

use crate::error::{Error, Result};
use crate::types::{Cell, SysColumn, decode};
use crate::value::Value;
use std::io::Read;

/// One column of a table as model.xml declares it, plus the SQL type facts the value
/// decoders need. The order columns appear in a table's "Columns" relationship is the
/// order their values appear in the data stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BacpacColumn {
    pub name: String,
    pub model_type: String,
    pub xtype: u8,
    pub nullable: bool,
    pub is_max: bool,
    pub length: i32,
    pub precision: u8,
    pub scale: u8,
}

impl BacpacColumn {
    pub fn from_model(
        name: &str,
        model_type: &str,
        nullable: bool,
        is_max: bool,
        length: i32,
        precision: u8,
        scale: u8,
    ) -> BacpacColumn {
        BacpacColumn {
            name: name.to_string(),
            model_type: model_type.to_string(),
            xtype: xtype_of(model_type),
            nullable,
            is_max,
            length,
            precision,
            scale,
        }
    }

    /// MaxLength in the syscolpars convention: bytes for string/binary types, -1 for MAX.
    pub fn max_length(&self) -> i16 {
        let l = self.length;
        match self.xtype {
            231 | 239 => {
                if self.is_max {
                    -1
                } else {
                    (2 * l) as i16
                }
            }
            167 | 175 | 165 | 173 => {
                if self.is_max {
                    -1
                } else {
                    l as i16
                }
            }
            34 | 35 | 99 | 241 => -1,
            106 | 108 => decimal_storage_bytes(self.precision) as i16,
            104 | 48 => 1,
            52 => 2,
            56 | 59 | 58 | 122 => 4,
            127 | 62 | 61 | 60 | 189 => 8,
            36 => 16,
            40 => 3,
            41 => crate::types::time_width(self.scale) as i16,
            42 => (crate::types::time_width(self.scale) + 3) as i16,
            43 => (crate::types::time_width(self.scale) + 5) as i16,
            _ => 0,
        }
    }

    pub fn to_sys_column(&self, col_id: i32) -> SysColumn {
        SysColumn {
            col_id,
            name: self.name.clone(),
            xtype: self.xtype,
            max_length: self.max_length(),
            precision: self.precision,
            scale: self.scale,
            is_nullable: self.nullable,
        }
    }
}

/// The system type id syscolpars would carry for this type, so the shared decoders and
/// the command line see one column shape whichever container a row came from. 0 = a model
/// type with no SQL Server system type id known to this reader; reading it throws.
fn xtype_of(model_type: &str) -> u8 {
    match model_type {
        "bit" => 104,
        "tinyint" => 48,
        "smallint" => 52,
        "int" => 56,
        "bigint" => 127,
        "real" => 59,
        "float" => 62,
        "decimal" => 106,
        "numeric" => 108,
        "money" => 60,
        "smallmoney" => 122,
        "datetime" => 61,
        "smalldatetime" => 58,
        "date" => 40,
        "time" => 41,
        "datetime2" => 42,
        "datetimeoffset" => 43,
        "uniqueidentifier" => 36,
        "rowversion" | "timestamp" => 189,
        "char" => 175,
        "varchar" => 167,
        "nchar" => 239,
        "nvarchar" => 231,
        "text" => 35,
        "ntext" => 99,
        "xml" => 241,
        "binary" => 173,
        "varbinary" => 165,
        "image" => 34,
        _ => 0,
    }
}

pub(crate) fn decimal_storage_bytes(precision: u8) -> usize {
    match precision {
        0..=9 => 5,
        10..=19 => 9,
        20..=28 => 13,
        _ => 17,
    }
}

pub struct BcpRowReader<'a> {
    table: &'a str,
    cols: &'a [BacpacColumn],
}

impl<'a> BcpRowReader<'a> {
    pub fn new(table: &'a str, cols: &'a [BacpacColumn]) -> Self {
        BcpRowReader { table, cols }
    }

    /// Width in bytes of a column's value, for the types whose prefix must state exactly
    /// that width; -1 where the width varies.
    pub fn fixed_width(c: &BacpacColumn) -> i32 {
        match c.xtype {
            104 | 48 => 1,
            52 => 2,
            56 | 59 => 4,
            127 | 62 => 8,
            61 => 8,
            40 => 3,
            41 => 5,
            42 => 8, // datetime; date; time; datetime2 (always full precision)
            36 => 16,
            189 => 8,
            106 | 108 => 19,     // decimal = prec+scale+sign+16
            175 => 2 * c.length, // char(n), written as UTF-16
            _ => -1,
        }
    }

    /// Width of the length prefix; -1 for a type with no derived framing.
    pub fn prefix_length(c: &BacpacColumn) -> i32 {
        if c.is_max {
            return 8;
        }
        match c.xtype {
            34 | 35 | 99 => 4,
            231 | 239 | 167 | 165 | 173 | 189 => 2,
            104 | 36 | 106 | 108 => 1,
            175 => {
                if c.nullable {
                    2
                } else {
                    0
                }
            }
            48 | 52 | 56 | 127 | 59 | 62 | 61 | 40 | 41 | 42 => i32::from(c.nullable),
            _ => -1,
        }
    }

    /// Rows from one data stream, each a vector of values in `want` order. The stream must
    /// end exactly on a row boundary. Returns `Ok(false)` when the sink stopped the read.
    pub fn read<R: Read>(
        &self,
        stream: R,
        want: &[BacpacColumn],
        sink: &mut dyn FnMut(&[Value]) -> Result<bool>,
    ) -> Result<bool> {
        let mut wanted: Vec<Option<usize>> = vec![None; self.cols.len()];
        for (w, c) in want.iter().enumerate() {
            let i = self
                .cols
                .iter()
                .position(|x| x.name == c.name)
                .ok_or_else(|| Error::Argument(format!("bacpac table {} has no column {}", self.table, c.name)))?;
            if wanted[i].is_some() {
                return Err(Error::Argument(format!(
                    "bacpac table {}: column {} was asked for twice",
                    self.table, c.name
                )));
            }
            wanted[i] = Some(w);
        }
        // Per column: the framing facts and, where wanted, the decoder's view of it.
        let sys: Vec<Option<SysColumn>> = self
            .cols
            .iter()
            .zip(&wanted)
            .map(|(c, w)| w.map(|_| c.to_sys_column(0)))
            .collect();
        let mut cur = Cursor::new(stream, self.table);
        let mut row: Vec<Value> = vec![Value::Null; want.len()];
        while !cur.at_end()? {
            for (i, c) in self.cols.iter().enumerate() {
                let pl = Self::prefix_length(c);
                if pl < 0 {
                    return Err(crate::unsupported!(
                        "bacpac table {}, column {}: type {} has no derived native-BCP framing — refusing to guess",
                        self.table,
                        c.name,
                        c.model_type
                    ));
                }
                let len: u64 = if pl == 0 {
                    Self::fixed_width(c) as u64
                } else {
                    let len = cur.read_prefix(pl as usize, &c.name)?;
                    if len == all_ones(pl as usize) {
                        if let Some(w) = wanted[i] {
                            row[w] = Value::Null;
                        }
                        continue;
                    }
                    if pl == 8 && len == 0xFFFF_FFFF_FFFF_FFFE {
                        return Err(crate::unsupported!(
                            "bacpac table {}, column {}: chunked (unknown-length) MAX value — that form is not derived, refusing to guess",
                            self.table,
                            c.name
                        ));
                    }
                    if len > i32::MAX as u64 {
                        return Err(crate::invalid!(
                            "bacpac table {}, column {}: length prefix {} is larger than this reader can address",
                            self.table,
                            c.name,
                            len
                        ));
                    }
                    let fw = Self::fixed_width(c);
                    if fw >= 0 && !c.is_max && len != fw as u64 {
                        return Err(crate::invalid!(
                            "bacpac table {}, column {}: {} length prefix {}, expected {}",
                            self.table,
                            c.name,
                            c.model_type,
                            len,
                            fw
                        ));
                    }
                    len
                };
                match wanted[i] {
                    None => cur.skip(len, &c.name)?,
                    Some(w) => {
                        let raw = cur.take(len as usize, &c.name)?;
                        row[w] = self.to_value(c, sys[i].as_ref().unwrap(), raw)?;
                    }
                }
            }
            if !sink(&row)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Turns a BCP value into a typed value; the shared decoder handles every type whose
    /// BCP bytes equal the storage bytes.
    fn to_value(&self, c: &BacpacColumn, sys: &SysColumn, raw: &[u8]) -> Result<Value> {
        match c.xtype {
            106 | 108 => self.decimal(c, raw),
            61 => {
                if raw.len() != 8 {
                    return Err(crate::invalid!(
                        "bacpac table {}, column {}: datetime value of {} bytes, expected 8",
                        self.table,
                        c.name,
                        raw.len()
                    ));
                }
                // BCP: [i32 days since 1900][u32 ticks of 1/300 s]; storage has the halves swapped.
                let mut storage = [0u8; 8];
                storage[..4].copy_from_slice(&raw[4..]);
                storage[4..].copy_from_slice(&raw[..4]);
                decode(Cell::Value(&storage), sys, false, None, true)
            }
            41 => {
                let units = self.time_units(c, raw, 5)?;
                Ok(Value::Time { units, scale: c.scale })
            }
            42 => {
                let units = self.time_units(c, raw, 8)?;
                let days = raw[5] as i64 | (raw[6] as i64) << 8 | (raw[7] as i64) << 16;
                crate::types::check_days(days, sys)?;
                Ok(Value::DateTime2 {
                    days,
                    units,
                    scale: c.scale,
                })
            }
            _ => decode(Cell::Value(raw), sys, false, None, true),
        }
    }

    /// BCP decimal is `[precision][scale][sign: 1 = positive][16-byte magnitude, LE]`;
    /// storage is `[sign][magnitude]` narrowed to the 4/8/12/16 bytes the precision needs.
    /// The magnitude bytes dropped must be zero, and the precision and scale the value
    /// carries must be the ones model.xml declares.
    fn decimal(&self, c: &BacpacColumn, raw: &[u8]) -> Result<Value> {
        if raw.len() != 19 {
            return Err(crate::invalid!(
                "bacpac table {}, column {}: decimal value of {} bytes, expected 19",
                self.table,
                c.name,
                raw.len()
            ));
        }
        if raw[0] != c.precision || raw[1] != c.scale {
            return Err(crate::invalid!(
                "bacpac table {}, column {}: value carries decimal({},{}) but model.xml declares decimal({},{})",
                self.table,
                c.name,
                raw[0],
                raw[1],
                c.precision,
                c.scale
            ));
        }
        let keep = c.max_length() as usize - 1;
        if raw[3 + keep..19].iter().any(|&b| b != 0) {
            return Err(crate::invalid!(
                "bacpac table {}, column {}: magnitude does not fit the {} bytes precision {} allows",
                self.table,
                c.name,
                keep,
                c.precision
            ));
        }
        let mut mag = [0u8; 16];
        mag[..keep].copy_from_slice(&raw[3..3 + keep]);
        Ok(Value::Decimal {
            negative: raw[2] == 0,
            magnitude: u128::from_le_bytes(mag),
            scale: c.scale,
        })
    }

    /// BCP writes time (and the time half of datetime2) as five bytes of 100-nanosecond
    /// units whatever the declared scale; storage uses units of 10^-scale seconds. A value
    /// that does not divide exactly is one the declared scale cannot hold.
    fn time_units(&self, c: &BacpacColumn, raw: &[u8], expect: usize) -> Result<i64> {
        if raw.len() != expect {
            return Err(crate::invalid!(
                "bacpac table {}, column {}: {} value of {} bytes, expected {}",
                self.table,
                c.name,
                c.model_type,
                raw.len(),
                expect
            ));
        }
        let units = raw[..5].iter().rev().fold(0i64, |u, &b| (u << 8) | b as i64);
        let mut divisor = 1i64;
        for _ in c.scale..7 {
            divisor *= 10;
        }
        if units % divisor != 0 {
            return Err(crate::invalid!(
                "bacpac table {}, column {}: time value {} (100 ns units) does not fit the declared scale {}",
                self.table,
                c.name,
                units,
                c.scale
            ));
        }
        let units = units / divisor;
        let width = crate::types::time_width(c.scale);
        if (units >> (8 * width)) != 0 {
            return Err(crate::invalid!(
                "bacpac table {}, column {}: time value does not fit {} bytes at scale {}",
                self.table,
                c.name,
                width,
                c.scale
            ));
        }
        Ok(units)
    }
}

fn all_ones(prefix_bytes: usize) -> u64 {
    if prefix_bytes == 8 {
        u64::MAX
    } else {
        (1u64 << (8 * prefix_bytes)) - 1
    }
}

/// A forward-only byte cursor that can tell a clean end of stream from a truncated row.
/// Values that fit the buffer are returned as borrows of it, so no value is copied.
struct Cursor<'t, R: Read> {
    r: R,
    table: &'t str,
    buf: Vec<u8>,
    pos: usize,
    len: usize,
    eof: bool,
}

impl<'t, R: Read> Cursor<'t, R> {
    const CHUNK: usize = 256 * 1024;

    fn new(r: R, table: &'t str) -> Self {
        Cursor {
            r,
            table,
            buf: vec![0; Self::CHUNK],
            pos: 0,
            len: 0,
            eof: false,
        }
    }

    fn truncated(&self, column: &str) -> Error {
        crate::invalid!(
            "bacpac table {}: the data stream ends inside column {} — a data file must hold whole rows",
            self.table,
            column
        )
    }

    /// Read more bytes after the current ones; false at end of stream.
    fn fill_more(&mut self) -> Result<bool> {
        if self.eof {
            return Ok(false);
        }
        if self.len == self.buf.len() {
            let n = self.buf.len() * 2;
            self.buf.resize(n, 0);
        }
        let n = self.r.read(&mut self.buf[self.len..])?;
        if n == 0 {
            self.eof = true;
            return Ok(false);
        }
        self.len += n;
        Ok(true)
    }

    fn at_end(&mut self) -> Result<bool> {
        if self.pos < self.len {
            return Ok(false);
        }
        self.pos = 0;
        self.len = 0;
        Ok(!self.fill_more()?)
    }

    /// Make `n` bytes available contiguously from `pos`.
    fn ensure(&mut self, n: usize, column: &str) -> Result<()> {
        if self.len - self.pos >= n {
            return Ok(());
        }
        // move the tail to the front, then read until n bytes are there
        self.buf.copy_within(self.pos..self.len, 0);
        self.len -= self.pos;
        self.pos = 0;
        if self.buf.len() < n {
            self.buf.resize(n.next_power_of_two().max(Self::CHUNK), 0);
        }
        while self.len < n {
            if !self.fill_more()? {
                return Err(self.truncated(column));
            }
        }
        Ok(())
    }

    fn read_prefix(&mut self, width: usize, column: &str) -> Result<u64> {
        self.ensure(width, column)?;
        let p = &self.buf[self.pos..self.pos + width];
        self.pos += width;
        Ok(match width {
            1 => p[0] as u64,
            2 => u16::from_le_bytes([p[0], p[1]]) as u64,
            4 => u32::from_le_bytes([p[0], p[1], p[2], p[3]]) as u64,
            _ => u64::from_le_bytes(p.try_into().unwrap()),
        })
    }

    fn take(&mut self, n: usize, column: &str) -> Result<&[u8]> {
        self.ensure(n, column)?;
        let s = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    fn skip(&mut self, mut n: u64, column: &str) -> Result<()> {
        while n > 0 {
            if self.pos >= self.len {
                self.pos = 0;
                self.len = 0;
                if !self.fill_more()? {
                    return Err(self.truncated(column));
                }
            }
            let take = n.min((self.len - self.pos) as u64) as usize;
            self.pos += take;
            n -= take as u64;
        }
        Ok(())
    }
}
