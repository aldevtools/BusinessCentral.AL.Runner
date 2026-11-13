//! The Blob value a record field or a Variant takes when a Blob moves into it: the one owner of "copy, empty or keep the buffer"
//! for an assignment, a builtin's write-back, `FieldRef.Value :=`, `FieldRef.Validate`, `RecordRef.GetTable` / `SetTable`,
//! `TransferFields`, a Variant (an assignment, a by-value parameter), `Record.Copy` and a record passed by value.

use crate::Session;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::builtin_id::{BuiltinId, ids};
use al_builtins::native::Signal;
use al_builtins::record_op::RecordOp;
use al_builtins::stream::{copy_blob_db_row_of, copy_for_assignment, new_blob};
use al_builtins::value::{ProbedSource, RecordState, RtType, Value};
use al_ir::{Attribute, Const};
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
    /// `RecordRef.SetTable(Rec, true)` between two temporary records, from a RecordRef that `GetTable` filled.
    SetTableShared,
    /// `Rec.TransferFields(Source, ...)`.
    TransferFields(TransferCall),
    /// `V := Source` with a Variant `V` that is a local variable or a parameter (`local`). Not `local`: a Blob field read
    /// that is stored in an element of a local array, or handed to a by-value Variant parameter ([`FieldRead`]). A record
    /// stored in a Variant is [`BlobMove::RecordByValue`].
    Variant { local: bool },
    /// A Blob field read stored in a global Variant of the running object ([`FieldRead`]), of a `temporary` record or not.
    GlobalVariant { temporary: bool },
    /// `Dst.Copy(Src [, ShareTable])`, from a `temporary` source record or not into a `temporary` target record or not.
    RecordCopy {
        source_temporary: bool,
        target_temporary: bool,
    },
    /// A record handed by value to a parameter of a procedure, or assigned to a Variant ([`Session::blobs_by_value`]): from a
    /// `temporary` record or not, what takes it, and by which kind of call.
    RecordByValue {
        source_temporary: bool,
        taker: Taker,
        call: Call,
    },
}

/// The arguments of a `TransferFields` call.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransferCall {
    /// `TransferFields(Source)`.
    Plain,
    /// `TransferFields(Source, InitPrimaryKeyFields)` with either value, or `TransferFields(Source, true, true)`; `temporary`
    /// is whether both records are `temporary` (`None`: one is and the other is not).
    Flags { temporary: Option<bool> },
    /// Three arguments with other values.
    Other,
}

impl TransferCall {
    /// The call with arguments `args` (the source first), from a `temporary` record or not into one that is or is not.
    pub(crate) fn of(args: &[Value], source_temporary: bool, target_temporary: bool) -> TransferCall {
        match args {
            [_] => TransferCall::Plain,
            [_, Value::Boolean(_)] | [_, Value::Boolean(true), Value::Boolean(true)] => TransferCall::Flags {
                temporary: (source_temporary == target_temporary).then_some(source_temporary),
            },
            _ => TransferCall::Other,
        }
    }
}

/// The kind of call or assignment that makes the by-value copy of a record ([`Argument::measured_call`]).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Call {
    /// A procedure of a codeunit without attributes, or an assignment to a Variant.
    Plain,
    /// A procedure of a table without attributes.
    TableProcedure,
    /// A procedure of a codeunit with exactly the `[TryFunction]` attribute.
    TryFunction,
    /// An event subscriber of a codeunit that is not manual, run for a publisher [`measured_publisher`] names.
    Subscriber,
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
    /// A global Variant of the running object the record is assigned to.
    GlobalVariant,
    /// An element of a one-dimensional local array of Variant the record is assigned to.
    ArrayVariant,
}

impl Taker {
    /// What an assignment of a record to the Variant at place `p` is, if the probes measured that place: a local
    /// (corpus PR 1372, streams/BlobValueCopyShapes.al:824), a global of the running object (PR 1375,
    /// streams/BlobCopyStates.al:1119 `GV := Src`) or an element of a one-dimensional local array (:1141 `VA[2] := Src`).
    pub(crate) fn variant_at(p: &al_ir::Place) -> Option<Taker> {
        use al_ir::Place;
        match p {
            Place::Local(_) => Some(Taker::LocalVariant),
            Place::Global(_) => Some(Taker::GlobalVariant),
            Place::Index(base, at) if matches!(**base, Place::Local(_)) && at.len() == 1 => Some(Taker::ArrayVariant),
            _ => None,
        }
    }
}

