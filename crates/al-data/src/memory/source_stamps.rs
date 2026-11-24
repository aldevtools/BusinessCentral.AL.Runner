//! The source stamp of a company's untouched tables (al-runner issue 8597). A table nothing wrote to in a company is
//! not in `tables`: it reads the backing's rows of the company's source (`backing_rows`), and its version and shape
//! stay 0. The source changes without a write to the table when `copies` or `dropped_backed_companies` change
//! (CopyCompany, Company.Delete, a rename, a copy read back from an export), so a cache keyed on the table's own stamp
//! alone would keep the rows of the previous source (a copy holds its source's rows,
//! session-system/company/Y22CopyCompany.al:65).
//!
//! Each such change gives the company a new source stamp (`source_stamps`). The stamp a cache keys on
//! ([`MemStore::stamp_of`]) is the later of the table's own stamp and its company's source stamp. Every stamp comes
//! from `next_stamp`, once in the whole process, and every change of the rows a table reads gives it or its company a
//! new one, so the later of the two names one state of the table's rows. `source_stamps` follows `copies` through
//! commit, rollback, a snapshot and a frozen store, so a rollback restores the stamp of the rows it restores.
//!
//! The source stamp of a company's untouched tables (al-runner issue 8597): they read their source's
//! backing rows, which change with `copies` and `dropped_backed_companies` (CopyCompany,
//! Company.Delete, rename, rollback) without a write to the table. Each such change stamps the
//! company (`source_stamps`), and `stamp_of` (the later of the table's own stamp and its company's)
//! is what `table_stamp`, `alt_cache`, the order cache and its carry, and the collation `KeptAt` key
//! on. Entry points: `stamp_of`, `source_changed`. Unit tests: `memory/source_stamps_tests.rs`.

use super::{MemStore, Slot, next_stamp};
use crate::pmap::PMap;
use std::sync::Arc;

impl MemStore {
    /// The stamp of the rows `slot` reads, as far as `own` goes (`versions`, or `shapes` for the place of the rows):
    /// the later of the table's own stamp in `own` and its company's source stamp.
    pub(super) fn stamp_of(&self, slot: &Slot, own: &PMap<Slot, u64>) -> u64 {
        let own = own.get(slot).copied().unwrap_or(0);
        if self.source_stamps.is_empty() {
            return own;
        }
        own.max(self.source_stamps.get(&slot.0).copied().unwrap_or(0))
    }

    /// The rows `company`'s untouched tables read changed (its entry in `copies` or `dropped_backed_companies`).
    pub(super) fn source_changed(&mut self, company: &str) {
        self.source_stamps.insert(Arc::from(company), next_stamp());
    }
}
