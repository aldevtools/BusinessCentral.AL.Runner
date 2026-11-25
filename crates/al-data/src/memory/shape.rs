//! What a store holds besides the contents of its rows, as a replay of the login checks it before it puts a row delta
//! on the store (al-vm `login_image`, `BaseShape`, al-runner issue 8211): the committed table entries with their row
//! counts (an empty entry is not an absent table, and a row delta changes a table row by row only where it has one),
//! the committed marks the rows leave out ([`CommittedMarks`]), and whether an access watch is on (it would see the
//! replay's writes); and the stamp of each committed table, by which al-testrun's login key keeps the digest of a
//! table's rows once per state of the table.

use super::MemStore;

/// The marks of the committed store its rows leave out (an export, `MemStore::export_committed`, leaves them out too),
/// each sorted. With any of them, two stores with the same rows can answer a read otherwise.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommittedMarks {
    /// (company, table object) of every table a DeleteAll without filters emptied.
    pub emptied: Vec<(String, u32)>,
    /// (copy, source) of every company copy: a read in a copy answers with its source's backing rows
    /// (`MemStore::note_copy`).
    pub copies: Vec<(String, String)>,
    /// The companies deleted with their rows (`MemStore::drop_company`): a read in a company made again of the name
    /// answers with no backing row (record/TestChangeCompanyFiltersCopyDeleteContracts.al:204-212).
    pub deleted: Vec<String>,
}

impl MemStore {
    /// The committed marks the rows leave out ([`CommittedMarks`]).
    pub fn committed_marks(&self) -> CommittedMarks {
        self.footprint_everything("an export of the store");
        let mut emptied: Vec<(String, u32)> = self.committed_emptied.keys().map(|s| (s.0.to_string(), s.1)).collect();
        emptied.sort();
        let mut copies: Vec<(String, String)> = (self.committed_copies.iter())
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        copies.sort();
        let mut deleted: Vec<String> = (self.committed_dropped_backed_companies.keys())
            .map(|c| c.to_string())
            .collect();
        deleted.sort();
        CommittedMarks {
            emptied,
            copies,
            deleted,
        }
    }

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

    /// Whether an access watch is on (`MemStore::watch`, `MemStore::watch_window`).
    pub fn watching(&self) -> bool {
        self.watch.is_some()
    }
}