/// How a record the probes measured as a source ([`Holder::Probed`]) was prepared: the record methods it went through since
/// it was declared, as the sources of corpus PR 1375 (streams/BlobCopyStates.al) went through them. Kept in
/// `RecordState::probed_source` (0: no measured source), set by [`new_variable`] and moved on by
/// [`Session::probed_source_before`], [`assigned`], [`set_table_target`] and [`Session::gettable_copy`]. A call none of
/// them names ends it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Prep {
    /// Declared, then only Init, Clear(Rec), IsTemporary and field writes (`PrepareF` :312-316 states mem and empty).
    Fresh = 1,
    /// Fresh, then one `Insert()` that inserted the row (`RecordState::inserted`; `PrepareF` :318-322 state ins; :2728 A1,
    /// :2752 A2, :2775 A3, :2825, :2852).
    Inserted = 2,
    /// Inserted, a Blob written, then `Modify()` (:2800 A4).
    Modified = 3,
    /// Inserted, then Init or Clear(Rec) (`PrepareF` :323-328; :2970 B4, :2994 B5).
    Emptied = 4,
    /// Emptied, then Get, and CalcFields or not (`PrepareF` :329-331 states calc and get; B4, B5).
    Read = 5,
    /// Fresh, then a record assignment from a Fresh record (`PrepareF` :333-347 state asg; :2901 B2).
    Assigned = 6,
    /// Fresh, then `RecordRef.SetTable(Rec)` from a RecordRef that `GetTable` filled from a Fresh record (:2923 B3).
    SetTable = 7,
}

/// The bits of `RecordState::probed_source` that hold a [`Prep`].
const PREP: u8 = 0x0f;
/// The record of a RecordRef that `RecordRef.GetTable` made of a measured source, with no RecordRef method, other handle or
/// `FieldRef.Validate` since (:1709). The [`Prep`] bits are the source's.
const TABLE_COPY: u8 = 0x10;
/// A [`TABLE_COPY`] record a `FieldRef.Value :=` of a Blob wrote since (:2923 B3): only `SetTable(Rec)` reads it.
const VALUED: u8 = 0x20;

impl Prep {
    fn of(b: u8) -> Option<Prep> {
        Some(match b & PREP {
            1 => Prep::Fresh,
            2 => Prep::Inserted,
            3 => Prep::Modified,
            4 => Prep::Emptied,
            5 => Prep::Read,
            6 => Prep::Assigned,
            7 => Prep::SetTable,
            _ => return None,
        })
    }

    /// What a record of `self` is after record method `method` with `args`, if still measured; `written` says whether a
    /// Blob of the record holds an unsaved write.
    fn after(self, method: RecordOp, args: &[Value], written: impl FnOnce() -> bool) -> Option<Prep> {
        match (self, method) {
            (p, RecordOp::IsTemporary) => Some(p),
            (Prep::Fresh, RecordOp::Init | RecordOp::Clear) => Some(Prep::Fresh),
            (Prep::Fresh, RecordOp::Insert) if args.is_empty() => Some(Prep::Inserted),
            (Prep::Inserted, RecordOp::Modify) if args.is_empty() && written() => Some(Prep::Modified),
            (Prep::Inserted, RecordOp::Init | RecordOp::Clear) => Some(Prep::Emptied),
            (Prep::Emptied, RecordOp::Get) => Some(Prep::Read),
            (Prep::Read, RecordOp::CalcFields) => Some(Prep::Read),
            _ => None,
        }
    }
}

/// What a move knows of the record whose field the source Blob is.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Holder {
    /// A record the probes measured as a source, prepared as the [`Prep`] says: a declared variable ([`new_variable`]) on
    /// which, since it was made, only field writes, streams on its Blob fields, the record methods [`Prep::after`] names
    /// and the builtins [`builtin_keeps_probed`] names ran; or the record `RecordRef.GetTable` made of such a record.
    Probed(Prep),
    /// Any other record, or one the move does not ask about.
    Other,
}

impl Holder {
    /// What record state `r` is as the source of a move. An `Insert()` that answered false leaves `RecordState::inserted`
    /// unset: the record is no [`Prep::Inserted`] one. A [`VALUED`] RecordRef record is none.
    pub(crate) fn of(r: &RecordState) -> Holder {
        let b = r.probed_source.0;
        match Prep::of(b) {
            _ if b & VALUED != 0 => Holder::Other,
            Some(Prep::Inserted) if !r.inserted && b & TABLE_COPY == 0 => Holder::Other,
            Some(p) => Holder::Probed(p),
            None => Holder::Other,
        }
    }
}

/// Marks the value a declared variable starts with as a source the probes measured ([`Prep::Fresh`]). The callers hand in
/// a local variable of a codeunit's procedure and a global of a codeunit, as the sources of corpus PR 1372
/// (streams/BlobValueCopyShapes.al:590 `Src`, :847 `GRow`) and PR 1375 (streams/BlobCopyStates.al:1048 `Src`, :3208 a global
/// record) are. A record in an array is not marked (not measured).
pub(crate) fn new_variable(v: &Value) {
    if let Value::Record(rc) = v {
        rc.borrow_mut().probed_source = ProbedSource(Prep::Fresh as u8);
    }
}

