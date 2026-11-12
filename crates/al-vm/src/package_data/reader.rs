//! The reader of a table data file of a package (`TAB<id>.navxdata`, `NavApp.LoadPackageData`). Pure: bytes in, rows out
//! (ADR-006). The layout is pinned by what the platform reads back in the corpus fixtures
//! tests/al-language-xapp-a/Y85XaPackageData.al (file layout and value encodings in its head comment) and the tests of
//! tests/al-language/install-upgrade/install-triggers/Y85PackageData.al:
//!
//! - header: u32 0xDA7A, u32 1, u32 table id, u32 field count, then that many field names (BinaryWriter strings, ignored);
//! - per row: byte 0x01, one value per stored column, 16 bytes SystemId (`Guid.ToByteArray`); the file ends with byte 0x00;
//! - Code and Text: BinaryWriter string; Integer, Option and Enum: int32; BigInteger: int64; Boolean: one byte; Decimal: the four
//!   int32 of `System.Decimal`; Guid: 16 bytes; Duration: int64 milliseconds; Date, Time and DateTime: int64
//!   `DateTime.ToBinary` (Time on 1753-01-01, the SQL minimum, which is the empty value); DateFormula: a string; BLOB with
//!   `Compressed = false`: int64 length and the bytes.
//!
//! Everything else is refused with the reason (hard rule 4): the file is only read as far as the corpus pins it. A value the
//! platform rejects at load (an install error, whose text depends on the company and the app) is refused here too.

use al_builtins::describe::Describe;
use al_builtins::value::{RtType, Value};

/// A stored column of the table, in file order.
pub(crate) struct Column {
    pub name: String,
    pub ty: RtType,
    /// A BLOB field's `Compressed` property is `false`.
    pub uncompressed_blob: bool,
    /// A field of the primary key.
    pub key: bool,
}

/// One row of the file.
pub(crate) struct Row {
    pub values: Vec<Value>,
    pub system_id: [u8; 16],
}

/// The table id in the header of `bytes`, `None` when it is not a data file.
pub(crate) fn header_table(bytes: &[u8]) -> Option<u32> {
    let mut c = Cursor { bytes, at: 0 };
    (c.u32().ok()? == MAGIC).then(|| c.u32().ok().and_then(|_| c.u32().ok()))?
}

const MAGIC: u32 = 0xDA7A;
/// Ticks per day, and the day (since 0001-01-01) of 1753-01-01, the SQL minimum date.
const TICKS_PER_DAY: i64 = 864_000_000_000;
const MIN_DAY: i64 = 639_905;
const MAX_DAY: i64 = 3_652_058;

type R<T> = Result<T, String>;

struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Cursor<'_> {
    fn take(&mut self, n: usize) -> R<&[u8]> {
        let end = self.at.checked_add(n).filter(|&e| e <= self.bytes.len());
        let end = end.ok_or("the file ends inside a value")?;
        let s = &self.bytes[self.at..end];
        self.at = end;
        Ok(s)
    }
    fn u8(&mut self) -> R<u8> {
        Ok(self.take(1)?[0])
    }
    fn u32(&mut self) -> R<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().expect("4 bytes")))
    }
    fn i32(&mut self) -> R<i32> {
        Ok(self.u32()? as i32)
    }
    fn i64(&mut self) -> R<i64> {
        Ok(i64::from_le_bytes(self.take(8)?.try_into().expect("8 bytes")))
    }
    fn string(&mut self) -> R<String> {
        let (mut len, mut shift) = (0usize, 0u32);
        loop {
            let b = self.u8()?;
            len |= usize::from(b & 0x7F) << shift;
            if b & 0x80 == 0 {
                break;
            }
            shift += 7;
            if shift > 28 {
                return Err("a string length that is not valid".into());
            }
        }
        let raw = self.take(len)?;
        String::from_utf8(raw.to_vec()).map_err(|_| "a string that is not UTF-8".to_string())
    }
}

