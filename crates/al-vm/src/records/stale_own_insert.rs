//! The verdict on a stale buffer from the variable's own Insert, two Commits later, when the new transaction wrote
//! (record/crud/Y27StaleOwnInsertMore.al, al-runner issue 6924).

use super::*;

impl Session {
    /// The verdict for a stale buffer from the variable's own Insert (no Modify after it) made two Commits back, when the open
    /// transaction has one of the shapes record/crud/Y27StaleOwnInsertMore.al pins on real BC. Another variable wrote the row in
    /// the transaction the last Commit closed (`committed_newer`: the committed row is newer than the transaction before's).
    /// - The new transaction Modified the row (once or twice, with Modify or Modify(true) and a subscriber that inserts a log
    ///   row) and beside that did nothing, or inserted other rows of the table, or modified other rows of the table, or inserted
    ///   rows of other tables (a log): written, the stale buffer winning (:503 Y27B_2C_BWroteAndInsertsOtherRow_Accepted, :525
    ///   _BWroteAndModifiesOtherRow_Accepted, :609 _BWroteAndLogInserted_Accepted, :733 _BWroteWithOnModifySubscriberWritingLog_Accepted).
    /// - The new transaction did not write the row and only inserted rows of other tables (a log): refused with DB:RecordChanged
    ///   (:650 Y27B_2C_OnlyLogInserted_Refused). Other rows and log rows never count as touching the row.
    ///
    /// Needs no rollback, no lock of any kind and no write left out of `tx.writes`. `None` for any other mix (a Delete, a Rename,
    /// a ModifyAll, other rows of the table beside log rows, other rows with the row never written), which the caller refuses.
    pub(super) fn stale_own_insert_new_tx_shape(
        &self,
        t: &TableDef,
        company: &Option<String>,
        pk: &[Value],
        committed_newer: bool,
    ) -> Option<StaleVerdict> {
        use crate::state::TxWriteKind;
        let tx = &self.tx;
        if !committed_newer
            || tx.writes.is_empty()
            || tx.rolled_back
            || tx.lock_tx
            || tx.lock_unpinned.is_some()
            || tx.isolation_lock
            || !tx.locked_tables.is_empty()
            || !self.writes_all_noted()
        {
            return None;
        }
        let same_key = |k: &[Value]| {
            k.len() == pk.len()
                && k.iter()
                    .zip(pk)
                    .all(|(a, b)| al_builtins::ops::compare(a, b).is_ok_and(|o| o == std::cmp::Ordering::Equal))
        };
        let own_row = |w: &crate::state::TxWrite| w.table == t.object && w.company == *company && same_key(&w.pk);
        let same_table = |w: &crate::state::TxWrite| w.table == t.object && w.company == *company;
        let (mine, others): (Vec<_>, Vec<_>) = tx.writes.iter().partition(|w| own_row(w));
        if mine.iter().any(|w| w.kind != TxWriteKind::Modify) {
            return None;
        }
        let others_ok =
            |kind: TxWriteKind, in_table: bool| others.iter().all(|w| w.kind == kind && same_table(w) == in_table);
        let log_inserts = others_ok(TxWriteKind::Insert, false);
        if mine.is_empty() {
            return log_inserts.then_some(StaleVerdict::Refused);
        }
        (others.is_empty()
            || log_inserts
            || others_ok(TxWriteKind::Insert, true)
            || others_ok(TxWriteKind::Modify, true))
        .then_some(StaleVerdict::Written)
    }
}
