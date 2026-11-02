//! Notes of a record's writes for `stale_verdict`.

use super::*;

impl Session {
    /// Records an Insert of `rec` for `stale_verdict`, keyed as the SQL row is (`sql_key`); a temporary table's is not
    /// recorded, so its key is not worked out.
    pub(super) fn note_insert(&mut self, rec: &Rec, t: &TableDef, before: u64) {
        if !rec.borrow().temporary {
            let pk = crate::sqltime::rounded_key(&t.primary_key(&rec.borrow().fields));
            self.note_write(rec, t, crate::state::TxWriteKind::Insert, &pk, before);
        }
    }
}
