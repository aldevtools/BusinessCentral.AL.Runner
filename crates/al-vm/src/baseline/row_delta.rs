//! A baseline as the rows it holds apart from another one ([`RowDelta`]), to hand to another thread instead of the whole
//! baseline as bytes (al-testrun `single_lane.rs`, ADR-066 amendment 5 decision 5, al-runner issue 6924).
//!
//! Unlike a [`Delta`](super::Delta), which predicts what a run would leave on another start, a row delta is exact: put
//! on top of a copy of the baseline it was taken from, it gives this baseline (the same fingerprint). The rows are
//! compared row by row, not table by table, so two carry-chain baselines that differ in the few rows one test codeunit
//! wrote give a delta of those rows.

use super::Baseline;
use al_builtins::value_codec::{Reader, decode_row, encode_row, put_u32};
use std::collections::{BTreeSet, HashMap};

/// One table's rows in a [`RowDelta`], encoded (so the delta can go to another thread).
#[derive(Clone, Debug)]
struct Rows {
    company: String,
    object: u32,
    whole: bool,
    pk_storage: al_data::memory::PkStorage,
    /// The rows to put: count, then key and row of each.
    put: Vec<u8>,
    /// The keys of the rows that are gone: count, then each key.
    gone: Vec<u8>,
}

/// A baseline as the rows it holds apart from another baseline, and everything else it holds as it is
/// ([`Baseline::row_delta_from`], [`Baseline::with_row_delta`]).
#[derive(Clone, Debug)]
pub struct RowDelta {
    changes: EncodedChanges,
    next_guid: u64,
    next_row_version: i64,
    committed_row_version: i64,
    autoinc: HashMap<(String, u32), i64>,
    isolated_storage: HashMap<(String, String), (String, bool, bool)>,
    sequences: HashMap<(String, String), crate::sessionops::Sequence>,
    last_stamp: i64,
    workspace_installs: Option<Vec<(BTreeSet<u32>, String)>>,
    company_built: bool,
    install_rows: Option<BTreeSet<u32>>,
    company_unknown: Vec<(String, u32, String)>,
    full_copies: Vec<String>,
    origin: u64,
}

impl RowDelta {
    /// About how many bytes the delta holds (its encoded rows).
    pub fn size(&self) -> usize {
        self.changes.size()
    }
}

/// Committed row changes (`al_data::memory::RowChanges`) encoded, so they can go to another thread: a [`RowDelta`]'s
/// rows, and a recorded run's (ADR-073, `recorded/mod.rs`).
#[derive(Clone, Debug)]
pub(crate) struct EncodedChanges {
    tables: Vec<Rows>,
    gone: Vec<(String, u32)>,
    emptied: Vec<(String, u32)>,
}

impl EncodedChanges {
    /// `changes` encoded; `None` when a row holds a value that is not plain.
    pub(crate) fn encode(changes: al_data::memory::RowChanges) -> Option<EncodedChanges> {
        let mut tables = Vec::with_capacity(changes.tables.len());
        for t in changes.tables {
            let mut put = Vec::new();
            put_u32(&mut put, t.put.len() as u32);
            for (k, r) in &t.put {
                if !encode_row(k, &mut put) || !encode_row(r, &mut put) {
                    return None;
                }
            }
            let mut gone = Vec::new();
            put_u32(&mut gone, t.gone.len() as u32);
            for k in &t.gone {
                if !encode_row(k, &mut gone) {
                    return None;
                }
            }
            tables.push(Rows {
                company: t.company,
                object: t.object,
                whole: t.whole,
                pk_storage: t.pk_storage,
                put,
                gone,
            });
        }
        Some(EncodedChanges {
            tables,
            gone: changes.gone,
            emptied: changes.emptied,
        })
    }

