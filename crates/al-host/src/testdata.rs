//! Test data from a database file (ADR-020 point 3): the capability a host gives the runtime to
//! read the rows of one table of a restored company, and the counts of what a run took from it.
//!
//! The core never sees a file. A host (`al-runner`) implements [`TableProvider`] over a BC database
//! backup; the runtime asks it for a table the first time a session touches that table. The
//! provider answers in plain typed cells under the table's SQL column names. Mapping those names
//! to the fields of the compiled program is the runtime's job (the program is the schema, never a
//! symbol file).

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, OnceLock};

/// One value as the database stores it (the same cases as `al-bcdb`'s `Value`, which a host
/// converts one to one).
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    Null,
    /// SQL `bit`.
    Bool(bool),
    /// tinyint, smallint, int, bigint.
    Int(i64),
    /// SQL `real` and `float`.
    Float(f64),
    /// decimal/numeric: `(-1)^negative * magnitude / 10^scale`.
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
    /// `datetime2(scale)`: days since 0001-01-01 and units of 10^-scale seconds.
    DateTime2 {
        days: i64,
        units: i64,
        scale: u8,
    },
    /// `uniqueidentifier` in SQL Server's storage byte order (.NET `Guid(byte[])`).
    Guid([u8; 16]),
    /// binary, varbinary, image.
    Binary(Vec<u8>),
    /// `rowversion`/`timestamp`.
    RowVersion(u64),
    Text(String),
}

impl std::fmt::Display for Cell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Cell::Null => f.write_str("NULL"),
            Cell::Bool(b) => write!(f, "bit {}", u8::from(*b)),
            Cell::Int(i) => write!(f, "int {i}"),
            Cell::Float(x) => write!(f, "float {x}"),
            Cell::Decimal {
                negative,
                magnitude,
                scale,
            } => {
                let digits = format!("{:0>width$}", magnitude, width = usize::from(*scale) + 1);
                let (whole, frac) = digits.split_at(digits.len() - usize::from(*scale));
                let sign = if *negative { "-" } else { "" };
                if frac.is_empty() {
                    write!(f, "decimal {sign}{whole}")
                } else {
                    write!(f, "decimal {sign}{whole}.{frac}")
                }
            }
            Cell::DateTime { days, .. } => write!(f, "datetime (day {days} since 1900-01-01)"),
            Cell::Date { days } => write!(f, "date (day {days} since 0001-01-01)"),
            Cell::Time { .. } => f.write_str("time"),
            Cell::DateTime2 { days, .. } => write!(f, "datetime2 (day {days} since 0001-01-01)"),
            Cell::Guid(_) => f.write_str("uniqueidentifier"),
            Cell::Binary(b) => write!(f, "binary of {} bytes", b.len()),
            Cell::RowVersion(_) => f.write_str("rowversion"),
            Cell::Text(s) => write!(f, "text '{s}'"),
        }
    }
}

/// What the runtime asks for: one table of the restored company.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TableRequest {
    /// The table's AL name (`G/L Entry`).
    pub name: String,
    /// The id of the app that declares the table; `None` for a platform table.
    pub app_id: Option<String>,
    /// `DataPerCompany`: false reads the table without a company.
    pub per_company: bool,
    /// The table's AL object id. Filled only for a provider that asks for it ([`TableProvider::wants_fields`]).
    pub table_id: Option<u32>,
    /// The fields of the compiled table, with the kind of value each holds. Filled only for a provider that
    /// asks for them: a source that stores its values as text (a configuration package) needs the schema to type them.
    pub fields: Vec<RequestField>,
}

/// One field of the compiled table a provider may need to type its values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestField {
    /// The field number.
    pub id: u32,
    /// The AL name of the field (`Due Date Calculation`).
    pub name: String,
    /// The column name the runtime expects for this field in [`TableRows::columns`].
    pub column: String,
    pub kind: FieldKind,
}

