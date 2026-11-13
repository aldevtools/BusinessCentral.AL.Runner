//! The Blob value a record field or a Variant takes when a Blob moves into it: the one owner of "copy, empty or keep the buffer"
//! for an assignment, a builtin's write-back, `FieldRef.Value :=`, `FieldRef.Validate`, `RecordRef.GetTable` / `SetTable`,
//! `TransferFields`, a Variant (an assignment, a by-value parameter), `Record.Copy` and a record passed by value.

use crate::Session;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::builtin_id::{BuiltinId, ids};
use al_builtins::native::Signal;
use al_builtins::record_op::RecordOp;
use al_builtins::stream::{copy_blob_db_row_of, copy_for_assignment, new_blob};
use al_builtins::value::{LibraryValue, ProbedSource, RecordState, RtType, Value};
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
    /// `V := Source` with a Variant `V` that is a local variable or a parameter. A record stored in a Variant is
    /// [`BlobMove::RecordByValue`].
    Variant,
    /// A Blob field read ([`FieldRead`]) of a `temporary` record or not, stored in a global Variant of the running object
    /// (`global`), or in an element of a local array or handed to a by-value Variant parameter (not `global`).
    ReadIntoVariant { global: bool, temporary: bool },
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
    /// A procedure of a table without attributes or with exactly `[TryFunction]`.
    TableProcedure,
    /// A procedure of a codeunit with exactly the `[TryFunction]` attribute.
    TryFunction,
    /// An event subscriber of a codeunit, run for an event [`BlobEvent`] names.
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
    /// What an assignment of a record to the Variant at place `p` is, if the probes measured that place: a local, a global
    /// of the running object or an element of a one-dimensional local array (corpus PR 1387,
    /// streams/BlobCopyNeighbors.al:665, :691, :717 stored; :1003, :1029, :1055 temporary).
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

/// `RecordState::probed_source` byte: a declared variable on which, since it was made, only Init, Clear(Rec), IsTemporary,
/// field writes and streams on its Blob fields ran (corpus PR 1375, streams/BlobCopyStates.al `PrepareF` :312-316 states
/// mem and empty; PR 1387, streams/BlobCopyNeighbors.al `Prep` :200-:212 states mem and empty).
const FRESH: u8 = 1;
/// `RecordState::probed_source` byte: a declared variable on which, since it was made, only the record methods [`after`]
/// names, the builtins [`builtin_keeps_probed`] names, record assignments between such variables, field writes and streams
/// ran, at least one of them not one a [`FRESH`] variable keeps (PR 1387 `Prep` :200-:303 and its 16 states; :3205, :3243 a
/// method between Insert and the by-value call).
const MEASURED: u8 = 2;
/// `RecordState::probed_source` byte: the record of a RecordRef that `RecordRef.GetTable` made of a [`FRESH`] or
/// [`MEASURED`] variable with no Commit since it was made, with since only the RecordRef and FieldRef calls
/// [`recordref_keeps_table_copy`] names (and the record methods they reach, [`after`]) and a RecordRef assignment (PR 1375
/// :1709; PR 1387 :2446).
const TABLE_COPY: u8 = 3;

/// What a record method `method` with `args` leaves of the fact `b` of its record (0: the fact ends). The calls
/// `Prep` and the method sequences of corpus PR 1387 (streams/BlobCopyNeighbors.al:200-:303, :3205, :3243) ran on a source
/// variable: Init, Clear(Rec) (`RecordOp::Clear`, through the native), IsTemporary, `Insert()` (also one that answers false,
/// state insfailedinsertwrite), `Modify()`, Get, CalcFields, `FindFirst()`. The RecordRef calls :2446 ran between GetTable
/// and SetTable(Rec, true): `FindFirst()`, CalcField, `Modify()`, `Insert()`, SetRange, Reset.
fn after(b: u8, method: RecordOp, args: &[Value]) -> u8 {
    let no_args = args.is_empty();
    match (b, method) {
        (FRESH, RecordOp::Init | RecordOp::Clear | RecordOp::IsTemporary) => FRESH,
        (
            FRESH | MEASURED,
            RecordOp::Init | RecordOp::Clear | RecordOp::IsTemporary | RecordOp::Get | RecordOp::CalcFields,
        ) => MEASURED,
        (FRESH | MEASURED, RecordOp::Insert | RecordOp::Modify | RecordOp::FindFirst) if no_args => MEASURED,
        (TABLE_COPY, RecordOp::CalcFields | RecordOp::SetRange | RecordOp::Reset) => TABLE_COPY,
        (TABLE_COPY, RecordOp::Insert | RecordOp::Modify | RecordOp::FindFirst) if no_args => TABLE_COPY,
        _ => 0,
    }
}

