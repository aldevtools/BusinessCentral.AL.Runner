//! The query surface both containers implement (port of `IBcSource`): the tables, their
//! columns, their clustered key, and their rows already decoded. Everything above this
//! line is BC semantics (company and app-id parsing, `$ext` companions, AL symbols) and
//! is shared by every container.
//!
//! The abstraction is "enumerate rows and column metadata for a table", not pages: a
//! `.bacpac` has no pages, no compression and no LOB indirection.

use crate::error::Result;
use crate::io::ByteSource;
use crate::types::SysColumn;
use crate::value::Value;

/// One readable table of a source, named the way SQL Server names the object.
#[derive(Debug, Clone)]
pub struct SourceTable {
    pub name: String,
    /// Storage compression of the rows in the file ("none"/"row"/"page"), or "-" where
    /// the container has none.
    pub compression: String,
    pub(crate) handle: usize,
}

/// What the read surface needs of a container. Implementations are `Sync`, so a caller
/// may read different tables from different threads.
pub trait BcSource: Send + Sync {
    fn tables(&self) -> Result<&[SourceTable]>;

    /// Row count, computed on demand (a `.bacpac` has to read its data stream to know it).
    fn row_count(&self, t: &SourceTable) -> Result<i64>;

    fn columns(&self, t: &SourceTable) -> Result<Vec<SysColumn>>;

    /// The columns a table's rows are identified by, in key order; empty when it has none.
    /// A `.bak` answers from the clustered index, a `.bacpac` from the primary-key
    /// constraint, and those are not always the same key. The one caller that joins on
    /// this key asks a `$ext` companion, where the two coincide.
    fn row_key_columns(&self, t: &SourceTable) -> Result<Vec<String>>;

    /// Rows as decoded values, one `Value` per requested column in `columns` order. Only
    /// those columns are decoded. `sink` returns `Ok(false)` to stop the read (a `--top`).
    fn read_rows(
        &self,
        t: &SourceTable,
        columns: &[SysColumn],
        sink: &mut dyn FnMut(&[Value]) -> Result<bool>,
    ) -> Result<()>;

    /// The database's SQL sequences (a `.bak`; a `.bacpac` holds no state of them).
    fn sequences(&self) -> Result<Vec<crate::catalog::SqlSequence>> {
        Ok(Vec::new())
    }

    /// The database's last row version (a `.bak`'s boot page timestamp, `catalog::db_timestamp`); a `.bacpac` holds none.
    fn last_row_version(&self) -> Result<Option<u64>> {
        Ok(None)
    }

    /// One line about what was opened, for the `tables` command's stderr note.
    fn banner(&self) -> Result<String>;
}

/// Collect every row. Convenience for tests and small tables.
pub fn read_all(src: &dyn BcSource, t: &SourceTable, columns: &[SysColumn]) -> Result<Vec<Vec<Value>>> {
    let mut rows = Vec::new();
    src.read_rows(t, columns, &mut |r| {
        rows.push(r.to_vec());
        Ok(true)
    })?;
    Ok(rows)
}

/// A `.bacpac` is an Open Packaging zip; a `.bak` starts with an MTF descriptor block.
pub fn is_bacpac(head: &[u8]) -> bool {
    head.len() >= 4 && head[0] == b'P' && head[1] == b'K' && head[2] == 3 && head[3] == 4
}

/// Open either container by looking at its first bytes.
pub fn open(src: Box<dyn ByteSource>) -> Result<Box<dyn BcSource>> {
    let mut head = [0u8; 4];
    if src.len() >= 4 {
        src.read_at(0, &mut head)?;
    }
    if is_bacpac(&head) {
        Ok(Box::new(crate::bacpac::BacpacSource::open(src)?))
    } else {
        Ok(Box::new(crate::bak::BakSource::open(src)?))
    }
}

/// [`open`], keeping a `.bak`'s page map in `store` between runs (a `.bacpac` has none).
pub fn open_indexed(src: Box<dyn ByteSource>, store: &dyn crate::pagefile::IndexStore) -> Result<Box<dyn BcSource>> {
    let mut head = [0u8; 4];
    if src.len() >= 4 {
        src.read_at(0, &mut head)?;
    }
    if is_bacpac(&head) {
        Ok(Box::new(crate::bacpac::BacpacSource::open(src)?))
    } else {
        Ok(Box::new(crate::bak::BakSource::open_indexed(src, store)?))
    }
}
