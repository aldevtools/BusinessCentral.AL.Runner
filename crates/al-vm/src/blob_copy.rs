//! The copy of a stored row's Blob that a read hands out.

use crate::Session;
use al_builtins::stream::{BlobRef, copy_blob_db_row_of};
use al_data::TableDef;

impl Session {
    /// The unloaded copy of a stored row's Blob field `pos`, marked when the field says `Compressed = false`: its stored size is then the
    /// content's (record/flowfields/BlobStoredSizeSign.al StoredSize_*_Result).
    pub(crate) fn copy_blob_db_row(&self, t: &TableDef, pos: usize, b: &BlobRef) -> BlobRef {
        let uncompressed = self
            .field_property(t, pos, "Compressed")
            .is_some_and(|v| v.trim().eq_ignore_ascii_case("false"));
        copy_blob_db_row_of(b, uncompressed)
    }
}
