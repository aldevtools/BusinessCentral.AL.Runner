//! Test data (ADR-020 point 3): the rows of a restored company, handed to the store when a session
//! first touches a table.
//!
//! The store asks [`TestBacking`] for the rows of a table it holds no entry for. The first ask of
//! a thread reads the table through the host's [`TableProvider`] (which reads each table once for
//! all threads), maps its columns onto the fields of the compiled table, rebuilds every value and
//! builds the table once; every later ask is a clone of that map. A table nothing touches is
//! never read.
//!
//! Names come from the compiled program, never from symbol files: a column `Name_` is the field
//! `Name.` (`sql_name`), a column `Field$<app id>` is the field of that name that the app's table
//! extension declares, and the platform's `timestamp` and `$system...` columns are the system
//! fields. Every exclusion is counted in the run's [`LoadStats`]: a column no field matches, an
//! extension column of an app the run does not install, a table the reader failed on, a table
//! whose values the runtime cannot rebuild. A value that cannot be rebuilt refuses the whole table
//! (it reads as empty, and the summary says so); a default is never put in its place.

use crate::Engine;
use al_builtins::describe::Describe;
use al_builtins::format::day_from_civil;
use al_builtins::value::{RtType, Value};
use al_data::memory::{Backing, Rows};
use al_data::{Row, TableDef};
use al_host::testdata::{
    Cell, Dropped, FieldKind, Outcome, Refusal, RequestField, TableError, TableRequest, TableRows, TestData,
};
use rust_decimal::Decimal;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

/// The first object id of the platform's tables (`Company`, `Field`, ...): the runtime keeps
/// their state itself.
const PLATFORM_FIRST: u32 = 2_000_000_000;
const SYSTEM_ROW_VERSION: u32 = 2_000_000_005;

/// BC's AL name to SQL identifier: the characters SQL Server does not allow in one become `_`.
pub(crate) fn sql_name(al_name: &str) -> String {
    al_name
        .chars()
        .map(|c| {
            if matches!(c, '.' | '"' | '\\' | '/' | '\'' | '%' | '[' | ']') {
                '_'
            } else {
                c
            }
        })
        .collect()
}

/// `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`.
fn is_guid(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    parts.len() == 5
        && [8, 4, 4, 4, 12]
            .iter()
            .zip(&parts)
            .all(|(n, p)| p.len() == *n && p.bytes().all(|b| b.is_ascii_hexdigit()))
}

/// A table extension's column `<SQL name>$<app id>`, split.
fn split_ext_column(col: &str) -> Option<(&str, &str)> {
    let i = col.rfind('$')?;
    (i > 0 && is_guid(&col[i + 1..])).then(|| (&col[..i], &col[i + 1..]))
}

fn eq_ci(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b) || (!a.is_ascii() || !b.is_ascii()) && a.to_uppercase() == b.to_uppercase()
}

/// How a column's cells become the values of the field it maps to.
#[derive(Clone)]
enum Kind {
    Text,
    Code,
    Bool,
    Int,
    BigInt,
    Byte,
    Option(std::sync::Arc<[String]>),
    Enum(u32),
    Decimal,
    Date,
    Time,
    DateTime,
    Duration,
    Guid,
    RowVersion,
    /// A DateFormula, stored as BC's token string.
    DateFormula,
    /// A Blob; `compressed` is the field's `Compressed` property (on unless it says false).
    Blob {
        compressed: bool,
    },
    /// A type whose stored form the runtime does not rebuild; only an empty cell is accepted.
    Unsupported(String),
}

fn kind_of_field(ty: &RtType, compressed: bool) -> Kind {
    match ty {
        RtType::DateFormula => Kind::DateFormula,
        RtType::Library(al_builtins::library_kind::LibraryKind::Blob) => Kind::Blob { compressed },
        RtType::Text(_) => Kind::Text,
        RtType::Code(_) => Kind::Code,
        RtType::Boolean => Kind::Bool,
        RtType::Integer => Kind::Int,
        RtType::BigInteger => Kind::BigInt,
        RtType::Byte => Kind::Byte,
        RtType::Option(names) => Kind::Option(std::sync::Arc::from(names.as_slice())),
        RtType::Enum(id) => Kind::Enum(*id),
        RtType::Decimal => Kind::Decimal,
        RtType::Date => Kind::Date,
        RtType::Time => Kind::Time,
        RtType::DateTime => Kind::DateTime,
        RtType::Duration => Kind::Duration,
        RtType::Guid => Kind::Guid,
        RtType::Library(
            al_builtins::library_kind::LibraryKind::Media | al_builtins::library_kind::LibraryKind::MediaSet,
        ) => Kind::Guid,
        other => Kind::Unsupported(other.describe()),
    }
}

