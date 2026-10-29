//! The rows of a restored company, from a SQL Server backup (`.bak`) or a BC cloud export
//! (`.bacpac`), as `al-runner test --test-data` reads them (ADR-020 point 3).
//!
//! The provider does no naming of its own beyond finding the database table: a table is its AL
//! name with SQL's character substitutions, selected by company and, when two apps declare the same
//! name, by the declaring app's id. Columns come back as the database names them (extension
//! columns already joined, `$ext` companions merged on their own key); the runtime maps them onto
//! the fields of the compiled program.

use crate::FileSource;
use al_bcdb::io::ByteSource as _;
use al_bcdb::query::{self, BcTable, ReadRequest, plan_read_table_in};
use al_bcdb::source::BcSource;
use al_bcdb::symbols::{ci_key, eq_ci, normalize_sql_name};
use al_bcdb::value::Value;
use al_host::testdata::{Cell, TableError, TableProvider, TableRequest, TableRows, TestSequence};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// The company of Microsoft's demo database, and of the corpus's oracle.
pub const DEFAULT_COMPANY: &str = "CRONUS International Ltd.";

pub struct BackupProvider {
    src: Box<dyn BcSource>,
    tables: Vec<BcTable>,
    /// The positions in `tables` of each table name (upper-cased, see [`ci_key`]), in list order: a read finds its
    /// table without comparing its name with every table of the database.
    by_name: std::collections::HashMap<String, Vec<usize>>,
    /// The company's name as it appears in table names (`CRONUS International Ltd_`).
    sql_company: String,
    /// The company's name as AL sees it (`CRONUS International Ltd.`).
    company: String,
    source: String,
    /// The backup file as the install baseline's key holds it (path, length, modification time).
    identity: Option<String>,
    /// How the open went and what reads cost, for `--timings`.
    cost: Cost,
    /// The company's sequences, read from the catalog when first asked: every session asks (al-runner issue 5856), and
    /// the backup does not change while it is open.
    sequences: std::sync::OnceLock<Vec<TestSequence>>,
    /// `TableProvider::last_row_version`, read once.
    last_row_version: std::sync::OnceLock<Option<u64>>,
    /// The companies of the database, read once (`TableProvider::companies`).
    companies: std::sync::OnceLock<Vec<String>>,
}

#[derive(Default)]
struct Cost {
    open: std::time::Duration,
    /// "kept map and catalog used", "map and catalog built and kept", or "not kept".
    index: &'static str,
    reads: AtomicU64,
    rows: AtomicU64,
    read_nanos: AtomicU64,
}

/// The page-map index of one backup, kept as a file.
struct FileIndexStore {
    file: PathBuf,
    saved: std::sync::atomic::AtomicBool,
}

/// The backup file's path, length and modification time: a new download or an edit changes it.
fn file_identity(path: &Path) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    let mtime = meta
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_nanos();
    let full = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    Some(format!("{}|{}|{}", full.display(), meta.len(), mtime))
}

impl FileIndexStore {
    fn for_backup(dir: &Path, backup: &Path, source: &FileSource) -> FileIndexStore {
        // FNV-1a: stable across Rust versions, unlike the standard hasher.
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut feed = |bytes: &[u8]| {
            for b in bytes {
                h = (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3);
            }
        };
        let canonical = backup.canonicalize().unwrap_or_else(|_| backup.to_path_buf());
        feed(canonical.to_string_lossy().as_bytes());
        feed(&source.len().to_le_bytes());
        let mtime = std::fs::metadata(backup)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_nanos());
        feed(&mtime.to_le_bytes());
        let stem = backup
            .file_stem()
            .map_or_else(|| "backup".to_string(), |s| s.to_string_lossy().into_owned());
        FileIndexStore {
            file: dir.join(format!("{stem}-{h:016x}.pagemap")),
            saved: false.into(),
        }
    }
}

impl al_bcdb::pagefile::IndexStore for FileIndexStore {
    fn load(&self) -> Option<Vec<u8>> {
        std::fs::read(&self.file).ok()
    }

