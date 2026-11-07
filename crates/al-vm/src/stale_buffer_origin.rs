//! Stale buffers by where their row version came from, and what the transaction after the Commit did beside inserting another
//! row of the table (corpus record/crud/Y76StaleBuffers.al).

use crate::Session;
use crate::records::StaleVerdict;
use crate::state::TxWriteKind;
use al_builtins::native::Signal;
use al_builtins::value::{RowOrigin, Value};
use al_data::TableDef;

type Pk = [Value];

fn same_key(a: &Pk, b: &Pk) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(x, y)| al_builtins::ops::compare(x, y).is_ok_and(|o| o == std::cmp::Ordering::Equal))
}

impl Session {
    /// The origin of a row version a load just read. `how` is [Get, GetBySystemId, a pinned Find, an unpinned Find].
    /// A version written in the open transaction (above the last Commit's) read by Get or GetBySystemId follows the rule of
    /// the buffer that wrote it: not pinned, the reporter's real-BC repro of al-runner issue 5913 (assumption
    /// stale-get-buffer-in-write-transaction, marked in `stale_verdict` when it decides). A Get after the Commit that
    /// followed the write keeps a Get origin (record/TestRecordStaleBufferWrite.al:604-660).
    pub(crate) fn load_origin(&self, version: Option<&Value>, how: [bool; 4]) -> Option<RowOrigin> {
        let [via_get, via_sysid, via_find, unpinned_read] = how;
        match version {
            Some(Value::BigInteger(v)) if (via_get || via_sysid || via_find) && *v > self.tx.committed_row_version => {
                Some(RowOrigin {
                    read_in_write_tx: true,
                    unpinned_read,
                    ..RowOrigin::new(true, *v, self.commits)
                })
            }
            Some(Value::BigInteger(v)) if via_get || via_find => Some(RowOrigin {
                unpinned_read,
                ..RowOrigin::new(false, *v, self.commits)
            }),
            _ => None,
        }
    }

    /// The refusal of a stale write of a buffer another form of Find read (Find('='), '-', '+', '<', '>'), or that got TransferFields
    /// out of such a record: only FindFirst, FindLast and FindSet are pinned (needs probe).
    pub(crate) fn stale_unpinned_find(t: &TableDef) -> Signal {
        Signal::Unsupported(format!(
            "a stale {} buffer read by Find('=', '-', '+', '<', '>') or TransferFields out of such a record (only FindFirst, FindLast and FindSet are pinned)",
            t.name
        ))
    }

    /// A buffer that only got TransferFields from a record read by Get or Find (Y76StaleBuffers.al
    /// Y76_Part1_Copy_TransferFromRecord, `transfer_from_record/*`): written after a LockTable and Get of the row and nothing else the
    /// new transaction did to it; every write to the row (changed or not, a Delete after it) is refused. Writes beside, other Commit
    /// counts and a rollback are not pinned.
    pub(crate) fn stale_lock_only_verdict(
        &self,
        t: &TableDef,
        company: &Option<String>,
        pk: &Pk,
        buffer_commits: u64,
        touched: bool,
        contact: bool,
    ) -> Result<StaleVerdict, Signal> {
        let writes_row = self
            .tx
            .writes
            .iter()
            .any(|w| w.table == t.object && w.company == *company && same_key(&w.pk, pk));
        if buffer_commits + 1 != self.commits || !self.writes_all_noted() || self.tx.rolled_back {
            return Err(Signal::Unsupported(format!(
                "a stale {} buffer from TransferFields, several Commits later or after a rollback (not pinned)",
                t.name
            )));
        }
        if writes_row {
            return Ok(StaleVerdict::Refused);
        }
        if touched && self.tx.writes.is_empty() {
            return Ok(StaleVerdict::Written);
        }
        if !contact {
            return Ok(StaleVerdict::Refused);
        }
        Err(Signal::Unsupported(format!(
            "a stale {} buffer from TransferFields after the new transaction locked or wrote the table in a way no test pins",
            t.name
        )))
    }

    /// Whether the arm of an Insert of another row decides the stale write of a buffer with origin `o`: one or two Commits after the
    /// buffer's own write, and three when `stale_three_commits_beside_pinned`.
    pub(crate) fn stale_other_insert_in_reach(
        &self,
        t: &TableDef,
        company: &Option<String>,
        pk: &Pk,
        o: RowOrigin,
    ) -> bool {
        let ago = self.commits.wrapping_sub(o.commits);
        ago == 1 || ago == 2 || (ago == 3 && self.stale_three_commits_beside_pinned(t, company, pk, o.own_modified))
    }

    /// Three Commits after the buffer's own write: whether the new transaction inserted another row of `t` and wrote beside it one
    /// Delete of another row of `t`, one Rename of another row (a buffer with an own Modify only) or one Insert into another table,
    /// and no Modify (Y76_Part2_ThreeCommits_AndOneCommit, `own_modify_3_*` and `own_insert_3_*`). A Modify of another row beside
    /// it, with an own Modify, is not pinned at three Commits and stays refused (stale_buffer_6028.rs).
    pub(crate) fn stale_three_commits_beside_pinned(
        &self,
        t: &TableDef,
        company: &Option<String>,
        pk: &Pk,
        own_modified: bool,
    ) -> bool {
        self.stale_other_insert_pinned(t, company, pk, 3, own_modified)
            && self.tx.writes.iter().all(|w| w.kind != TxWriteKind::Modify)
            && self.tx.writes.iter().any(|w| {
                matches!(w.kind, TxWriteKind::Delete | TxWriteKind::Rename) && w.table == t.object
                    || w.kind == TxWriteKind::Insert && w.table != t.object
            })
    }

    /// Whether what the open transaction wrote beside the Insert of another row of `t` is pinned for a stale buffer of the
    /// variable's own Insert (or its Modify after it, `own_modified`) made `ago` Commits back. Pinned: Inserts and Modifies of other
    /// rows of the table alone (SbStaleOwnModify.al:666-745, :2484-2580), and one other write beside them (Y76StaleBuffers.al
    /// Y76_Part2_OwnModify_TwoCommits, _OwnInsertOnly_TwoCommits, _ThreeCommits_AndOneCommit): a Delete of another row at 1, 2 and 3
    /// Commits, a Rename of another row at 1 and 2 (3 with an own Modify), an Insert into another table at 2 and 3 (1 with an own
    /// Modify).
    pub(crate) fn stale_other_insert_pinned(
        &self,
        t: &TableDef,
        company: &Option<String>,
        pk: &Pk,
        ago: u64,
        own_modified: bool,
    ) -> bool {
        if !self.writes_all_noted() || !(1..=3).contains(&ago) {
            return false;
        }
        let (mut delete, mut rename, mut elsewhere) = (false, false, false);
        for w in &self.tx.writes {
            let same_table = w.table == t.object && w.company == *company;
            if same_table && same_key(&w.pk, pk) {
                return false;
            }
            match w.kind {
                TxWriteKind::Insert | TxWriteKind::Modify if same_table => {}
                TxWriteKind::Delete if same_table => delete = true,
                TxWriteKind::Rename if same_table => rename = true,
                TxWriteKind::Insert if !same_table => elsewhere = true,
                _ => return false,
            }
        }
        match (delete, rename, elsewhere) {
            (false, false, false) | (true, false, false) => true,
            (false, true, false) => ago <= 2 || own_modified,
            (false, false, true) => ago >= 2 || own_modified,
            _ => false,
        }
    }
}