/// The kind of a field's value for a source whose values are text.
fn field_kind(ty: &RtType) -> FieldKind {
    match ty {
        RtType::DateFormula => FieldKind::DateFormula,
        RtType::Text(_) => FieldKind::Text,
        RtType::Code(_) => FieldKind::Code,
        RtType::Boolean => FieldKind::Boolean,
        RtType::Integer => FieldKind::Integer,
        RtType::BigInteger => FieldKind::BigInteger,
        RtType::Byte => FieldKind::Byte,
        RtType::Option(_) | RtType::Enum(_) => FieldKind::Ordinal,
        RtType::Decimal => FieldKind::Decimal,
        RtType::Date => FieldKind::Date,
        RtType::Time => FieldKind::Time,
        RtType::DateTime => FieldKind::DateTime,
        RtType::Duration => FieldKind::Duration,
        RtType::Guid => FieldKind::Guid,
        RtType::Library(
            al_builtins::library_kind::LibraryKind::Blob
            | al_builtins::library_kind::LibraryKind::Media
            | al_builtins::library_kind::LibraryKind::MediaSet,
        ) => FieldKind::Binary,
        _ => FieldKind::Other,
    }
}

/// The text order of a Guid the database stores in .NET's byte order.
fn guid(b: &[u8; 16]) -> [u8; 16] {
    [
        b[3], b[2], b[1], b[0], b[5], b[4], b[7], b[6], b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15],
    ]
}

const MS_PER_DAY: i64 = 86_400_000;

/// A date-time cell as (day number, milliseconds of the day); day 1 is 0001-01-01.
fn instant(c: &Cell) -> Option<(i32, i64)> {
    match c {
        Cell::DateTime { days, ticks } => {
            // SQL `datetime` counts 1/300 s; .NET rounds the tick count to whole milliseconds.
            let ms = (i64::from(*ticks) * 20 + 3) / 6;
            Some((day_from_civil(1900, 1, 1) + days, ms))
        }
        Cell::Date { days } => Some((i32::try_from(days + 1).ok()?, 0)),
        Cell::DateTime2 { days, units, scale } => {
            let per_ms = 10i64.checked_pow(u32::from(*scale))?.max(1);
            let ms = units.checked_mul(1000)? / per_ms;
            Some((i32::try_from(days + 1).ok()?, ms))
        }
        _ => None,
    }
}

/// 1753-01-01 00:00:00, the `datetime` value BC stores for a blank Date, Time or DateTime.
fn is_blank_instant(day: i32, ms: i64) -> bool {
    day == day_from_civil(1753, 1, 1) && ms == 0
}

fn int_of(c: &Cell) -> Option<i64> {
    match c {
        Cell::Int(i) => Some(*i),
        Cell::Bool(b) => Some(i64::from(*b)),
        _ => None,
    }
}