/// Whether a local slot of `routine`, a routine of an object of kind `object`, holds a variable [`new_variable`] marks: a
/// declared local variable of a procedure of a codeunit. A parameter, a named return value, a local of a trigger and a
/// local of a routine of a page, a table, a report or any other object are not measured.
pub(crate) fn measured_local(object: al_ir::ObjKind, routine: &al_ir::Routine, slot: usize) -> bool {
    object == al_ir::ObjKind::Codeunit
        && !routine.is_trigger
        && slot >= routine.params
        && routine.ret_local != Some(slot)
}

/// Whether builtin `id` leaves the records among its operands measured sources ([`Holder::Probed`]) as they are. Only the
/// calls the measured sources went through, by name: `Clear(Rec)` (it reaches `RecordOp::Clear`, [`Prep::after`]; corpus
/// PR 1372, streams/BlobValueCopyShapes.al:847; PR 1375, streams/BlobCopyStates.al:334), the record of
/// `RecordRef.GetTable(Rec)` (PR 1372 :712; PR 1375 :1709), the target of `RecordRef.SetTable(Rec)` with one argument
/// ([`set_table_target`] says what it is after), and the record as the source of `Record.Copy` and `TransferFields` (PR 1372
/// :741-:781, :906-:952; PR 1375 :1073, :1093). A record method's own record is [`Prep::after`]'s.
fn builtin_keeps_probed(id: BuiltinId) -> bool {
    matches!(
        id,
        ids::SYSTEM_CLEAR_1
            | ids::RECORDREF_GETTABLE_1
            | ids::RECORDREF_SETTABLE_1
            | ids::RECORD_COPY_1
            | ids::RECORD_TRANSFERFIELDS_1
            | ids::RECORD_TRANSFERFIELDS_2
    )
}

/// Builtin `id` is about to run on receiver `recv` and operands `operands`: the one place a record goes by reference into
/// platform code. A record among them is a measured source ([`Holder::Probed`]) no longer, whatever the builtin does with
/// it (a page that hands back the picked row, `SetSelectionFilter`, `Codeunit.Run`, `Page.GetRecord`), unless
/// [`builtin_keeps_probed`] names the builtin. A Record receiver is the record of a record method, which reaches
/// `Session::record_op` and is [`Prep::after`]'s.
#[inline]
pub(crate) fn records_into_builtin(id: BuiltinId, recv: Option<&Value>, operands: &[Value]) {
    let own = recv.filter(|v| !matches!(v, Value::Record(_)));
    for v in own.into_iter().chain(operands) {
        if let Value::Record(rc) = v
            && rc.borrow().probed_source.0 != 0
            && !builtin_keeps_probed(id)
        {
            rc.borrow_mut().probed_source = ProbedSource(0);
        }
    }
}

/// Whether publisher routine `r` of an object of kind `object` raises the event whose subscribers the probes measured: a
/// procedure of a codeunit with exactly `[IntegrationEvent(false, false)]` (corpus PR 1375, streams/BlobCopyStates.al:
/// 388-403). Table events, a `[BusinessEvent]`, an IncludeSender event and every other publisher are not measured.
pub(crate) fn measured_publisher(object: al_ir::ObjKind, r: &al_ir::Routine) -> bool {
    object == al_ir::ObjKind::Codeunit
        && matches!(&r.attributes[..], [Attribute { name, args, .. }]
            if name.eq_ignore_ascii_case("IntegrationEvent")
                && args[..] == [Const::Boolean(false), Const::Boolean(false)])
}

/// The by-value copy `copy` the publisher made of record `source` for its subscribers: a measured source prepared as `source`
/// when `source` is one and the publisher is one [`measured_publisher`] names (`measured`), so that a by-value subscriber
/// takes the copy [`Argument::measured_call`] measures (:391 a codeunit's IntegrationEvent with a by-value record, :412 its
/// by-value subscriber, :2300 kind 1, :2568-2606).
pub(crate) fn publisher_copy(measured: bool, source: &Rc<RefCell<RecordState>>, copy: &Value) {
    if let (true, Value::Record(c)) = (measured, copy) {
        let b = source.borrow().probed_source.0;
        if Prep::of(b).is_some() && b & !PREP == 0 {
            c.borrow_mut().probed_source = ProbedSource(b);
        }
    }
}

/// What the target of the record assignment `Target := Source` is afterwards, from what both were: [`Prep::Assigned`] when
/// both are [`Prep::Fresh`] (corpus PR 1375, streams/BlobCopyStates.al:340-347 state `asg`, :2901 `Mid := Src`).
pub(crate) fn assigned(target: &RecordState, source: &RecordState) -> ProbedSource {
    let fresh = |r: &RecordState| r.probed_source.0 == Prep::Fresh as u8;
    ProbedSource(if fresh(target) && fresh(source) {
        Prep::Assigned as u8
    } else {
        0
    })
}

