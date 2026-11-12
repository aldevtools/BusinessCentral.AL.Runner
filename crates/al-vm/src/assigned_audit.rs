//! Assigned audit fields (al-runner issue 5207): what a `Modify`, `Rename` and `ModifyAll` write when AL code assigned
//! SystemCreatedAt, SystemCreatedBy, SystemModifiedAt or SystemModifiedBy (record/system-fields/AssignedAuditFields.al).
//!
//! Each rule enters only on its measured cell, every fact by equality: a Record variable (not a RecordRef), a table whose
//! write runs no trigger, event subscriber or global trigger (table 61800, AssignedAuditFields.al:17-27), and for a database
//! record a buffer that is the stored row as a read in the open transaction gave it (the `Seed` at :140-148: Insert, then
//! Get). Every other shape writes what the write wrote before these rules (`hostvalues.rs`).

use crate::Session;
use crate::globaltriggers::GlobalOp;
use crate::hostvalues::{
    SYSTEM_CREATED_AT, SYSTEM_CREATED_BY, SYSTEM_ID, SYSTEM_MODIFIED_AT, SYSTEM_MODIFIED_BY, SYSTEM_ROW_VERSION,
    changed, position_of,
};
use al_builtins::native::{Signal, unsupported};
use al_builtins::value::{RowOrigin, Value};

/// What a write knows beyond the fields, as it starts.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AuditWrite {
    /// The write runs through a RecordRef (`Session::write_through_recordref`).
    pub(crate) through_recordref: bool,
    /// The RunTrigger argument; a Rename passes true.
    pub(crate) run_trigger: bool,
    /// The buffer's SystemRowVersion.
    pub(crate) version: Option<i64>,
    /// Where the buffer's row version came from (`RecordState::row_origin`).
    pub(crate) origin: Option<RowOrigin>,
    /// The write runs no trigger, no event subscriber and no global trigger of its table (`Session::audit_write_plain`).
    pub(crate) plain: bool,
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

/// The positions of fields `ids`; `None` when the table lacks one.
fn positions<const N: usize>(t: &al_data::TableDef, ids: [u32; N]) -> Option<[usize; N]> {
    let mut out = [0; N];
    for (slot, id) in out.iter_mut().zip(ids) {
        *slot = position_of(t, id)?;
    }
    Some(out)
}

/// The buffer is the stored row as a Get or a find in the open write transaction gave it, with nothing written through it
/// since (`stale_buffer_origin::load_origin`), and it holds the stored row's SystemRowVersion and SystemId.
fn read_image(t: &al_data::TableDef, stored: &[Value], buffer: &[Value], w: &AuditWrite) -> bool {
    let (Some([v, id]), Some(version), Some(o)) = (positions(t, [SYSTEM_ROW_VERSION, SYSTEM_ID]), w.version, w.origin)
    else {
        return false;
    };
    let read = RowOrigin {
        read_in_write_tx: true,
        ..RowOrigin::new(true, version, o.commits)
    };
    o == read && matches!(&stored[v], Value::BigInteger(n) if *n == version) && same(&stored[id], &buffer[id])
}

/// A database `Modify(false)` that changes no field, after the test assigned exactly one field of the modified pair a
/// non-blank value and nothing else: the buffer keeps the assigned value and the row keeps the stored pair
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
    let entry = !w.through_recordref && !w.run_trigger && w.plain;
    if !entry || !read_image(t, stored, buffer, w) || changed(t, stored, buffer) {
        return false;
    }
    let created_same = same(&stored[ca], &buffer[ca]) && same(&stored[cb], &buffer[cb]);
    let differing: Vec<usize> = [ma, mb]
        .into_iter()
        .filter(|&i| !same(&stored[i], &buffer[i]))
        .collect();
    created_same && matches!(differing.as_slice(), [i] if non_blank(&buffer[*i]))
}

