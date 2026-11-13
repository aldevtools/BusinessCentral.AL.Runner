//! The Blob value a record field or a Variant takes when a Blob moves into it: the one owner of "copy, empty or keep the buffer"
//! for an assignment, a builtin's write-back, `FieldRef.Value :=`, `FieldRef.Validate`, `RecordRef.GetTable` / `SetTable`,
//! `TransferFields`, a Variant (an assignment, a by-value parameter), `Record.Copy` and a record passed by value.

use crate::Session;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::native::Signal;
use al_builtins::record_op::RecordOp;
use al_builtins::stream::{WriteOrigin, copy_blob_db_row_of, copy_for_assignment, new_blob};
use al_builtins::value::{OwnVariable, RecordState, RtType, Value};
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
    /// `V := Source` with a Variant `V` that is a local variable or a parameter (`local`). Not `local`: a Blob field read
    /// that is stored in a global Variant or an element of a local array, or handed to a by-value Variant parameter
    /// ([`FieldRead`]).
    Variant { local: bool },
    /// `Dst.Copy(Src [, ShareTable])` between two temporary records or two that are not.
    RecordCopy,
    /// A record handed by value to a parameter of a procedure, or assigned to a local Variant ([`Session::blobs_by_value`]).
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

/// What a move knows of the record whose field the source Blob is.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Holder {
    /// A declared Record variable's own state, never inserted, modified or read (`OwnVariable`): the source record of the
    /// cells [`measured`] takes from corpus PR 1372.
    OwnVariable,
    /// Any other record, or one the move does not ask about.
    Other,
}

impl Holder {
    /// What record state `r` is as the source of a move.
    pub(crate) fn of(r: &RecordState) -> Holder {
        if r.own_variable.0 {
            Holder::OwnVariable
        } else {
            Holder::Other
        }
    }
}

/// Marks the value a declared variable starts with as that variable's own record (`OwnVariable`): a local variable of a
/// routine and a global of an object, as the sources of corpus PR 1372 are (streams/BlobValueCopyShapes.al:590 `Src`, :847
/// `GRow`). A record in an array is not marked (not measured).
pub(crate) fn new_variable(v: &Value) {
    if let Value::Record(rc) = v {
        rc.borrow_mut().own_variable = OwnVariable(true);
    }
}

/// Whether record method `method` leaves a record its variable's own (`OwnVariable`). Only what the measured sources ran
/// before their move does: `Clear` of the variable (corpus PR 1372, streams/BlobValueCopyShapes.al:847). Insert, Modify,
/// Init, every read and every other method end it.
pub(crate) fn keeps_own_variable(method: RecordOp) -> bool {
    method == RecordOp::Clear
}

/// Record `r` took its buffer from elsewhere (a record assignment, RecordRef.SetTable, Page.GetRecord): it is no longer its
/// variable's own (`OwnVariable`).
pub(crate) fn buffer_replaced(r: &mut RecordState) {
    r.own_variable = OwnVariable(false);
}

/// What an argument of a procedure call is for the Blob rule, read before the call binds it to its parameter.
#[derive(Clone, Copy)]
pub(crate) enum Argument {
    /// A record handed by value: whether it is temporary, what it is as a source, and what takes it.
    Record(bool, Holder, Taker),
    /// Anything else: bound as before.
    Other,
}

/// A read of a record field as the whole right-hand side of an assignment or as a whole call argument: the place a Blob
/// that moves into a Variant that is not local must come from. Corpus PR 1372 (streams/BlobValueCopyShapes.al) read the
/// field of a local temporary record variable (`Src.Data`, :590 V3, :652 V5, :672 GV).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct FieldRead {
    /// The local slot of the record variable.
    pub(crate) record: u16,
    /// The position of the field.
    pub(crate) pos: u32,
}

impl FieldRead {
    /// The field read `e` is, if it is one of a local variable of a routine with `params` parameters. A parameter, a
    /// global record, an element of an array, and any other expression (a Variant that holds a Blob, a call) are none.
    pub(crate) fn of(e: &al_ir::Expr, params: usize) -> Option<FieldRead> {
        use al_ir::{Expr, Place};
        let e = match e {
            Expr::Convert(RtType::Variant, inner) => &**inner,
            e => e,
        };
        match e {
            Expr::Load(Place::Field(base, pos)) => match &**base {
                Place::Local(i) if *i >= params => Some(FieldRead {
                    record: u16::try_from(*i).ok()?,
                    pos: u32::try_from(*pos).ok()?,
                }),
                _ => None,
            },
            _ => None,
        }
    }