    fn save(&self, bytes: &[u8]) {
        self.saved.store(true, Ordering::Relaxed);
        let _ = (|| -> std::io::Result<()> {
            if let Some(dir) = self.file.parent() {
                std::fs::create_dir_all(dir)?;
            }
            // Written aside and renamed, so a run that reads while another writes sees all or nothing.
            let tmp = self.file.with_extension(format!("tmp{}", std::process::id()));
            std::fs::write(&tmp, bytes)?;
            std::fs::rename(&tmp, &self.file)
        })();
    }
}

fn cell(v: &Value) -> Cell {
    match v {
        Value::Null => Cell::Null,
        Value::Bool(b) => Cell::Bool(*b),
        Value::Int(i) => Cell::Int(*i),
        Value::Real(f) => Cell::Float(f64::from(*f)),
        Value::Float(f) => Cell::Float(*f),
        Value::Decimal {
            negative,
            magnitude,
            scale,
        } => Cell::Decimal {
            negative: *negative,
            magnitude: *magnitude,
            scale: *scale,
        },
        Value::DateTime { days, ticks } => Cell::DateTime {
            days: *days,
            ticks: *ticks,
        },
        Value::Date { days } => Cell::Date { days: *days },
        Value::Time { units, scale } => Cell::Time {
            units: *units,
            scale: *scale,
        },
        Value::DateTime2 { days, units, scale } => Cell::DateTime2 {
            days: *days,
            units: *units,
            scale: *scale,
        },
        Value::Guid(g) => Cell::Guid(*g),
        Value::Binary(b) => Cell::Binary(b.clone()),
        Value::RowVersion(v) => Cell::RowVersion(*v),
        Value::Text(s) => Cell::Text(s.clone()),
    }
}

impl BackupProvider {
    /// Opens a `.bak` or `.bacpac`. `company` names the company to restore (exact, then any case,
    /// then a prefix); without it the database's only company, or else the demo company
    /// [`DEFAULT_COMPANY`]. A database with several companies and none of them the demo company
    /// needs the name.
    pub fn open(path: &Path, company: Option<&str>) -> Result<BackupProvider, String> {
        Self::open_with_index_dir(path, company, None)
    }

    /// [`open`](BackupProvider::open), keeping the backup's page map in `index_dir` (one file per
    /// backup, named for its path, length and modification time). A directory that cannot be
    /// written to costs the saving, never the run.
    pub fn open_with_index_dir(
        path: &Path,
        company: Option<&str>,
        index_dir: Option<&Path>,
    ) -> Result<BackupProvider, String> {
        let started = std::time::Instant::now();
        let file = FileSource::open(path)?;
        let (opened, index) = match index_dir {
            Some(dir) => {
                let store = FileIndexStore::for_backup(dir, path, &file);
                let r = al_bcdb::open_indexed(Box::new(file), &store);
                let state = if store.saved.load(Ordering::Relaxed) {
                    "kept map and catalog built and saved"
                } else {
                    "kept map and catalog used"
                };
                (r, state)
            }
            None => (al_bcdb::open(Box::new(file)), "map and catalog not kept"),
        };
        let src = opened.map_err(|e| format!("{}: {e}", path.display()))?;
        let file_name = path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let mut p = Self::from_source(src, company, file_name).map_err(|e| format!("{}: {e}", path.display()))?;
        p.identity = file_identity(path).map(|f| format!("{f}|{}|{}", p.sql_company, p.company));
        p.cost.open = started.elapsed();
        p.cost.index = index;
        Ok(p)
    }