/// The kind of value a field holds, as far as a source with untyped values needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    Text,
    Code,
    Boolean,
    Integer,
    BigInteger,
    Byte,
    /// An Option or Enum: the ordinal.
    Ordinal,
    Decimal,
    Date,
    Time,
    DateTime,
    /// Milliseconds.
    Duration,
    Guid,
    DateFormula,
    /// A Blob, Media or MediaSet.
    Binary,
    /// Anything else; only a blank value can be given.
    Other,
}

/// The rows of one table, columns named as the database names them: a base column by its SQL
/// name (`Reten_ Pol_ Filtering`), a table extension's by `<SQL name>$<app id of the extension>`,
/// a system field by `$systemId`, the row version by `timestamp`. Extension columns are already
/// joined to the base rows (`$ext` companions merged on their own key).
#[derive(Debug, Clone, PartialEq)]
pub struct TableRows {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Cell>>,
}

/// Why a provider has no rows for a table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableError {
    /// The database has no such table in this company (the table holds no data there).
    Absent,
    /// Several tables of the database answer to the name and no app id selects one.
    Ambiguous(String),
    /// The reader failed on this table; its own text, never fatal to other tables.
    Reader(String),
}

/// A number sequence the restored database holds (`NumberSequence` of AL, a SQL sequence in the backup).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestSequence {
    /// The name AL code uses.
    pub name: String,
    /// A sequence of the restored company; false: one that belongs to no company.
    pub company_specific: bool,
    /// The sequence's start value.
    pub start: i64,
    pub increment: i64,
    /// The last value handed out; equal to `start` when it is not known to have handed one out.
    pub current: i64,
}

/// Rows of a restored company, read table by table on demand. One provider is shared by every test
/// thread; [`TestData::read`] makes sure it is asked for a table once.
pub trait TableProvider: Send + Sync {
    /// The restored company's name as AL sees it (`CompanyName()`).
    fn company(&self) -> &str;
    /// A short name of the source for the summary line (the file name).
    fn source(&self) -> String;
    /// Reads the rows of the table from the source.
    fn read(&self, table: &TableRequest) -> Result<Arc<TableRows>, TableError>;
    /// What opening the source and reading from it cost so far, for `--timings`. Additive: a
    /// provider that does not measure answers `None`.
    fn cost(&self) -> Option<String> {
        None
    }
    /// What the rows are, for a key of something computed from them (the install baseline in the compile cache):
    /// the same text means the same rows. `None`: the source cannot say, and nothing is kept for it.
    fn identity(&self) -> Option<String> {
        None
    }
    /// Whether [`TableRequest::table_id`] and [`TableRequest::fields`] are filled for this provider. A backup knows
    /// its columns' types and says no; a configuration package stores text and says yes. Additive.
    fn wants_fields(&self) -> bool {
        false
    }
    /// The number sequences of the restored database (a company's own and the global ones). Additive: a
    /// provider without any answers none.
    fn sequences(&self) -> Vec<TestSequence> {
        Vec::new()
    }
    /// The last row version the database handed out (SQL Server's `@@DBTS` when the backup was taken), at or above the
    /// `timestamp` of every row it holds: a restored session hands out row versions after it. `None` when the source
    /// does not hold it (a `.bacpac`, a configuration package). Additive.
    fn last_row_version(&self) -> Option<u64> {
        None
    }
    /// The names of every company the database holds, as AL spells them, in the key order of its Company table
    /// (`--companies all`, ADR-046 decision 2). Additive: a provider that knows one company answers it.
    fn companies(&self) -> Vec<String> {
        vec![self.company().to_string()]
    }
    /// Whether the database holds rows of `table` in `company` (as AL spells it), another company than [`company`]
    /// (ADR-046 amendment 4): `Some(false)` when it holds none there. `None`: not known. Additive.
    ///
    /// [`company`]: TableProvider::company
    fn holds_rows_in(&self, company: &str, table: &TableRequest) -> Option<bool> {
        let _ = (company, table);
        None
    }
}

