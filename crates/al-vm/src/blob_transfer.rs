//! The Blob value a record field or a Variant takes when a Blob moves into it: the one owner of "copy, empty or keep the buffer"
//! for an assignment, a builtin's write-back, `FieldRef.Value :=`, `FieldRef.Validate`, `RecordRef.GetTable` / `SetTable`,
//! `TransferFields` and an assignment to a Variant.

use crate::Session;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::native::Signal;
use al_builtins::stream::{copy_for_assignment, new_blob};
use al_builtins::value::{RecordState, Value};
use std::cell::RefCell;
use std::rc::Rc;

/// How a Blob reaches its target.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlobMove {
    /// `Target.Field := Source`.
    Assign,
    /// A builtin's write-back of its by-reference receiver or argument (`Session::write_native_back`: Clear, CreateOutStream).
    WriteBack,
    /// `FieldRef.Value := Source`.
    FieldRefValue,
    /// `FieldRef.Validate(Source)`.
    FieldRefValidate,
    /// `RecordRef.GetTable(Rec)`.
    GetTable,
    /// `RecordRef.SetTable(Rec)`, one argument.
    SetTable,
    /// `Rec.TransferFields(Source)`; `plain` is the call with one argument.
    TransferFields { plain: bool },
    /// `V := Source` with a local Variant `V`.
    Variant,
}

impl BlobMove {
    fn text(self) -> &'static str {
        match self {
            BlobMove::Assign => "an assignment",
            BlobMove::WriteBack => "a write-back",
            BlobMove::FieldRefValue => "FieldRef.Value",
            BlobMove::FieldRefValidate => "FieldRef.Validate",
            BlobMove::GetTable => "RecordRef.GetTable",
            BlobMove::SetTable => "RecordRef.SetTable",
            BlobMove::TransferFields { .. } => "TransferFields",
            BlobMove::Variant => "an assignment to a Variant",
        }
    }
}

/// The value the target holds after `v` moved into it by `how`; `cur` is what the target field holds now. A value that is no
/// Blob, and the target's own buffer written back, stay as they are.
///
/// `Assign` and `WriteBack` are pinned. A Blob of a record read from the database without CalcFields holds none of its content:
/// assigned to another field it gives an empty Blob (record/flowfields/BlobCalcFieldsLength.al:404, 'target after assign'
/// has=No|len=0, and the Modify after it stores nothing). A temporary row read back (Get, FindFirst, FindSet) without CalcFields
/// gives the same empty copy (record/temporary/TemporaryBlobAutoCalc.al:174-310, al-runner issue 6434). Any other Blob that
/// another holder still has (the source field, a Variant) is copied: the field gets a value of its own, which a later Clear of
/// the source or a new write to it does not change, nor the reverse (record/lt-blob-field-assign/BlobFieldAssignCopy.al:73-176,
/// al-runner issue 7128). A write-back keeps the buffer.
///
/// `TransferFields` of a Blob of a row read without CalcFields, and not written since, moves an empty one: the target has no
/// value, and inserting it stores none (record/crud/RcBTransfer.al:486-493, Transfer_Blob_Field_BQ `withoutCalcFields`).
///
/// Every other copy runs through assumption `blob-transfer-copies` (corpus PR 1369, recordref-fieldref/fieldref/
/// FieldRefValueBlobCopy.al; the lines are in the entry), for the cells [`measured`] names. `measured_field` says whether the
/// fields the Blob moves between are of the measured kind ([`Session::blob_field_measured`]); it is asked only for a Blob in
/// such a cell. There, content in a field that is not measured is refused, and an empty Blob in one is stored as it comes.
/// A Blob outside those cells is stored as it comes, as before.
pub(crate) fn moved(
    cur: Option<&Value>,
    v: Value,
    how: BlobMove,
    measured_field: impl FnOnce() -> bool,
) -> Result<Value, Signal> {
    let Value::Blob(b) = &v else { return Ok(v) };
    if matches!(cur, Some(Value::Blob(c)) if std::sync::Arc::ptr_eq(c, b)) {
        return Ok(v);
    }
    let s = {
        let buf = b.borrow();
        Source {
            loaded: buf.loaded,
            row_copy: buf.db_row || buf.temp_row,
            written: buf.written,
            content: !buf.bytes.is_empty(),
            uncompressed: buf.uncompressed,
        }
    };
    let cell = match how {
        BlobMove::WriteBack => return Ok(v),
        BlobMove::Assign if s.row_copy && !s.loaded => return Ok(Value::Blob(new_blob())),
        BlobMove::Assign if std::sync::Arc::strong_count(b) > 1 => return Ok(assigned_copy(cur, b, &s)),
        BlobMove::Assign => return Ok(v),
        BlobMove::TransferFields { .. } if !s.loaded && !s.written => return Ok(Value::Blob(new_blob())),
        _ => measured(how, &s),
    };
    let Some(cell) = cell else { return Ok(v) };
    let content = s.content || cell == Cell::Unread;
    if !(measured_field() && !s.uncompressed) {
        if content {
            return Err(Signal::unsupported(format!(
                "a Blob with content moved by {} into or from a field that a table extension declares, a FlowField or a field with the Compressed property (whether it is copied is not pinned)",
                how.text()
            )));
        }
        return Ok(v);
    }
    assume(Assumption::BlobTransferCopies);
    Ok(Value::Blob(match cell {
        Cell::Copy => copy_for_assignment(b),
        Cell::Unread => new_blob(),
    }))
}

