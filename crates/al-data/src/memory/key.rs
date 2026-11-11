//! A primary-key tuple and the order of its table (al-runner issue 7618).
//!
//! The order of a table's keys is a property of the table: the storage of its primary key fields
//! ([`crate::code_storage::pk_storage`]). Every key of a table carries the same storage, so two keys of one table always
//! compare the same way. A key is made only from a table definition ([`Key::of`]) or from the storage a table already
//! has ([`Key::in_table`], which an import takes from the exported table): no key is made without its table's order.

use crate::{CodeStorage, TableDef, Value};
use al_builtins::filter::key_cmp;
use std::cmp::Ordering;
use std::sync::Arc;

/// How a table's primary key fields are stored, when any of them is not text; `None` for the usual table (text order).
pub type PkStorage = Option<Arc<[CodeStorage]>>;

/// A primary-key tuple ordered field by field.
#[derive(Clone, Debug)]
pub(super) struct Key(pub(super) Vec<Value>, PkStorage);

impl Key {
    /// The key of the primary key values `pk` of table `t`. Values of Code fields the database stores as a number order
    /// by number (Y82PkTests.al `PkInt_NumericOrder`).
    pub(super) fn of(t: &TableDef, pk: Vec<Value>) -> Key {
        Key(pk, crate::code_storage::pk_storage(t))
    }

    /// The key `pk` of a table whose primary key is stored as `storage` (see [`table_storage`]).
    pub(super) fn in_table(pk: Vec<Value>, storage: &PkStorage) -> Key {
        Key(pk, storage.clone())
    }
}

/// The storage of the primary key of `table` (its keys all carry the same); `None` for an empty table.
pub(super) fn table_storage(table: &super::Table) -> PkStorage {
    table.keys().next().and_then(|k| k.1.clone())
}

impl PartialEq for Key {
    fn eq(&self, o: &Self) -> bool {
        self.cmp(o) == Ordering::Equal
    }
}
impl Eq for Key {}
impl PartialOrd for Key {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Key {
    fn cmp(&self, o: &Self) -> Ordering {
        // The keys of one table carry the same storage; `or` keeps the comparison symmetric all the same.
        match self.1.as_deref().or(o.1.as_deref()) {
            Some(storage) => crate::code_storage::cmp_pk(storage, &self.0, &o.0),
            None => key_cmp(&self.0, &o.0),
        }
    }
}
