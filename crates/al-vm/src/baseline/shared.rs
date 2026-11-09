//! One install baseline for every test thread of a run (ADR-069 decisions 1 to 3, 8 and 11).
//!
//! [`Baseline::freeze`] makes a [`SharedBaseline`] once, after the install code; each test thread takes its own
//! [`Baseline`] from it with [`SharedBaseline::thaw`], which copies no row: its store's maps share every chunk with the
//! frozen ones, and the store's first write to a table copies only that table's spine and one chunk (decision 2). The
//! counters (next SystemId, row versions, AutoIncrement, number sequences, isolated storage, last stamp) are plain
//! values, copied into each thread's baseline and from there into each session at restore (decision 8).

use super::Baseline;

/// The install baseline frozen for every test thread (see the module comment). `Send + Sync`: its store is an
/// `al_data::memory::FrozenStore` (where the safety argument sits); every other field is plain owned data.
pub struct SharedBaseline {
    store: al_data::memory::FrozenStore,
    next_guid: u64,
    next_row_version: i64,
    committed_row_version: i64,
    autoinc: std::collections::HashMap<(String, u32), i64>,
    isolated_storage: std::collections::HashMap<(String, String), (String, bool, bool)>,
    sequences: std::collections::HashMap<(String, String), crate::sessionops::Sequence>,
    last_stamp: i64,
    workspace_installs: Option<Vec<(std::collections::BTreeSet<u32>, String)>>,
    company_built: bool,
    company_unknown: Vec<(String, u32, String)>,
    full_copies: Vec<String>,
    origin: u64,
    /// `Baseline::to_bytes` of the frozen baseline, to check at the end of a run that nothing wrote to it.
    bytes: Vec<u8>,
}

impl Baseline {
    /// This baseline frozen for every test thread to share (ADR-069 decision 1), or why it cannot be: a row holds a
    /// value that is not plain (decision 3), and each thread then keeps a copy of its own.
    ///
    /// The frozen tables are those a copy read from this baseline's bytes holds (`to_bytes`, `from_bytes`), so each
    /// thread starts from exactly what the copy per thread it replaces held: the same rows, keys and counters, and
    /// Blobs that no store of the install thread shares. That is one copy of the install rows per run (a few MB).
    pub fn freeze(&self) -> Result<SharedBaseline, &'static str> {
        const NOT_PLAIN: &str = "a row of the install baseline holds a value that is not plain";
        let bytes = self.to_bytes().ok_or(NOT_PLAIN)?;
        let copy = Baseline::from_bytes(&bytes).ok_or("the install baseline does not read back from its bytes")?;
        Ok(SharedBaseline {
            store: al_data::memory::FrozenStore::freeze(&copy.store).ok_or(NOT_PLAIN)?,
            next_guid: copy.next_guid,
            next_row_version: copy.next_row_version,
            committed_row_version: copy.committed_row_version,
            autoinc: copy.autoinc,
            isolated_storage: copy.isolated_storage,
            sequences: copy.sequences,
            last_stamp: copy.last_stamp,
            workspace_installs: copy.workspace_installs,
            company_built: copy.company_built,
            company_unknown: copy.company_unknown,
            full_copies: copy.full_copies,
            origin: self.origin,
            bytes,
        })
    }
}

impl SharedBaseline {
    /// A test thread's own baseline over the frozen one, over the test data of `engine` when it has any (as
    /// `Baseline::backed_by` puts it on a copy read from bytes). It has the frozen baseline's origin: it holds the same
    /// data. Costs a clone of the maps' roots and of the counters.
    pub fn thaw(&self, engine: Option<&std::sync::Arc<crate::Engine>>) -> Baseline {
        let b = Baseline {
            store: self.store.thaw(),
            next_guid: self.next_guid,
            next_row_version: self.next_row_version,
            committed_row_version: self.committed_row_version,
            autoinc: self.autoinc.clone(),
            isolated_storage: self.isolated_storage.clone(),
            sequences: self.sequences.clone(),
            last_stamp: self.last_stamp,
            workspace_installs: self.workspace_installs.clone(),
            company_built: self.company_built,
            install_rows: None,
            company_unknown: self.company_unknown.clone(),
            full_copies: self.full_copies.clone(),
            origin: self.origin,
        };
        match engine {
            Some(e) => b.backed_by(e),
            None => b,
        }
    }

    /// How many rows the frozen install baseline holds.
    pub fn rows(&self) -> usize {
        self.store.row_count()
    }

    /// Whether the frozen baseline still holds what it held when it was frozen (ADR-069 risk 1, "Base unchanged"):
    /// its bytes now are the bytes then.
    pub fn unchanged(&self) -> bool {
        self.thaw(None).to_bytes().as_deref() == Some(&self.bytes[..])
    }
}

#[cfg(test)]
mod tests;