type Read = Result<Arc<TableRows>, TableError>;

/// The test data of a run: the provider, the counts of what the runtime took from it, and the
/// tables already read. Cloning shares all three.
#[derive(Clone)]
pub struct TestData {
    pub provider: Arc<dyn TableProvider>,
    pub stats: Arc<LoadStats>,
    /// One slot per table. A thread that finds the slot empty reads the table; one that arrives
    /// meanwhile waits for it, and reads of other tables go on in parallel.
    reads: Arc<Mutex<HashMap<String, Arc<OnceLock<Read>>>>>,
    /// [`TableProvider::holds_rows_in`] by company and table, asked once.
    elsewhere: Arc<Mutex<HashMap<String, Option<bool>>>>,
    /// A number no other test data of the process has, shared by the clones: the same id means the same rows.
    pub id: u64,
}

static NEXT_TEST_DATA_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl std::fmt::Debug for TestData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TestData({})", self.provider.source())
    }
}

impl TestData {
    pub fn new(provider: Arc<dyn TableProvider>) -> TestData {
        TestData {
            provider,
            stats: Arc::new(LoadStats::default()),
            reads: Arc::default(),
            elsewhere: Arc::default(),
            id: NEXT_TEST_DATA_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        }
    }

    /// The rows of a table. The provider reads it once, whichever thread asks first; every later
    /// asker, on any thread, gets the same rows (or the same error).
    pub fn read(&self, table: &TableRequest) -> Read {
        let key = format!("{}|{:?}|{}", table.name.to_lowercase(), table.app_id, table.per_company);
        let slot = {
            let mut g = self.reads.lock().unwrap_or_else(|e| e.into_inner());
            g.entry(key).or_default().clone()
        };
        slot.get_or_init(|| self.provider.read(table)).clone()
    }

    /// [`TableProvider::holds_rows_in`], asked of the provider once per company and table.
    pub fn holds_rows_in(&self, company: &str, table: &TableRequest) -> Option<bool> {
        let key = format!(
            "{company}|{}|{}|{}",
            table.name.to_lowercase(),
            table.app_id.as_deref().unwrap_or(""),
            table.per_company
        );
        if let Some(hit) = self.elsewhere.lock().unwrap_or_else(|e| e.into_inner()).get(&key) {
            return *hit;
        }
        let answer = self.provider.holds_rows_in(company, table);
        self.elsewhere
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(key, answer);
        answer
    }
}

/// What happened to one table the first time a run touched it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Rows were taken. `dropped` counts columns left out, by reason.
    Loaded { rows: usize, dropped: Dropped },
    /// The database holds no rows for the table (or no table).
    Empty,
    /// Not loaded, and why. The table reads as empty.
    Refused(Refusal),
}

/// Columns of a loaded table that were left out, by reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Dropped {
    /// Extension columns of an app this run does not install.
    pub uninstalled_app: usize,
    /// Columns no field of the compiled table matches.
    pub no_such_field: usize,
}

/// Why a table was not loaded. Each reason is counted on its own in the summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// A name no app id can select.
    Ambiguous(String),
    /// A value the runtime cannot rebuild, or a row shape it cannot map.
    Unsupported(String),
    /// The reader failed; its text.
    Reader(String),
    /// A platform table: the runtime keeps that state itself.
    Platform,
}

#[derive(Default)]
struct Inner {
    /// By table object index: the outcome of its first touch.
    tables: BTreeMap<u32, (String, Outcome)>,
    /// The largest `timestamp` any loaded row carried.
    max_row_version: u64,
}

/// The counts behind the summary line. Shared by every thread of a run; a table is recorded once,
/// by the thread that touches it first.
#[derive(Default)]
pub struct LoadStats {
    inner: Mutex<Inner>,
}

impl LoadStats {
    /// Records the first touch of a table. Returns false (and records nothing) when another
    /// thread, or another engine of the same program, recorded it already.
    pub fn record(&self, object: u32, name: &str, outcome: Outcome) -> bool {
        let mut g = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if g.tables.contains_key(&object) {
            return false;
        }
        g.tables.insert(object, (name.to_string(), outcome));
        true
    }

