//! A store's committed tables frozen for every test thread of a run to read (ADR-069 decisions 1 to 3).
//!
//! A [`FrozenStore`] holds the same persistent maps a [`MemStore`] holds, so freezing copies no row. Each thread takes a
//! store of its own with [`FrozenStore::thaw`]: a clone of the maps' roots. The first write of that store to a table
//! copies the table's spine and the one chunk it changes (`pmap.rs`), so a write never reaches the frozen maps, and a
//! rollback drops only what the thread's store changed (decision 2).
//!
//! A table whose rows hold a value that is not [`shareable`] but has a plain form in `value_codec` (a RecordId, a
//! TableFilter: a large partner app's install code leaves Job Queue Entry rows with a `Record ID to Process`) is held as
//! those bytes instead, and each thread's [`FrozenStore::thaw`] reads its own copy of it: the copy per thread of
//! decision 3, for that table alone. Every other table stays shared.

use super::{MemStore, Rows, Slot, Tables};
use al_builtins::describe::Describe;
use al_builtins::value::Value;

/// Committed tables, with their stamps and marks, that every thread may read and none writes (see the module comment).
pub struct FrozenStore {
    tables: Tables,
    versions: crate::pmap::PMap<Slot, u64>,
    shapes: crate::pmap::PMap<Slot, u64>,
    moves: crate::pmap::PMap<(Slot, usize), u64>,
    copies: crate::pmap::PMap<std::sync::Arc<str>, std::sync::Arc<str>>,
    emptied: crate::pmap::PMap<Slot, ()>,
    unknown_cols: std::collections::HashMap<u32, Vec<usize>>,
    counter: u64,
    /// The tables whose rows hold a value that is not [`shareable`], as bytes (see the module comment).
    held: Vec<Held>,
}

/// A table held as bytes: its place, the order of its keys, its rows written with `value_codec::encode_row` (key, then
/// row), and how many.
struct Held {
    slot: Slot,
    pk_storage: super::PkStorage,
    bytes: Vec<u8>,
    rows: usize,
}

// SAFETY (ADR-069 decision 3; a change here needs the owner, ADR-069 risk 2):
// `Value` is neither `Send` nor `Sync` only because some variants hold `Rc` handles (records, codeunits, lists, ...).
// `FrozenStore::freeze` is the only way to make a `FrozenStore`, and it checks every key and row value with
// `shareable`, an exhaustive `match` on `Value` (no `_` arm), so a new variant does not compile until it is classified.
// The values it accepts hold no `Rc` and no `Cell`/`RefCell`: plain data, or `Arc`s of `Sync` contents (a text's
// `Arc<Inner>` whose only interior mutability is two atomic hint flags; an Option's `Arc<[String]>`; an unknown column's
// `Arc<str>`; a Blob's `Arc<BlobCell>`, a `RwLock`). A key's code storage is an `Arc<[CodeStorage]>` of plain enums. The
// maps themselves are `Arc`-shared spines, chunks and entries (`pmap.rs`). A table with a value `shareable` refuses
// is not in the maps at all: `freeze` writes it as plain bytes (`Held`: an `Arc<str>` company, a `Vec<u8>`), and each
// `thaw` reads a new copy of it on the calling thread, so its handles are made on, and never leave, that thread.
// Nothing writes through a `&FrozenStore`: there is no method that hands out `&mut` to its maps. `thaw` clones the
// roots (atomic count increments). The thawed store's writes go through `Arc::make_mut` / `Arc::try_unwrap`, which copy
// the spine, chunk or entry whenever another owner holds it, and the `FrozenStore` holds every one of them for as long
// as it lives; so no thread ever mutates a shared spine, chunk or entry in place. What remains shared and mutable is
// thread-safe by type: the text hint atomics, and a Blob's buffer behind its `RwLock` (a read copies a Blob before
// anything can write to it, `copy_blob_db_row`; ADR-069 risk 1, checked by `Baseline::fingerprint` at the end of a run).
// Dropping the last reference to an entry on any thread drops only such `Send` contents.
unsafe impl Send for FrozenStore {}
unsafe impl Sync for FrozenStore {}

/// Whether a value may sit in a [`FrozenStore`]: the plain values `Baseline::to_bytes` writes.
pub fn shareable(v: &Value) -> bool {
    not_shareable(v).is_none()
}

