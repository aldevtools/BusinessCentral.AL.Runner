//! Stale buffers by where their row version came from (corpus record/crud/Y76StaleBuffers.al); the rule itself is
//! `records/stale_rule.rs`.

use crate::Session;
use crate::records::StaleVerdict;
use al_builtins::native::Signal;
use al_builtins::value::{ReadBy, RowOrigin, Value};
use al_data::TableDef;

use crate::records::same_key;

type Pk = [Value];
type Rec = std::rc::Rc<std::cell::RefCell<al_builtins::value::RecordState>>;

impl Session {
    /// The origin after the variable's own Modify (false or true, the first or a later one) of `rec` was written.
    /// `before`: the SystemRowVersion field's position, its version before the Modify and the origin then. After its own
    /// Insert or Get in the same transaction the buffer's new version is its own write (record/crud/SbStaleOwnModify.al:
    /// 132-264, :410-448); an own Modify in a later transaction leaves the origin behind, as before.
    pub(crate) fn set_own_modify_origin(&self, rec: &Rec, before: Option<(usize, i64, Option<RowOrigin>)>) {
        let Some((i, was, Some(o))) = before else {
            return;
        };
        let now = if let Value::BigInteger(n) = &rec.borrow().fields[i] {
            *n
        } else {
            0
        };
        if o.version != was || now == was {
            return;
        }
        rec.borrow_mut().row_origin = (o.commits == self.commits).then_some(RowOrigin {
            own_write: true,
            version: now,
            read_in_write_tx: false,
            own_modified: true,
            ..o
        });
    }

    /// Gives `rec` the origin of the version `v` the variable's own Insert into `t` gave it, and its version entry
    /// (`version_entry.rs`).
    pub(crate) fn set_own_insert_origin(&self, t: &TableDef, rec: &Rec, v: i64) {
        rec.borrow_mut().row_origin = Some(RowOrigin::new(true, v, self.commits));
        self.stamp_own_write(t, rec, al_builtins::value::VersionHow::OwnInsert, None);
    }

    /// The origin of a row version a load into `rec` just read. `how` is [Get, GetBySystemId, a Find].
    /// A version written in the open transaction (above the last Commit's) read by Get, GetBySystemId or a Find follows the
    /// rule of the buffer that wrote it (record/crud/SbStaleOwnModify.al:467-640; record/crud/StaleRuleMatrix.al
    /// GetUncommitted_2C). A Get after the Commit that followed the write keeps a Get origin
    /// (record/TestRecordStaleBufferWrite.al:604-660).
    pub(crate) fn load_origin(&self, rec: &Rec, version: Option<&Value>, how: [bool; 3]) -> Option<RowOrigin> {
        let [via_get, via_sysid, via_find] = how;
        let read_by = if via_get {
            ReadBy::Get
        } else if via_find && self.is_al_record_call(al_builtins::record_op::RecordOp::FindFirst, rec) {
            ReadBy::AlFindFirst
        } else {
            ReadBy::OtherRead
        };
        match version {
            Some(Value::BigInteger(v)) if (via_get || via_sysid || via_find) && *v > self.tx.committed_row_version => {
                Some(RowOrigin {
                    read_in_write_tx: true,
                    read_by,
                    ..RowOrigin::new(true, *v, self.commits)
                })
            }
            Some(Value::BigInteger(v)) if via_get || via_find => Some(RowOrigin {
                from_find: !via_get,
                read_by,
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