/// The rows of the data file `bytes` of table `table`, whose stored columns are `columns`.
pub(crate) fn read(bytes: &[u8], table: u32, columns: &[Column]) -> R<Vec<Row>> {
    let mut c = Cursor { bytes, at: 0 };
    if c.u32()? != MAGIC {
        return Err("the file does not start with the package data marker".into());
    }
    if c.u32()? != 1 {
        return Err("a package data file whose version is not 1".into());
    }
    if c.u32()? != table {
        return Err("a package data file of another table".into());
    }
    let listed = c.u32()? as usize;
    let mut names = Vec::with_capacity(listed.min(1024));
    for _ in 0..listed {
        names.push(c.string()?);
    }
    // The names are ignored (tests/al-language-xapp-a/Y85XaPackageData.al:2-3). A header may also list the column `$systemId`
    // last, one more name than the table has stored fields; the rows are the same (the SystemId follows the values either way)
    // (tests/al-language-xapp-a/Y88XaPackageData.al 94692, 94693; al-language install-upgrade/install-triggers/Y88PackageData.al
    // Y88_SystemIdColumn_*; AL Runner issue 7366).
    let system_id_listed = listed == columns.len() + 1 && names.last().is_some_and(|n| n == "$systemId");
    if listed != columns.len() && !system_id_listed {
        return Err(format!(
            "a package data file that lists {listed} fields for a table with {} stored fields (not pinned)",
            columns.len()
        ));
    }
    let mut rows = Vec::new();
    loop {
        match c.u8()? {
            0 => break,
            1 => {}
            other => return Err(format!("a row marker {other} (not pinned)")),
        }
        let mut values = Vec::with_capacity(columns.len());
        for col in columns {
            let v = value(&mut c, col).map_err(|e| format!("field {}: {e}", col.name))?;
            values.push(v);
        }
        let system_id = guid(&mut c)?;
        rows.push(Row { values, system_id });
    }
    if c.at != bytes.len() {
        return Err("bytes after the end marker of a package data file (not pinned)".into());
    }
    Ok(rows)
}

fn value(c: &mut Cursor, col: &Column) -> R<Value> {
    match &col.ty {
        RtType::Text(len) => Ok(Value::Text(text(c, *len)?.as_str().into())),
        RtType::Code(len) => Ok(Value::Code(code(c, *len, col.key)?.as_str().into())),
        RtType::Integer => Ok(Value::Integer(c.i32()?)),
        RtType::BigInteger => Ok(Value::BigInteger(c.i64()?)),
        RtType::Boolean => match c.u8()? {
            0 => Ok(Value::Boolean(false)),
            1 => Ok(Value::Boolean(true)),
            other => Err(format!("a Boolean byte {other} (not pinned)")),
        },
        RtType::Decimal => decimal(c),
        RtType::Guid => Ok(Value::Guid(guid(c)?)),
        RtType::Duration => Ok(Value::Duration(c.i64()?)),
        // An ordinal the field or enum does not define loads as it is (Y88PackageData.al, Y88_Option_UndefinedOrdinal:
        // 7 of Zero,One,Two; Y88_Enum_UndefinedOrdinal: 9 and -1).
        RtType::Option(names) => Ok(Value::Option(c.i32()?, names.clone())),
        RtType::Enum(id) => Ok(Value::Enum(*id, c.i32()?)),
        RtType::Date => date(c),
        RtType::Time => time(c),
        RtType::DateTime => date_time(c),
        RtType::DateFormula => date_formula(c),
        RtType::Library(al_builtins::library_kind::LibraryKind::Blob) if col.uncompressed_blob => blob(c),
        other => Err(format!("a field of type {} (not pinned)", other.describe())),
    }
}

/// A string within the field's length.
fn text(c: &mut Cursor, len: Option<u32>) -> R<String> {
    let s = c.string()?;
    if len.is_some_and(|n| s.chars().count() > n as usize) {
        return Err(
            "a value longer than the field (the platform raises an install error whose text is not pinned)".into(),
        );
    }
    Ok(s)
}

