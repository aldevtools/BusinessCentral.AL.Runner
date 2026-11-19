//! A backup opened through its kept index (`<cache>/backup-index/`) gives a run the rows a fresh open gives, and a kept
//! index that is not the saved bytes is a miss: rebuilt, written again, never read.

use al_bcdb::query::{self, BcTable};
use al_bcdb::symbols::{eq_ci, normalize_sql_name};
use al_bcdb_std::{BackupProvider, FileSource};
use al_host::testdata::{TableError, TableProvider, TableRequest, TableRows, TestSequence};
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../al-bcdb/tests/fixtures")
        .join(name)
}

/// What a provider gives a run for its company: every table the company can read (its own and the ones without a
/// company), the sequences and the last row version.
#[derive(Debug, PartialEq)]
struct Dump {
    tables: Vec<(String, Result<Arc<TableRows>, TableError>)>,
    sequences: Vec<TestSequence>,
    last_row_version: Option<u64>,
}

/// The requests for every table of `company` in the backup at `path`, listed from a plain open of the file.
fn requests(path: &Path, company: &str) -> Vec<TableRequest> {
    let src = al_bcdb::open(Box::new(FileSource::open(path).unwrap())).unwrap();
    let sql_company = normalize_sql_name(company);
    let tables: Vec<BcTable> = query::bc_tables(&*src).unwrap();
    tables
        .into_iter()
        .filter(|t| t.company.as_deref().is_none_or(|c| eq_ci(c, &sql_company)))
        .map(|t| TableRequest {
            name: t.table_name,
            app_id: t.app_id,
            per_company: t.company.is_some(),
            ..Default::default()
        })
        .collect()
}

fn dump(p: &BackupProvider, requests: &[TableRequest]) -> Dump {
    Dump {
        tables: requests
            .iter()
            .map(|r| (format!("{} {:?} {}", r.name, r.app_id, r.per_company), p.read(r)))
            .collect(),
        sequences: p.sequences(),
        last_row_version: p.last_row_version(),
    }
}

/// The one file an open with an index directory keeps there.
fn kept_file(dir: &Path) -> PathBuf {
    let files: Vec<PathBuf> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
    assert_eq!(files.len(), 1, "one index file: {files:?}");
    files.into_iter().next().unwrap()
}

#[test]
fn a_flipped_byte_in_the_kept_file_is_replaced_and_reads_the_same_rows() {
    let bak = fixture("typeprobe.bak");
    let fresh = BackupProvider::open(&bak, Some("TP")).unwrap();
    let requests = requests(&bak, fresh.company());
    let want = dump(&fresh, &requests);
    assert!(want.tables.iter().filter(|(_, r)| r.is_ok()).count() > 3);
    let dir = tempfile::tempdir().unwrap();
    BackupProvider::open_with_index_dir(&bak, Some("TP"), Some(dir.path())).unwrap();
    let file = kept_file(dir.path());
    let good = std::fs::read(&file).unwrap();
    // The file is the magic, the map's length, the catalog's length and the catalog's checksum (8 bytes each), then
    // the map, then the catalog. The byte flipped is one of the map's page offsets, moved by one page.
    let map_len = u64::from_le_bytes(good[8..16].try_into().unwrap()) as usize;
    let at = 32 + map_len / 2;
    let mut bad = good.clone();
    let offset = at - (at - 32) % 8;
    assert_ne!(good[offset..offset + 8], [0xff; 8], "a mapped page");
    bad[offset + 1] ^= 0x20;
    std::fs::write(&file, &bad).unwrap();
    let again = BackupProvider::open_with_index_dir(&bak, Some("TP"), Some(dir.path())).unwrap();
    assert!(
        std::fs::read(&file).unwrap() == good,
        "the damaged file is written again"
    );
    assert_eq!(dump(&again, &requests), want);
}

/// Every table of the default company of the backup `ALRUNNER_DEMO_BAK` names (a CRONUS demo backup), read through a
/// fresh open, an index built by this open and the same index read by the next, is equal. Skipped without the variable:
/// Microsoft's data is never part of the repository.
#[test]
fn the_demo_backup_reads_the_same_rows_with_a_kept_index() {
    let Some(bak) = std::env::var_os("ALRUNNER_DEMO_BAK").map(PathBuf::from) else {
        eprintln!("skipped: ALRUNNER_DEMO_BAK names no backup");
        return;
    };
    let fresh = BackupProvider::open(&bak, None).unwrap();
    let requests = requests(&bak, fresh.company());
    let want = dump(&fresh, &requests);
    drop(fresh);
    let dir = tempfile::tempdir().unwrap();
    let built = BackupProvider::open_with_index_dir(&bak, None, Some(dir.path())).unwrap();
    let file = kept_file(dir.path());
    let saved = std::fs::read(&file).unwrap();
    assert_eq!(dump(&built, &requests), want, "built");
    drop(built);
    let read = BackupProvider::open_with_index_dir(&bak, None, Some(dir.path())).unwrap();
    assert!(
        std::fs::read(&file).unwrap() == saved,
        "the second open reads the kept index"
    );
    assert_eq!(dump(&read, &requests), want, "read");
    let ok = want.tables.iter().filter(|(_, r)| r.is_ok()).count();
    let rows: usize = want
        .tables
        .iter()
        .filter_map(|(_, r)| r.as_ref().ok())
        .map(|t| t.rows.len())
        .sum();
    eprintln!(
        "{}: company {}, {} tables ({} read, {} refused by the reader), {} rows, {} sequences, equal on all three opens",
        bak.display(),
        read.company(),
        want.tables.len(),
        ok,
        want.tables.len() - ok,
        rows,
        want.sequences.len()
    );
}