    /// A provider over an opened source; `source` is the name the summary line shows.
    pub fn from_source(
        src: Box<dyn BcSource>,
        company: Option<&str>,
        source: String,
    ) -> Result<BackupProvider, String> {
        let tables = query::bc_tables(&*src).map_err(|e| e.to_string())?;
        let companies = companies_of(&tables);
        let file_name = source;
        let sql_company = pick_company(&companies, company, &file_name)?;
        let company = al_company_name(&*src, &tables, &sql_company);
        let mut by_name: std::collections::HashMap<String, Vec<usize>> = std::collections::HashMap::new();
        for (i, t) in tables.iter().enumerate() {
            by_name.entry(ci_key(&t.table_name)).or_default().push(i);
        }
        Ok(BackupProvider {
            src,
            tables,
            by_name,
            sql_company,
            company,
            source: file_name,
            identity: None,
            cost: Cost::default(),
            sequences: std::sync::OnceLock::new(),
            last_row_version: std::sync::OnceLock::new(),
            companies: std::sync::OnceLock::new(),
        })
    }

    /// The companies a database holds, in their SQL spelling.
    pub fn companies(path: &Path) -> Result<Vec<String>, String> {
        let file = FileSource::open(path)?;
        let src = al_bcdb::open(Box::new(file)).map_err(|e| format!("{}: {e}", path.display()))?;
        query::companies(&*src).map_err(|e| e.to_string())
    }

    /// [`TableProvider::companies`], read from the source.
    fn read_companies(&self) -> Vec<String> {
        let read = (|| {
            let t = self
                .tables
                .iter()
                .find(|t| t.company.is_none() && t.app_id.is_none() && eq_ci(&t.table_name, "Company"))?;
            let cols = self.src.columns(&t.table).ok()?;
            let name = cols.iter().find(|c| eq_ci(&c.name, "Name"))?.clone();
            let mut names = Vec::new();
            self.src
                .read_rows(&t.table, &[name], &mut |row| {
                    if let Some(Value::Text(n)) = row.first() {
                        names.push(n.clone());
                    }
                    Ok(true)
                })
                .ok()?;
            Some(names)
        })();
        let mut names = read.unwrap_or_else(|| companies_of(&self.tables));
        names.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()).then_with(|| a.cmp(b)));
        names.dedup();
        names
    }

    fn find(&self, req: &TableRequest) -> Result<&BcTable, TableError> {
        let want = normalize_sql_name(&req.name);
        let mut hits: Vec<&BcTable> = self
            .by_name
            .get(&ci_key(&want))
            .map_or(&[][..], Vec::as_slice)
            .iter()
            .map(|&i| &self.tables[i])
            .filter(|t| eq_ci(&t.table_name, &want))
            .filter(|t| match &t.company {
                Some(c) => req.per_company && eq_ci(c, &self.sql_company),
                None => !req.per_company,
            })
            .collect();
        if hits.len() > 1 {
            // Two apps may declare a table of one name; the id of the app the compiled table
            // belongs to is the only way to tell them apart.
            let by_app: Vec<&BcTable> = hits
                .iter()
                .copied()
                .filter(|t| match (&t.app_id, &req.app_id) {
                    (Some(a), Some(b)) => eq_ci(a, b),
                    _ => false,
                })
                .collect();
            if by_app.len() != 1 {
                let names: Vec<&str> = hits.iter().map(|t| t.sql_name()).collect();
                return Err(TableError::Ambiguous(format!(
                    "{} tables answer to '{}' ({}) and the declaring app's id selects {}",
                    hits.len(),
                    req.name,
                    names.join(" | "),
                    by_app.len()
                )));
            }
            hits = by_app;
        }
        hits.first().copied().ok_or(TableError::Absent)
    }
}

/// The companies of a table list: the distinct company segments, ordinal order.
fn companies_of(tables: &[BcTable]) -> Vec<String> {
    let mut v: Vec<String> = tables
        .iter()
        .filter_map(|t| t.company.clone())
        .filter(|c| !c.is_empty())
        .collect();
    v.sort();
    v.dedup();
    v
}