/// A Code value is stored as the file has it and read in upper case (Y85XaPackageData.al head comment). A key value with a
/// lower case letter cannot be found by `Get` there, which the runtime does not model: not pinned. Any other field reads the
/// same in upper case.
fn code(c: &mut Cursor, len: Option<u32>, key: bool) -> R<String> {
    let s = text(c, len)?;
    if key && s.to_uppercase() != s {
        return Err("a Code key value with lower case letters (not pinned)".into());
    }
    Ok(s.to_uppercase())
}

/// A Guid as `Guid.ToByteArray` writes it: the first three groups little-endian. Held in the order Format prints it
/// (Y85PackageData.al, Y85_Types_DurationGuidOptionEnumDateFormulaBlob, T1).
fn guid(c: &mut Cursor) -> R<[u8; 16]> {
    let b: [u8; 16] = c.take(16)?.try_into().expect("16 bytes");
    let mut g = b;
    g[..4].copy_from_slice(&[b[3], b[2], b[1], b[0]]);
    g[4..6].copy_from_slice(&[b[5], b[4]]);
    g[6..8].copy_from_slice(&[b[7], b[6]]);
    Ok(g)
}

fn decimal(c: &mut Cursor) -> R<Value> {
    let (lo, mid, hi, flags) = (c.u32()?, c.u32()?, c.u32()?, c.u32()?);
    let (scale, negative) = ((flags >> 16) & 0xFF, flags >> 31 == 1);
    // The file's 1E-28 (lo 1, scale 28, positive) loads as 0 (corpus install-upgrade/install-triggers/Y88PackageData.al:220,
    // fixture tests/al-language-xapp-a/Y88XaPackageData.al:8). Only that value is pinned; every other Decimal with a scale
    // above 9 is refused below.
    if (lo, mid, hi, flags) == (1, 0, 0, 28 << 16) {
        return Ok(Value::Decimal(rust_decimal::Decimal::ZERO));
    }
    if flags & 0x7F00_FFFF != 0 || scale > 9 {
        return Err("a Decimal with a scale above 9 or reserved bits (not pinned)".into());
    }
    let d = rust_decimal::Decimal::from_parts(lo, mid, hi, negative, scale);
    if d.mantissa().unsigned_abs() > 999_999_999_999_999_999 {
        return Err(
            "a Decimal outside the AL range (the platform raises an install error whose text is not pinned)".into(),
        );
    }
    Ok(Value::Decimal(d))
}

/// The ticks of a `DateTime.ToBinary` value. Unspecified and UTC kinds read alike; a local one is not pinned.
fn ticks(c: &mut Cursor) -> R<i64> {
    let raw = c.i64()?;
    match (raw >> 62) & 3 {
        0 | 1 => Ok(raw & 0x3FFF_FFFF_FFFF_FFFF),
        _ => Err("a local DateTime kind (not pinned)".into()),
    }
}

fn date(c: &mut Cursor) -> R<Value> {
    let t = ticks(c)?;
    if t % TICKS_PER_DAY != 0 {
        return Err(
            "a Date that is not midnight (the platform raises an install error whose text is not pinned)".into(),
        );
    }
    match t / TICKS_PER_DAY {
        MIN_DAY => Ok(Value::Date(0, false)),
        d if (MIN_DAY..=MAX_DAY).contains(&d) => Ok(Value::Date(d as i32 + 1, false)),
        _ => Err(
            "a Date outside 1753-01-01 to 9999-12-31 (the platform raises an install error whose text is not pinned)"
                .into(),
        ),
    }
}

fn time(c: &mut Cursor) -> R<Value> {
    let t = ticks(c)?;
    let (day, of_day) = (t / TICKS_PER_DAY, t % TICKS_PER_DAY);
    if day != MIN_DAY || of_day % 10_000 != 0 {
        return Err("a Time that is not on 1753-01-01 or has a part of a millisecond (not pinned)".into());
    }
    let ms = of_day / 10_000;
    if !crate::sqltime::holds_ms(ms % 1000) {
        return Err("a Time the SQL datetime column rounds (not pinned)".into());
    }
    Ok(Value::Time(if ms == 0 { 0 } else { ms + 1 }))
}