    /// Whether a table's first touch was recorded.
    pub fn seen(&self, object: u32) -> bool {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .tables
            .contains_key(&object)
    }

    /// Notes the `timestamp` of a loaded row.
    pub fn note_row_version(&self, v: u64) {
        let mut g = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        g.max_row_version = g.max_row_version.max(v);
    }

    /// The outcome recorded for a table.
    pub fn outcome(&self, object: u32) -> Option<Outcome> {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .tables
            .get(&object)
            .map(|(_, o)| o.clone())
    }

    /// The counts of the run so far.
    pub fn summary(&self) -> Summary {
        let g = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let mut s = Summary::default();
        for (name, o) in g.tables.values() {
            match o {
                Outcome::Loaded { rows, dropped } => {
                    s.tables += 1;
                    s.rows += rows;
                    s.dropped_uninstalled_app += dropped.uninstalled_app;
                    s.dropped_no_such_field += dropped.no_such_field;
                }
                Outcome::Empty => s.empty += 1,
                Outcome::Refused(Refusal::Ambiguous(why)) => s.ambiguous.push(format!("{name}: {why}")),
                Outcome::Refused(Refusal::Unsupported(why)) => s.unsupported.push(format!("{name}: {why}")),
                Outcome::Refused(Refusal::Reader(why)) => s.reader.push(format!("{name}: {why}")),
                Outcome::Refused(Refusal::Platform) => s.platform += 1,
            }
        }
        s
    }
}

/// The counts of a run's test data, with the reasons of every refusal.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Summary {
    pub tables: usize,
    pub rows: usize,
    /// Touched tables the database holds no rows for.
    pub empty: usize,
    pub dropped_uninstalled_app: usize,
    pub dropped_no_such_field: usize,
    pub ambiguous: Vec<String>,
    pub unsupported: Vec<String>,
    pub reader: Vec<String>,
    pub platform: usize,
}

impl Summary {
    /// The one line of the run summary. Each exclusion appears only when it happened.
    pub fn line(&self, source: &str, company: &str) -> String {
        let mut notes = Vec::new();
        if !self.ambiguous.is_empty() {
            notes.push(format!(
                "{} skipped (a name no app id could select)",
                self.ambiguous.len()
            ));
        }
        if !self.unsupported.is_empty() {
            notes.push(format!(
                "{} refused (a value type or row shape the runtime cannot rebuild)",
                self.unsupported.len()
            ));
        }
        if !self.reader.is_empty() {
            notes.push(format!("{} refused by the backup reader", self.reader.len()));
        }
        if self.dropped_uninstalled_app > 0 {
            notes.push(format!(
                "{} extension column(s) dropped for apps this run does not install",
                self.dropped_uninstalled_app
            ));
        }
        if self.dropped_no_such_field > 0 {
            notes.push(format!(
                "{} column(s) dropped that the compiled tables have no field for",
                self.dropped_no_such_field
            ));
        }
        if self.platform > 0 {
            notes.push(format!("{} platform table(s) not loaded", self.platform));
        }
        format!(
            "Test data: {} rows loaded from {} tables of company '{company}' in {source}{}",
            self.rows,
            self.tables,
            if notes.is_empty() {
                String::new()
            } else {
                format!("; {}", notes.join(", "))
            }
        )
    }