/// Which company a database restores.
fn pick_company(companies: &[String], asked: Option<&str>, file: &str) -> Result<String, String> {
    let list = || {
        companies
            .iter()
            .map(|c| format!("'{c}'"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    if let Some(name) = asked {
        let n = normalize_sql_name(name);
        if let Some(c) = companies.iter().find(|c| **c == n || **c == name) {
            return Ok(c.clone());
        }
        let ci: Vec<&String> = companies.iter().filter(|c| eq_ci(c, &n)).collect();
        if let [c] = ci.as_slice() {
            return Ok((*c).clone());
        }
        let prefix: Vec<&String> = companies
            .iter()
            .filter(|c| c.to_uppercase().starts_with(&n.to_uppercase()))
            .collect();
        return match prefix.as_slice() {
            [c] => Ok((*c).clone()),
            [] => Err(format!(
                "--test-data-company '{name}' matches no company in {file}, which holds {}",
                list()
            )),
            many => Err(format!(
                "--test-data-company '{name}' matches {} companies in {file} ({}); use a longer prefix",
                many.len(),
                many.iter().map(|c| format!("'{c}'")).collect::<Vec<_>>().join(", ")
            )),
        };
    }
    match companies {
        [] => Err(format!("{file} holds no company, so there is no test data in it")),
        [only] => Ok(only.clone()),
        many => {
            let demo = normalize_sql_name(DEFAULT_COMPANY);
            // The demo company of another country has another name (`CRONUS USA, Inc.`): the one company whose
            // name starts with CRONUS. Several of them, or none, need the name.
            let cronus: Vec<&String> = many.iter().filter(|c| c.to_uppercase().starts_with("CRONUS")).collect();
            many.iter().find(|c| eq_ci(c, &demo)).cloned().or_else(|| match cronus.as_slice() {
                [only] => Some((*only).clone()),
                _ => None,
            }).ok_or_else(|| {
                format!(
                    "{file} holds {} companies ({}) and none is '{DEFAULT_COMPANY}'; pick one with --test-data-company",
                    many.len(),
                    list()
                )
            })
        }
    }
}

/// The company's name as AL spells it: the `Name` of its row in the database's `Company` table
/// (SQL replaced the dot of `Ltd.`). A database without that table gives the SQL spelling.
fn al_company_name(src: &dyn BcSource, tables: &[BcTable], sql_company: &str) -> String {
    let found = (|| {
        let t = tables
            .iter()
            .find(|t| t.company.is_none() && t.app_id.is_none() && eq_ci(&t.table_name, "Company"))?;
        let cols = src.columns(&t.table).ok()?;
        let name = cols.iter().find(|c| eq_ci(&c.name, "Name"))?.clone();
        let mut hit = None;
        src.read_rows(&t.table, &[name], &mut |row| {
            if let Some(Value::Text(n)) = row.first() {
                if eq_ci(&normalize_sql_name(n), sql_company) {
                    hit = Some(n.clone());
                    return Ok(false);
                }
            }
            Ok(true)
        })
        .ok()?;
        hit
    })();
    found.unwrap_or_else(|| sql_company.to_string())
}

impl TableProvider for BackupProvider {
    fn company(&self) -> &str {
        &self.company
    }

    fn source(&self) -> String {
        self.source.clone()
    }

    fn identity(&self) -> Option<String> {
        self.identity.clone()
    }

    fn cost(&self) -> Option<String> {
        let c = &self.cost;
        let ms = |ns: u64| ns as f64 / 1e6;
        Some(format!(
            "opened in {:.0} ms ({}); {} table read(s), {} rows, {:.0} ms",
            c.open.as_secs_f64() * 1e3,
            c.index,
            c.reads.load(Ordering::Relaxed),
            c.rows.load(Ordering::Relaxed),
            ms(c.read_nanos.load(Ordering::Relaxed))
        ))
    }

    /// The `Name` of every row of the database's Company table, sorted without regard to case (the key order of a
    /// case-insensitive collation); a database without that table gives the SQL spelling of its companies.
    fn companies(&self) -> Vec<String> {
        self.companies.get_or_init(|| self.read_companies()).clone()
    }

    fn last_row_version(&self) -> Option<u64> {
        *self
            .last_row_version
            .get_or_init(|| self.src.last_row_version().ok().flatten())
    }

    fn sequences(&self) -> Vec<TestSequence> {
        self.sequences
            .get_or_init(|| match self.src.sequences() {
                Ok(all) => all.iter().filter_map(|s| sequence_of(s, &self.sql_company)).collect(),
                Err(_) => Vec::new(),
            })
            .clone()
    }

    fn read(&self, req: &TableRequest) -> Result<Arc<TableRows>, TableError> {
        let started = std::time::Instant::now();
        let r = self.read_table(req);
        if let Ok(rows) = &r {
            self.cost.reads.fetch_add(1, Ordering::Relaxed);
            self.cost.rows.fetch_add(rows.rows.len() as u64, Ordering::Relaxed);
            self.cost
                .read_nanos
                .fetch_add(started.elapsed().as_nanos() as u64, Ordering::Relaxed);
        }
        r
    }
}

impl BackupProvider {
    fn read_table(&self, req: &TableRequest) -> Result<Arc<TableRows>, TableError> {
        let table = self.find(req)?;
        let reader = |e: al_bcdb::Error| TableError::Reader(e.to_string());
        let plan = plan_read_table_in(
            &*self.src,
            None,
            &self.tables,
            table,
            &ReadRequest {
                merge_extensions: true,
                ..Default::default()
            },
        )
        .map_err(reader)?;
        let mut rows = Vec::new();
        plan.run(&*self.src, &mut |row| {
            rows.push(row.iter().map(cell).collect());
            Ok(true)
        })
        .map_err(reader)?;
        Ok(Arc::new(TableRows {
            columns: plan.headers.clone(),
            rows,
        }))
    }
}

/// The demo backup of the cached Microsoft artifact for a BC build: the folder under
/// `~/.bcartifacts.cache/sandbox` named like the first of `builds` that has one, else the newest
/// whose name starts with `major.minor`, else the newest of the major; then its `w1` (else `us`,
/// else the only) country folder. `builds` are the versions the run knows of the Microsoft apps it
/// loaded, so an artifact of another build than the apps' is never picked while theirs exists.
pub fn default_backup(builds: &[String], major: u16, minor: Option<u16>) -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME").map(PathBuf::from).ok_or("HOME is not set")?;
    default_backup_in(&home.join(".bcartifacts.cache").join("sandbox"), builds, major, minor)
}

/// [`default_backup`] in the artifact cache `root`.
pub fn default_backup_in(root: &Path, builds: &[String], major: u16, minor: Option<u16>) -> Result<PathBuf, String> {
    default_backup_in_country(root, builds, major, minor, None)
}

/// [`default_backup_in`] for `country`: with `Some`, only that country's backup answers (an explicit
/// `--country` never falls back to another country's data); with `None`, `w1`, else `us`, else the only one.
pub fn default_backup_in_country(
    root: &Path,
    builds: &[String],
    major: u16,
    minor: Option<u16>,
    country: Option<&str>,
) -> Result<PathBuf, String> {
    let mut versions: Vec<String> = std::fs::read_dir(root)
        .map_err(|e| {
            format!(
                "no cached BC artifact in {} ({e}); name a backup: --test-data PATH.bak",
                root.display()
            )
        })?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    versions.sort_by_key(|v| v.split('.').map(|p| p.parse::<u64>().unwrap_or(0)).collect::<Vec<_>>());
    let exact = builds.iter().find_map(|b| versions.iter().find(|v| *v == b));
    let by_minor = || {
        let mi = minor?;
        versions.iter().rev().find(|v| v.starts_with(&format!("{major}.{mi}.")))
    };
    let by_major = || versions.iter().rev().find(|v| v.starts_with(&format!("{major}.")));
    let version = exact.or_else(by_minor).or_else(by_major).ok_or_else(|| {
        format!(
            "no cached BC artifact for BC {major} in {} (found: {}); name a backup: --test-data PATH.bak",
            root.display(),
            if versions.is_empty() {
                "none".to_string()
            } else {
                versions.join(", ")
            }
        )
    })?;
    let dir = root.join(version);
    let country = match country {
        Some(c) => Some(dir.join(c.to_ascii_lowercase())).filter(|p| p.is_dir()),
        None => ["w1", "us"]
            .iter()
            .map(|c| dir.join(c))
            .find(|p| p.is_dir())
            .or_else(|| {
                std::fs::read_dir(&dir)
                    .ok()?
                    .filter_map(|e| e.ok().map(|e| e.path()))
                    .find(|p| p.is_dir() && p.file_name().is_some_and(|n| n != "platform"))
            }),
    }
    .ok_or_else(|| match country {
        Some(c) => format!("{} holds no folder for country {c}", dir.display()),
        None => format!("{} holds no country folder", dir.display()),
    })?;
    let name = country
        .file_name()
        .map(|n| n.to_string_lossy().to_uppercase())
        .unwrap_or_default();
    let bak = country.join(format!("BusinessCentral-{name}.bak"));
    if bak.is_file() {
        Ok(bak)
    } else {
        Err(format!("{} has no BusinessCentral-{name}.bak", country.display()))
    }
}

/// The AL number sequence behind a SQL sequence of the database, if it is one of the company `sql_company` or of none.
/// BC names them `$SEQ$NumberSequence$<name>$<company>` (company specific) and `$SEQ$NumberSequence$<name>`.
fn sequence_of(s: &al_bcdb::catalog::SqlSequence, sql_company: &str) -> Option<TestSequence> {
    let rest = s.name.strip_prefix("$SEQ$NumberSequence$")?;
    let (name, company_specific) = match rest.rsplit_once('$') {
        Some((name, company)) if company == sql_company => (name, true),
        Some(_) => return None,
        None => (rest, false),
    };
    Some(TestSequence {
        name: name.to_string(),
        company_specific,
        start: s.start,
        increment: s.increment,
        current: s.current,
    })
}

#[cfg(test)]
mod company_tests {
    use super::*;

    #[test]
    fn the_demo_company_of_another_country_is_the_one_cronus_company() {
        let two = ["CRONUS USA, Inc_".to_string(), "My Company".to_string()];
        assert_eq!(pick_company(&two, None, "f.bak").unwrap(), "CRONUS USA, Inc_");
        let w1 = ["CRONUS International Ltd_".to_string(), "My Company".to_string()];
        assert_eq!(pick_company(&w1, None, "f.bak").unwrap(), "CRONUS International Ltd_");
        let two_cronus = ["CRONUS USA, Inc_".to_string(), "CRONUS Test".to_string()];
        assert!(pick_company(&two_cronus, None, "f.bak").is_err());
        let none = ["A".to_string(), "B".to_string()];
        assert!(pick_company(&none, None, "f.bak").is_err());
    }
}

#[cfg(test)]
mod sequence_tests {
    use super::*;
    use al_bcdb::catalog::SqlSequence;

    fn sql(name: &str) -> SqlSequence {
        SqlSequence {
            name: name.into(),
            start: 0,
            increment: 1,
            current: 24,
        }
    }

    #[test]
    fn a_sequence_belongs_to_the_company_in_its_name_or_to_none() {
        let c = "CRONUS International Ltd_";
        let own = sequence_of(&sql("$SEQ$NumberSequence$28ABF101$CRONUS International Ltd_"), c).unwrap();
        assert_eq!(
            (own.name.as_str(), own.company_specific, own.current),
            ("28ABF101", true, 24)
        );
        let global = sequence_of(&sql("$SEQ$NumberSequence$GlobalOne"), c).unwrap();
        assert_eq!((global.name.as_str(), global.company_specific), ("GlobalOne", false));
        assert!(sequence_of(&sql("$SEQ$NumberSequence$X$My Company"), c).is_none());
        assert!(sequence_of(&sql("$ndo$recordactivity$seq"), c).is_none());
    }
}