fn date_time(c: &mut Cursor) -> R<Value> {
    let t = ticks(c)?;
    let min = MIN_DAY * TICKS_PER_DAY;
    if t % 10_000 != 0 || t < min || t >= (MAX_DAY + 1) * TICKS_PER_DAY {
        return Err("a DateTime outside the SQL range or with a part of a millisecond (not pinned)".into());
    }
    let ms = t / 10_000;
    if !crate::sqltime::holds_ms(ms % 1000) {
        return Err("a DateTime the SQL datetime column rounds (not pinned)".into());
    }
    Ok(Value::DateTime(if t == min { 0 } else { ms + 1 }))
}

/// The formula as the file has it. A text that is not a date formula is not pinned.
fn date_formula(c: &mut Cursor) -> R<Value> {
    let s = c.string()?;
    if !matches!(
        al_builtins::datefns::evaluate_formula(&s),
        Ok(al_builtins::datefns::Evaluated::Stored(_))
    ) {
        return Err("a DateFormula text that Evaluate does not accept (not pinned)".into());
    }
    // The platform reads `<CM+1D>` back as `<CM+1D>` with brackets (Y85PackageData.al, Y85_Types_DurationGuidOptionEnumDateFormulaBlob,
    // T1), where Format of a formula made by Evaluate prints it without. This runtime's Format drops one pair of brackets, so a
    // text with brackets is held inside a second pair (the assumption package-data-dateformula-brackets).
    Ok(Value::DateFormula(if s.starts_with('<') || s.ends_with('>') {
        format!("<{s}>")
    } else {
        s
    }))
}

