//! Stale buffers by where their row version came from (corpus record/crud/Y76StaleBuffers.al); the rule itself is
//! `records/stale_rule.rs`.

use crate::Session;
use crate::records::StaleVerdict;
use al_builtins::native::Signal;
use al_builtins::value::{RowOrigin, Value};
use al_data::TableDef;

use crate::records::same_key;

type Pk = [Value];

impl Session {
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
                    ..RowOrigin::new(true, *v, self.commits)
                })
            }
            Some(Value::BigInteger(v)) if via_get || via_find => Some(RowOrigin {
                from_find: !via_get,
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
        if touched && self.tx.writes.is_empty() {
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
