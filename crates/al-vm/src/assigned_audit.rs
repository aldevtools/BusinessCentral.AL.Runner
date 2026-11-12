//! Assigned audit fields (al-runner issue 5207): what a `Modify` and a `Rename` write when AL code assigned SystemCreatedAt,
//! SystemCreatedBy, SystemModifiedAt or SystemModifiedBy (record/system-fields/AssignedAuditFields.al).
//!
//! Three rules, one per measured cell. Each enters only when every fact of its cell holds, by equality; any other write does
//! what it did before these rules (`hostvalues.rs`, `recmeta.rs`). The facts all four share:
//! - a Record variable, not a RecordRef (:140, :643), of a table whose write runs no trigger, event subscriber or global
//!   trigger (table 61800, :17-27);
//! - the buffer is the row the variable itself inserted with `Insert()` and then read with `Get` (`OwnRead`: `Seed` at
//!   :140-148, the temporary record at :650-655), with no other record method through the variable in between but the ones
//!   the cell names, no Commit and no write to the record's store since the Insert, and no filter on the variable.
//!
//! `ModifyAll` on an audit field (:594-632) has no rule: two of its three observations run after the rollback of the
//! `asserterror` before them, over a row the variable did not insert, and no case measures that state apart from the first.

use crate::Session;
use crate::globaltriggers::GlobalOp;
use crate::hostvalues::{
    SYSTEM_CREATED_AT, SYSTEM_CREATED_BY, SYSTEM_ID, SYSTEM_MODIFIED_AT, SYSTEM_MODIFIED_BY, SYSTEM_ROW_VERSION,
    changed, position_of,
};
use al_builtins::native::Signal;
use al_builtins::record_op::RecordOp;
use al_builtins::value::{OwnRead, OwnReadAt, RowOrigin, Value};

/// What a write knows beyond the fields, as it starts.
#[derive(Clone, Debug)]
pub(crate) struct AuditWrite {
    /// The write runs through a RecordRef (`Session::through_recordref`).
    pub(crate) through_recordref: bool,
    /// The RunTrigger argument; a Rename passes true.
    pub(crate) run_trigger: bool,
    /// The write runs no trigger, no event subscriber and no global trigger of its table (`Session::audit_write_plain`).
    pub(crate) plain: bool,
    /// The variable's `OwnRead` as the write starts.
    pub(crate) own_read: Option<OwnReadAt>,
    /// The buffer's SystemRowVersion.
    pub(crate) version: Option<i64>,
    /// Where the buffer's row version came from (`RecordState::row_origin`).
    pub(crate) origin: Option<RowOrigin>,
    /// The session's commit count.
    pub(crate) commits: u64,
    /// The change counter of the record's store.
    pub(crate) store_changes: u64,
    /// The variable has no filter, no MarkedOnly and no other company.
    pub(crate) unfiltered: bool,
}

/// The record methods the variable ran between its Insert and the write, as a cell measured them.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Sequence {
    /// Insert, Get (`Seed`, :140-148).
    InsertGet,
    /// Insert, Clear, Get (:650-655).
    InsertClearGet,
}

/// What a `Rename` of a database record writes for the created pair.
#[derive(Debug)]
pub(crate) enum RenameAudit {
    /// The buffer under its new key, as before these rules.
    AsBuffer,
    /// The stored row's created pair (position, value) over the buffer's.
    StoredCreatedPair(Vec<(usize, Value)>),
}

impl RenameAudit {
    /// The row the Rename writes from the buffer `buffer` (its fields after the triggers and the stamps).
    pub(crate) fn row(self, buffer: &[Value]) -> Vec<Value> {
        let mut row = buffer.to_vec();
        if let RenameAudit::StoredCreatedPair(pair) = self {
            for (i, v) in pair {
                row[i] = v;
            }
        }
        row
    }
}

fn same(a: &Value, b: &Value) -> bool {
    al_builtins::ops::compare(a, b).ok() == Some(std::cmp::Ordering::Equal)
}

fn non_blank(v: &Value) -> bool {
    matches!(crate::recmeta::is_blank(v), Ok(false))
}

fn blank(v: &Value) -> bool {
    matches!(crate::recmeta::is_blank(v), Ok(true))
}

/// The positions of fields `ids`; `None` when the table lacks one.
fn positions<const N: usize>(t: &al_data::TableDef, ids: [u32; N]) -> Option<[usize; N]> {
    let mut out = [0; N];
    for (slot, id) in out.iter_mut().zip(ids) {
        *slot = position_of(t, id)?;
    }
    Some(out)
}

fn version_of(t: &al_data::TableDef, row: &[Value]) -> Option<i64> {
    match &row[position_of(t, SYSTEM_ROW_VERSION)?] {
        Value::BigInteger(n) => Some(*n),
        _ => None,
    }
}