/// The kind of a value that may not sit in a [`FrozenStore`], for the message that says why a store was not frozen;
/// `None` for a plain value (see [`shareable`]). Every variant is named (no `_` arm): a new one does not compile here
/// until someone decides whether it holds a thread-bound handle.
pub fn not_shareable(v: &Value) -> Option<&'static str> {
    match v {
        Value::Null
        | Value::Boolean(_)
        | Value::Integer(_)
        | Value::BigInteger(_)
        | Value::Decimal(_)
        | Value::Byte(_)
        | Value::Char(_)
        | Value::Text(_)
        | Value::Code(_)
        | Value::Date(..)
        | Value::Time(_)
        | Value::DateTime(_)
        | Value::Duration(_)
        | Value::Guid(_)
        | Value::DateFormula(_)
        | Value::Option(..)
        | Value::Enum(..)
        | Value::Blob(_)
        | Value::Unknown(_) => None,
        // Not in a stored row, and `to_bytes` does not write them either (the same rule, decision 3).
        Value::TextConst(_) => Some("TextConst"),
        Value::SqlCount(_) => Some("SqlCount"),
        Value::PlaceholderSize(_) => Some("PlaceholderSize"),
        // Handles: `Rc`, bound to their thread.
        Value::List(_) => Some("List"),
        Value::Dictionary(_) => Some("Dictionary"),
        Value::Array(_) => Some("Array"),
        Value::Codeunit(_) => Some("Codeunit"),
        Value::Library(..) => Some("Library"),
        Value::Record(_) => Some("Record"),
        Value::TestPage(_) => Some("TestPage"),
        Value::TestControl(_) => Some("TestControl"),
        Value::Query(_) => Some("Query"),
        Value::DotNet(_) => Some("DotNet"),
    }
}

/// Where a store's committed tables first hold a value that is not [`shareable`]: why it cannot be frozen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotShareable {
    pub company: String,
    /// The table's object index (as `TableDef::object`).
    pub object: u32,
    /// The value is in the row's primary key (`position` counts the key's fields), not in the row (`position` counts
    /// the table's fields).
    pub in_key: bool,
    pub position: usize,
    /// What the value is: its variant, a library type with its name and what its handle holds
    /// (`Library RecordId holding RecordId`).
    pub kind: String,
}

impl MemStore {
    /// The first value of the committed tables that is not [`shareable`], in table and key order; `None` when every
    /// value is plain.
    pub fn first_not_shareable(&self) -> Option<NotShareable> {
        self.first_value_where(|v| !shareable(v))
    }

    /// The first value of the committed tables, in table and key order, for which `blocks` says true: where it is and
    /// what it is.
    pub fn first_value_where(&self, blocks: impl Fn(&Value) -> bool) -> Option<NotShareable> {
        self.committed.iter().find_map(|((company, object), rows)| {
            rows.iter().find_map(|(k, r)| {
                let at = |in_key: bool, vals: &[Value]| {
                    vals.iter()
                        .enumerate()
                        .find(|(_, v)| blocks(v))
                        .map(|(position, v)| NotShareable {
                            company: company.to_string(),
                            object: *object,
                            in_key,
                            position,
                            kind: kind_of(v),
                        })
                };
                at(true, &k.0).or_else(|| at(false, r))
            })
        })
    }
}

/// What a value is, for a message: `Describe::describe`, which names a library handle by its type.
fn kind_of(v: &Value) -> String {
    v.describe()
}

impl FrozenStore {
    /// The committed tables of `store`, frozen; `Err` names the first key or row value that is not [`shareable`].
    /// What `store` has not committed is not part of it. The provider of virtual tables, the backing, the caches, the
    /// access watch and the log of restored changes stay with `store` (decision 9): [`FrozenStore::thaw`] gives none.
    /// A table that holds a value that is not shareable is held as bytes (see the module comment); `Err` only when
    /// such a value has no bytes form either.
    pub fn freeze(store: &MemStore) -> Result<FrozenStore, NotShareable> {
        let no_form = |v: &Value| !shareable(v) && !al_builtins::value_codec::encode(v, &mut Vec::new());
        if let Some(why) = store.first_value_where(no_form) {
            return Err(why);
        }
        let mut tables = store.committed.clone();
        let mut held = Vec::new();
        for (slot, rows) in store.committed.iter() {
            if rows
                .iter()
                .all(|(k, r)| k.0.iter().all(shareable) && r.iter().all(shareable))
            {
                continue;
            }
            let mut bytes = Vec::new();
            for (k, r) in rows.iter() {
                if !al_builtins::value_codec::encode_row(&k.0, &mut bytes)
                    || !al_builtins::value_codec::encode_row(r, &mut bytes)
                {
                    unreachable!("every value has a bytes form (checked above)");
                }
            }
            held.push(Held {
                slot: slot.clone(),
                pk_storage: super::table_storage(rows),
                bytes,
                rows: rows.len(),
            });
        }
        for h in &held {
            tables.remove(&h.slot);
        }
        Ok(FrozenStore {
            held,
            tables,
            versions: store.committed_versions.clone(),
            shapes: store.committed_shapes.clone(),
            moves: store.committed_moves.clone(),
            copies: store.committed_copies.clone(),
            emptied: store.committed_emptied.clone(),
            unknown_cols: store.unknown_cols.clone(),
            counter: store.counter,
        })
    }