    /// One line per refused table, with the reason (the reader's own text for a reader error).
    pub fn refusals(&self) -> Vec<String> {
        let mut out = Vec::new();
        out.extend(self.ambiguous.iter().map(|r| format!("test data: skipped {r}")));
        out.extend(self.unsupported.iter().map(|r| format!("test data: refused {r}")));
        out.extend(self.reader.iter().map(|r| format!("test data: reader refused {r}")));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A provider that takes its time and counts how often it is asked.
    struct Slow {
        asked: AtomicUsize,
    }

    impl TableProvider for Slow {
        fn company(&self) -> &str {
            "C"
        }
        fn source(&self) -> String {
            "slow".into()
        }
        fn read(&self, table: &TableRequest) -> Read {
            self.asked.fetch_add(1, Ordering::SeqCst);
            std::thread::sleep(std::time::Duration::from_millis(50));
            Ok(Arc::new(TableRows {
                columns: vec![table.name.clone()],
                rows: vec![vec![Cell::Int(1)]],
            }))
        }
    }

    #[test]
    fn threads_asking_for_one_table_at_once_read_it_once() {
        let slow = Arc::new(Slow {
            asked: AtomicUsize::new(0),
        });
        let data = TestData::new(slow.clone());
        let request = |n: &str| TableRequest {
            name: n.into(),
            app_id: None,
            per_company: true,
            ..Default::default()
        };
        std::thread::scope(|s| {
            let hs: Vec<_> = (0..8)
                .map(|i| {
                    let data = data.clone();
                    // two tables, four askers each
                    let table = request(if i % 2 == 0 { "Customer" } else { "Item" });
                    s.spawn(move || data.read(&table).unwrap().columns[0].clone())
                })
                .collect();
            for (i, h) in hs.into_iter().enumerate() {
                assert_eq!(h.join().unwrap(), if i % 2 == 0 { "Customer" } else { "Item" });
            }
        });
        assert_eq!(slow.asked.load(Ordering::SeqCst), 2, "each table was read once");
        // a later ask, even with another spelling of the name, is answered from the first read
        data.read(&request("CUSTOMER")).unwrap();
        assert_eq!(slow.asked.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn a_table_is_recorded_once() {
        let s = LoadStats::default();
        let loaded = |rows| Outcome::Loaded {
            rows,
            dropped: Dropped::default(),
        };
        assert!(s.record(7, "Customer", loaded(3)));
        assert!(!s.record(7, "Customer", loaded(99)), "the second thread adds nothing");
        assert_eq!(s.summary().rows, 3);
    }

    #[test]
    fn the_summary_line_names_each_exclusion_that_happened_and_only_those() {
        let s = LoadStats::default();
        s.record(
            1,
            "A",
            Outcome::Loaded {
                rows: 10,
                dropped: Dropped {
                    uninstalled_app: 2,
                    no_such_field: 1,
                },
            },
        );
        s.record(2, "B", Outcome::Refused(Refusal::Reader("page 7 is torn".into())));
        s.record(
            3,
            "C",
            Outcome::Refused(Refusal::Unsupported("column X (RecordId)".into())),
        );
        let line = s.summary().line("demo.bak", "CRONUS International Ltd.");
        assert_eq!(
            line,
            "Test data: 10 rows loaded from 1 tables of company 'CRONUS International Ltd.' in demo.bak; \
             1 refused (a value type or row shape the runtime cannot rebuild), 1 refused by the backup reader, \
             2 extension column(s) dropped for apps this run does not install, \
             1 column(s) dropped that the compiled tables have no field for"
        );
        let refusals = s.summary().refusals();
        assert!(refusals.iter().any(|r| r.contains("page 7 is torn")), "{refusals:?}");
    }

    #[test]
    fn a_cell_reads_as_its_sql_value() {
        assert_eq!(Cell::Null.to_string(), "NULL");
        assert_eq!(Cell::Int(5).to_string(), "int 5");
        assert_eq!(Cell::Text("a".into()).to_string(), "text 'a'");
        assert_eq!(
            Cell::Decimal {
                negative: true,
                magnitude: 125,
                scale: 2
            }
            .to_string(),
            "decimal -1.25"
        );
        assert_eq!(
            Cell::DateTime { days: 3, ticks: 0 }.to_string(),
            "datetime (day 3 since 1900-01-01)"
        );
        assert_eq!(Cell::Binary(vec![1, 2, 3]).to_string(), "binary of 3 bytes");
    }
}