    /// Whether the assignment target `p` is a place corpus PR 1372 stored such a read in: a global variable of the running
    /// object (:672 GV) or an element of a one-dimensional local array (:652 V5). A local Variant takes any Blob
    /// (`Session::write`, as before).
    pub(crate) fn stored_in(p: &al_ir::Place) -> bool {
        use al_ir::Place;
        match p {
            Place::Global(_) => true,
            Place::Index(base, at) => matches!(**base, Place::Local(_)) && at.len() == 1,
            _ => false,
        }
    }

    /// Whether routine `callee`, called without an instance (`own_object`), takes argument `arg` as corpus PR 1372
    /// measured (:551 TakeVar, :590 V3): a by-value Variant parameter of a procedure of the calling object without
    /// attributes.
    pub(crate) fn handed_to(callee: &al_ir::Routine, own_object: bool, arg: usize) -> bool {
        own_object
            && !callee.is_trigger
            && callee.attributes.is_empty()
            && arg < callee.params
            && callee.by_var.get(arg).copied() == Some(false)
            && matches!(callee.locals[arg].ty, RtType::Variant)
    }
}

impl Argument {
    /// The argument `a` of a call for its parameter of type `param`. `measured_call` is true for a by-value parameter of a
    /// procedure without attributes that is called as a procedure (corpus PR 1372, streams/BlobValueCopyShapes.al:562
    /// FromVariant, :576 ByValueTemp, :582 ByValue); the arguments of a `var` parameter, a trigger, an event publisher or
    /// subscriber and any other procedure with an attribute are bound as before.
    pub(crate) fn of(a: &Value, measured_call: bool, param: &RtType) -> Argument {
        let taker = match param {
            RtType::Record(_, false) => Taker::RecordParam,
            RtType::Record(_, true) => Taker::TemporaryParam,
            RtType::Variant => Taker::VariantParam,
            _ => return Argument::Other,
        };
        match a {
            Value::Record(rc) if measured_call => {
                let r = rc.borrow();
                Argument::Record(r.temporary, Holder::of(&r), taker)
            }
            _ => Argument::Other,
        }
    }
}

impl BlobMove {
    /// The move `Dst.Copy(Src [, ShareTable])` between a source record that is temporary or not (`source_temporary`) and a
    /// target that is (`target_temporary`): measured between two temporary records or two that are not (corpus PR 1372,
    /// streams/BlobValueCopyShapes.al:906 X2, :929 X2s, :952 X2r). Between one of each the Blobs come as before (`None`).
    pub(crate) fn record_copy(source_temporary: bool, target_temporary: bool) -> Option<BlobMove> {
        (source_temporary == target_temporary).then_some(BlobMove::RecordCopy)
    }

    /// The move of `RecordRef.SetTable` with `arguments` arguments, the second one `share`: one argument (corpus PR 1369,
    /// FieldRefValueBlobCopy.al:941, :964) and ShareTable true (PR 1372, streams/BlobValueCopyShapes.al:712 X6, between two
    /// temporary records). The caller raises the Argument error of ShareTable true with a record that is not temporary,
    /// and refuses a temporary and a normal record mixed, before it asks here, so `share` means both are temporary. With
    /// ShareTable false the Blobs come as before (`None`).
    pub(crate) fn set_table(arguments: usize, share: bool) -> Option<BlobMove> {
        match arguments {
            1 => Some(BlobMove::SetTable),
            2 if share => Some(BlobMove::SetTableShared),
            _ => None,
        }
    }
}

/// The kind of Blob field a move reads or writes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum FieldKind {
    /// A normal field the table itself declares, without the `Compressed` property (corpus PR 1369, FieldRefValueBlobCopy.al:
    /// 23-73; PR 1372, streams/BlobValueCopyShapes.al:38-39 Data and Other, :41 Memo, :64 the second table).
    Plain,
    /// A normal field the table declares with `Compressed = false` (PR 1372 :40), or one a table extension declares without
    /// the property (:53).
    Variation,
    /// Any other field: a FlowField, a FlowFilter, another value of `Compressed`, a table extension field with the property.
    /// Not measured.
    Other,
}