    /// The changes [`EncodedChanges::encode`] encoded; `None` when the rows do not read back.
    pub(crate) fn decode(&self) -> Option<al_data::memory::RowChanges> {
        let mut tables = Vec::with_capacity(self.tables.len());
        for t in &self.tables {
            let mut r = Reader::new(&t.put);
            let mut put = Vec::new();
            for _ in 0..r.u32()? {
                let k = decode_row(&mut r)?;
                put.push((k, decode_row(&mut r)?));
            }
            if !r.is_empty() {
                return None;
            }
            let mut r = Reader::new(&t.gone);
            let mut gone = Vec::new();
            for _ in 0..r.u32()? {
                gone.push(decode_row(&mut r)?);
            }
            if !r.is_empty() {
                return None;
            }
            tables.push(al_data::memory::TableRows {
                company: t.company.clone(),
                object: t.object,
                whole: t.whole,
                pk_storage: t.pk_storage.clone(),
                put,
                gone,
            });
        }
        Some(al_data::memory::RowChanges {
            tables,
            gone: self.gone.clone(),
            emptied: self.emptied.clone(),
        })
    }

    /// About how many bytes the encoded rows hold.
    pub(crate) fn size(&self) -> usize {
        self.tables.iter().map(|t| t.put.len() + t.gone.len()).sum()
    }
}

/// The bytes of a row, or `None` when it holds a value that is not plain.
fn bytes(row: &[al_builtins::value::Value], buf: &mut Vec<u8>) -> Option<()> {
    buf.clear();
    encode_row(row, buf).then_some(())
}

impl Baseline {
    /// A baseline that holds nothing, to take a [`RowDelta`] from when a run has no install baseline (each of its rows
    /// is then in the delta). Only the rows and the marks of the delta's baseline are what it starts with: the delta
    /// sets every other part as it is.
    pub fn empty() -> Baseline {
        Baseline {
            store: al_data::memory::MemStore::default(),
            next_guid: 0,
            next_row_version: 0,
            committed_row_version: 0,
            autoinc: HashMap::new(),
            isolated_storage: HashMap::new(),
            sequences: HashMap::new(),
            last_stamp: 0,
            workspace_installs: None,
            company_built: false,
            install_rows: None,
            company_unknown: Vec::new(),
            full_copies: Vec::new(),
            origin: super::new_origin(),
        }
    }

    /// This baseline as the rows it holds apart from `base` and everything else as it is: put on top of `base` (or of a
    /// copy read from `base`'s bytes) with [`Baseline::with_row_delta`] it gives this baseline. Rows are the same when
    /// their bytes are. `None` when a row that differs is not a plain value, or the company copies differ.
    pub fn row_delta_from(&self, base: &Baseline) -> Option<RowDelta> {
        let (mut x, mut y) = (Vec::new(), Vec::new());
        let mut same = |a: &[al_builtins::value::Value], b: &[al_builtins::value::Value]| {
            bytes(a, &mut x)?;
            bytes(b, &mut y)?;
            Some(x == y)
        };
        let changes = self.store.committed_row_changes_from(&base.store, &mut same)?;
        let changes = EncodedChanges::encode(changes)?;
        Some(RowDelta {
            changes,
            next_guid: self.next_guid,
            next_row_version: self.next_row_version,
            committed_row_version: self.committed_row_version,
            autoinc: self.autoinc.clone(),
            isolated_storage: self.isolated_storage.clone(),
            sequences: self.sequences.clone(),
            last_stamp: self.last_stamp,
            workspace_installs: self.workspace_installs.clone(),
            company_built: self.company_built,
            install_rows: self.install_rows.clone(),
            company_unknown: self.company_unknown.clone(),
            full_copies: self.full_copies.clone(),
            origin: self.origin,
        })
    }