/// What the target of `RecordRef.SetTable(Target)` with one argument is afterwards, from what it was and what the
/// RecordRef's record `rr` is: [`Prep::SetTable`] when the target is [`Prep::Fresh`] and `rr` is what `GetTable` made of
/// a Fresh record, with a `FieldRef.Value :=` since or not (corpus PR 1375, streams/BlobCopyStates.al:2923 B3).
pub(crate) fn set_table_target(target: &RecordState, rr: &RecordState) -> ProbedSource {
    let fresh_copy = matches!(rr.probed_source.0 & !VALUED, b if b == TABLE_COPY | Prep::Fresh as u8);
    ProbedSource(if target.probed_source.0 == Prep::Fresh as u8 && fresh_copy {
        Prep::SetTable as u8
    } else {
        0
    })
}

/// A `FieldRef.Value :=` (`validate` false) or a `FieldRef.Validate` of value `v` wrote the record `r` of a RecordRef: a
/// [`TABLE_COPY`] record a Blob was assigned to is [`VALUED`]; every other write ends the fact.
pub(crate) fn written_through_fieldref(r: &mut RecordState, v: &Value, validate: bool) {
    let b = r.probed_source.0;
    if b != 0 {
        let valued = b & TABLE_COPY != 0 && !validate && matches!(v, Value::Blob(_));
        r.probed_source = ProbedSource(if valued { b | VALUED } else { 0 });
    }
}

/// A second RecordRef handle on record `r` was made (an assignment, a Variant, a parameter): a [`TABLE_COPY`] record is a
/// measured source no longer.
pub(crate) fn recordref_aliased(r: &Rc<RefCell<RecordState>>) {
    if r.borrow().probed_source.0 & TABLE_COPY != 0 {
        r.borrow_mut().probed_source = ProbedSource(0);
    }
}

/// What an argument of a procedure call is for the Blob rule, read before the call binds it to its parameter.
#[derive(Clone, Copy)]
pub(crate) enum Argument {
    /// A record handed by value: whether it is temporary, what it is as a source, what takes it, and by which call.
    Record(bool, Holder, Taker, Call),
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
    /// The read is stored in a global Variant ([`BlobMove::GlobalVariant`]).
    pub(crate) global: bool,
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
                    global: false,
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
    /// The kind of call in which a by-value parameter of `callee`, a routine of an object of kind `object`, takes its argument,
    /// if the probes measured it: a procedure of a codeunit without attributes (corpus PR 1372, streams/
    /// BlobValueCopyShapes.al:562-582; PR 1375, streams/BlobCopyStates.al:3135 another codeunit, :3153 an interface method),
    /// a procedure of a table without attributes (PR 1375 :3173), a codeunit's `[TryFunction]` (:2200), and a codeunit's
    /// event subscriber when the call is the raise of an event [`measured_publisher`] names to a subscriber codeunit that is
    /// not manual (`measured_event`; :2300, :406-:428). The arguments of a `var` parameter, a trigger, a procedure of a page
    /// or a report, a table's `[TryFunction]`, a procedure with any other attribute and every other subscriber are bound as
    /// before (an event publisher too: its by-value parameter is the copy its subscribers get, :2300 kind 1).
    pub(crate) fn measured_call(object: al_ir::ObjKind, callee: &al_ir::Routine, measured_event: bool) -> Option<Call> {
        let only = |name: &str| matches!(&callee.attributes[..], [a] if a.name.eq_ignore_ascii_case(name));
        if callee.is_trigger {
            return None;
        }
        match object {
            al_ir::ObjKind::Codeunit if callee.attributes.is_empty() => Some(Call::Plain),
            al_ir::ObjKind::Codeunit if only("TryFunction") => Some(Call::TryFunction),
            al_ir::ObjKind::Codeunit if measured_event && only("EventSubscriber") => Some(Call::Subscriber),
            al_ir::ObjKind::Table if callee.attributes.is_empty() => Some(Call::TableProcedure),
            _ => None,
        }
    }

    /// The argument `a` of a call for its parameter of type `param`. `call` is [`Argument::measured_call`] for a by-value
    /// parameter.
    pub(crate) fn of(a: &Value, call: Option<Call>, param: &RtType) -> Argument {
        let taker = match param {
            RtType::Record(_, false) => Taker::RecordParam,
            RtType::Record(_, true) => Taker::TemporaryParam,
            RtType::Variant => Taker::VariantParam,
            _ => return Argument::Other,
        };
        match (a, call) {
            (Value::Record(rc), Some(call)) => {
                let r = rc.borrow();
                Argument::Record(r.temporary, Holder::of(&r), taker, call)
            }
            _ => Argument::Other,
        }
    }
}