fn blob(c: &mut Cursor) -> R<Value> {
    let len = c.i64()?;
    let len = usize::try_from(len).map_err(|_| "a negative BLOB length".to_string())?;
    let bytes = c.take(len)?;
    let blob = al_builtins::stream::new_blob();
    if !bytes.is_empty() {
        al_builtins::stream::write_bytes(&blob, bytes);
    }
    Ok(Value::Blob(blob))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(table: u32, names: &[&str]) -> Vec<u8> {
        let mut f = Vec::new();
        for n in [MAGIC, 1, table, names.len() as u32] {
            f.extend_from_slice(&n.to_le_bytes());
        }
        for n in names {
            f.push(n.len() as u8);
            f.extend_from_slice(n.as_bytes());
        }
        f
    }

    fn col(ty: RtType) -> Column {
        Column {
            name: "F".into(),
            ty,
            uncompressed_blob: false,
            key: false,
        }
    }

    #[test]
    fn a_guid_is_read_in_the_order_of_guid_to_byte_array() {
        // Y85PackageData.al T1: {11223344-5566-7788-99AA-BBCCDDEEFF00}.
        let mut f = header(7, &["G"]);
        f.push(1);
        f.extend_from_slice(&[
            0x44, 0x33, 0x22, 0x11, 0x66, 0x55, 0x88, 0x77, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00,
        ]);
        f.extend_from_slice(&[0; 16]);
        f.push(0);
        let rows = read(&f, 7, &[col(RtType::Guid)]).unwrap();
        let Value::Guid(g) = rows[0].values[0] else { panic!() };
        assert_eq!(
            g,
            [
                0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00
            ]
        );
    }

    #[test]
    fn a_header_that_lists_system_id_last_reads_the_same_rows() {
        // Y88XaPackageData.al 94692: the stored fields, then `$systemId` (AL Runner issue 7366).
        let row = |names: &[&str]| {
            let mut f = header(7, names);
            f.push(1);
            f.extend_from_slice(&5i32.to_le_bytes());
            f.extend_from_slice(&[9; 16]);
            f.push(0);
            f
        };
        let plain = read(&row(&["No."]), 7, &[col(RtType::Integer)]).unwrap();
        let listed = read(&row(&["No.", "$systemId"]), 7, &[col(RtType::Integer)]).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].system_id, plain[0].system_id);
        assert!(matches!(listed[0].values[0], Value::Integer(5)));
        // Any other extra name, or `$systemId` not last, stays refused.
        assert!(read(&row(&["No.", "Extra"]), 7, &[col(RtType::Integer)]).is_err());
        assert!(read(&row(&["$systemId", "No."]), 7, &[col(RtType::Integer)]).is_err());
    }

    #[test]
    fn a_file_for_another_table_a_bad_marker_and_trailing_bytes_are_refused() {
        let mut f = header(7, &[]);
        f.push(0);
        assert!(read(&f, 7, &[]).unwrap().is_empty());
        assert!(read(&f, 8, &[]).is_err());
        f.push(0);
        assert!(read(&f, 7, &[]).is_err());
        let mut bad = header(7, &[]);
        bad.push(2);
        assert!(read(&bad, 7, &[]).is_err());
        assert_eq!(header_table(&f), Some(7));
        assert_eq!(header_table(b"junk"), None);
    }

    #[test]
    fn a_decimal_keeps_its_scale_and_one_beyond_the_al_range_is_refused() {
        let one = |lo: u32, flags: u32| {
            let mut f = header(7, &["D"]);
            f.push(1);
            for n in [lo, 0, 0, flags] {
                f.extend_from_slice(&n.to_le_bytes());
            }
            f.extend_from_slice(&[0; 16]);
            f.push(0);
            read(&f, 7, &[col(RtType::Decimal)])
        };
        let rows = one(250, 2 << 16).unwrap();
        assert!(matches!(&rows[0].values[0], Value::Decimal(d) if d.to_string() == "2.50"));
        assert!(one(1, 12 << 16).is_err());
    }

    /// The file's 1E-28 (scale 28) loads as 0 (corpus install-upgrade/install-triggers/Y88PackageData.al:220, fixture
    /// tests/al-language-xapp-a/Y88XaPackageData.al:8). Any other value with a scale above 9 is not pinned and is refused.
    #[test]
    fn only_the_pinned_tiny_decimal_loads_as_zero() {
        let one = |lo: u32, flags: u32| {
            let mut f = header(7, &["D"]);
            f.push(1);
            for n in [lo, 0, 0, flags] {
                f.extend_from_slice(&n.to_le_bytes());
            }
            f.extend_from_slice(&[0; 16]);
            f.push(0);
            read(&f, 7, &[col(RtType::Decimal)])
        };
        let rows = one(1, 28 << 16).unwrap();
        assert!(
            matches!(&rows[0].values[0], Value::Decimal(d) if d.is_zero()),
            "{:?}",
            rows[0].values[0]
        );
        // 2E-28, 1E-27, 1E-25 and a negative 1E-28 are not pinned.
        assert!(one(2, 28 << 16).is_err());
        assert!(one(1, 27 << 16).is_err());
        assert!(one(1, 25 << 16).is_err());
        assert!(one(1, (28 << 16) | (1 << 31)).is_err());
    }

    /// Y88PackageData.al, Y88_Option_UndefinedOrdinal and Y88_Enum_UndefinedOrdinal: an ordinal the field does not define
    /// loads as it is (7 of an Option Zero,One,Two; 9 and -1 of an enum).
    #[test]
    fn an_undefined_option_or_enum_ordinal_loads_as_it_is() {
        let one = |n: i32, c: Column| {
            let mut f = header(7, &["O"]);
            f.push(1);
            f.extend_from_slice(&n.to_le_bytes());
            f.extend_from_slice(&[0; 16]);
            f.push(0);
            read(&f, 7, &[c]).map(|mut r| r.remove(0).values.remove(0))
        };
        let names: Vec<String> = ["Zero", "One", "Two"].iter().map(|s| s.to_string()).collect();
        assert!(matches!(
            one(7, col(RtType::Option(names.clone().into()))),
            Ok(Value::Option(7, _))
        ));
        assert!(matches!(one(9, col(RtType::Enum(3))), Ok(Value::Enum(3, 9))));
        assert!(matches!(one(-1, col(RtType::Enum(3))), Ok(Value::Enum(3, -1))));
    }
}