    /// This baseline with `delta` put on top ([`Baseline::row_delta_from`]): the baseline the delta was taken from,
    /// with its origin. `None` when the rows do not read back or a table to change is not held here.
    pub fn with_row_delta(&self, delta: &RowDelta) -> Option<Baseline> {
        let changes = delta.changes.decode()?;
        let mut b = self.clone();
        b.store = b.store.with_committed_row_changes(changes)?;
        b.next_guid = delta.next_guid;
        b.next_row_version = delta.next_row_version;
        b.committed_row_version = delta.committed_row_version;
        b.autoinc = delta.autoinc.clone();
        b.isolated_storage = delta.isolated_storage.clone();
        b.sequences = delta.sequences.clone();
        b.last_stamp = delta.last_stamp;
        b.workspace_installs = delta.workspace_installs.clone();
        b.company_built = delta.company_built;
        b.install_rows = delta.install_rows.clone();
        b.company_unknown = delta.company_unknown.clone();
        b.full_copies = delta.full_copies.clone();
        b.origin = delta.origin;
        Some(b)
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::sample;
    use al_builtins::value::Value;
    use al_data::memory::ExportedTable;

    /// What a session restored from `b` starts from: its bytes without the last stamp, the install rows, and the marks
    /// (the tables a DeleteAll emptied, the company copies).
    fn fingerprint(b: &super::super::Baseline) -> Option<Vec<u8>> {
        let mut b = b.clone();
        b.last_stamp = 0;
        let mut out = b.to_bytes()?;
        let rows = b.install_rows.clone().unwrap_or_else(|| b.nonempty_tables());
        out.extend(rows.iter().flat_map(|t| t.to_le_bytes()));
        out.extend(format!("{:?}", b.store.committed_marks()).into_bytes());
        Some(out)
    }

    fn rows(object: u32, keys: impl Iterator<Item = i32>, v: i32) -> ExportedTable {
        ExportedTable {
            pk_storage: None,
            company: "CRONUS".into(),
            object,
            rows: keys
                .map(|k| (vec![Value::Integer(k)], vec![Value::Integer(k), Value::Integer(v)]))
                .collect(),
        }
    }

    /// ADR-066 amendment 5, decision 5 (issue 6924): a carried start goes to a worker thread as the rows it holds apart
    /// from the install baseline. Put on top of a copy read from the install baseline's bytes it is the very start: the
    /// same fingerprint, rows, counters, marks and origin. A table the start no longer holds is gone; a new one is whole.
    #[test]
    fn a_row_delta_on_a_copy_of_its_base_gives_the_baseline_it_was_taken_from() {
        let mut base = sample();
        base.store = base
            .store
            .clone()
            .with_committed_tables(vec![(rows(18, 1..=500, 1), false), (rows(27, 1..=3, 1), false)], &[]);
        let mut later = base.clone();
        // A few rows of a big table differ, one is gone, one is new; table 27 is gone and 36 is new.
        let mut big = rows(18, (1..=500).filter(|&k| k != 7), 1);
        big.rows[100].1[1] = Value::Integer(99);
        big.rows
            .push((vec![Value::Integer(501)], vec![Value::Integer(501), Value::Integer(5)]));
        later.store = later
            .store
            .clone()
            .with_committed_tables(vec![(big, false), (rows(36, 1..=2, 3), true)], &[("CRONUS".into(), 27)]);
        later.next_guid += 5;
        later.last_stamp += 3;
        later.isolated_storage.clear();
        let d = later.row_delta_from(&base).expect("plain rows");
        let copy = super::super::Baseline::from_bytes(&base.to_bytes().unwrap()).unwrap();
        let got = copy.with_row_delta(&d).expect("applies");
        assert_eq!(fingerprint(&got), fingerprint(&later));
        assert_eq!(got.to_bytes(), later.to_bytes());
        assert_eq!(got.origin(), later.origin());
        // Only the rows that differ are in it, not the 500 of table 18.
        assert!(d.size() < 400, "{}", d.size());
        // Put on a baseline that does not hold a table it changes row by row, it does not apply.
        assert!(sample().with_row_delta(&d).is_none());
        // Without an install baseline: from nothing, every row is in the delta.
        let empty = super::super::Baseline::empty();
        let all = later.row_delta_from(&empty).unwrap();
        assert_eq!(fingerprint(&empty.with_row_delta(&all).unwrap()), fingerprint(&later));
    }
}