/// The shared buffer facts (module doc): the variable inserted the row `stored` and read it as `sequence` says, and nothing
/// committed, wrote to the store or filtered the variable since.
fn own_row(t: &al_data::TableDef, stored: &[Value], w: &AuditWrite, sequence: Sequence) -> bool {
    let (Some(m), Some([id])) = (&w.own_read, positions(t, [SYSTEM_ID])) else {
        return false;
    };
    let ran = if m.cleared {
        Sequence::InsertClearGet
    } else {
        Sequence::InsertGet
    };
    m.got
        && ran == sequence
        && m.commits == w.commits
        && m.store_changes == w.store_changes
        && m.version == version_of(t, stored)
        && same(&m.system_id, &stored[id])
        && w.unfiltered
}

/// `own_row`, and the buffer still holds that row's SystemId and SystemRowVersion.
fn own_buffer(t: &al_data::TableDef, stored: &[Value], buffer: &[Value], w: &AuditWrite, sequence: Sequence) -> bool {
    let Some([id]) = positions(t, [SYSTEM_ID]) else {
        return false;
    };
    own_row(t, stored, w, sequence) && same(&stored[id], &buffer[id]) && w.version == version_of(t, stored)
}

/// The buffer of a database record got its row version from a Get of a row written in the open transaction
/// (`stale_buffer_origin::load_origin`), as `Seed` leaves it.
fn read_in_open_transaction(w: &AuditWrite) -> bool {
    w.version.is_some_and(|version| {
        w.origin
            == Some(RowOrigin {
                read_in_write_tx: true,
                ..RowOrigin::new(true, version, w.commits)
            })
    })
}

/// A database `Modify()` that changes no field, after `Seed` and the assignment of a non-blank value to exactly one field
/// of the modified pair: the buffer keeps the assigned value and the row keeps the stored pair
/// (AssignedAuditFields.al:271-295, `buffer after Modify` = assigned, `stored row` = same as before).
pub(crate) fn modify_keeps_assigned_modified(
    t: &al_data::TableDef,
    stored: &[Value],
    buffer: &[Value],
    w: &AuditWrite,
) -> bool {
    let Some([ca, cb, ma, mb]) = positions(
        t,
        [
            SYSTEM_CREATED_AT,
            SYSTEM_CREATED_BY,
            SYSTEM_MODIFIED_AT,
            SYSTEM_MODIFIED_BY,
        ],
    ) else {
        return false;
    };
    let assigned: Vec<usize> = [ca, cb, ma, mb]
        .into_iter()
        .filter(|&i| !same(&stored[i], &buffer[i]))
        .collect();
    !w.through_recordref
        && !w.run_trigger
        && w.plain
        && own_buffer(t, stored, buffer, w, Sequence::InsertGet)
        && read_in_open_transaction(w)
        && !changed(t, stored, buffer)
        && matches!(assigned.as_slice(), [i] if (*i == ma || *i == mb) && non_blank(&buffer[*i]))
}

/// A temporary `Modify()` that changes another field, after the record's Insert, Clear and Get and after SystemCreatedAt,
/// stored non-blank, was set blank: the row takes the blank SystemCreatedAt (AssignedAuditFields.al:638-666, `blank /
/// assigned / assigned / assigned` after Get).
pub(crate) fn temporary_modify_writes_created_at(
    t: &al_data::TableDef,
    stored: &[Value],
    buffer: &[Value],
    w: &AuditWrite,
) -> bool {
    let Some([ca, cb, ma, mb]) = positions(
        t,
        [
            SYSTEM_CREATED_AT,
            SYSTEM_CREATED_BY,
            SYSTEM_MODIFIED_AT,
            SYSTEM_MODIFIED_BY,
        ],
    ) else {
        return false;
    };
    let assigned: Vec<usize> = [ca, cb, ma, mb]
        .into_iter()
        .filter(|&i| !same(&stored[i], &buffer[i]))
        .collect();
    !w.through_recordref
        && !w.run_trigger
        && w.plain
        && own_buffer(t, stored, buffer, w, Sequence::InsertClearGet)
        && changed(t, stored, buffer)
        && assigned == [ca]
        && non_blank(&stored[ca])
        && blank(&buffer[ca])
}