impl BlobMove {
    /// The move of `RecordRef.SetTable` with `arguments` arguments, the second one `share`: one argument (corpus PR 1369,
    /// FieldRefValueBlobCopy.al:941, :964) and ShareTable true (PR 1372, streams/BlobValueCopyShapes.al:712 X6; PR 1375,
    /// streams/BlobCopyStates.al:1709, between two temporary records). The caller raises the Argument error of ShareTable
    /// true with a record that is not temporary (PR 1375 :1734), and refuses a temporary and a normal record mixed, before
    /// it asks here, so `share` means both are temporary. With ShareTable false the Blobs come as before (`None`).
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
    /// A normal field the table declares with `Compressed = false` (PR 1372 :40).
    Variation,
    /// A normal field a table extension declares without the `Compressed` property (PR 1372 :53). It takes what a
    /// [`FieldKind::Variation`] field takes, except in the moves only that kind was measured in ([`other_kind_move`]).
    Extension,
    /// A normal field the table declares with `Compressed = true` (corpus PR 1375, streams/BlobCopyStates.al:52). Measured
    /// only in the moves [`other_kind_move`] names; refused as [`FieldKind::Other`] in the others.
    Compressed,
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
            .map(|(_, v)| v.trim().to_ascii_lowercase());
        match (own, extension, uncompressed_declared.as_deref()) {
            (true, false, None) => FieldKind::Plain,
            (true, false, Some("false")) => FieldKind::Variation,
            (false, true, None) => FieldKind::Extension,
            (true, false, Some("true")) => FieldKind::Compressed,
            _ => FieldKind::Other,
        }
    }

    /// The kind of a move between a field of kind `self` and one of kind `other`: theirs when they are the same, and
    /// [`FieldKind::Variation`] between it and [`FieldKind::Extension`] (one kind before they were told apart).
    pub(crate) fn with(self, other: FieldKind) -> FieldKind {
        match (self, other) {
            (a, b) if a == b => a,
            (FieldKind::Variation, FieldKind::Extension) | (FieldKind::Extension, FieldKind::Variation) => {
                FieldKind::Variation
            }
            _ => FieldKind::Other,
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
/// FieldRefValueBlobCopy.al; PR 1372, streams/BlobValueCopyShapes.al; PR 1375, streams/BlobCopyStates.al; the lines are in
/// the entry), for the cells
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
            content: !buf.bytes.is_empty(),
            unread_row: buf.db_row
                && !buf.loaded
                && !buf.written
                && !buf.first_write_truncates
                && !buf.bytes.is_empty(),
            in_memory: buf.loaded && !row_copy && !buf.inserted && !buf.persisted && !buf.refetch,
            unsaved_since_insert: buf.loaded && !row_copy && !buf.persisted && !buf.refetch,
            inserted: buf.inserted,
            base_content: buf.written_base.as_ref().is_some_and(|b| !b.is_empty()),
            plain_buffer: !buf.file
                && buf.dotnet_view == 0
                && buf.render_pending.is_none()
                && !buf.unread_content
                && !buf.first_write_truncates
                && !buf.assigned_unsaved,
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
        FieldKind::Variation | FieldKind::Extension if first_probe_move(how) => plain,
        FieldKind::Variation | FieldKind::Extension | FieldKind::Compressed
            if other_kind_move(how, kind, &s, holder) =>
        {
            plain
        }
        FieldKind::Variation | FieldKind::Extension => None,
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
        Cell::Copy => copy_for_assignment(b),
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
            BlobMove::Variant { .. } | BlobMove::GlobalVariant { .. } => "an assignment to a Variant",
            BlobMove::Assign => "an assignment",
            BlobMove::WriteBack => "a write-back",
            BlobMove::RecordCopy { .. } => "Record.Copy",
            BlobMove::RecordByValue { .. } => "a record passed by value",
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
    content: bool,
    /// The copy a read of a database row handed out, whose row holds content, not loaded and with no stream created on it.
    unread_row: bool,
    /// Loaded, and not tied to a row: not the copy a read handed out, not what an Insert or a Modify stored, not the copy a
    /// record assignment made.
    in_memory: bool,
    /// As `in_memory`, but an Insert may have stored the buffer before.
    unsaved_since_insert: bool,
    /// An Insert stored the content, and no Modify or stream write came after (`BlobBuf::inserted`).
    inserted: bool,
    /// The buffer held content when the first stream write since its last save or fetch started (`BlobBuf::written_base`).
    base_content: bool,
    /// No mark of another rule: not the content of a `File`, not handed to a .NET stream, no rendered report pending, not
    /// the empty Blob `FieldRef.Value` hands out for unread content, no first write that truncates, no unsaved assignment
    /// (assumption `blob-assign-counts-as-write`). The buffers the probes measured carry none of them.
    plain_buffer: bool,
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
/// A source in a record the probes measured as a source ([`Holder::Probed`]), in a [`State`] the probes measured for the
/// move: a copy. P3 is corpus PR 1375 (streams/BlobCopyStates.al): a record by value ([`by_value_measured`]), Record.Copy
/// ([`copy_measured`]), TransferFields with two arguments between two records that are not temporary in the six `PrepareF`
/// states (:1093, :1369-1542) and between two temporary records written in memory (P2 :741 TF2, :762 TF2f, :781 TF3) or
/// written again after their Insert (P3 :2825), SetTable(Rec, true) between temporary records in the six states (:1709,
/// :1859-1939). A field read stored in a global Variant: of a temporary record written in memory (P2 :672 GV) or written
/// again after its Insert (P3 :2825), of a record that is not temporary written again after its Insert (P3 :2852). A field
/// read of a temporary record written in memory stored in an element of a local array or handed to a by-value Variant
/// parameter (P2 :590 V3, :652 V5; P3 :3017, :3087).
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
    let st = state(s, holder);
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
        _ if st.is_some_and(|st| state_measured(how, st)) => Some(Cell::Copy),
        BlobMove::FieldRefValidate | BlobMove::Variant { local: true } if s.unread_row => Some(Cell::Unread),
        BlobMove::SetTable if s.unread_row => Some(Cell::UnreadRow),
        _ => None,
    }
}