/// What a move knows of the record whose field the source Blob is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Holder {
    /// A [`FRESH`] variable.
    Fresh,
    /// A [`MEASURED`] variable.
    Measured,
    /// A [`TABLE_COPY`] record.
    TableCopy,
    /// Any other record, one a Commit ran after its fact started, or one the move does not ask about.
    Other,
}

impl Holder {
    /// What record state `r` is as the source of a move, with `commits` the session's commit count now. A [`MEASURED`]
    /// variable and a [`TABLE_COPY`] record ran no Commit since they were made (the probes ran none there,
    /// [`measured_variable`]); a [`FRESH`] variable may have run one before (corpus PR 1375, streams/BlobCopyStates.al:3208
    /// D5, Clear and a write on a global record of a test codeunit made before earlier tests ran).
    pub(crate) fn of(r: &RecordState, commits: u64) -> Holder {
        match r.probed_source {
            ProbedSource(FRESH, _) => Holder::Fresh,
            ProbedSource(MEASURED, at) if at == commits => Holder::Measured,
            ProbedSource(TABLE_COPY, at) if at == commits => Holder::TableCopy,
            _ => Holder::Other,
        }
    }

    /// A declared variable the probes measured as a source.
    fn variable(self) -> bool {
        matches!(self, Holder::Fresh | Holder::Measured)
    }
}

/// Whether record state `r` may become or stay [`MEASURED`], with `commits` the session's commit count now: a [`FRESH`] or
/// [`MEASURED`] variable on which no Commit ran since it was made. Every Insert, Modify, Get, CalcFields, FindFirst, record
/// assignment and SetTable the probes ran on a source ran with no Commit before it in the test (PR 1387
/// streams/BlobCopyNeighbors.al `Prep` :200-:303, :3205, :3243); a Commit before them is not measured.
fn measured_variable(r: &RecordState, commits: u64) -> bool {
    matches!(r.probed_source, ProbedSource(FRESH | MEASURED, at) if at == commits)
}

/// Marks the value a declared variable starts with as a [`FRESH`] source; `commits` is the session's commit count, `sources`
/// the session's [`Sources`]. The callers
/// hand in a local variable of a codeunit's procedure and a global of a codeunit, as the sources of corpus PR 1372
/// (streams/BlobValueCopyShapes.al:590 `Src`, :847 `GRow`), PR 1375 (streams/BlobCopyStates.al:1048 `Src`, :3208 a global
/// record) and PR 1387 (streams/BlobCopyNeighbors.al:492 `CellRecordParam`) are. A record in an array is not marked (not measured).
pub(crate) fn new_variable(v: &Value, commits: u64, sources: &mut Sources) {
    if let Value::Record(rc) = v {
        rc.borrow_mut().probed_source = ProbedSource(FRESH, commits);
        sources.records_marked();
    }
}

/// Whether a record of the session may be a measured source: the session marked a record [`FRESH`] ([`new_variable`]) or
/// took over the single instances of an earlier session (`Session::adopt_single_instances`), whose globals may hold such
/// records. Every other non-zero `RecordState::probed_source` comes from one of these (a clone gives 0), so while this is
/// `false` no record of the session is a measured source and [`records_into_builtin`] has nothing to end. It is never reset.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Sources(bool);

impl Sources {
    /// Whether any record of the session may be a measured source.
    #[inline]
    pub(crate) fn any(self) -> bool {
        self.0
    }

