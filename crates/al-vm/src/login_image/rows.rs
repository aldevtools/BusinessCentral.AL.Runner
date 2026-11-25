//! The committed rows a login image holds (al-runner issue 8211): the rows the login wrote, apart from the store it
//! started from ([`LoginRows::take`]), and those rows put on a thaw target's store ([`LoginRows::put_on`]).
//!
//! The rows go through their bytes (al-builtins `value_codec`), so the image shares no Blob buffer with the session
//! that goes on to run its tests, and any thread may keep it. al-data finds what differs (`committed_row_changes_from`,
//! which compares pointers first and only the rows that differ by value) and puts whole tables
//! (`with_committed_tables`, which notes their held columns as `from_exported` does): each table the login touched is
//! rebuilt from the target's own rows, the rows the login removed dropped and the rows it wrote put, matched by their
//! encoded keys. The tables a DeleteAll without filters emptied are the image's marks, and every other table keeps its
//! rows and stamp.

use al_builtins::value::Value;
use al_builtins::value_codec::{Reader, decode_row, encode_row, put_u32};
use al_data::memory::{ExportedTable, MemStore, PkStorage};
use std::collections::{BTreeMap, BTreeSet};

/// A table of a store: (company, table object).
type Slot = (String, u32);

/// One table the login changed, encoded.
struct TableRows {
    company: String,
    object: u32,
    /// The store the login started from held no entry for the table: `put` holds every row of it.
    whole: bool,
    pk_storage: PkStorage,
    /// The rows to put: count, then key and row of each.
    put: Vec<u8>,
    /// The keys of the rows that are gone: count, then each key.
    gone: Vec<u8>,
}

/// The committed rows a login wrote, apart from the store it started from (see the module comment).
pub(super) struct LoginRows {
    tables: Vec<TableRows>,
    /// The tables the start's store held an entry for and the login's store does not.
    gone: Vec<Slot>,
    /// Every table a DeleteAll without filters emptied, as the login left them (`MemStore::committed_marks`).
    emptied: Vec<Slot>,
}

impl LoginRows {
    /// The committed rows of `store` apart from `base`, the store the login started from, whose entries `base_tables`
    /// lists ((company, object, rows) of `Session::base_shape`). Rows are the same when their bytes are. `None` when a
    /// row that differs has no bytes form, or the company copies or the deleted companies differ.
    pub(super) fn take(store: &MemStore, base: &MemStore, base_tables: &[(String, u32, usize)]) -> Option<LoginRows> {
        let (mut x, mut y) = (Vec::new(), Vec::new());
        let mut same = |a: &[Value], b: &[Value]| (bytes(a, &mut x) && bytes(b, &mut y)).then(|| x == y);
        let changes = store.committed_row_changes_from(base, &mut same)?;
        let held: BTreeSet<(&str, u32)> = base_tables.iter().map(|(c, o, _)| (c.as_str(), *o)).collect();
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
            tables.push(TableRows {
                whole: !held.contains(&(t.company.as_str(), t.object)),
                company: t.company,
                object: t.object,
                pk_storage: t.pk_storage,
                put,
                gone,
            });
        }
        Some(LoginRows {
            tables,
            gone: changes.gone,
            emptied: store.committed_marks().emptied,
        })
    }

    /// Puts the rows on `store`, the thaw target's: each table the login changed is rebuilt from `store`'s own rows,
    /// the tables it removed are gone and the emptied marks are the login's. `None`, with `store` unchanged, when the
    /// rows do not read back or `store` holds no entry for a table the login changed row by row.
    pub(super) fn put_on(&self, store: &mut MemStore) -> Option<()> {
        let emptied: BTreeSet<&Slot> = self.emptied.iter().collect();
        let mut tables = Vec::with_capacity(self.tables.len());
        let mut touched: BTreeSet<Slot> = self.gone.iter().cloned().collect();
        for t in &self.tables {
            let slot = (t.company.clone(), t.object);
            let rows = t.rows_on(store)?;
            let table = ExportedTable {
                company: t.company.clone(),
                object: t.object,
                pk_storage: t.pk_storage.clone(),
                rows,
            };
            tables.push((table, emptied.contains(&slot)));
            touched.insert(slot);
        }
        tables.extend(marked_as(store, &self.emptied, &touched)?);
        *store = std::mem::take(store).with_committed_tables(tables, &self.gone);
        Some(())
    }

    /// About how many bytes the rows hold (their encoded rows).
    pub(super) fn size(&self) -> usize {
        self.tables.iter().map(|t| t.put.len() + t.gone.len()).sum()
    }
}

impl TableRows {
    /// The table's rows once the login's changes are put on `store`'s: every row of a `whole` table, else `store`'s
    /// rows without the keys that are gone and with the rows put in place of those with the same key. `None` when the bytes
    /// do not read back, a key has no bytes form, or `store` holds no entry for the table.
    fn rows_on(&self, store: &MemStore) -> Option<Vec<(Vec<Value>, Vec<Value>)>> {
        let mut r = Reader::new(&self.put);
        let mut put = Vec::new();
        for _ in 0..r.u32()? {
            let k = decode_row(&mut r)?;
            put.push((k, decode_row(&mut r)?));
        }
        if !r.is_empty() {
            return None;
        }
        if self.whole {
            return Some(put);
        }
        let mut r = Reader::new(&self.gone);
        let mut gone = BTreeSet::new();
        for _ in 0..r.u32()? {
            gone.insert(key_bytes(&decode_row(&mut r)?)?);
        }
        if !r.is_empty() {
            return None;
        }
        let (current, _) = store.committed_table(&self.company, self.object)?;
        let mut rows = BTreeMap::new();
        for (k, row) in current.rows {
            let at = key_bytes(&k)?;
            if !gone.contains(&at) {
                rows.insert(at, (k, row));
            }
        }
        for (k, row) in put {
            rows.insert(key_bytes(&k)?, (k, row));
        }
        Some(rows.into_values().collect())
    }
}

/// The tables of `store` outside `touched` whose emptied mark is not the one `emptied` gives, each with its rows and
/// that mark: put with `with_committed_tables`, they leave the store's marks as `emptied` lists them. `None` when a
/// table to mark has no entry in `store`.
pub(super) fn marked_as(
    store: &MemStore,
    emptied: &[Slot],
    touched: &BTreeSet<Slot>,
) -> Option<Vec<(ExportedTable, bool)>> {
    let want: BTreeSet<&Slot> = emptied.iter().collect();
    let have = store.committed_marks().emptied;
    let have: BTreeSet<&Slot> = have.iter().collect();
    let mut out = Vec::new();
    for slot in want.symmetric_difference(&have) {
        if touched.contains(*slot) {
            continue;
        }
        let (table, _) = store.committed_table(&slot.0, slot.1)?;
        out.push((table, want.contains(slot)));
    }
    Some(out)
}

/// The bytes of a row (or key) in `buf`; `false` when it holds a value that is not plain.
fn bytes(row: &[Value], buf: &mut Vec<u8>) -> bool {
    buf.clear();
    encode_row(row, buf)
}

/// The bytes of a key, to match keys by; `None` when it holds a value that is not plain.
fn key_bytes(key: &[Value]) -> Option<Vec<u8>> {
    let mut b = Vec::new();
    bytes(key, &mut b).then_some(b)
}