/// The state of the source of a move in a measured source record ([`Holder::Probed`]) whose buffer carries no mark of
/// another rule (`Source::plain_buffer`): how the record was prepared ([`Prep`]) and what its buffer holds. The six states
/// of `PrepareF` (corpus PR 1375, streams/BlobCopyStates.al:306-350) and the sequences of :2728-3016.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum State {
    /// `mem`: written in memory, not stored (:314).
    Mem,
    /// `ins`: written, then inserted (:318-322).
    Ins,
    /// `calc`: stored, read again with Get and CalcFields (:323-331).
    Calc,
    /// `get`: stored, read again with Get only, not loaded (:323-331).
    Get,
    /// `empty`: Init only, not stored (:312).
    Empty,
    /// `asg`: filled by a record assignment from a record that holds content (:333-347).
    Asg,
    /// Written, inserted, then written again (:2728 A1, :2752 A2, :2775 A3, :2825 A5, :2852).
    InsW,
    /// Inserted, written, modified, then written again (:2800 A4).
    ModW,
    /// Fresh, filled by a Blob field assignment, not written (:2880 B1, :2949 B1c).
    FieldAsg,
    /// Filled by a record assignment, then written (:2901 B2).
    AsgW,
    /// Filled by `RecordRef.SetTable(Rec)` (:2923 B3).
    SetTable,
    /// Read with Get and CalcFields, then written (:2970 B4).
    CalcW,
    /// Read with Get only, then written (:2994 B5).
    GetW,
}

/// The [`State`] of source `s` in a record `holder`, if the probes measured it.
fn state(s: &Source, holder: Holder) -> Option<State> {
    let Holder::Probed(prep) = holder else { return None };
    if !s.plain_buffer {
        return None;
    }
    let get_only = s.row_copy && !s.loaded;
    let loaded = s.loaded && s.content;
    Some(match (prep, s.written) {
        (Prep::Fresh, true) if s.in_memory && s.content => State::Mem,
        (Prep::Fresh, false) if s.in_memory && s.content => State::FieldAsg,
        (Prep::Fresh, false) if s.in_memory => State::Empty,
        (Prep::Inserted, false) if s.inserted && s.content => State::Ins,
        (Prep::Inserted, true) if s.content && s.base_content && s.unsaved_since_insert => State::InsW,
        (Prep::Modified, true) if s.content && s.unsaved_since_insert => State::ModW,
        (Prep::Read, false) if loaded => State::Calc,
        (Prep::Read, true) if loaded => State::CalcW,
        (Prep::Read, false) if get_only => State::Get,
        (Prep::Read, true) if get_only => State::GetW,
        (Prep::Assigned, false) if loaded => State::Asg,
        (Prep::Assigned, true) if loaded => State::AsgW,
        (Prep::SetTable, false) if loaded => State::SetTable,
        _ => return None,
    })
}

/// The six states of `PrepareF`.
fn prepare_f(st: State) -> bool {
    matches!(
        st,
        State::Mem | State::Ins | State::Calc | State::Get | State::Empty | State::Asg
    )
}

/// Whether move `how` was measured from a source in state `st` (see [`measured`]).
fn state_measured(how: BlobMove, st: State) -> bool {
    match how {
        BlobMove::RecordByValue {
            source_temporary,
            taker,
            call,
        } => by_value_measured(source_temporary, taker, call, st),
        BlobMove::RecordCopy {
            source_temporary,
            target_temporary,
        } => copy_measured(source_temporary, target_temporary, st),
        BlobMove::TransferFields(TransferCall::Flags { temporary: Some(false) }) | BlobMove::SetTableShared => {
            prepare_f(st)
        }
        BlobMove::TransferFields(TransferCall::Flags { temporary: Some(true) }) => {
            matches!(st, State::Mem | State::InsW)
        }
        BlobMove::GlobalVariant { temporary: true } => matches!(st, State::Mem | State::InsW),
        BlobMove::GlobalVariant { temporary: false } => st == State::InsW,
        BlobMove::Variant { local: false } => st == State::Mem,
        _ => false,
    }
}