/// The value a stored cell is for a field of kind `kind`, or why the runtime cannot rebuild it.
/// `default` is the field's blank, which a database NULL becomes.
fn convert(cell: &Cell, kind: &Kind, default: &Value) -> Result<Value, String> {
    // A blank blob is a buffer of its own: rows must not share one.
    let blank = || match kind {
        Kind::Blob { .. } => Value::Blob(al_builtins::stream::new_blob()),
        _ => default.clone(),
    };
    if matches!(cell, Cell::Null) {
        return Ok(blank());
    }
    let bad = |what: &str| Err(format!("expected {what}, got {}", cell));
    Ok(match kind {
        Kind::Text | Kind::Code => match cell {
            Cell::Text(s) => {
                if matches!(kind, Kind::Code) {
                    Value::Code(s.clone().into())
                } else {
                    Value::Text(s.clone().into())
                }
            }
            _ => return bad("text"),
        },
        Kind::Bool => match int_of(cell) {
            Some(0) => Value::Boolean(false),
            Some(1) => Value::Boolean(true),
            _ => return bad("0 or 1"),
        },
        Kind::Int => match int_of(cell).and_then(|i| i32::try_from(i).ok()) {
            Some(i) => Value::Integer(i),
            None => return bad("an integer"),
        },
        Kind::BigInt => match int_of(cell) {
            Some(i) => Value::BigInteger(i),
            None => return bad("a big integer"),
        },
        Kind::Byte => match int_of(cell).and_then(|i| u8::try_from(i).ok()) {
            Some(i) => Value::Byte(i),
            None => return bad("a byte"),
        },
        Kind::Option(names) => match int_of(cell).and_then(|i| i32::try_from(i).ok()) {
            Some(i) => Value::Option(i, names.clone()),
            None => return bad("an option ordinal"),
        },
        Kind::Enum(id) => match int_of(cell).and_then(|i| i32::try_from(i).ok()) {
            Some(i) => Value::Enum(*id, i),
            None => return bad("an enum ordinal"),
        },
        Kind::Decimal => match cell {
            Cell::Decimal {
                negative,
                magnitude,
                scale,
            } => {
                let m = i128::try_from(*magnitude).map_err(|_| format!("the decimal {} is too large", cell))?;
                let m = if *negative { -m } else { m };
                match Decimal::try_from_i128_with_scale(m, u32::from(*scale)) {
                    Ok(d) => Value::Decimal(d.normalize()),
                    Err(_) => return Err(format!("the decimal {} does not fit a Decimal", cell)),
                }
            }
            Cell::Int(i) => Value::Decimal(Decimal::from(*i)),
            _ => return bad("a decimal"),
        },
        Kind::Date => {
            let Some((day, ms)) = instant(cell) else {
                return bad("a date");
            };
            if is_blank_instant(day, ms) {
                Value::Date(0, false)
            } else if matches!(cell, Cell::DateTime { .. }) && day < day_from_civil(1754, 1, 1) {
                // BC writes no real date below 1754-01-01 to a `datetime` column, so this is not a
                // value it stored.
                return Err(format!("the date {cell} is before 1754-01-01 and not the blank marker"));
            } else {
                Value::Date(day, false)
            }
        }
        Kind::Time => {
            if let Cell::Time { units, scale } = cell {
                // `time(n)` has no blank marker: midnight is a time.
                let per_ms = 10i64
                    .checked_pow(u32::from(*scale))
                    .ok_or("a time scale out of range")?
                    .max(1);
                return Ok(Value::Time(units.saturating_mul(1000) / per_ms + 1));
            }
            let Some((day, ms)) = instant(cell) else {
                return bad("a time");
            };
            if is_blank_instant(day, ms) {
                Value::Time(0)
            } else {
                Value::Time(ms.rem_euclid(MS_PER_DAY) + 1)
            }
        }
        Kind::DateTime => {
            let Some((day, ms)) = instant(cell) else {
                return bad("a date and time");
            };
            if is_blank_instant(day, ms) {
                Value::DateTime(0)
            } else {
                Value::DateTime(i64::from(day - 1) * MS_PER_DAY + ms + 1)
            }
        }
        Kind::Duration => match int_of(cell) {
            Some(ms) => Value::Duration(ms),
            None => return bad("a duration in milliseconds"),
        },
        Kind::Guid => match cell {
            Cell::Guid(b) => Value::Guid(guid(b)),
            _ => return bad("a GUID"),
        },
        Kind::RowVersion => match cell {
            Cell::RowVersion(v) => Value::BigInteger(*v as i64),
            _ => return bad("a row version"),
        },
        Kind::DateFormula => match cell {
            Cell::Text(s) if s.is_empty() => default.clone(),
            Cell::Text(s) => Value::DateFormula(date_formula_text(s)),
            _ => return bad("a date formula"),
        },
        Kind::Blob { compressed } => match cell {
            Cell::Binary(stored) if stored.is_empty() => blank(),
            Cell::Binary(stored) => {
                let bytes = blob_bytes(stored, *compressed)?;
                let blob = al_builtins::stream::new_blob();
                al_builtins::stream::write_bytes(&blob, &bytes);
                Value::Blob(blob)
            }
            _ => return bad("binary data"),
        },
        Kind::Unsupported(ty) => {
            // The blank of a type is what an empty cell means in every case.
            let empty = match cell {
                Cell::Text(s) => s.is_empty(),
                // A binary column BC reads into a zero-padded buffer (RecordId, TableFilter) holds
                // the blank value as zeros: its parser stops at the first zero.
                Cell::Binary(b) => b.iter().all(|&x| x == 0),
                _ => false,
            };
            if empty {
                return Ok(blank());
            }
            return Err(format!("a stored {ty} value is not rebuilt yet"));
        }
    })
}

