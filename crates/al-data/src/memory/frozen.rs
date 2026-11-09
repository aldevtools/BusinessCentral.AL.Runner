//! A store's committed tables frozen for every test thread of a run to read (ADR-069 decisions 1 to 3).
//!
//! A [`FrozenStore`] holds the same persistent maps a [`MemStore`] holds, so freezing copies no row. Each thread takes a
//! store of its own with [`FrozenStore::thaw`]: a clone of the maps' roots. The first write of that store to a table
//! copies the table's spine and the one chunk it changes (`pmap.rs`), so a write never reaches the frozen maps, and a
//! rollback drops only what the thread's store changed (decision 2).

use super::{MemStore, Slot, Tables};
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
}

// SAFETY (ADR-069 decision 3; a change here needs the owner, ADR-069 risk 2):
// `Value` is neither `Send` nor `Sync` only because some variants hold `Rc` handles (records, codeunits, lists, ...).
// `FrozenStore::freeze` is the only way to make a `FrozenStore`, and it checks every key and row value with
// `shareable`, an exhaustive `match` on `Value` (no `_` arm), so a new variant does not compile until it is classified.
// The values it accepts hold no `Rc` and no `Cell`/`RefCell`: plain data, or `Arc`s of `Sync` contents (a text's
// `Arc<Inner>` whose only interior mutability is two atomic hint flags; an Option's `Arc<[String]>`; an unknown column's
// `Arc<str>`; a Blob's `Arc<BlobCell>`, a `RwLock`). A key's code storage is an `Arc<[CodeStorage]>` of plain enums. The
// maps themselves are `Arc`-shared spines, chunks and entries (`pmap.rs`).
// Nothing writes through a `&FrozenStore`: there is no method that hands out `&mut` to its maps. `thaw` clones the
// roots (atomic count increments). The thawed store's writes go through `Arc::make_mut` / `Arc::try_unwrap`, which copy
// the spine, chunk or entry whenever another owner holds it, and the `FrozenStore` holds every one of them for as long
// as it lives; so no thread ever mutates a shared spine, chunk or entry in place. What remains shared and mutable is
// thread-safe by type: the text hint atomics, and a Blob's buffer behind its `RwLock` (a read copies a Blob before
// anything can write to it, `copy_blob_db_row`; ADR-069 risk 1, checked by `Baseline::fingerprint` at the end of a run).
// Dropping the last reference to an entry on any thread drops only such `Send` contents.
unsafe impl Send for FrozenStore {}
unsafe impl Sync for FrozenStore {}

/// Whether a value may sit in a [`FrozenStore`]: the plain values `Baseline::to_bytes` writes. Every variant is named
/// (no `_` arm): a new one does not compile here until someone decides whether it holds a thread-bound handle.
pub fn shareable(v: &Value) -> bool {
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
        | Value::Unknown(_) => true,
        // Not in a stored row, and `to_bytes` does not write them either (the same rule, decision 3).
        Value::TextConst(_) | Value::SqlCount(_) => false,
        // Handles: `Rc`, bound to their thread.
        Value::List(_)
        | Value::Dictionary(_)
        | Value::Array(_)
        | Value::Codeunit(_)
        | Value::Library(..)
        | Value::Record(_)
        | Value::TestPage(_)
        | Value::TestControl(_)
        | Value::Query(_)
        | Value::DotNet(_) => false,
    }
}

impl FrozenStore {
    /// The committed tables of `store`, frozen; `None` when a key or a row holds a value that is not [`shareable`].
    /// What `store` has not committed is not part of it. The provider of virtual tables, the backing, the caches, the
    /// access watch and the log of restored changes stay with `store` (decision 9): [`FrozenStore::thaw`] gives none.
    pub fn freeze(store: &MemStore) -> Option<FrozenStore> {
        let plain = store.committed.values().all(|t| {
            t.iter()
                .all(|(k, r)| k.0.iter().all(shareable) && r.iter().all(shareable))
        });
        plain.then(|| FrozenStore {
            tables: store.committed.clone(),
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
    pub fn thaw(&self) -> MemStore {
        MemStore {
            tables: self.tables.clone(),
            committed: self.tables.clone(),
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
        self.tables.values().map(|t| t.len()).sum()
    }
}

#[cfg(test)]
mod tests;