    /// A store of this thread's own holding the frozen tables, committed, with fresh caches and no provider or backing
    /// (`MemStore::with_backing` adds one). A clone of the maps' roots: no row is copied.
    /// A table held as bytes is read into a copy of this thread's own, its keys in the order it had.
    pub fn thaw(&self) -> MemStore {
        let mut tables = self.tables.clone();
        for h in &self.held {
            let mut r = al_builtins::value_codec::Reader::new(&h.bytes);
            let table: super::Table = (0..h.rows)
                .map(|_| {
                    let k = al_builtins::value_codec::decode_row(&mut r).expect("bytes `freeze` wrote");
                    let row = al_builtins::value_codec::decode_row(&mut r).expect("bytes `freeze` wrote");
                    (super::Key::in_table(k, &h.pk_storage), row)
                })
                .collect();
            tables.insert(h.slot.clone(), table);
        }
        MemStore {
            tables: tables.clone(),
            committed: tables,
            versions: self.versions.clone(),
            committed_versions: self.versions.clone(),
            shapes: self.shapes.clone(),
            committed_shapes: self.shapes.clone(),
            moves: self.moves.clone(),
            committed_moves: self.moves.clone(),
            copies: self.copies.clone(),
            committed_copies: self.copies.clone(),
            emptied: self.emptied.clone(),
            committed_emptied: self.emptied.clone(),
            unknown_cols: self.unknown_cols.clone(),
            counter: self.counter,
            ..Default::default()
        }
    }

    /// How many rows the frozen tables hold.
    pub fn row_count(&self) -> usize {
        self.tables.values().map(|t| t.len()).sum::<usize>() + self.held.iter().map(|h| h.rows).sum::<usize>()
    }

    /// The tables held as bytes, a copy per thread (see the module comment): (company, table object index, rows).
    pub fn held_tables(&self) -> Vec<(String, u32, usize)> {
        self.held
            .iter()
            .map(|h| (h.slot.0.to_string(), h.slot.1, h.rows))
            .collect()
    }

    /// About how many bytes the frozen tables hold (see [`table_bytes`]): what the memory budget counts once for the
    /// whole run (ADR-069 decision 10). A table held as bytes is each thread's own cost, not counted here.
    pub fn bytes(&self) -> u64 {
        self.tables.values().map(table_bytes).sum()
    }
}

/// About how many bytes a table's rows hold: per row its key and row vectors with their values, the text, Blob and other
/// heap contents the values own, and the map's entry. An estimate for the memory budget, never compared exactly.
fn table_bytes(t: &super::Table) -> u64 {
    const ENTRY: usize = 48; // the map's `Arc`-held entry and its share of a chunk
    let vec = std::mem::size_of::<Vec<Value>>();
    let value = std::mem::size_of::<Value>();
    t.iter()
        .map(|(k, r)| {
            let values = k.0.iter().chain(r.iter());
            (ENTRY + 2 * vec + (k.0.len() + r.len()) * value + values.map(heap_bytes).sum::<usize>()) as u64
        })
        .sum()
}

/// About how many bytes on the heap a value owns beyond its own size.
fn heap_bytes(v: &Value) -> usize {
    const ARC: usize = 16; // an `Arc`'s two counts
    match v {
        // `Txt`: an `Arc` of the string and two flags, and the string's bytes.
        Value::Text(t) | Value::Code(t) => ARC + std::mem::size_of::<String>() + 8 + t.as_str().len(),
        Value::Blob(b) => ARC + 64 + b.borrow().bytes.len(),
        Value::DateFormula(f) => f.len(),
        Value::Unknown(why) => why.len(),
        _ => 0,
    }
}

/// The rows of one table of the test data, built once per process and read by every thread (ADR-069 decision 4).
pub struct FrozenRows(Rows);

// SAFETY: the argument of `FrozenStore` above: `FrozenRows::freeze` checks every key and row value with `shareable`,
// nothing writes through a `&FrozenRows`, and `thaw` clones the map's root, whose writers copy what they change.
unsafe impl Send for FrozenRows {}
unsafe impl Sync for FrozenRows {}

impl FrozenRows {
    /// `rows` frozen; `Err` gives them back when a key or a row holds a value that is not [`shareable`].
    pub fn freeze(rows: Rows) -> Result<FrozenRows, Rows> {
        let plain = rows
            .0
            .iter()
            .all(|(k, r)| k.0.iter().all(shareable) && r.iter().all(shareable));
        if plain { Ok(FrozenRows(rows)) } else { Err(rows) }
    }

    /// About how many bytes the rows hold (as `FrozenStore::bytes`).
    pub fn bytes(&self) -> u64 {
        table_bytes(&(self.0).0)
    }

    /// A thread's own handle on the rows: a clone of the map's root.
    pub fn thaw(&self) -> Rows {
        self.0.clone()
    }
}

#[cfg(test)]
mod tests;