/// A DateFormula column as the formula text the runtime keeps (`<1M+CM>`). BC stores each of the
/// seven quantors as one control character, `C D WD W M Q Y` as 1 to 7, and the rest as written: the
/// demo database shows it (Payment Terms "10 DAYS" holds `10` and 2, Company Information's `1Y` holds
/// `1` and 7). The table is not pinned by a corpus test (docs/probes-needed.md, "Test data"); AL code
/// reads a stored DateFormula field without brackets (recordref/lt-refs/RfTValues.al QP_DateFormula_*).
fn date_formula_text(stored: &str) -> String {
    const QUANTORS: [&str; 7] = ["C", "D", "WD", "W", "M", "Q", "Y"];
    let mut out = String::from("<");
    for c in stored.chars() {
        match (c as usize).checked_sub(1).and_then(|i| QUANTORS.get(i)) {
            Some(q) => out.push_str(q),
            None => out.push(c),
        }
    }
    out.push('>');
    out
}

/// BC's marker in front of a compressed Blob (`NavBLOB.BlobMagic`).
const BLOB_MAGIC: [u8; 4] = [0x02, 0x45, 0x7D, 0x5B];

/// A Blob column's content (`NavSqlCommand.GetBlobDataFromReader`): bytes as stored when the field
/// is not compressed; the marker and a raw Deflate stream when it is. A column that does not match
/// what its field says is refused: BC's own reader fails there rather than read the bytes as
/// content, and storing the container as the value would be a wrong blob.
fn blob_bytes(stored: &[u8], compressed: bool) -> Result<Vec<u8>, String> {
    let marked = stored.starts_with(&BLOB_MAGIC);
    if !compressed {
        if marked {
            return Err(
                "the field is not compressed but the stored bytes start with BC's compressed-BLOB marker".into(),
            );
        }
        return Ok(stored.to_vec());
    }
    if !marked {
        return Err(format!(
            "the field is compressed but its {} stored bytes do not start with BC's compressed-BLOB marker",
            stored.len()
        ));
    }
    use std::io::Read;
    let mut out = Vec::new();
    flate2::read::DeflateDecoder::new(&stored[BLOB_MAGIC.len()..])
        .read_to_end(&mut out)
        .map_err(|e| format!("the compressed-BLOB marker is followed by no Deflate stream ({e})"))?;
    Ok(out)
}

/// One column of the provider's rows, mapped to a field.
struct Mapped {
    column: usize,
    field: usize,
    kind: Kind,
}

/// How a table's columns map onto its compiled fields.
struct Plan {
    mapped: Vec<Mapped>,
    dropped: Dropped,
}

pub(crate) struct TestBacking {
    engine: Arc<Engine>,
    data: TestData,
    /// By table object: the table built from the restored rows, once per thread.
    cache: RefCell<HashMap<u32, Option<Rows>>>,
}

impl TestBacking {
    pub(crate) fn new(engine: Arc<Engine>, data: TestData) -> TestBacking {
        TestBacking {
            engine,
            data,
            cache: RefCell::default(),
        }
    }

    fn app_id(&self, app: u32) -> Option<&str> {
        self.engine.program.apps.get(app as usize).map(|a| a.id.as_str())
    }

