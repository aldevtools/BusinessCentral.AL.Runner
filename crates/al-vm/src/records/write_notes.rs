//! Notes of a record's writes for `stale_verdict`.

use super::*;
use crate::state::{TxWrite, TxWriteKind};

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

/// Whether the open transaction inserted the row of `pk` in `table` and `company` and wrote nothing to it since, as far as
/// `writes` tell: the last of them that is to that key or to the whole table (a Rename, ModifyAll, DeleteAll, Truncate or other
/// write whose rows are not told apart) is an Insert of the row. Whether every store write is in `writes` is the caller's
/// question (`Session::writes_all_noted`).
pub(crate) fn inserted_unwritten(writes: &[TxWrite], table: u32, company: &Option<String>, pk: &[Value]) -> bool {
    let last = writes.iter().rev().find(|w| {
        w.table == table
            && w.company == *company
            && (same_key(&w.pk, pk)
                || !matches!(w.kind, TxWriteKind::Insert | TxWriteKind::Modify | TxWriteKind::Delete))
    });
    last.is_some_and(|w| w.kind == TxWriteKind::Insert && same_key(&w.pk, pk))
}