/// The system fields a `Modify` writes from the stored row `stored`, whatever the buffer holds: SystemId and the created
/// pair (record/TestRecordSystemId.al:176-215, :285-330; AssignedAuditFields.al:243-267, :301-325). On the temporary cell of
/// `temporary_modify_writes_created_at` the row takes the buffer's SystemCreatedAt.
pub(crate) fn modify_kept(
    t: &al_data::TableDef,
    stored: Option<&[Value]>,
    buffer_created_at: bool,
) -> Vec<(usize, Value)> {
    let Some(old) = stored else {
        return Vec::new();
    };
    [SYSTEM_ID, SYSTEM_CREATED_AT, SYSTEM_CREATED_BY]
        .into_iter()
        .filter(|&id| !(buffer_created_at && id == SYSTEM_CREATED_AT))
        .filter_map(|id| position_of(t, id))
        .map(|i| (i, old[i].clone()))
        .collect()
}

/// A database `Rename` that changes nothing but the key, after `Seed` and the assignment of non-blank values to both fields
/// of the created pair and to no other audit field: the row takes the stored pair, the buffer keeps the assigned values
/// (AssignedAuditFields.al:568-590). Every other buffer is written as it is.
pub(crate) fn rename_audit(
    t: &al_data::TableDef,
    stored: Option<&[Value]>,
    buffer: &[Value],
    w: &AuditWrite,
) -> RenameAudit {
    let (Some(stored), Some([ca, cb, ma, mb])) = (
        stored,
        positions(
            t,
            [
                SYSTEM_CREATED_AT,
                SYSTEM_CREATED_BY,
                SYSTEM_MODIFIED_AT,
                SYSTEM_MODIFIED_BY,
            ],
        ),
    ) else {
        return RenameAudit::AsBuffer;
    };
    let assigned: Vec<usize> = [ca, cb, ma, mb]
        .into_iter()
        .filter(|&i| !same(&stored[i], &buffer[i]))
        .collect();
    let measured = !w.through_recordref
        && w.plain
        && own_buffer(t, stored, buffer, w, Sequence::InsertGet)
        && read_in_open_transaction(w)
        && !changed(t, stored, buffer)
        && assigned == [ca, cb]
        && non_blank(&buffer[ca])
        && non_blank(&buffer[cb]);
    if measured {
        RenameAudit::StoredCreatedPair(vec![(ca, stored[ca].clone()), (cb, stored[cb].clone())])
    } else {
        RenameAudit::AsBuffer
    }
}

impl Session {
    /// Whether a write `op` (Modify or Rename) of table `t` runs no AL code of its own: the table and its extensions declare no
    /// trigger of it, no subscriber of its table events is declared, and (for a database record) the global trigger of it is
    /// known to be off. The measured table 61800 declares none (AssignedAuditFields.al:17-27).
    pub(crate) fn audit_write_plain(&self, t: &al_data::TableDef, op: GlobalOp, temporary: bool) -> bool {
        let (trigger, ext, events): (&str, [&str; 3], [&str; 2]) = match op {
            GlobalOp::Modify => (
                "OnModify",
                ["OnBeforeModify", "OnModify", "OnAfterModify"],
                ["onbeforemodifyevent", "onaftermodifyevent"],
            ),
            GlobalOp::Rename => (
                "OnRename",
                ["OnBeforeRename", "OnRename", "OnAfterRename"],
                ["onbeforerenameevent", "onafterrenameevent"],
            ),
            GlobalOp::Insert | GlobalOp::Delete => return false,
        };
        self.table_trigger(t, trigger).is_none()
            && ext.iter().all(|n| self.ext_triggers(t, n).is_empty())
            && events
                .iter()
                .all(|e| !self.engine.events.is_subscribed(t.object as usize, e))
            && (temporary || self.global_known_off(t, op))
    }

    /// The facts of a write of `rec` as it starts.
    pub(crate) fn audit_write(
        &mut self,
        t: &al_data::TableDef,
        rec: &crate::records::Rec,
        op: GlobalOp,
        run: bool,
    ) -> AuditWrite {
        let store_changes = self.store_for(rec).change_counter();
        let r = rec.borrow();
        AuditWrite {
            through_recordref: self.through_recordref,
            run_trigger: run,
            plain: self.audit_write_plain(t, op, r.temporary),
            own_read: r.own_read.0.clone(),
            version: version_of(t, &r.fields),
            origin: r.row_origin,
            commits: self.commits,
            store_changes,
            unfiltered: unfiltered(&r),
        }
    }

    /// The facts of a `Modify` of `rec` as it starts, and the system fields it writes from the stored row `stored`
    /// (`modify_kept`).
    pub(crate) fn modify_audit(
        &mut self,
        t: &al_data::TableDef,
        rec: &crate::records::Rec,
        stored: Option<&[Value]>,
        run: bool,
    ) -> (AuditWrite, Vec<(usize, Value)>) {
        let w = self.audit_write(t, rec, GlobalOp::Modify, run);
        let temporary_cell = rec.borrow().temporary
            && stored.is_some_and(|s| temporary_modify_writes_created_at(t, s, &rec.borrow().fields, &w));
        (w, modify_kept(t, stored, temporary_cell))
    }