/// Whether a record from a source record that is `temporary` or not (`source_temporary`), taken by `taker` in a call or
/// assignment of kind `call`, was measured in state `st`.
/// - A procedure of a codeunit, not temporary into a Record parameter: the six states (:1048 `ByValue`, :1165-1284), A1
///   (:2728), A4 (:2800), B1 to B5 (:2880-3016).
/// - Temporary into a Record parameter, and not temporary into a `temporary` parameter: `mem`, `ins`, `get` (:2018-2045,
///   :2059-2087).
/// - Temporary into a `temporary` parameter: `mem` (corpus PR 1372, streams/BlobValueCopyShapes.al:975 P5t), A2 (:2752).
/// - Temporary into a by-value Variant parameter or a local Variant: `mem` (PR 1372 :847 P2, :824 P1).
/// - Not temporary into a global Variant or an element of a local array of Variant: the six states (:1119, :1141,
///   :1543-1699).
/// - Not temporary into a Record parameter of a table procedure: `mem` (:3173); of a codeunit's `[TryFunction]` and of an
///   event subscriber: `mem`, `ins`, `get` (:2200, :2331-2359; :2300, :2568-2606).
///
/// Every other pair, call and state keeps the shared buffer, as before.
fn by_value_measured(source_temporary: bool, taker: Taker, call: Call, st: State) -> bool {
    let mem_ins_get = matches!(st, State::Mem | State::Ins | State::Get);
    match (call, source_temporary, taker) {
        (Call::Plain, false, Taker::RecordParam) => {
            prepare_f(st)
                || matches!(
                    st,
                    State::InsW
                        | State::ModW
                        | State::FieldAsg
                        | State::AsgW
                        | State::SetTable
                        | State::CalcW
                        | State::GetW
                )
        }
        (Call::Plain, true, Taker::RecordParam) | (Call::Plain, false, Taker::TemporaryParam) => mem_ins_get,
        (Call::Plain, true, Taker::TemporaryParam) => matches!(st, State::Mem | State::InsW),
        (Call::Plain, true, Taker::VariantParam | Taker::LocalVariant) => st == State::Mem,
        (Call::Plain, false, Taker::GlobalVariant | Taker::ArrayVariant) => prepare_f(st),
        (Call::TableProcedure, false, Taker::RecordParam) => st == State::Mem,
        (Call::TryFunction | Call::Subscriber, false, Taker::RecordParam) => mem_ins_get,
        _ => false,
    }
}

/// Whether Record.Copy from a source record that is `temporary` or not (`source_temporary`) into a target that is or is not
/// (`target_temporary`) was measured in state `st`.
/// - Two records that are not temporary: the six states (:1285-1368 `Copy_*`), A3 (:2775), B1c (:2949), E2 (:3274).
/// - Not temporary into temporary: `mem`, `ins`, `get` (:2101, :2116, :2131).
/// - Temporary into not temporary: `mem`, `ins` (:2146, :2160).
/// - Two temporary records: `mem` (corpus PR 1372, streams/BlobValueCopyShapes.al:906 X2, :929 X2s, :952 X2r).
///
/// Every other pair of records and state keeps the shared buffer, as before.
fn copy_measured(source_temporary: bool, target_temporary: bool, st: State) -> bool {
    match (source_temporary, target_temporary) {
        (false, false) => prepare_f(st) || matches!(st, State::InsW | State::FieldAsg),
        (false, true) => matches!(st, State::Mem | State::Ins | State::Get),
        (true, false) => matches!(st, State::Mem | State::Ins),
        (true, true) => st == State::Mem,
    }
}

/// The moves in which a [`FieldKind::Variation`] field gives what a plain field gives ([`measured`]). Corpus PR 1372
/// (streams/BlobValueCopyShapes.al) measured loaded content through GetTable, SetTable and TransferFields(Source) (:1174
/// Compressed = false, :1190 and :1206 a table extension field), and such a field as the source of FieldRef.Value and of a
/// local Variant (:1174, :1190). Such a field as the target of FieldRef.Value and FieldRef.Validate copies in every source
/// state, and a source read with Get only gives an empty target (corpus PR 1375, streams/BlobCopyStates.al, procedure Into,
/// :454, :557-1010). Its empty and not-loaded states in the other moves are taken to follow: these moves were refused
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