impl FieldKind {
    /// The kind of Blob field `f` of table object `table` (`TableFacts::blob_fields` holds it).
    pub(crate) fn of(heads: &[al_ir::ObjectHead], table: usize, f: &al_ir::Field) -> FieldKind {
        if !(f.stored && !f.flow_filter && f.calc.is_none() && f.calc_refusal.is_none()) {
            return FieldKind::Other;
        }
        let own = f.declared_in == Some(table as u32);
        let extension = f
            .declared_in
            .is_some_and(|d| matches!(heads.get(d as usize), Some(o) if o.kind == al_ir::ObjKind::TableExtension));
        let uncompressed_declared = f
            .properties
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Compressed"))
            .map(|(_, v)| v.trim().eq_ignore_ascii_case("false"));
        match (own, extension, uncompressed_declared) {
            (true, false, None) => FieldKind::Plain,
            (true, false, Some(true)) | (false, true, None) => FieldKind::Variation,
            _ => FieldKind::Other,
        }
    }

    /// The kind of a move between a field of kind `self` and one of kind `other`: theirs when they are the same.
    pub(crate) fn with(self, other: FieldKind) -> FieldKind {
        if self == other { self } else { FieldKind::Other }
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
/// FieldRefValueBlobCopy.al, and PR 1372, streams/BlobValueCopyShapes.al; the lines are in the entry), for the cells
/// [`measured`] names. `holder` is the record the source Blob is a field of. `kind` says of which kind the fields are the Blob
/// moves between; it is asked only for a Blob in such a cell or in one [`refused_before`] names. A Blob outside the measured
/// cells is stored as it comes, as before, or refused where it was refused before.
pub(crate) fn moved(
    cur: Option<&Value>,
    v: Value,
    how: BlobMove,
    holder: Holder,
    kind: impl FnOnce() -> FieldKind,
) -> Result<Value, Signal> {
    let Value::Blob(b) = &v else { return Ok(v) };
    if matches!(cur, Some(Value::Blob(c)) if std::sync::Arc::ptr_eq(c, b)) {
        return Ok(v);
    }
    let (s, uncompressed) = {
        let buf = b.borrow();
        let row_copy = buf.db_row || buf.temp_row;
        let s = Source {
            loaded: buf.loaded,
            row_copy,
            written: buf.written,
            write_origin: buf.write_origin,
            content: !buf.bytes.is_empty(),
            unread_row: buf.db_row
                && !buf.loaded
                && !buf.written
                && !buf.first_write_truncates
                && !buf.bytes.is_empty(),
            in_memory: buf.loaded && !row_copy && !buf.inserted && !buf.persisted && !buf.refetch,
        };
        (s, buf.uncompressed)
    };
    match how {
        BlobMove::WriteBack => return Ok(v),
        BlobMove::Assign if s.row_copy && !s.loaded => return Ok(Value::Blob(new_blob())),
        BlobMove::Assign if std::sync::Arc::strong_count(b) > 1 => return Ok(assigned_copy(cur, b, &s)),
        BlobMove::Assign => return Ok(v),
        BlobMove::TransferFields(_) if !s.loaded && !s.written => return Ok(Value::Blob(new_blob())),
        _ => {}
    }
    let plain = measured(how, &s, holder);
    let refused = refused_before(how, &s);
    if plain.is_none() && !refused {
        return Ok(v);
    }
    // A buffer a read of a `Compressed = false` field handed out counts as a field of that kind.
    let kind = match (kind(), uncompressed) {
        (FieldKind::Plain, true) => FieldKind::Variation,
        (kind, _) => kind,
    };
    let cell = match kind {
        FieldKind::Plain => plain,
        FieldKind::Variation if first_probe_move(how) => plain,
        FieldKind::Variation => None,
        _ if refused => {
            return Err(Signal::unsupported(format!(
                "a Blob with content moved by {} into or from a FlowField or a field with a Compressed property other than Compressed = false in a table (whether it is copied is not pinned)",
                how.text()
            )));
        }
        _ => None,
    };
    let Some(cell) = cell else { return Ok(v) };
    assume(Assumption::BlobTransferCopies);
    Ok(Value::Blob(match cell {
        Cell::Copy => {
            let copy = copy_for_assignment(b);
            // The copy GetTable makes of a Blob its own variable wrote: SetTable(Rec, true) moves it on (:712).
            if how == BlobMove::GetTable && s.written_by(holder) {
                copy.borrow_mut().write_origin = WriteOrigin::GetTableCopy;
            }
            copy
        }
        Cell::Unread => new_blob(),
        Cell::UnreadRow => copy_blob_db_row_of(b, uncompressed),
    }))
}

impl BlobMove {
    /// The name of the move in the refusal of [`moved`], for the moves [`refused_before`] names.
    fn text(self) -> &'static str {
        match self {
            BlobMove::FieldRefValue => "FieldRef.Value",
            BlobMove::FieldRefValidate => "FieldRef.Validate",
            BlobMove::GetTable => "RecordRef.GetTable",
            BlobMove::SetTable | BlobMove::SetTableShared => "RecordRef.SetTable",
            BlobMove::TransferFields(_) => "TransferFields",
            BlobMove::Variant { .. } => "an assignment to a Variant",
            BlobMove::Assign => "an assignment",
            BlobMove::WriteBack => "a write-back",
            BlobMove::RecordCopy => "Record.Copy",
            BlobMove::RecordByValue => "a record passed by value",
        }
    }
}

/// The copy `Target.Field := Source` stores when another holder has the source Blob `b` too (`copy_for_assignment`); `cur`
/// is what the target holds now. The copy is marked `BlobBuf::assigned_unsaved` only in the shape of assumption
/// `blob-assign-counts-as-write` (al-runner issue 7961, the reporter's repro): the source is the Blob a temporary record's
/// Insert stored while it held content, with no stream created on it and no unsaved write since (`temp_inserted`; in the
/// repro a stream wrote the content), and the target is a loaded Blob without content and without an unsaved write.
/// `Session::load_blobs` reads the mark. Every other copy is as before.
fn assigned_copy(cur: Option<&Value>, b: &al_builtins::stream::BlobRef, s: &Source) -> Value {
    let copy = copy_for_assignment(b);
    let source_in_cell = matches!(
        (b.borrow().temp_inserted, s.loaded, s.content, s.written),
        (true, true, true, false)
    );
    let target_in_cell = matches!(cur, Some(Value::Blob(c))
        if matches!((c.borrow().loaded, c.borrow().bytes.is_empty(), c.borrow().written), (true, true, false)));
    if source_in_cell && target_in_cell {
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
    write_origin: WriteOrigin,
    content: bool,
    /// The copy a read of a database row handed out, whose row holds content, not loaded and with no stream created on it.
    unread_row: bool,
    /// Loaded, and not tied to a row: not the copy a read handed out, not what an Insert or a Modify stored, not the copy a
    /// record assignment made.
    in_memory: bool,
}

impl Source {
    /// Content a stream wrote on this buffer, in memory, in a record that is its variable's own (`holder`): written, and
    /// neither the buffer nor the record was stored or read.
    fn written_by(&self, holder: Holder) -> bool {
        self.in_memory
            && self.content
            && self.written
            && self.write_origin == WriteOrigin::Here
            && holder == Holder::OwnVariable
    }

    /// The copy GetTable made of such content ([`Source::written_by`]), in memory.
    fn gettable_copy_of_written(&self) -> bool {
        self.in_memory && self.content && self.written && self.write_origin == WriteOrigin::GetTableCopy
    }
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

/// The measured cell a move between [`FieldKind::Plain`] fields belongs to, if any; [`FieldKind::Variation`] fields take the
/// same cell in the moves [`first_probe_move`] names. P1 is corpus PR 1369 (FieldRefValueBlobCopy.al), P2 is corpus PR 1372
/// (streams/BlobValueCopyShapes.al).
///
/// A loaded source with content (written in memory, or read with CalcFields): a copy. P1 for FieldRef.Value,
/// FieldRef.Validate, GetTable, SetTable, TransferFields(Source) and a local Variant.
///
/// A source a stream wrote in memory, in a record that is its variable's own ([`Source::written_by`]: a declared variable never
/// inserted, modified or read, written once or twice): a copy. P2 for TransferFields with two or three arguments (:741 TF2, :762
/// TF2f, :781 TF3), a field read stored in a Variant that is a by-value parameter (:590 V3), an array element (:652 V5) or a
/// global (:672 GV), Record.Copy (:906 X2, :929 X2s, :952 X2r) and a record by value (:975 P5t, :990 P5, :824 P1, :847 P2).
/// SetTable(Rec, true) takes the copy GetTable made of such a source (:712 X6). A buffer whose write came over with another
/// copy (a field assignment, FieldRef.Value, SetTable, a record assignment), and a record that was inserted, modified, read,
/// set back with Init or filled from another record before, are not measured: the buffer is kept, as before.
///
/// A loaded source without content, in memory (never written, or emptied with Clear): a copy, which stays empty. P1 for
/// FieldRef.Value, whatever the buffer was before (:1807 S5, :1918 Life_EmptySource); P2 for GetTable (:169 E1, :188 E2,
/// :321 E7, :342 E8), SetTable (:214 E3), TransferFields(Source) (:235 E4, :252 E4b), FieldRef.Validate (:278 E5) and a local
/// Variant (:297 E6).
///
/// A source read with Get and no CalcFields: FieldRef.Value gives an empty Blob, from a temporary or a stored table (P1
/// :587-660, :1518 A09, :1540 A10); so do FieldRef.Validate (P2 :468 N5) and a local Variant (P2 :491 N6) from a stored row
/// with content. SetTable gives the record a not-loaded Blob of its own (P2 :417 N3, :441 N4: the record's CalcFields does
/// not load the RecordRef's, and RecordRef.Modify after a write to the record keeps the stored value). GetTable takes its
/// own before it comes here (`recordref_copy`; P2 :368 N1, :393 N2 are the result as before).
fn measured(how: BlobMove, s: &Source, holder: Holder) -> Option<Cell> {
    let content = s.loaded && s.content;
    let empty = s.in_memory && !s.content;
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
        BlobMove::SetTableShared if s.gettable_copy_of_written() => Some(Cell::Copy),
        BlobMove::TransferFields(TransferCall::Flags)
        | BlobMove::Variant { local: false }
        | BlobMove::RecordCopy
        | BlobMove::RecordByValue
            if s.written_by(holder) =>
        {
            Some(Cell::Copy)
        }
        BlobMove::FieldRefValidate | BlobMove::Variant { local: true } if s.unread_row => Some(Cell::Unread),
        BlobMove::SetTable if s.unread_row => Some(Cell::UnreadRow),
        _ => None,
    }
}

/// The moves in which a [`FieldKind::Variation`] field gives what a plain field gives ([`measured`]). Corpus PR 1372
/// (streams/BlobValueCopyShapes.al) measured loaded content through GetTable, SetTable and TransferFields(Source) (:1174
/// Compressed = false, :1190 and :1206 a table extension field), and such a field as the source of FieldRef.Value and of a
/// local Variant (:1174, :1190). Such a field as the target of FieldRef.Value and FieldRef.Validate copies in every source
/// state, and a source read with Get only gives an empty target (corpus PR 1375, streams/BlobCopyStates.al, procedure Into,
/// :536-1002 at d96306a8). Its empty and not-loaded states in the other moves are taken to follow: these moves were refused
/// for these fields between al-runner PR 7936 and this rule, which cost tests of an outside reporter (al-runner issue 6924).
fn first_probe_move(how: BlobMove) -> bool {
    matches!(
        how,
        BlobMove::FieldRefValue
            | BlobMove::FieldRefValidate
            | BlobMove::GetTable
            | BlobMove::SetTable
            | BlobMove::TransferFields(TransferCall::Plain)
            | BlobMove::Variant { local: true }
    )
}

/// The moves of content that are refused for a field of [`FieldKind::Other`] (as since al-runner PR 7936).
fn refused_before(how: BlobMove, s: &Source) -> bool {
    match how {
        BlobMove::FieldRefValue => (s.loaded && s.content) || (!s.loaded && s.row_copy),
        BlobMove::FieldRefValidate
        | BlobMove::GetTable
        | BlobMove::SetTable
        | BlobMove::TransferFields(TransferCall::Plain)
        | BlobMove::Variant { local: true } => s.loaded && s.content,
        _ => false,
    }
}

impl Session {
    /// The kind of the Blob field at `pos` of table object `table`.
    pub(crate) fn blob_field_kind(&self, table: usize, pos: usize) -> FieldKind {
        let fields = &self.engine.table_facts[table].blob_fields;
        fields
            .iter()
            .find(|(p, _)| *p == pos)
            .map_or(FieldKind::Other, |(_, k)| *k)
    }

    /// Moves the Blobs among `fields`, which a record-wide move (`how`) just took over from the record `holder` of table
    /// object `table`, as [`moved`] says; `cur` is what the target held before.
    pub(crate) fn move_blobs(
        &self,
        table: usize,
        fields: &mut [Value],
        cur: Option<&[Value]>,
        how: BlobMove,
        holder: Holder,
    ) -> Result<(), Signal> {
        for &(pos, kind) in self.engine.table_facts[table].blob_fields.iter() {
            let Some(v) = fields.get_mut(pos) else { continue };
            if let Value::Blob(_) = v {
                let old = cur.and_then(|c| c.get(pos));
                *v = moved(old, v.clone(), how, holder, || kind)?;
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
        let (mut copy, holder) = {
            let src = src.borrow();
            (
                al_builtins::record_into_recordref::recordref_copy(&src),
                Holder::of(&src),
            )
        };
        if from_record {
            self.move_blobs(copy.table as usize, &mut copy.fields, None, BlobMove::GetTable, holder)?;
        }
        Ok(copy)
    }

    /// The by-value copy `copy` of a record: its Blobs are moved as [`moved`] says in the pairs of source and taker corpus
    /// PR 1372 measured (streams/BlobValueCopyShapes.al): a record that is not temporary into a Record parameter (:990 P5),
    /// and a temporary record into a `temporary` parameter (:975 P5t), a Variant parameter (:847 P2) or a local Variant
    /// (:824 P1). In every other pair the copy shares the Blobs with the source, as before. `holder` is the source record.
    pub(crate) fn blobs_by_value(
        &self,
        copy: Value,
        source_temporary: bool,
        holder: Holder,
        taker: Taker,
    ) -> Result<Value, Signal> {
        let measured = matches!(
            (source_temporary, taker),
            (false, Taker::RecordParam)
                | (true, Taker::TemporaryParam)
                | (true, Taker::VariantParam)
                | (true, Taker::LocalVariant)
        );
        if let (Value::Record(rc), true) = (&copy, measured) {
            let mut r = rc.borrow_mut();
            let table = r.table as usize;
            self.move_blobs(table, &mut r.fields, None, BlobMove::RecordByValue, holder)?;
        }
        Ok(copy)
    }

    /// The value `bound` a call made of its argument `argument`, with its Blobs moved as [`Session::blobs_by_value`] says.
    pub(crate) fn bind_blobs(&self, argument: Argument, bound: Value) -> Result<Value, Signal> {
        match argument {
            Argument::Record(temporary, holder, taker) => self.blobs_by_value(bound, temporary, holder, taker),
            Argument::Other => Ok(bound),
        }
    }

    /// The value a Variant that is not local takes when `v`, the value of field read `read` in the frame with locals
    /// `locals`, moves into it ([`BlobMove::Variant`]). A Blob of a temporary record (corpus PR 1372,
    /// streams/BlobValueCopyShapes.al:590, :652, :672: every source is temporary) moves as [`moved`] says, with the record
    /// and the field kind of the read. Anything else stays as it is.
    pub(crate) fn field_read_into_variant(&self, v: Value, read: FieldRead, locals: &[Value]) -> Result<Value, Signal> {
        let (Value::Blob(_), Some(Value::Record(rc))) = (&v, locals.get(usize::from(read.record))) else {
            return Ok(v);
        };
        let (temporary, holder, table) = {
            let r = rc.borrow();
            (r.temporary, Holder::of(&r), r.table as usize)
        };
        if !temporary {
            return Ok(v);
        }
        let kind = || self.blob_field_kind(table, read.pos as usize);
        moved(None, v, BlobMove::Variant { local: false }, holder, kind)
    }

    /// Record method `method` starts on `rec`: a method [`keeps_own_variable`] does not name ends `OwnVariable`.
    #[inline]
    pub(crate) fn own_variable_before(method: RecordOp, rec: &Rc<RefCell<RecordState>>) {
        if !keeps_own_variable(method) && rec.borrow().own_variable.0 {
            buffer_replaced(&mut rec.borrow_mut());
        }
    }
}
