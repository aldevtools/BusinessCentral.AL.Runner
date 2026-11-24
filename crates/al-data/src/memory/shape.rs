//! What a store holds besides the contents of its rows, as a replay of the login checks it before it puts a row delta
//! on the store (al-vm `login_image`, `BaseShape`, al-runner issue 8211): the committed table entries with their row
//! counts (an empty entry is not an absent table, and a row delta changes a table row by row only where it has one),
//! and whether an access watch is on (it would see the replay's writes); and the stamp of each committed table, by which
//! al-testrun's login key keeps the digest of a table's rows once per state of the table, and the rows it hashes.

use super::MemStore;

impl MemStore {
    /// (company, table object, rows) of every committed table entry, sorted. Marks nothing as read.
    pub fn committed_entries(&self) -> Vec<(String, u32, usize)> {
        let mut out: Vec<(String, u32, usize)> = (self.committed.iter())
            .map(|((company, object), t)| (company.to_string(), *object, t.len()))
            .collect();
        out.sort();
        out
    }

    /// (company, table object, stamp) of every committed table entry, in the store's order: a stamp names one state of one
    /// table in every store of the process (`next_stamp`), 0 when the entry has none.
    pub fn committed_stamps(&self) -> Vec<(String, u32, u64)> {
        (self.committed.iter())
            .map(|(slot, _)| {
                let stamp = self.committed_versions.get(slot).copied().unwrap_or(0);
                (slot.0.to_string(), slot.1, stamp)
            })
            .collect()
    }

    /// The committed rows of table `object` in `company`, in key order, by reference: what al-testrun's login key hashes
    /// without a copy of the table (`MemStore::committed_table` clones every row). `None` when the store holds no entry
    /// for it. Marks nothing as read.
    pub fn committed_rows(&self, company: &str, object: u32) -> Option<impl Iterator<Item = &crate::Row> + '_> {
        let slot: super::Slot = (std::sync::Arc::from(company), object);
        Some(self.committed.get(&slot)?.iter().map(|(_, row)| row))
    }

    /// Whether an access watch is on (`MemStore::watch`, `MemStore::watch_window`).
    pub fn watching(&self) -> bool {
        self.watch.is_some()
    }
}