/// Whether a move of a field of kind `kind` (not [`FieldKind::Plain`] nor [`FieldKind::Other`]) by `how` takes what a
/// plain field takes in the cells corpus PR 1375 measured (streams/BlobCopyStates.al): content a stream wrote in memory in a
/// measured source, moved by a record that is not temporary by value to a Record parameter of a codeunit procedure (:3108 C6,
/// every kind), as a field read handed to a by-value Variant parameter
/// (:3017 C1-C3), read from a temporary record and stored in a global Variant or an array element (:3087 C4, C5), and by
/// Record.Copy between two records that are not temporary for a `Compressed = false` field (:3274 E2).
fn other_kind_move(how: BlobMove, kind: FieldKind, s: &Source, holder: Holder) -> bool {
    let mem = state(s, holder) == Some(State::Mem);
    match how {
        BlobMove::RecordByValue {
            source_temporary: false,
            taker: Taker::RecordParam,
            call: Call::Plain,
        }
        | BlobMove::Variant { local: false }
        | BlobMove::GlobalVariant { temporary: true } => mem,
        BlobMove::RecordCopy {
            source_temporary: false,
            target_temporary: false,
        } => mem && kind == FieldKind::Variation,
        _ => false,
    }
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
        // The RecordRef's record is a measured source prepared as the record when the record is one: SetTable(Rec, true)
        // moves its Blobs on (corpus PR 1375, streams/BlobCopyStates.al:1709, :1859-1939).
        if let (true, Holder::Probed(p)) = (from_record, holder) {
            copy.probed_source = ProbedSource(p as u8 | TABLE_COPY);
        }
        if from_record {
            self.move_blobs(copy.table as usize, &mut copy.fields, None, BlobMove::GetTable, holder)?;
        }
        Ok(copy)
    }

    /// The by-value copy `copy` of a record, from a `temporary` record or not, taken by `taker` in a call or assignment of
    /// kind `call`: its Blobs are moved as [`moved`] says ([`by_value_measured`]). `holder` is the source record.
    pub(crate) fn blobs_by_value(
        &self,
        copy: Value,
        source_temporary: bool,
        holder: Holder,
        taker: Taker,
        call: Call,
    ) -> Result<Value, Signal> {
        if let Value::Record(rc) = &copy {
            let mut r = rc.borrow_mut();
            let table = r.table as usize;
            let how = BlobMove::RecordByValue {
                source_temporary,
                taker,
                call,
            };
            self.move_blobs(table, &mut r.fields, None, how, holder)?;
        }
        Ok(copy)
    }

    /// The value `bound` a call made of its argument `argument`, with its Blobs moved as [`Session::blobs_by_value`] says.
    pub(crate) fn bind_blobs(&self, argument: Argument, bound: Value) -> Result<Value, Signal> {
        match argument {
            Argument::Record(temporary, holder, taker, call) => {
                self.blobs_by_value(bound, temporary, holder, taker, call)
            }
            Argument::Other => Ok(bound),
        }
    }

    /// The value a Variant that is not local takes when `v`, the value of field read `read` in the frame with locals
    /// `locals`, moves into it ([`BlobMove::Variant`], [`BlobMove::GlobalVariant`]). A Blob of a temporary record (corpus PR
    /// 1372, streams/BlobValueCopyShapes.al:590, :652, :672: every source is temporary), and one of a record that is not
    /// temporary stored in a global Variant (corpus PR 1375, streams/BlobCopyStates.al:2852), moves as [`moved`] says, with
    /// the record and the field kind of the read. Anything else stays as it is.
    pub(crate) fn field_read_into_variant(&self, v: Value, read: FieldRead, locals: &[Value]) -> Result<Value, Signal> {
        let (Value::Blob(_), Some(Value::Record(rc))) = (&v, locals.get(usize::from(read.record))) else {
            return Ok(v);
        };
        let (temporary, holder, table) = {
            let r = rc.borrow();
            (r.temporary, Holder::of(&r), r.table as usize)
        };
        if !(temporary || read.global) {
            return Ok(v);
        }
        let kind = || self.blob_field_kind(table, read.pos as usize);
        let how = if read.global {
            BlobMove::GlobalVariant { temporary }
        } else {
            BlobMove::Variant { local: false }
        };
        moved(None, v, how, holder, kind)
    }

    /// Record method `method` starts on `rec` with arguments `args`: the record is prepared as [`Prep::after`] says, or a
    /// measured source no longer. Every method of a RecordRef on a [`TABLE_COPY`] record ends the fact.
    #[inline]
    pub(crate) fn probed_source_before(method: RecordOp, rec: &Rc<RefCell<RecordState>>, args: &[Value]) {
        let b = rec.borrow().probed_source.0;
        if b == 0 {
            return;
        }
        let written = || {
            rec.borrow()
                .fields
                .iter()
                .any(|v| matches!(v, Value::Blob(x) if x.borrow().written))
        };
        let next = match Prep::of(b) {
            Some(p) if b & !PREP == 0 => p.after(method, args, written).map_or(0, |p| p as u8),
            _ => 0,
        };
        if next != b {
            rec.borrow_mut().probed_source = ProbedSource(next);
        }
    }
}
