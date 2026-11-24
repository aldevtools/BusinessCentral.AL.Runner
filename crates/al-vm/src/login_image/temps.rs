//! A temporary table in the arena: the rows its store holds now (such a store never commits), per
//! table, as plain values and Blob nodes, so a row and a record that share a Blob buffer still share it after the thaw.
//! Only a table a record node names is imaged: no AL value reaches another one. The thaw makes a new table of the
//! thawn session, with an id from its own counter: a temporary table's id is never observable.
//!
//! The token's note of an Insert that stored a row while a Blob of the record was empty (al-builtins `TempId`, al-vm
//! `temp_blob_share.rs`) goes with the table: the key, and the record that inserted it as a node while it lives.

use super::NotImaged;
use super::arena::{Frozen, NodeId};
use super::freeze::{Holder, Walker};
use super::thaw::Maker;

/// The rows of one temporary table's store.
#[derive(Debug)]
pub(crate) struct TempImage {
    pub(crate) tables: Vec<TempTableRows>,
    /// `TempId::note_empty_blob_insert`: the key it noted, and its inserter while that lives.
    pub(crate) empty_blob_insert: Option<(Vec<Frozen>, Option<NodeId>)>,
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
    /// The rows of the temporary table of `token` (none when the session holds no store for it yet: no row was written),
    /// and the token's empty-Blob Insert note.
    pub(crate) fn temp_table(
        &mut self,
        token: &al_builtins::value::TempId,
        path: &str,
    ) -> Result<TempImage, NotImaged> {
        let id = token.id();
        let key = token.with_empty_blob_insert_key(|k| k.map(<[_]>::to_vec));
        let empty_blob_insert = match key {
            Some(key) => {
                let key = self.field_values(&key, &format!("{path} empty-Blob insert key"))?;
                let inserter = (token.empty_blob_inserter())
                    .map(|r| self.handle(Holder::Record(r), 0, format!("{path} empty-Blob inserter")));
                Some((key, inserter))
            }
            None => None,
        };
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
        Ok(TempImage {
            tables,
            empty_blob_insert,
        })
    }
}

impl Maker<'_> {
    /// Writes the empty-Blob Insert note of `image` on the thawn token `token`, after every record is made: the inserter's
    /// own record, or, when it is gone, the note's key alone (noted with `holder`, a record of the table, then forgotten).
    pub(crate) fn note_empty_blob_insert(
        &self,
        image: &TempImage,
        token: &al_builtins::value::TempId,
        holder: Option<NodeId>,
    ) {
        let Some((key, inserter)) = &image.empty_blob_insert else {
            return;
        };
        let key = self.values(key);
        match (inserter, holder) {
            (Some(r), _) => token.note_empty_blob_insert(&self.record_handle(*r), key.iter()),
            (None, Some(h)) => {
                token.note_empty_blob_insert(&self.record_handle(h), key.iter());
                token.forget_empty_blob_inserter();
            }
            (None, None) => unreachable!("a temporary table is in the image only through a record of it"),
        }
    }
}