    /// Maps the columns of `rows` onto the fields of table `object`. `Err`: the table cannot be
    /// loaded and why.
    fn plan(&self, object: u32, rows: &TableRows) -> Result<Plan, String> {
        let program = &self.engine.program;
        let obj = &program.objects[object as usize];
        let schema = obj.table.as_ref().ok_or("not a table")?;
        let table_app = self.app_id(obj.app);
        let normalized: Vec<String> = schema.fields.iter().map(|f| sql_name(&f.name)).collect();
        let owner_of = |fi: usize| -> Option<&str> {
            let d = schema.fields[fi].declared_in?;
            self.app_id(program.objects[d as usize].app)
        };
        let mut mapped: Vec<Mapped> = Vec::new();
        let mut dropped = Dropped::default();
        let mut taken: HashMap<usize, usize> = HashMap::new();
        for (ci, col) in rows.columns.iter().enumerate() {
            let candidates: Vec<usize>;
            if col == "timestamp" {
                candidates = (0..schema.fields.len())
                    .filter(|&i| schema.fields[i].id == SYSTEM_ROW_VERSION)
                    .collect();
            } else if let Some(sys) = col.strip_prefix('$') {
                candidates = (0..schema.fields.len())
                    .filter(|&i| schema.fields[i].id >= PLATFORM_FIRST && eq_ci(&schema.fields[i].name, sys))
                    .collect();
            } else if let Some((base, app)) = split_ext_column(col) {
                let installed = program.apps.iter().any(|a| eq_ci(&a.id, app));
                if !installed {
                    dropped.uninstalled_app += 1;
                    continue;
                }
                candidates = (0..schema.fields.len())
                    .filter(|&i| eq_ci(&normalized[i], base) && owner_of(i).is_some_and(|o| eq_ci(o, app)))
                    .collect();
            } else {
                // A plain column belongs to the table's own app: its table and the extensions that
                // app declares (BC stores those in the base table itself).
                candidates = (0..schema.fields.len())
                    .filter(|&i| {
                        eq_ci(&normalized[i], col)
                            && schema.fields[i].id < PLATFORM_FIRST
                            && match (owner_of(i), table_app) {
                                (Some(o), Some(t)) => eq_ci(o, t),
                                _ => true,
                            }
                    })
                    .collect();
            }
            match candidates.as_slice() {
                [] => dropped.no_such_field += 1,
                [fi] => {
                    if let Some(other) = taken.insert(*fi, ci) {
                        return Err(format!(
                            "columns '{}' and '{col}' both map to field '{}'",
                            rows.columns[other], schema.fields[*fi].name
                        ));
                    }
                    let kind = if col == "timestamp" {
                        Kind::RowVersion
                    } else {
                        // A Blob is compressed unless its field says `Compressed = false`.
                        let compressed = !schema.fields[*fi].properties.iter().any(|(k, v)| {
                            k.eq_ignore_ascii_case("Compressed") && v.trim().eq_ignore_ascii_case("false")
                        });
                        kind_of_field(&schema.fields[*fi].ty, compressed)
                    };
                    mapped.push(Mapped {
                        column: ci,
                        field: *fi,
                        kind,
                    });
                }
                many => {
                    return Err(format!(
                        "column '{col}' fits {} fields ({})",
                        many.len(),
                        many.iter()
                            .map(|&i| schema.fields[i].name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
            }
        }
        if mapped.is_empty() {
            return Err("its rows share no column with the compiled table".into());
        }
        Ok(Plan { mapped, dropped })
    }

    /// Builds the table from the provider's rows. `Err`: why the runtime refuses it.
    fn build(&self, t: &TableDef, rows: &TableRows, plan: &Plan) -> Result<Rows, String> {
        let defaults: Vec<Value> = t.fields.iter().map(|f| Value::default_for(&f.ty)).collect();
        // A field no column fills still gets a blob of its own in each row.
        let blobs: Vec<usize> = (0..t.fields.len())
            .filter(|&i| matches!(defaults[i], Value::Blob(_)))
            .collect();
        // The AutoIncrement field of the table and whether the source holds a value for it. A source of untyped values
        // (a configuration package) holds none: the database numbered those rows as it inserted them.
        let auto = self.engine.program.objects[t.object as usize]
            .table
            .as_ref()
            .and_then(|s| s.fields.iter().position(|f| f.auto_increment));
        let numbered = auto
            .filter(|&i| self.data.provider.wants_fields() && !plan.mapped.iter().any(|m| m.field == i))
            .filter(|&i| matches!(t.fields[i].ty, RtType::Integer | RtType::BigInteger));
        let mut out: Vec<Row> = Vec::with_capacity(rows.rows.len());
        for r in &rows.rows {
            let mut row = defaults.clone();
            for &i in &blobs {
                row[i] = Value::default_for(&t.fields[i].ty);
            }
            for m in &plan.mapped {
                let cell = r.get(m.column).unwrap_or(&Cell::Null);
                row[m.field] = convert(cell, &m.kind, &defaults[m.field]).map_err(|why| {
                    format!(
                        "column '{}' (field '{}'): {why}",
                        rows.columns[m.column], t.fields[m.field].name
                    )
                })?;
                if let (Kind::RowVersion, Cell::RowVersion(v)) = (&m.kind, cell) {
                    self.data.stats.note_row_version(*v);
                }
            }
            if let Some(i) = numbered {
                let n = out.len() as i64 + 1;
                row[i] = if matches!(t.fields[i].ty, RtType::BigInteger) {
                    Value::BigInteger(n)
                } else {
                    Value::Integer(n as i32)
                };
            }
            out.push(row);
        }
        if numbered.is_some() && !out.is_empty() {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::PackageAutoIncrementFromOne);
        }
        let floor = auto.map_or(0, |i| {
            out.iter()
                .filter_map(|r| match &r[i] {
                    Value::Integer(v) => Some(i64::from(*v)),
                    Value::BigInteger(v) => Some(*v),
                    _ => None,
                })
                .max()
                .unwrap_or(0)
        });
        Rows::from_rows(t, out)
            .map(|r| r.with_auto_increment_floor(floor))
            .map_err(|n| format!("{n} rows share a primary key of the compiled table"))
    }

    /// The fields of a table for a source that needs the schema to type its values: each with the column name the
    /// plan below expects (the SQL name; `<SQL name>$<app id>` for a field a table extension of another app declares).
    fn request_fields(&self, obj: &al_ir::Object) -> Vec<RequestField> {
        let Some(schema) = obj.table.as_ref() else {
            return Vec::new();
        };
        let table_app = self.app_id(obj.app);
        schema
            .fields
            .iter()
            .filter(|f| f.id < PLATFORM_FIRST)
            .map(|f| {
                let owner = f
                    .declared_in
                    .and_then(|d| self.app_id(self.engine.program.objects[d as usize].app));
                let column = match (owner, table_app) {
                    (Some(o), Some(t)) if !eq_ci(o, t) => format!("{}${o}", sql_name(&f.name)),
                    _ => sql_name(&f.name),
                };
                RequestField {
                    id: f.id,
                    name: f.name.clone(),
                    column,
                    kind: field_kind(&f.ty),
                }
            })
            .collect()
    }

    fn load(&self, object: u32, t: &TableDef) -> Option<Rows> {
        let program = &self.engine.program;
        let obj = &program.objects[object as usize];
        let stats = &self.data.stats;
        let record = |o: Outcome| {
            stats.record(object, &obj.name, o);
        };
        // A temporary table has no rows in a database.
        if al_ir::ObjectFacts::of(obj).temporary() {
            return None;
        }
        if obj.id.unwrap_or(0) >= PLATFORM_FIRST {
            record(Outcome::Refused(Refusal::Platform));
            return None;
        }
        let mut request = TableRequest {
            name: obj.name.clone(),
            app_id: self.app_id(obj.app).map(str::to_string),
            per_company: t.per_company,
            ..Default::default()
        };
        if self.data.provider.wants_fields() {
            request.table_id = obj.id;
            request.fields = self.request_fields(obj);
        }
        let rows = match self.data.read(&request) {
            Ok(r) => r,
            Err(TableError::Absent) => {
                record(Outcome::Empty);
                return None;
            }
            Err(TableError::Ambiguous(why)) => {
                record(Outcome::Refused(Refusal::Ambiguous(why)));
                return None;
            }
            Err(TableError::Reader(why)) => {
                record(Outcome::Refused(Refusal::Reader(why)));
                return None;
            }
        };
        if rows.rows.is_empty() {
            record(Outcome::Empty);
            return None;
        }
        let built = self
            .plan(object, &rows)
            .and_then(|plan| self.build(t, &rows, &plan).map(|b| (b, plan.dropped)));
        match built {
            Ok((table, dropped)) => {
                record(Outcome::Loaded {
                    rows: table.len(),
                    dropped,
                });
                Some(table)
            }
            Err(why) => {
                record(Outcome::Refused(Refusal::Unsupported(why)));
                None
            }
        }
    }
}

impl Backing for TestBacking {
    fn company(&self) -> &str {
        self.data.provider.company()
    }

    fn rows(&self, company: &str, object: u32) -> Option<Rows> {
        let t = self.engine.tables.get(object as usize)?.as_ref()?;
        let wanted = if t.per_company { self.company() } else { "" };
        if !eq_ci(company, wanted) {
            return None;
        }
        if let Some(hit) = self.cache.borrow().get(&object) {
            return hit.clone();
        }
        let built = self.load(object, t);
        self.cache.borrow_mut().insert(object, built.clone());
        built
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Value` has no `PartialEq`; its debug text names every case and payload.
    macro_rules! assert_value {
        ($got:expr, $want:expr) => {
            assert_eq!(format!("{:?}", $got), format!("{:?}", $want))
        };
    }

    fn conv(cell: Cell, ty: &RtType) -> Result<Value, String> {
        convert(&cell, &kind_of_field(ty, true), &Value::default_for(ty))
    }

    #[test]
    fn column_names_follow_the_sql_rules() {
        assert_eq!(sql_name("Reten. Pol. Filtering "), "Reten_ Pol_ Filtering ");
        assert_eq!(sql_name("G/L Account \"No.\""), "G_L Account _No__");
        assert_eq!(
            split_ext_column("Extra$bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb")
                .unwrap()
                .0,
            "Extra"
        );
        assert!(split_ext_column("Cost$Center").is_none());
    }

    #[test]
    fn a_database_null_is_the_fields_blank() {
        assert_value!(conv(Cell::Null, &RtType::Integer).unwrap(), Value::Integer(0));
        assert_value!(
            conv(Cell::Null, &RtType::Code(Some(10))).unwrap(),
            Value::Code(String::new().into())
        );
        assert_value!(conv(Cell::Null, &RtType::Date).unwrap(), Value::Date(0, false));
    }

    #[test]
    fn scalars_are_rebuilt_by_the_fields_type() {
        assert_value!(conv(Cell::Int(7), &RtType::Integer).unwrap(), Value::Integer(7));
        assert_value!(conv(Cell::Int(1), &RtType::Boolean).unwrap(), Value::Boolean(true));
        assert_value!(conv(Cell::Int(3), &RtType::Enum(9)).unwrap(), Value::Enum(9, 3));
        assert!(conv(Cell::Int(2), &RtType::Boolean).is_err());
        assert!(conv(Cell::Int(1 << 40), &RtType::Integer).is_err());
        assert_value!(
            conv(Cell::Text("A".into()), &RtType::Code(None)).unwrap(),
            Value::Code("A".into())
        );
        let d = conv(
            Cell::Decimal {
                negative: true,
                magnitude: 1_250_000_000_000_000_000_000,
                scale: 20,
            },
            &RtType::Decimal,
        )
        .unwrap();
        assert_value!(d, Value::Decimal(Decimal::new(-125, 1)));
    }

    #[test]
    fn dates_times_and_date_times_keep_the_blank_apart_from_real_values() {
        let blank = Cell::DateTime { days: -53690, ticks: 0 };
        assert_value!(conv(blank.clone(), &RtType::Date).unwrap(), Value::Date(0, false));
        assert_value!(conv(blank.clone(), &RtType::Time).unwrap(), Value::Time(0));
        assert_value!(conv(blank, &RtType::DateTime).unwrap(), Value::DateTime(0));
        // 2020-12-31 12:00:00 is day 43830 after 1900-01-01
        let day = day_from_civil(2020, 12, 31);
        let noon = Cell::DateTime {
            days: day - day_from_civil(1900, 1, 1),
            ticks: 12 * 3600 * 300,
        };
        assert_value!(conv(noon.clone(), &RtType::Date).unwrap(), Value::Date(day, false));
        assert_value!(
            conv(noon.clone(), &RtType::Time).unwrap(),
            Value::Time(12 * 3_600_000 + 1)
        );
        assert_value!(
            conv(noon, &RtType::DateTime).unwrap(),
            Value::DateTime(i64::from(day - 1) * 86_400_000 + 12 * 3_600_000 + 1)
        );
        // midnight on a real day is a real time, not the blank
        let midnight = Cell::DateTime {
            days: day - day_from_civil(1900, 1, 1),
            ticks: 0,
        };
        assert_value!(conv(midnight, &RtType::Time).unwrap(), Value::Time(1));
    }

    #[test]
    fn a_blob_is_inflated_when_its_field_is_compressed_and_taken_as_stored_when_not() {
        use std::io::Write;
        let content = b"hello hello hello hello".to_vec();
        let mut z = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
        z.write_all(&content).unwrap();
        let mut stored = BLOB_MAGIC.to_vec();
        stored.extend(z.finish().unwrap());
        let blob = |c: &Cell, compressed| {
            convert(
                c,
                &Kind::Blob { compressed },
                &Value::default_for(&RtType::Library("Blob".into())),
            )
        };
        let Value::Blob(b) = blob(&Cell::Binary(stored.clone()), true).unwrap() else {
            panic!("a blob")
        };
        assert_eq!(b.borrow().bytes, content);
        let Value::Blob(b) = blob(&Cell::Binary(content.clone()), false).unwrap() else {
            panic!("a blob")
        };
        assert_eq!(b.borrow().bytes, content);
        // the field and the bytes disagree: refused, never read as content
        assert!(
            blob(&Cell::Binary(content), true)
                .unwrap_err()
                .contains("do not start with")
        );
        assert!(
            blob(&Cell::Binary(stored), false)
                .unwrap_err()
                .contains("not compressed")
        );
        // empty and NULL are the blank blob
        let Value::Blob(b) = blob(&Cell::Binary(Vec::new()), true).unwrap() else {
            panic!("a blob")
        };
        assert!(b.borrow().bytes.is_empty());
        assert!(matches!(blob(&Cell::Null, true).unwrap(), Value::Blob(_)));
    }

    #[test]
    fn a_sql_time_column_is_a_time_of_day() {
        // 13:14:15.1234567 in `time(7)` units of 100 ns
        let cell = Cell::Time {
            units: (13 * 3600 + 14 * 60 + 15) * 10_000_000 + 1_234_567,
            scale: 7,
        };
        assert_value!(
            conv(cell, &RtType::Time).unwrap(),
            Value::Time((13 * 3600 + 14 * 60 + 15) * 1000 + 123 + 1)
        );
    }

    #[test]
    fn a_guid_is_turned_from_storage_order_to_text_order() {
        let stored = [
            0x78, 0x56, 0x34, 0x12, 0xBC, 0x9A, 0xF0, 0xDE, 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF,
        ];
        assert_value!(
            conv(Cell::Guid(stored), &RtType::Guid).unwrap(),
            Value::Guid([
                0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF
            ])
        );
    }

    #[test]
    fn a_date_formula_is_decoded_from_bcs_token_string() {
        let ty = RtType::DateFormula;
        let df = |s: &str| conv(Cell::Text(s.into()), &ty).unwrap();
        assert_value!(df(""), Value::DateFormula(String::new()));
        // measured on the demo database: Payment Terms "10 DAYS" is "10" and token 2; the
        // quantors are C D WD W M Q Y in that order (Microsoft's NavDateFormulaFormatter)
        assert_value!(df("10\u{2}"), Value::DateFormula("<10D>".into()));
        assert_value!(df("1\u{7}"), Value::DateFormula("<1Y>".into()));
        assert_value!(df("1\u{5}+\u{1}\u{5}"), Value::DateFormula("<1M+CM>".into()));
        assert_value!(df("-\u{3}1+2\u{4}"), Value::DateFormula("<-WD1+2W>".into()));
        assert_value!(df("1\u{6}"), Value::DateFormula("<1Q>".into()));
    }

    #[test]
    fn a_value_of_a_type_the_runtime_does_not_rebuild_is_refused_unless_it_is_empty() {
        let ty = RtType::Library("RecordId".into());
        assert!(conv(Cell::Binary(Vec::new()), &ty).is_ok());
        // BC reads such a column into a zero-padded buffer, and its parser stops at the first
        // zero, so a cell of zeros is the blank value (the demo database holds six zero bytes)
        assert!(conv(Cell::Binary(vec![0; 6]), &ty).is_ok());
        assert!(
            conv(Cell::Binary(vec![0, 0, 1, 0]), &ty)
                .unwrap_err()
                .contains("not rebuilt")
        );
        assert!(conv(Cell::Binary(vec![1, 2]), &ty).unwrap_err().contains("not rebuilt"));
    }
}
