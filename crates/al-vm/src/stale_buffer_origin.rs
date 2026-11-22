//! Stale buffers by where their row version came from (corpus record/crud/Y76StaleBuffers.al); the rule itself is
//! `records/stale_rule.rs`.

use crate::Session;
use crate::records::StaleVerdict;
use al_builtins::native::Signal;
use al_builtins::value::{RowOrigin, Value};
use al_data::TableDef;

use crate::records::same_key;

type Pk = [Value];
type Rec = std::rc::Rc<std::cell::RefCell<al_builtins::value::RecordState>>;

/// The origin of the version the variable's own Modify gave a buffer whose origin `o` was its own Insert or Get in the
/// same transaction: its own write (record/crud/SbStaleOwnModify.al:132-264, :410-448).
pub(crate) fn own_modify_origin(o: RowOrigin, now: i64) -> RowOrigin {
    RowOrigin {
        own_write: true,
        version: now,
        read_in_write_tx: false,
        own_modified: true,
        ..o
    }
}

impl Session {
    /// Gives `rec` the origin of the version `v` the variable's own Insert into `t` gave it, and its version entry
    /// (`version_entry.rs`).
    pub(crate) fn set_own_insert_origin(&self, t: &TableDef, rec: &Rec, v: i64) {
        rec.borrow_mut().row_origin = Some(RowOrigin::new(true, v, self.commits));
        self.stamp_own_write(t, rec, al_builtins::value::VersionHow::OwnInsert, None);
    }

    /// The origin of a row version a load just read. `how` is [Get, GetBySystemId, a Find].
    /// A version written in the open transaction (above the last Commit's) read by Get, GetBySystemId or a Find follows the
    /// rule of the buffer that wrote it (record/crud/SbStaleOwnModify.al:467-640; record/crud/StaleRuleMatrix.al
    /// GetUncommitted_2C). A Get after the Commit that followed the write keeps a Get origin
    /// (record/TestRecordStaleBufferWrite.al:604-660).
    pub(crate) fn load_origin(&self, version: Option<&Value>, how: [bool; 3]) -> Option<RowOrigin> {
        let [via_get, via_sysid, via_find] = how;
        match version {
            Some(Value::BigInteger(v)) if (via_get || via_sysid || via_find) && *v > self.tx.committed_row_version => {
                Some(RowOrigin {
                    read_in_write_tx: true,
                    by_get: via_get,
                    ..RowOrigin::new(true, *v, self.commits)
                })
            }
            Some(Value::BigInteger(v)) if via_get || via_find => Some(RowOrigin {
                from_find: !via_get,
                by_get: via_get,
                ..RowOrigin::new(false, *v, self.commits)
            }),
            _ => None,
        }
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
            .open
            .writes
            .iter()
            .any(|w| w.table == t.object && w.company == *company && same_key(&w.pk, pk));
        if buffer_commits + 1 != self.commits || !self.writes_all_noted() || self.tx.rolled_back {
            return Err(Signal::unsupported(format!(
                "a stale {} buffer from TransferFields, several Commits later or after a rollback (not pinned)",
                t.name
            )));
        }
        if writes_row {
            return Ok(StaleVerdict::Refused);
        }
        if touched && self.tx.open.writes.is_empty() {
            return Ok(StaleVerdict::Written);
        }
        if !contact {
            return Ok(StaleVerdict::Refused);
        }
        Err(Signal::unsupported(format!(
            "a stale {} buffer from TransferFields after the new transaction locked or wrote the table in a way no test pins",
            t.name
        )))
    }
}
