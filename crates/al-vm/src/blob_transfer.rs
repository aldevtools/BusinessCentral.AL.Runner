//! The Blob value a record field or a Variant takes when a Blob moves into it: the one owner of "copy, empty or keep the buffer"
//! for an assignment, a builtin's write-back, `FieldRef.Value :=`, `FieldRef.Validate`, `RecordRef.GetTable` / `SetTable`,
//! `TransferFields`, a Variant (an assignment, a by-value parameter), `Record.Copy` and a record passed by value.

use crate::Session;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::stream::{copy_blob_db_row_of, copy_for_assignment, new_blob};
use al_builtins::value::{RecordState, RtType, Value};
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
    /// `RecordRef.SetTable(Rec, true)` between two temporary records.
    SetTableShared,
    /// `Rec.TransferFields(Source, ...)`.
    TransferFields(TransferCall),
    /// `V := Source` with a Variant `V` that is a local variable or a parameter (`local`), or a global variable or an element
    /// of a local array ([`BlobMove::into_variant`]); also a Blob handed to a by-value Variant parameter of a procedure.
    Variant { local: bool },
    /// `Dst.Copy(Src [, ShareTable])` between two temporary records or two that are not.
    RecordCopy,
    /// A record handed by value to a parameter of a procedure, or assigned to a local Variant ([`by_value`]).
    RecordByValue,
}

/// The arguments of a `TransferFields` call.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransferCall {
    /// `TransferFields(Source)`.
    Plain,
    /// `TransferFields(Source, InitPrimaryKeyFields)` with either value, or `TransferFields(Source, true, true)`.
    Flags,
    /// Three arguments with other values.
    Other,
}

impl TransferCall {
    /// The call with arguments `args` (the source first).
    pub(crate) fn of(args: &[Value]) -> TransferCall {
        match args {
            [_] => TransferCall::Plain,
            [_, Value::Boolean(_)] | [_, Value::Boolean(true), Value::Boolean(true)] => TransferCall::Flags,
            _ => TransferCall::Other,
        }
    }
}

/// What takes the by-value copy of a record.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Taker {
    /// A Record parameter of a procedure that is not declared `temporary`.
    RecordParam,
    /// A Record parameter of a procedure declared `temporary`.
    TemporaryParam,
    /// A by-value Variant parameter of a procedure.
    VariantParam,
    /// A local Variant the record is assigned to.
    LocalVariant,
}

/// The by-value copy `copy` of a record: its Blobs are moved as [`moved`] says in the pairs of source and taker corpus PR 1372
/// measured (streams/BlobValueCopyShapes.al): a record that is not temporary into a Record parameter (:990 P5), and a
/// temporary record into a `temporary` parameter (:975 P5t), a Variant parameter (:847 P2) or a local Variant (:824 P1). In
/// every other pair the copy shares the Blobs with the source, as before.
pub(crate) fn by_value(copy: Value, source_temporary: bool, taker: Taker) -> Value {
    let measured = matches!(
        (source_temporary, taker),
        (false, Taker::RecordParam)
            | (true, Taker::TemporaryParam)
            | (true, Taker::VariantParam)
            | (true, Taker::LocalVariant)
    );
    if let (Value::Record(rc), true) = (&copy, measured) {
        move_blobs(&mut rc.borrow_mut().fields, None, BlobMove::RecordByValue);
    }
    copy
}

/// What an argument of a procedure call is for the Blob rule, read before the call binds it to its parameter.
#[derive(Clone, Copy)]
pub(crate) enum Argument {
    /// A record handed by value: whether it is temporary, and what takes it.
    Record(bool, Taker),
    /// A Blob handed to a by-value Variant parameter (corpus PR 1372, streams/BlobValueCopyShapes.al:590 V3).
    Blob,
    /// Anything else: bound as before.
    Other,
}