    /// A record of the session may be a measured source from now on.
    pub(crate) fn records_marked(&mut self) {
        self.0 = true;
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

/// Whether builtin `id` with operands `operands` leaves the records among them measured sources as they are. Only the calls
/// the measured sources went through, by name: `Clear(Rec)` (it reaches `RecordOp::Clear`, [`after`]; corpus PR 1387,
/// streams/BlobCopyNeighbors.al:248), the record of `RecordRef.GetTable(Rec)` (:2424 `RrGetTable`), the target of
/// `RecordRef.SetTable(Rec)` and `SetTable(Rec, ShareTable)` ([`set_table_target`] says what it is after; :2424), and the
/// record as the source of `Record.Copy` and `TransferFields` (:1204-:1308, :769). A record method's own record is
/// [`after`]'s.
fn builtin_keeps_probed(id: BuiltinId) -> bool {
    matches!(
        id,
        ids::SYSTEM_CLEAR_1
            | ids::RECORDREF_GETTABLE_1
            | ids::RECORDREF_SETTABLE_1
            | ids::RECORDREF_SETTABLE_2
            | ids::RECORD_COPY_1
            | ids::RECORD_TRANSFERFIELDS_1
            | ids::RECORD_TRANSFERFIELDS_2
    )
}

/// Whether builtin `id` on a RecordRef or FieldRef receiver with operands `operands` leaves the [`TABLE_COPY`] record it works
/// on as it is. Only the calls corpus PR 1387 ran between GetTable and SetTable(Rec, true) (streams/BlobCopyNeighbors.al:2446
/// `ShareFindCalc`, `ShareFieldValue`, `ShareModify`, `ShareInsert`, `ShareSetRange`, `ShareReset`): `RecordRef.Field(No)`,
/// `FindFirst()`, `Modify()`, `Insert()`, `Reset()`, `FieldRef.CalcField()`, `FieldRef.SetRange` with one value,
/// `FieldRef.Value :=` (one operand), and SetTable(Rec, true) itself, which moves the Blobs ([`BlobMove::SetTableShared`]).
/// The record methods these reach are [`after`]'s.
fn recordref_keeps_table_copy(id: BuiltinId, operands: &[Value]) -> bool {
    matches!(
        (id, operands.len()),
        (
            ids::RECORDREF_FIELD_2 | ids::FIELDREF_SETRANGE_1 | ids::FIELDREF_VALUE_1,
            1
        ) | (ids::RECORDREF_SETTABLE_2, 2)
            | (
                ids::RECORDREF_FINDFIRST_1
                    | ids::RECORDREF_MODIFY_1
                    | ids::RECORDREF_INSERT_1
                    | ids::RECORDREF_RESET_1
                    | ids::FIELDREF_CALCFIELD_1,
                0
            )
    )
}

/// The [`TABLE_COPY`] record of RecordRef, FieldRef or KeyRef value `v` is a measured source no longer.
fn end_table_copy(v: &Value) {
    if let Some(rc) = ref_record(v)
        && rc.borrow().probed_source.0 == TABLE_COPY
    {
        rc.borrow_mut().probed_source.0 = 0;
    }
}

/// The record a RecordRef, FieldRef or KeyRef value works on.
fn ref_record(v: &Value) -> Option<Rc<RefCell<RecordState>>> {
    let Value::Library(_, lib) = v else {
        return None;
    };
    match &*lib.borrow() {
        LibraryValue::RecordRef(r)
        | LibraryValue::FieldRef(r, _)
        | LibraryValue::KeyFieldRef(r, _)
        | LibraryValue::KeyRef(r, _)
        | LibraryValue::UserNameFieldRef { rec: r, .. } => Some(r.clone()),
        _ => None,
    }
}

/// Builtin `id` is about to run on receiver `recv` and operands `operands`: the one place a record goes by reference into
/// platform code. A record among them is a measured source no longer, whatever the builtin does with it (a page that hands
/// back the picked row, `SetSelectionFilter`, `Codeunit.Run`, `Page.GetRecord`), unless [`builtin_keeps_probed`] names the
/// builtin. A Record receiver is the record of a record method, which reaches `Session::record_op` and is [`after`]'s. The
/// [`TABLE_COPY`] record of a RecordRef, FieldRef or KeyRef receiver or operand ends too, unless the receiver's call is one
/// [`recordref_keeps_table_copy`] names. `sources` is the session's [`Sources`]: without any measured source there is
/// nothing to end.
#[inline]
pub(crate) fn records_into_builtin(sources: Sources, id: BuiltinId, recv: Option<&Value>, operands: &[Value]) {
    if !sources.any() {
        return;
    }
    let own = recv.filter(|v| !matches!(v, Value::Record(_)));
    for v in own.into_iter().chain(operands) {
        if let Value::Record(rc) = v
            && rc.borrow().probed_source.0 != 0
            && !builtin_keeps_probed(id)
        {
            rc.borrow_mut().probed_source.0 = 0;
        }
    }
    if let (Some(r), false) = (recv, recordref_keeps_table_copy(id, operands)) {
        end_table_copy(r);
    }
    operands.iter().for_each(end_table_copy);
}

/// The events whose by-value subscribers the probes measured.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum BlobEvent {
    /// An event a publisher procedure raises ([`measured_publisher`]); a manual subscriber too.
    Publisher,
    /// A table event of a record that is not temporary ([`measured_table_event`]); not a manual subscriber.
    TableEvent,
}

/// Whether publisher routine `r` of an object of kind `object` raises an event whose by-value subscribers the probes
/// measured: a procedure of a codeunit with exactly `[IntegrationEvent(false, false)]`, `[IntegrationEvent(true, false)]` or
/// `[BusinessEvent(false)]` (corpus PR 1387, streams/BlobCopyNeighbors.al:1792-:1948; PR 1375, streams/BlobCopyStates.al:
/// 388-403), or a procedure of a table with exactly `[IntegrationEvent(false, false)]` (PR 1387 :1974, :2000). Every other
/// publisher is not measured.
pub(crate) fn measured_publisher(object: al_ir::ObjKind, r: &al_ir::Routine) -> Option<BlobEvent> {
    let [Attribute { name, args, .. }] = &r.attributes[..] else {
        return None;
    };
    let integration = |include_sender: bool| {
        name.eq_ignore_ascii_case("IntegrationEvent")
            && args[..] == [Const::Boolean(include_sender), Const::Boolean(false)]
    };
    let business = name.eq_ignore_ascii_case("BusinessEvent") && args[..] == [Const::Boolean(false)];
    let measured = match object {
        al_ir::ObjKind::Codeunit => integration(false) || integration(true) || business,
        al_ir::ObjKind::Table => integration(false),
        _ => false,
    };
    measured.then_some(BlobEvent::Publisher)
}

/// Whether table event `event` (lower-case) of a record that is `temporary` or not is one whose by-value subscribers the
/// probes measured: OnBeforeInsertEvent, OnAfterInsertEvent, OnBeforeModifyEvent and OnAfterModifyEvent of a record that is
/// not temporary (corpus PR 1387, streams/BlobCopyNeighbors.al:2078, :3205 `SeqStoredModifySubscriber`), and OnAfterModifyEvent
/// of a temporary record (:3243 `SeqTempModifySubscriber`). The other table events are not measured.
pub(crate) fn measured_table_event(event: &str, temporary: bool) -> Option<BlobEvent> {
    let measured = matches!(
        (event, temporary),
        (
            "onbeforeinsertevent" | "onafterinsertevent" | "onbeforemodifyevent",
            false
        ) | ("onaftermodifyevent", _)
    );
    measured.then_some(BlobEvent::TableEvent)
}

/// Whether a subscriber, `manual` or not, of an event `event` names is called in a measured call ([`Call::Subscriber`]): a
/// manual subscriber only of a publisher (PR 1387 :1948).
pub(crate) fn measured_subscriber(event: Option<BlobEvent>, manual: bool) -> bool {
    matches!(
        (event, manual),
        (Some(BlobEvent::Publisher), _) | (Some(BlobEvent::TableEvent), false)
    )
}

/// The by-value copy `copy` the publisher made of record `source` for its subscribers: a source prepared as `source` when
/// `source` is a measured variable and the publisher is one [`measured_publisher`] names (`measured`), so that a by-value
/// subscriber takes the copy [`Argument::measured_call`] measures (PR 1387 :1792 a codeunit's IntegrationEvent with a by-value
/// record, :1870 a BusinessEvent, :1922 IncludeSender, :1974 a table's IntegrationEvent).
pub(crate) fn publisher_copy(measured: bool, source: &Rc<RefCell<RecordState>>, copy: &Value) {
    if let (true, Value::Record(c)) = (measured, copy) {
        let ProbedSource(b, at) = source.borrow().probed_source;
        if matches!(b, FRESH | MEASURED) {
            c.borrow_mut().probed_source = ProbedSource(b, at);
        }
    }
}

/// What the target of the record assignment `Target := Source` is afterwards, from what both were and the session's commit
/// count `commits`: [`MEASURED`] when both are measured variables (corpus PR 1387, streams/BlobCopyNeighbors.al:257, :261 state asg;
/// :2650, :2943 `Src := Other` after an Insert, measured at :3205, :3243).
pub(crate) fn assigned(target: &RecordState, source: &RecordState, commits: u64) -> ProbedSource {
    let both = measured_variable(target, commits) && measured_variable(source, commits);
    ProbedSource(if both { MEASURED } else { 0 }, commits)
}

/// What the target of `RecordRef.SetTable(Target [, share])` is afterwards, from what it was and the session's commit count
/// `commits`: [`MEASURED`] when it was a measured variable and the call shares no table (corpus PR 1387,
/// streams/BlobCopyNeighbors.al:2424: after GetTable of the same or another record, `SetTable(Src, false)`, Open and
/// FindFirst, CalcField, `FieldRef.Value :=` and `RecordRef.Modify`).
pub(crate) fn set_table_target(target: &RecordState, share: bool, commits: u64) -> ProbedSource {
    let kept = matches!((share, measured_variable(target, commits)), (false, true));
    ProbedSource(if kept { MEASURED } else { 0 }, commits)
}

/// A `FieldRef.Validate` wrote the record `r` of a RecordRef: it is a measured source no longer (not measured).
pub(crate) fn validated_through_fieldref(r: &mut RecordState) {
    r.probed_source.0 = 0;
}

/// A second RecordRef handle on record `r` was made through a Variant or a parameter: a [`TABLE_COPY`] record is a measured
/// source no longer (a RecordRef assignment keeps it, PR 1387 :2446 `ShareAssigned`).
pub(crate) fn recordref_aliased(r: &Rc<RefCell<RecordState>>) {
    if r.borrow().probed_source.0 == TABLE_COPY {
        r.borrow_mut().probed_source.0 = 0;
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
    /// if the probes measured it: a procedure of a codeunit without attributes (corpus PR 1387, streams/
    /// BlobCopyNeighbors.al:587-:1081; PR 1375, streams/BlobCopyStates.al:3135 another codeunit, :3153 an interface method),
    /// a procedure of a table without attributes or with exactly `[TryFunction]` (PR 1387 :2026, :2052), a codeunit's
    /// `[TryFunction]` (PR 1375 :2200), and a codeunit's event subscriber when the call is the raise of an event to a
    /// subscriber [`measured_subscriber`] names (`measured_event`; PR 1387 :1792-:2078). The arguments of a `var` parameter, a
    /// trigger, a procedure of a page or a report, a procedure with any other attribute and every other subscriber are bound
    /// as before (an event publisher too: its by-value parameter is the copy its subscribers get, [`publisher_copy`]).
    pub(crate) fn measured_call(object: al_ir::ObjKind, callee: &al_ir::Routine, measured_event: bool) -> Option<Call> {
        let only = |name: &str| matches!(&callee.attributes[..], [a] if a.name.eq_ignore_ascii_case(name));
        if callee.is_trigger {
            return None;
        }
        match object {
            al_ir::ObjKind::Codeunit if callee.attributes.is_empty() => Some(Call::Plain),
            al_ir::ObjKind::Codeunit if only("TryFunction") => Some(Call::TryFunction),
            al_ir::ObjKind::Codeunit if measured_event && only("EventSubscriber") => Some(Call::Subscriber),
            al_ir::ObjKind::Table if callee.attributes.is_empty() || only("TryFunction") => Some(Call::TableProcedure),
            _ => None,
        }
    }

    /// The argument `a` of a call for its parameter of type `param`, with `commits` the session's commit count. `call` is
    /// [`Argument::measured_call`] for a by-value parameter.
    pub(crate) fn of(a: &Value, call: Option<Call>, param: &RtType, commits: u64) -> Argument {
        let taker = match param {
            RtType::Record(_, false) => Taker::RecordParam,
            RtType::Record(_, true) => Taker::TemporaryParam,
            RtType::Variant => Taker::VariantParam,
            _ => return Argument::Other,
        };
        match (a, call) {
            (Value::Record(rc), Some(call)) => {
                let r = rc.borrow();
                Argument::Record(r.temporary, Holder::of(&r, commits), taker, call)
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
            BlobMove::Variant | BlobMove::ReadIntoVariant { .. } => "an assignment to a Variant",
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
/// A source in a record the probes measured as a source, in a move [`record_cell`] names: a copy, or for a field read of a
/// Blob a read handed out unloaded an empty Blob.
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
        | BlobMove::Variant
            if content || empty =>
        {
            Some(Cell::Copy)
        }
        BlobMove::ReadIntoVariant { .. } if record_cell(how, s, holder) && s.row_copy && !s.loaded => {
            Some(Cell::Unread)
        }
        _ if record_cell(how, s, holder) => Some(Cell::Copy),
        BlobMove::FieldRefValidate | BlobMove::Variant if s.unread_row => Some(Cell::Unread),
        BlobMove::SetTable if s.unread_row => Some(Cell::UnreadRow),
        _ => None,
    }
}

/// Whether move `how` of source `s` in record `holder` is a cell corpus PR 1375 (streams/BlobCopyStates.al) and PR 1387
/// (streams/BlobCopyNeighbors.al) measured from a measured source record, in every preparation they ran: BC copies the Blob
/// in each. The buffer carries no mark of another rule (`Source::plain_buffer`).
/// - A record by value from a measured variable ([`Holder::Fresh`], [`Holder::Measured`]), as [`by_value_measured`] says.
/// - Record.Copy from a measured variable into a record, both `temporary` or not, with ShareTable or not (PR 1387
///   :1204-:1308).
/// - TransferFields with flags between two records that are both `temporary` or both not, from a measured variable (PR 1387
///   :769, :1107; PR 1375 :1093; corpus PR 1372, streams/BlobValueCopyShapes.al:741-:781).
/// - SetTable(Rec, true) from a [`Holder::TableCopy`] record (PR 1375 :1709; PR 1387 :2446).
/// - A field read of a measured variable, `temporary` or not, stored in a global Variant or an element of a local array or
///   handed to a by-value Variant parameter (PR 1387 :1434-:1564): a Blob a read handed out unloaded (state get) gives an
///   empty Blob there (HasValue false), every other one a copy.
///
/// Every other move, holder and pair keeps the shared buffer, as before.
fn record_cell(how: BlobMove, s: &Source, holder: Holder) -> bool {
    s.plain_buffer
        && match how {
            BlobMove::RecordByValue {
                source_temporary,
                taker,
                call,
            } => holder.variable() && by_value_measured(source_temporary, taker, call),
            BlobMove::RecordCopy { .. }
            | BlobMove::TransferFields(TransferCall::Flags { temporary: Some(_) })
            | BlobMove::ReadIntoVariant { .. } => holder.variable(),
            BlobMove::SetTableShared => holder == Holder::TableCopy,
            _ => false,
        }
}

/// Whether a record from a source record that is `temporary` or not (`source_temporary`), taken by `taker` in a call or
/// assignment of kind `call`, was measured (corpus PR 1387, streams/BlobCopyNeighbors.al).
/// - A procedure of a codeunit, or an assignment to a Variant: every taker, from either kind of record (:587-:769 not
///   temporary, :925-:1107 temporary).
/// - A table procedure, a `[TryFunction]` of a table or a codeunit, an event subscriber: a record that is not temporary into
///   a Record parameter (:1792-:2078; PR 1375, streams/BlobCopyStates.al:2200).
///
/// Every other pair and call keeps the shared buffer, as before.
fn by_value_measured(source_temporary: bool, taker: Taker, call: Call) -> bool {
    matches!(
        (call, source_temporary, taker),
        (Call::Plain, _, _)
            | (
                Call::TableProcedure | Call::TryFunction | Call::Subscriber,
                false,
                Taker::RecordParam
            )
    )
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
            | BlobMove::Variant
    )
}

/// Whether a move of a field of kind `kind` (not [`FieldKind::Plain`] nor [`FieldKind::Other`]) by `how` takes what a
/// plain field takes in the cells corpus PR 1375 measured (streams/BlobCopyStates.al): content a stream wrote in memory in a
/// [`Holder::Fresh`] variable (state mem), moved by a record that is not temporary by value to a Record parameter of a
/// codeunit procedure (:3108 C6, every kind), as a field read of a temporary record handed to a by-value Variant parameter
/// (:3017 C1-C3) or stored in a global Variant or an array element (:3087 C4, C5), and by Record.Copy between two records
/// that are not temporary for a `Compressed = false` field written in memory in a measured variable, also one inserted
/// before the field was written (:3274 E2).
fn other_kind_move(how: BlobMove, kind: FieldKind, s: &Source, holder: Holder) -> bool {
    let written_in_memory = s.plain_buffer && s.in_memory && s.content && s.written;
    let mem = holder == Holder::Fresh && written_in_memory;
    match how {
        BlobMove::RecordByValue {
            source_temporary: false,
            taker: Taker::RecordParam,
            call: Call::Plain,
        }
        | BlobMove::ReadIntoVariant { temporary: true, .. } => mem,
        BlobMove::RecordCopy {
            source_temporary: false,
            target_temporary: false,
        } => holder.variable() && written_in_memory && kind == FieldKind::Variation,
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
        | BlobMove::Variant => s.loaded && s.content,
        _ => false,
    }
}

impl Session {
    /// What record state `r` is as the source of a move ([`Holder::of`] with the session's commit count).
    pub(crate) fn holder(&self, r: &RecordState) -> Holder {
        Holder::of(r, self.commits)
    }

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
        let (mut copy, holder, measured) = {
            let src = src.borrow();
            (
                al_builtins::record_into_recordref::recordref_copy(&src),
                self.holder(&src),
                measured_variable(&src, self.commits),
            )
        };
        // The RecordRef's record is a measured source when the record is a measured variable with no Commit since it was
        // made: SetTable(Rec, true) moves its Blobs on (corpus PR 1375, streams/BlobCopyStates.al:1709; PR 1387,
        // streams/BlobCopyNeighbors.al:2446).
        if from_record && measured {
            copy.probed_source = ProbedSource(TABLE_COPY, self.commits);
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
    /// `locals`, moves into it ([`BlobMove::Variant`], [`BlobMove::GlobalVariant`]): a Blob moves as [`moved`] says, with the
    /// record and the field kind of the read (corpus PR 1387, streams/BlobCopyNeighbors.al:1434-:1564, records `temporary`
    /// or not). Anything else stays as it is.
    pub(crate) fn field_read_into_variant(&self, v: Value, read: FieldRead, locals: &[Value]) -> Result<Value, Signal> {
        let (Value::Blob(_), Some(Value::Record(rc))) = (&v, locals.get(usize::from(read.record))) else {
            return Ok(v);
        };
        let (temporary, holder, table) = {
            let r = rc.borrow();
            (r.temporary, self.holder(&r), r.table as usize)
        };
        let kind = || self.blob_field_kind(table, read.pos as usize);
        let how = BlobMove::ReadIntoVariant {
            global: read.global,
            temporary,
        };
        moved(None, v, how, holder, kind)
    }

    /// Record method `method` starts on `rec` with arguments `args`: the fact of the record goes on as [`after`] says, or
    /// ends. A [`FRESH`] variable becomes [`MEASURED`] only when no Commit ran since it was made ([`measured_variable`]).
    #[inline]
    pub(crate) fn probed_source_before(&self, method: RecordOp, rec: &Rc<RefCell<RecordState>>, args: &[Value]) {
        let ProbedSource(b, at) = rec.borrow().probed_source;
        if b == 0 {
            return;
        }
        let next = match (b, after(b, method, args)) {
            (FRESH, MEASURED) if measured_variable(&rec.borrow(), self.commits) => MEASURED,
            (FRESH, MEASURED) => 0,
            (_, next) => next,
        };
        if next != b {
            rec.borrow_mut().probed_source = ProbedSource(next, at);
        }
    }
}