/// A temporary `Modify(false)` that changes a field, after the test set SystemCreatedAt blank and left every other system
/// field as stored: the row takes the blank SystemCreatedAt (AssignedAuditFields.al:638-666, `blank / assigned / assigned /
/// assigned` after Get).
pub(crate) fn temporary_modify_writes_created_at(
    t: &al_data::TableDef,
    stored: &[Value],
    buffer: &[Value],
    w: &AuditWrite,
) -> bool {
    let Some([id, ca, cb, ma, mb]) = positions(
        t,
        [
            SYSTEM_ID,
            SYSTEM_CREATED_AT,
            SYSTEM_CREATED_BY,
            SYSTEM_MODIFIED_AT,
            SYSTEM_MODIFIED_BY,
        ],
    ) else {
        return false;
    };
    let entry = !w.through_recordref && !w.run_trigger && w.plain;
    let others_same = [id, cb, ma, mb].into_iter().all(|i| same(&stored[i], &buffer[i]));
    let blank_created_at = !same(&stored[ca], &buffer[ca]) && matches!(crate::recmeta::is_blank(&buffer[ca]), Ok(true));
    entry && others_same && blank_created_at && changed(t, stored, buffer)
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

/// A database `Rename` after the test assigned both fields of the created pair non-blank values and nothing else: the row
/// takes the stored pair, the buffer keeps the assigned values (AssignedAuditFields.al:568-590). Only one of the two
/// assigned on the same entry is not measured and is refused. Every other buffer is written as it is.
pub(crate) fn rename_audit(
    t: &al_data::TableDef,
    stored: Option<&[Value]>,
    buffer: &[Value],
    w: &AuditWrite,
) -> Result<RenameAudit, Signal> {
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
        return Ok(RenameAudit::AsBuffer);
    };
    let entry = !w.through_recordref && w.plain && read_image(t, stored, buffer, w);
    if !entry || !same(&stored[ma], &buffer[ma]) || !same(&stored[mb], &buffer[mb]) {
        return Ok(RenameAudit::AsBuffer);
    }
    let differing: Vec<usize> = [ca, cb]
        .into_iter()
        .filter(|&i| !same(&stored[i], &buffer[i]))
        .collect();
    match differing.as_slice() {
        [_, _] if non_blank(&buffer[ca]) && non_blank(&buffer[cb]) => Ok(RenameAudit::StoredCreatedPair(vec![
            (ca, stored[ca].clone()),
            (cb, stored[cb].clone()),
        ])),
        [_] => unsupported("a Rename after only one of SystemCreatedAt and SystemCreatedBy was assigned (not pinned)"),
        _ => Ok(RenameAudit::AsBuffer),
    }
}

/// The facts of the measured `ModifyAll` cell.
pub(crate) struct ModifyAllShape {
    pub(crate) rows: usize,
    pub(crate) run_trigger: bool,
    pub(crate) temporary: bool,
    pub(crate) table_caption_is_name: bool,
    /// No filter and no MarkedOnly on the record (the test calls Reset first, :601).
    pub(crate) unfiltered: bool,
    /// `Session::audit_write_plain` for a Modify.
    pub(crate) plain: bool,
}

/// The error text of `ModifyAll` on an audit field: SystemCreatedAt, SystemCreatedBy and SystemModifiedAt cannot be changed
/// (AssignedAuditFields.al:594-632: one row, no filter, RunTrigger false, a database table with no trigger or subscriber;
/// code `DB:Ok`, which the caller sets). The text names the field and the table by their names, which are their captions
/// there. `None` for any other field. Every other shape on an audit field is not pinned and is refused.
pub(crate) fn modify_all_audit_error(
    t: &al_data::TableDef,
    pos: usize,
    s: &ModifyAllShape,
) -> Option<Result<String, Signal>> {
    let id = t.fields[pos].id;
    if !(SYSTEM_CREATED_AT..=SYSTEM_MODIFIED_BY).contains(&id) {
        return None;
    }
    let pinned = matches!(id, SYSTEM_CREATED_AT | SYSTEM_CREATED_BY | SYSTEM_MODIFIED_AT)
        && s.rows == 1
        && !s.run_trigger
        && !s.temporary
        && s.table_caption_is_name
        && s.unfiltered
        && s.plain;
    if !pinned {
        return Some(unsupported(format!(
            "ModifyAll on {} other than one unfiltered row of a database table without triggers, subscribers or RunTrigger \
             (not pinned)",
            t.fields[pos].name
        )));
    }
    Some(Ok(crate::recmeta::cannot_change_value_text(
        &t.fields[pos].name,
        &t.name,
    )))
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
        &self,
        t: &al_data::TableDef,
        rec: &crate::records::Rec,
        op: GlobalOp,
        run: bool,
    ) -> AuditWrite {
        let r = rec.borrow();
        AuditWrite {
            through_recordref: self.write_through_recordref,
            run_trigger: run,
            version: position_of(t, SYSTEM_ROW_VERSION).and_then(|i| match &r.fields[i] {
                Value::BigInteger(n) => Some(*n),
                _ => None,
            }),
            origin: r.row_origin,
            plain: self.audit_write_plain(t, op, r.temporary),
        }
    }

    /// The facts of a `Modify` of `rec` as it starts, and the system fields it writes from the stored row `stored`
    /// (`modify_kept`).
    pub(crate) fn modify_audit(
        &self,
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
        if rec.borrow().temporary {
            return Ok(RenameAudit::AsBuffer);
        }
        let w = self.audit_write(t, rec, GlobalOp::Rename, true);
        if w.through_recordref || !w.plain {
            return Ok(RenameAudit::AsBuffer);
        }
        let stored =
            al_data::Store::get(self.store_for(rec), cx, t, old_pk).map_err(|e| Signal::unsupported(format!("{e}")))?;
        rename_audit(t, stored.as_deref(), &rec.borrow().fields, &w)
    }
}