impl Argument {
    /// The argument `a` of a call of a procedure for its parameter of type `param`. `unmeasured` is true for a `var` parameter
    /// and for a call that runs an event subscriber: those are bound as before.
    pub(crate) fn of(a: &Value, unmeasured: bool, param: &RtType) -> Argument {
        let taker = match param {
            RtType::Record(_, false) => Taker::RecordParam,
            RtType::Record(_, true) => Taker::TemporaryParam,
            RtType::Variant => Taker::VariantParam,
            _ => return Argument::Other,
        };
        match a {
            _ if unmeasured => Argument::Other,
            Value::Record(rc) => Argument::Record(rc.borrow().temporary, taker),
            Value::Blob(_) if taker == Taker::VariantParam => Argument::Blob,
            _ => Argument::Other,
        }
    }

    /// The value `bound` the call made of the argument, with its Blobs moved as [`moved`] and [`by_value`] say.
    pub(crate) fn bind(self, bound: Value) -> Value {
        match self {
            Argument::Record(temporary, taker) => by_value(bound, temporary, taker),
            Argument::Blob => moved(None, bound, BlobMove::Variant { local: false }),
            Argument::Other => bound,
        }
    }
}

impl BlobMove {
    /// The move `V := Blob` is for the Variant at place `p`, if that place is measured: a local variable or a parameter
    /// (corpus PR 1369; PR 1372 BlobValueCopyShapes.al:607 V1 for a `var` parameter), a global variable of the running object
    /// (:672 GV) or an element of a local array (:652 V5).
    pub(crate) fn into_variant(p: &al_ir::Place) -> Option<BlobMove> {
        use al_ir::Place;
        match p {
            Place::Local(_) => Some(BlobMove::Variant { local: true }),
            Place::Global(_) => Some(BlobMove::Variant { local: false }),
            Place::Index(base, _) if matches!(**base, Place::Local(_)) => Some(BlobMove::Variant { local: false }),
            _ => None,
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
/// FieldRefValueBlobCopy.al, and PR 1372, streams/BlobValueCopyShapes.al; the lines are in the entry), for the cells [`measured`]
/// names. A Blob outside those cells is stored as it comes, as before.
pub(crate) fn moved(cur: Option<&Value>, v: Value, how: BlobMove) -> Value {
    let Value::Blob(b) = &v else { return v };
    if matches!(cur, Some(Value::Blob(c)) if std::sync::Arc::ptr_eq(c, b)) {
        return v;
    }
    let (s, uncompressed) = {
        let buf = b.borrow();
        let s = Source {
            loaded: buf.loaded,
            row_copy: buf.db_row || buf.temp_row,
            written: buf.written,
            content: !buf.bytes.is_empty(),
            unread_row: buf.db_row && !buf.loaded && !buf.written && !buf.first_write_truncates && !buf.bytes.is_empty(),
        };
        (s, buf.uncompressed)
    };
    let cell = match how {
        BlobMove::WriteBack => return v,
        BlobMove::Assign if s.row_copy && !s.loaded => return Value::Blob(new_blob()),
        BlobMove::Assign if std::sync::Arc::strong_count(b) > 1 => return Value::Blob(copy_for_assignment(b)),
        BlobMove::Assign => return v,
        BlobMove::TransferFields(_) if !s.loaded && !s.written => return Value::Blob(new_blob()),
        _ => measured(how, &s),
    };
    let Some(cell) = cell else { return v };
    assume(Assumption::BlobTransferCopies);
    Value::Blob(match cell {
        Cell::Copy => copy_for_assignment(b),
        Cell::Unread => new_blob(),
        Cell::UnreadRow => copy_blob_db_row_of(b, uncompressed),
    })
}

/// What [`moved`] reads of the source buffer.
struct Source {
    loaded: bool,
    /// The unloaded copy a read of a database row or of a temporary row handed out.
    row_copy: bool,
    written: bool,
    content: bool,
    /// The copy a read of a database row handed out, whose row holds content, not loaded and with no stream created on it.
    unread_row: bool,
}

/// What a measured cell of assumption `blob-transfer-copies` gives the target.
#[derive(PartialEq, Eq)]
enum Cell {
    /// A copy of the source.
    Copy,
    /// An empty Blob: the source was read with Get and no CalcFields.
    Unread,
    /// A not-loaded Blob of the row, of its own: HasValue is Yes and a stream reads nothing until CalcFields loads it.
    UnreadRow,
}

/// The measured cell a move belongs to, if any. P1 is corpus PR 1369 (FieldRefValueBlobCopy.al), P2 is corpus PR 1372
/// (streams/BlobValueCopyShapes.al).
///
/// A loaded source with content (written in memory, or read with CalcFields): a copy, for every kind of move. P1 for
/// FieldRef.Value, FieldRef.Validate, GetTable, SetTable, TransferFields(Source) and a local Variant; P2 for SetTable(Rec, true)
/// (:712 X6), TransferFields with two or three arguments (:741 TF2, :762 TF2f, :781 TF3), a Variant that is a by-value
/// parameter (:590 V3), an array element (:652 V5) or a global (:672 GV), Record.Copy (:906 X2, :929 X2s, :952 X2r) and a
/// record by value (:975 P5t, :990 P5, :824 P1, :847 P2).
///
/// A loaded source without content (never written, or emptied with Clear): a copy, which stays empty. P1 for FieldRef.Value
/// (:1807 S5, :1918 Life_EmptySource); P2 for GetTable (:169 E1, :188 E2, :321 E7, :342 E8), SetTable (:214 E3),
/// TransferFields(Source) (:235 E4, :252 E4b), FieldRef.Validate (:278 E5) and a local Variant (:297 E6).
///
/// A source read with Get and no CalcFields: FieldRef.Value gives an empty Blob, from a temporary or a stored table (P1
/// :587-660, :1518 A09, :1540 A10); so do FieldRef.Validate (P2 :468 N5) and a local Variant (P2 :491 N6) from a stored row
/// with content. SetTable gives the record a not-loaded Blob of its own (P2 :417 N3, :441 N4: the record's CalcFields does
/// not load the RecordRef's, and RecordRef.Modify after a write to the record keeps the stored value). GetTable takes its
/// own before it comes here (`recordref_copy`; P2 :368 N1, :393 N2 are main's result).
fn measured(how: BlobMove, s: &Source) -> Option<Cell> {
    let content = s.loaded && s.content;
    let empty = s.loaded && !s.content;
    match how {
        BlobMove::FieldRefValue if s.loaded => Some(Cell::Copy),
        BlobMove::FieldRefValue if s.row_copy => Some(Cell::Unread),
        BlobMove::FieldRefValidate
        | BlobMove::GetTable
        | BlobMove::SetTable
        | BlobMove::TransferFields(TransferCall::Plain)
        | BlobMove::Variant { local: true }
            if content || empty =>
        {
            Some(Cell::Copy)
        }
        BlobMove::SetTableShared
        | BlobMove::TransferFields(TransferCall::Flags)
        | BlobMove::Variant { local: false }
        | BlobMove::RecordCopy
        | BlobMove::RecordByValue
            if content =>
        {
            Some(Cell::Copy)
        }
        BlobMove::FieldRefValidate | BlobMove::Variant { local: true } if s.unread_row => Some(Cell::Unread),
        BlobMove::SetTable if s.unread_row => Some(Cell::UnreadRow),
        _ => None,
    }
}

/// Moves the Blobs among `fields`, which a record-wide move (`how`) just took over from the other record, as [`moved`]
/// says; `cur` is what the target held before.
pub(crate) fn move_blobs(fields: &mut [Value], cur: Option<&[Value]>, how: BlobMove) {
    for (pos, v) in fields.iter_mut().enumerate() {
        if let Value::Blob(_) = v {
            let old = cur.and_then(|c| c.get(pos));
            *v = moved(old, v.clone(), how);
        }
    }
}

impl Session {
    /// The record a RecordRef holds after `RecordRef.GetTable(src)`. From a Record (`from_record`) its Blobs are moved as
    /// [`moved`] says; from a RecordRef a Variant holds, the copy is as before.
    pub(crate) fn gettable_copy(&self, src: &Rc<RefCell<RecordState>>, from_record: bool) -> RecordState {
        let mut copy = al_builtins::record_into_recordref::recordref_copy(&src.borrow());
        if from_record {
            move_blobs(&mut copy.fields, None, BlobMove::GetTable);
        }
        copy
    }
}