    /// The `RenameAudit` of a database `Rename` of `rec`, read before its triggers, with the stored row under `old_pk`.
    pub(crate) fn rename_audit(
        &mut self,
        cx: &al_data::AccessContext,
        t: &al_data::TableDef,
        rec: &crate::records::Rec,
        old_pk: &[Value],
    ) -> Result<RenameAudit, Signal> {
        if rec.borrow().temporary || rec.borrow().own_read.0.is_none() {
            return Ok(RenameAudit::AsBuffer);
        }
        let w = self.audit_write(t, rec, GlobalOp::Rename, true);
        let stored =
            al_data::Store::get(self.store_for(rec), cx, t, old_pk).map_err(|e| Signal::unsupported(format!("{e}")))?;
        Ok(rename_audit(t, stored.as_deref(), &rec.borrow().fields, &w))
    }

    /// Follows `OwnRead` of `rec` as record method `method` starts. Only the methods the measured cells name keep it: a Get
    /// after the Insert, a Clear between the two (:653), and the write that reads it (Modify, Rename). Returns the store's change counter when `method` is an `Insert()` AL code called on a Record
    /// variable, for `own_read_after`.
    pub(crate) fn own_read_before(
        &mut self,
        method: RecordOp,
        rec: &crate::records::Rec,
        args: &[Value],
    ) -> Option<u64> {
        if method == RecordOp::Insert {
            rec.borrow_mut().own_read = OwnRead(None);
            let plain = self.tx.insert_from_al
                && !self.through_recordref
                && matches!(args.first(), None | Some(Value::Boolean(false)));
            return plain.then(|| self.store_for(rec).change_counter());
        }
        let mut r = rec.borrow_mut();
        let Some(m) = r.own_read.0.as_mut() else {
            return None;
        };
        let keeps = match method {
            RecordOp::Get => !m.got,
            RecordOp::Clear => !m.got && !std::mem::replace(&mut m.cleared, true),
            RecordOp::Modify | RecordOp::Rename => m.got,
            _ => false,
        };
        if !keeps || self.through_recordref {
            r.own_read = OwnRead(None);
        }
        None
    }

    /// Follows `OwnRead` of `rec` after record method `method` ended: an `Insert()` that wrote its row starts it
    /// (`inserted` from `own_read_before`), a Get that loaded nothing and a Modify or Rename end it.
    pub(crate) fn own_read_after(
        &mut self,
        method: RecordOp,
        rec: &crate::records::Rec,
        inserted: Option<u64>,
        ok: bool,
    ) {
        match method {
            RecordOp::Insert => {
                let Some(before) = inserted else { return };
                let store_changes = self.store_for(rec).change_counter();
                let Ok(t) = self.table(rec) else { return };
                let mut r = rec.borrow_mut();
                if let (true, Some(id)) = (ok && store_changes != before, position_of(&t, SYSTEM_ID)) {
                    r.own_read = OwnRead(Some(OwnReadAt {
                        got: false,
                        cleared: false,
                        system_id: r.fields[id].clone(),
                        version: version_of(&t, &r.fields),
                        commits: self.commits,
                        store_changes,
                    }));
                }
            }
            RecordOp::Get => {
                let mut r = rec.borrow_mut();
                if r.own_read.0.as_ref().is_some_and(|m| !m.got) {
                    r.own_read = OwnRead(None);
                }
            }
            RecordOp::Modify | RecordOp::Rename if rec.borrow().own_read.0.is_some() => {
                rec.borrow_mut().own_read = OwnRead(None);
            }
            _ => {}
        }
    }

    /// `OwnRead` of `rec` after a load of `row` into it: the variable's Get of the row it inserted, with no filter on the
    /// variable and no Commit or store write since the Insert, is the read the cells measure; any other load ends it.
    pub(crate) fn own_read_after_load(
        &mut self,
        t: &al_data::TableDef,
        rec: &crate::records::Rec,
        row: &[Value],
        by_get: bool,
    ) -> OwnRead {
        let Some(mut m) = rec.borrow_mut().own_read.0.take() else {
            return OwnRead(None);
        };
        let store_changes = self.store_for(rec).change_counter();
        let read = by_get
            && !self.through_recordref
            && !m.got
            && m.commits == self.commits
            && m.store_changes == store_changes
            && m.version == version_of(t, row)
            && position_of(t, SYSTEM_ID).is_some_and(|id| same(&m.system_id, &row[id]))
            && unfiltered(&rec.borrow());
        m.got = true;
        OwnRead(read.then_some(m))
    }
}

fn unfiltered(r: &al_builtins::value::RecordState) -> bool {
    r.filters.is_empty() && !r.marked_only && r.company.is_none()
}