/// The copy `Target.Field := Source` stores when another holder has the source Blob `b` too (`copy_for_assignment`); `cur`
/// is what the target holds now. In the cell of assumption `blob-assign-counts-as-write` (al-runner issue 7961, the
/// reporter's repro: a temporary record's Blob after its Insert, assigned to a Blob without an unsaved write) the copy is
/// marked `BlobBuf::assigned_unsaved`: a loaded source with content and no unsaved write of its own, and a target Blob with
/// none. `Session::load_blobs` reads the mark and raises the flag. Every other copy is as before.
fn assigned_copy(cur: Option<&Value>, b: &al_builtins::stream::BlobRef, s: &Source) -> Value {
    let copy = copy_for_assignment(b);
    let target_unwritten = matches!(cur, Some(Value::Blob(c)) if !c.borrow().written);
    if target_unwritten && s.loaded && s.content && !s.written {
        copy.borrow_mut().assigned_unsaved = true;
    }
    Value::Blob(copy)
}

/// What [`moved`] reads of the source buffer.
struct Source {
    loaded: bool,
    /// The unloaded copy a read of a database row or of a temporary row handed out.
    row_copy: bool,
    written: bool,
    content: bool,
    uncompressed: bool,
}

/// What a measured cell of assumption `blob-transfer-copies` gives the target.
#[derive(PartialEq, Eq)]
enum Cell {
    /// A copy of the source.
    Copy,
    /// An empty Blob: the source was read with Get and no CalcFields.
    Unread,
}

/// The measured cell a move belongs to, if any (corpus PR 1369, FieldRefValueBlobCopy.al). The source is loaded (written in
/// memory, or read with CalcFields) and holds content: a copy, for every kind of move. `FieldRef.Value` alone is also measured
/// with an empty loaded source (:1807 S5, :1918 Life_EmptySource: a copy, the target is empty) and with a source read with Get
/// and no CalcFields from a temporary or a stored table (:587-660, :1518 A09, :1540 A10: an empty Blob).
fn measured(how: BlobMove, s: &Source) -> Option<Cell> {
    match how {
        BlobMove::FieldRefValue if s.loaded => Some(Cell::Copy),
        BlobMove::FieldRefValue if s.row_copy => Some(Cell::Unread),
        BlobMove::FieldRefValidate
        | BlobMove::GetTable
        | BlobMove::SetTable
        | BlobMove::TransferFields { plain: true }
        | BlobMove::Variant
            if s.loaded && s.content =>
        {
            Some(Cell::Copy)
        }
        _ => None,
    }
}

impl Session {
    /// The Blob field at `pos` of table object `table` is of the kind corpus PR 1369 measured (FieldRefValueBlobCopy.al:23-73,
    /// its three tables): the table itself declares it, it is a normal field, and it does not set `Compressed`.
    pub(crate) fn blob_field_measured(&self, table: usize, pos: usize) -> bool {
        let field = self.engine.program.objects[table]
            .table
            .as_ref()
            .and_then(|s| s.fields.get(pos));
        field.is_some_and(|f| {
            f.declared_in == Some(table as u32)
                && f.stored
                && !f.flow_filter
                && f.calc.is_none()
                && f.calc_refusal.is_none()
                && !f.properties.iter().any(|(k, _)| k.eq_ignore_ascii_case("Compressed"))
        })
    }

    /// Moves the Blobs among `fields`, which `RecordRef.GetTable` or `SetTable` (`how`) just took over from the other record
    /// of table object `table`, as [`moved`] says; `cur` is what the target held before.
    pub(crate) fn move_blobs(
        &self,
        table: usize,
        fields: &mut [Value],
        cur: Option<&[Value]>,
        how: BlobMove,
    ) -> Result<(), Signal> {
        for (pos, v) in fields.iter_mut().enumerate() {
            if let Value::Blob(_) = v {
                let old = cur.and_then(|c| c.get(pos));
                *v = moved(old, v.clone(), how, || self.blob_field_measured(table, pos))?;
            }
        }
        Ok(())
    }

    /// The record a RecordRef holds after `RecordRef.GetTable(src)`. From a Record (`from_record`) its Blobs are moved as
    /// [`moved`] says; from a RecordRef a Variant holds, the copy is as before.
    pub(crate) fn gettable_copy(
        &self,
        src: &Rc<RefCell<RecordState>>,
        from_record: bool,
    ) -> Result<RecordState, Signal> {
        let mut copy = al_builtins::record_into_recordref::recordref_copy(&src.borrow());
        if from_record {
            self.move_blobs(copy.table as usize, &mut copy.fields, None, BlobMove::GetTable)?;
        }
        Ok(copy)
    }
}
