//! A temporary table in the arena (design section 3.2): the rows its store holds now (such a store never commits), per
//! table, as plain values and Blob nodes, so a row and a record that share a Blob buffer still share it after the thaw.
//! Only a table a record node names is imaged: no AL value reaches another one. The thaw makes a new table of the
//! thawn session, with an id from its own counter: a temporary table's id is never observable.

use super::NotImaged;
use super::arena::Frozen;
use super::freeze::Walker;

/// The rows of one temporary table's store.
#[derive(Debug)]
pub(crate) struct TempImage {
    pub(crate) tables: Vec<TempTableRows>,
}

/// One (company, table) of a temporary table's store, its rows in key order.
#[derive(Debug)]
pub(crate) struct TempTableRows {
    pub(crate) company: String,
    pub(crate) object: u32,
    pub(crate) pk_storage: al_data::memory::PkStorage,
    pub(crate) rows: Vec<(Vec<Frozen>, Vec<Frozen>)>,
}

impl TempImage {
    /// How many rows the table holds.
    pub(crate) fn row_count(&self) -> usize {
        self.tables.iter().map(|t| t.rows.len()).sum()
    }
}

impl Walker<'_> {
    /// The rows of the temporary table `id` (none when the session holds no store for it yet: no row was written).
    pub(crate) fn temp_table(&mut self, id: u64, path: &str) -> Result<TempImage, NotImaged> {
        let exported = self
            .session
            .temps
            .get(&id)
            .map(|s| s.export_current())
            .unwrap_or_default();
        let mut tables = Vec::with_capacity(exported.len());
        for t in exported {
            let mut rows = Vec::with_capacity(t.rows.len());
            for (i, (key, row)) in t.rows.iter().enumerate() {
                let at = format!("{path} row {i}");
                rows.push((self.field_values(key, &at)?, self.field_values(row, &at)?));
            }
            tables.push(TempTableRows {
                company: t.company,
                object: t.object,
                pk_storage: t.pk_storage,
                rows,
            });
        }
        Ok(TempImage { tables })
    }
}
