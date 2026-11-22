//! The view a record gets when it is handed by value to a Record parameter of a procedure or of an event subscriber: the
//! parameter is not declared temporary and is of a table that is not TableType = Temporary. Entry:
//! `Session::record_param_copy` for a record of a stored table, `Session::temporary_into_stored_param` for a temporary one (by
//! the parameter binding of `call_inner` in `lib.rs`, which describes the call in a [`ByValueCall`]; for a subscriber,
//! `dispatch.rs` `raise_to_subscribers` asks [`Session::event_args`] what was measured of each parameter). What a copy whose view is not pinned
//! answers afterwards: its own record methods (`Session::by_value_view_runs`, by `record_op_inner`), and every hand-off of it to
//! code that reads its view (`Session::by_value_view_handed`: a record method with it as the source, `RecordRef.GetTable`, a
//! page, report, XmlPort or FilterPageBuilder it is handed to); Copy and CopyFilters hand its marks to their target.

use crate::Session;
use crate::events::{EventKind, Subscriber};
use crate::records::{FLOWFIELD_KEY, Rec};
use al_builtins::assumptions::{self, Assumption};
use al_builtins::native::{Signal, unsupported};
use al_builtins::record_op::RecordOp;
use al_builtins::value::{Instance, RecordState, RtType, Value};
use al_ir::{self as ir, ObjKind};
use std::cell::RefCell;
use std::rc::Rc;

/// The `RecordState::unpinned_buffer` reason of a by-value copy whose view [`Session::by_value_view`] does not decide.
pub(crate) const BY_VALUE_VIEW: &str = "a record handed by value whose view the probe did not measure (a filter in filter group 1 or in a group other than -1, 0, 2 and 4, the caller standing in a group other than 0 and 2, a field sorted descending by SetAscending, a SetLoadFields after a read, a SetAutoCalcFields outside the measured shape or ReadIsolation on the record, a call through an interface, a parameter of an object other than a codeunit or of a table procedure other than Row.Proc(Row) or Buffer.Proc(Entry) of another table from a codeunit or a third table, or an event subscriber's parameter whose publisher parameter is a by-value Record, other than in the one cell record/filters/ByValueRecordView.al:286-308 and :964-978 measures): what the copy's view holds is not pinned";

/// The `RecordState::unpinned_buffer` reason of the copy an event subscriber's parameter gets in the cell
/// record/filters/ByValueRecordView.al:286-308 measures ([`subscriber_view`]): the subscriber reads GetFilters and Count of it
/// only (:306), so every other read of its view refuses.
pub(crate) const BY_VALUE_EVENT_READS: &str = "a by-value event subscriber record read other than by GetFilters and Count, the two reads record/filters/ByValueRecordView.al:306 measures: what it answers is not pinned";

/// Whether `why` is one of the marks of a by-value copy whose view is not pinned for every read: [`BY_VALUE_VIEW`] and
/// [`BY_VALUE_EVENT_READS`]. Field reads of such a copy run (`records.rs` `buffer_read_refused`).
pub(crate) fn by_value_mark(why: &str) -> bool {
    why == BY_VALUE_VIEW || why == BY_VALUE_EVENT_READS
}

/// What record/filters/ByValueRecordView.al pins of one Record parameter of an event subscriber ([`Session::event_args`]).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum EventArg {
    /// The parameter of the measured cell (record/filters/ByValueRecordView.al:286-308): the publisher, its parameter, the raise
    /// and the subscriber have every fact [`Session::event_publisher`] and [`Session::event_args`] name. The caller's record
    /// is checked at the copy (`Session::cell_record`).
    Measured,
    /// The publisher's parameter is a by-value Record, the fact the cell measured (:294), but another fact of the publisher
    /// or the subscriber is not the cell's.
    PublisherByValue,
    /// Every other parameter: the publisher's parameter is `var` or not a Record, the subscriber's parameter takes the
    /// sender, or the platform raises the event (its Rec and xRec are `var`). Not measured; the copy keeps main's view.
    Main,
}

/// The publisher routine of one raise (`publish` sets it) and whether its own facts are the measured cell's. Made once per
/// raise by [`Session::event_publisher`]; [`Session::event_args`] reads it for each subscriber.
pub(crate) struct EventPublisher {
    object: usize,
    routine: usize,
    measured: bool,
}

/// The evidence for a callee's by-value view ([`Session::measured_callee`]): a corpus pin, or an assumption of the registry.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Evidence {
    Pinned,
    Assumed(Assumption),
}

impl Evidence {
    fn assumption(self) -> Option<Assumption> {
        match self {
            Evidence::Pinned => None,
            Evidence::Assumed(a) => Some(a),
        }
    }
}

/// How the procedure a record is handed to by value is reached.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Dispatch {
    /// A call of the procedure by name.
    Direct,
    /// An event raise to a subscriber (`dispatch.rs` sets `next_is_subscriber` and `next_event_args`).
    Subscriber,
    /// A call through an interface (`exec/run.rs` `run_iface` sets `Session::next_interface_call`).
    Interface,
}

/// The call a by-value Record parameter belongs to: the callee's object, the instance the procedure runs on (a table
/// procedure's holds its `Rec`, `enter_record`), how it is reached and, for a subscriber, what
/// [`Session::event_args`] says of each of its parameters. Made once per call by `call_inner` in `lib.rs`.
pub(crate) struct ByValueCall<'a> {
    callee: usize,
    instance: &'a Rc<RefCell<Instance>>,
    dispatch: Dispatch,
    event_args: Vec<EventArg>,
}

impl<'a> ByValueCall<'a> {
    /// `subscriber` and `interface`: the call is an event raise or an interface call; `event_args`: per parameter of a
    /// subscriber, what [`Session::event_args`] says of it (each taken once, at the call).
    pub(crate) fn new(
        callee: usize,
        instance: &'a Rc<RefCell<Instance>>,
        subscriber: bool,
        interface: bool,
        event_args: Vec<EventArg>,
    ) -> Self {
        let dispatch = match (subscriber, interface) {
            (true, _) => Dispatch::Subscriber,
            (false, true) => Dispatch::Interface,
            (false, false) => Dispatch::Direct,
        };
        ByValueCall {
            callee,
            instance,
            dispatch,
            event_args,
        }
    }

    /// What [`Session::event_args`] says of parameter `index` of a subscriber; [`EventArg::Main`] for one it left out.
    fn event_arg(&self, index: usize) -> EventArg {
        self.event_args.get(index).copied().unwrap_or(EventArg::Main)
    }
}

/// Whether record method `method` reads neither the view (filters, key, order) nor the marks of the record it is called on: it
/// reads or writes the record's own fields or its row by primary key, reads the table's metadata, or names the fields to load.
/// The one list [`Session::by_value_view_read`] and [`Session::by_value_view_runs`] take.
fn reads_no_view(method: RecordOp) -> bool {
    matches!(
        method,
        RecordOp::Get
            | RecordOp::GetBySystemId
            | RecordOp::Insert
            | RecordOp::Modify
            | RecordOp::Delete
            | RecordOp::Rename
            | RecordOp::Init
            | RecordOp::TestField
            | RecordOp::Validate
            | RecordOp::FieldNo
            | RecordOp::FieldName
            | RecordOp::FieldCaption
            | RecordOp::FieldError
            | RecordOp::IsTemporary
            | RecordOp::TableName
            | RecordOp::TableCaption
            | RecordOp::RecordId
            | RecordOp::TransferFields
            | RecordOp::SetLoadFields
            | RecordOp::AddLoadFields
            | RecordOp::LockTable
            | RecordOp::IsDirty
    )
}

/// Whether record/filters/ByValueRecordView.al pins the view of `s`, handed by value to a callee [`Session::measured_callee`]
/// names: every filter in filter
/// group -1, 0, 2 or 4 (`ByValue_GroupMinus1Filter`, `ByValue_GroupMinus1_TwoFields`, `ByValue_Group0Filter_*`,
/// `ByValue_Group2Filter`, `ByValue_Group4Filter`, `ByValue_FlowFieldFilter_*`, `ByValue_CalcFields_*`), the caller standing
/// in group 0 or 2 (`ByValue_CallerInGroup2_*`), no field sorted descending by SetAscending. The key (primary, declared
/// secondary in `ByValue_SecondaryKey_*`, ad hoc in `ByValue_AdHocKey_Group0Filter`), Ascending (`ByValue_Descending_*`)
/// and the marks with MarkedOnly on or off (`ByValue_Marks_*`) are each measured. A filter in group 1 is seen by every
/// record of the table in the session (`ByValue_Group1Filter_OtherInstance`), which the runtime does not model.
/// `auto_calc_dropped`: the caller's SetAutoCalcFields is one [`Session::measured_auto_calc`] names, and the copy drops it.
fn measured_view(s: &RecordState, auto_calc_dropped: bool) -> bool {
    s.filters.iter().all(|f| matches!(f.group, -1 | 0 | 2 | 4))
        && matches!(s.filter_group, 0 | 2)
        && s.field_descending == 0
        && (measured_state(s) || auto_calc_dropped)
}

/// Whether the probe measured what else `s` holds that a copy could keep: no SetLoadFields, or one before any read
/// (`ByValue_SetLoadFields`; `load.buffer` is set by a read), no SetAutoCalcFields, ReadIsolation at its default. Every cell
/// of the probe has these. SecurityFiltering of a by-value parameter is pinned on its own
/// (al-language/reports/runtime/SFSecurityFilteringFlow.al, `RecordState::security_filtering_unpinned`).
fn measured_state(s: &RecordState) -> bool {
    (s.load.request.is_none() || s.load.buffer.is_none()) && s.auto_calc.is_empty() && s.read_isolation == 0
}

/// Whether every filter of `s` is in filter group 0 and `s` otherwise has the view of a fresh record: standing in group 0,
/// the primary key (no ad hoc key), ascending, no field sorted descending, no marks, MarkedOnly off, no SetLoadFields, and
/// the state [`measured_state`] names. The caller's record of `ByValue_EventSubscriberParameter`
/// (record/filters/ByValueRecordView.al:967-972, `SetRange(Status, Posted)` on a fresh record) and of
/// `ByValue_TemporaryToPlainParameter` (:984-995).
fn group0_view(s: &RecordState) -> bool {
    s.filters.iter().all(|f| f.group == 0)
        && s.filter_group == 0
        && s.key == 0
        && s.adhoc.is_none()
        && s.ascending
        && s.field_descending == 0
        && s.marks.is_empty()
        && !s.marked_only
        && s.load.request.is_none()
        && measured_state(s)
}

/// Whether `s` has the view of a fresh record ([`primary_view`]) and no SetLoadFields: a by-value copy of it has no filter,
/// key, order or mark that a rule could drop.
fn fresh_view(s: &RecordState) -> bool {
    s.filters.is_empty() && group0_view(s)
}

/// The view of `copy`, the by-value copy of `s` for a Record parameter of an event subscriber, of which
/// [`Session::event_args`] says `arg`.
/// - [`EventArg::Main`]: the copy keeps main's view (not measured).
/// - `s` has the view of a fresh record ([`fresh_view`]): the copy keeps it (main; there is nothing to drop).
/// - [`EventArg::Measured`] and `cell_record` (`s` is the caller's record of the cell, `Session::cell_record`): the view of
///   [`primary_view`] (record/filters/ByValueRecordView.al:975 `sub f={} n=3;`), marked [`BY_VALUE_EVENT_READS`]: only
///   GetFilters and Count of it run (:306).
/// - Else (the publisher's parameter is a by-value Record) the copy keeps main's view and is marked [`BY_VALUE_VIEW`]: a
///   read of its view refuses. A copy of a marked `s` keeps main's view and the mark.
fn subscriber_view(s: &RecordState, copy: &Rec, arg: EventArg, cell_record: bool) {
    let mut c = copy.borrow_mut();
    match (s.unpinned_buffer, arg) {
        (Some(_), _) | (None, EventArg::Main) => {}
        (None, _) if fresh_view(s) => {}
        (None, EventArg::Measured) if cell_record => {
            primary_view(&mut c);
            c.unpinned_buffer = Some(BY_VALUE_EVENT_READS);
        }
        (None, EventArg::Measured | EventArg::PublisherByValue) => c.unpinned_buffer = Some(BY_VALUE_VIEW),
    }
}

/// Whether `a` is the subscriber attribute of the cell (record/filters/ByValueRecordView.al:301): `[EventSubscriber]` with six
/// arguments, the element '' and SkipOnMissingLicense and SkipOnMissingPermission `false`.
fn cell_subscriber_attribute(a: &ir::Attribute) -> bool {
    let args = crate::events::split_args(&a.text);
    a.name.eq_ignore_ascii_case("EventSubscriber")
        && matches!(args.as_slice(), [_, _, _, element, license, permission]
            if crate::events::unquote(element).is_empty()
                && license.eq_ignore_ascii_case("false")
                && permission.eq_ignore_ascii_case("false"))
}

/// The view of a fresh record: no filters in any filter group, filter group 0, the primary key ascending, no marks,
/// MarkedOnly off. What a by-value copy gets in [`Session::by_value_view`] (each callee:
/// `f={} n=3 key=Id asc=Yes grp=0 first=1`, `mo=No n=3 marked n=0`), and a copy for a temporary parameter (al-vm
/// `Session::temporary_param_copy`).
pub(crate) fn primary_view(st: &mut RecordState) {
    st.key = 0;
    st.adhoc = None;
    st.ascending = true;
    st.field_descending = 0;
    st.marks.clear();
    st.marked_only = false;
    st.filters.clear();
    st.filter_group = 0;
}

impl Session {
    /// The copy of `rc` a by-value parameter of type `param` of the procedure of `call` gets (`record_by_value_into`): it keeps
    /// what Record.IsDirty compares with (record/Y8RecordIsDirtyShapes.al:321-324 `by_value_dirty` Yes, `by_value_clean` No)
    /// and has the view [`Session::by_value_view`] gives it. `index`: the parameter's position.
    pub(crate) fn record_param_copy(&mut self, rc: &Rec, param: &RtType, call: &ByValueCall, index: usize) -> Value {
        let v = self.record_by_value_into(rc, param);
        if let Value::Record(copy) = &v {
            copy.borrow_mut().is_dirty_base = rc.borrow().is_dirty_base.copy();
            self.by_value_view(rc, copy, param, call, index);
        }
        v
    }

    /// The publisher of one raise (`publisher`: the routine `publish` raises and whether its argument is a `var` parameter of
    /// the raising routine, [`Session::raise_arg_var_param`]; `None` for an event the platform raises) and whether its own
    /// facts are those of the pinned cell (record/filters/ByValueRecordView.al:286-296), every fact by equality: a codeunit
    /// that states no property (:286-296), the event a `local` procedure with exactly one parameter (:294), declared
    /// `[IntegrationEvent(false, false)]` with IncludeSender, GlobalVarAccess and Isolated off and no `[CommitBehavior]`
    /// attribute (:293), raised by a call of the publisher procedure whose argument is a `var` parameter of the raising
    /// procedure (:288-290).
    pub(crate) fn event_publisher(&self, publisher: Option<(usize, usize, bool)>) -> Option<EventPublisher> {
        let (object, routine, var_param_arg) = publisher?;
        let engine = &self.engine;
        let o = &engine.program.objects[object];
        let r = &o.routines[routine];
        let measured = o.kind == ObjKind::Codeunit
            && o.properties.is_empty()
            && r.local
            && r.params == 1
            && var_param_arg
            && engine.routine_facts(object, routine).commit_behavior.is_none()
            && engine.events.publishers.get(&(object, routine)).is_some_and(|p| {
                p.kind == EventKind::Integration && !p.include_sender && !p.global_var_access && !p.isolated
            });
        Some(EventPublisher {
            object,
            routine,
            measured,
        })
    }

    /// Per parameter of subscriber `s` (`map`: the publisher's parameter each one takes) of the raise of `publisher`
    /// ([`Session::event_publisher`]), one of `subscribers` the raise reaches, what the corpus pins of it
    /// (record/filters/ByValueRecordView.al:286-308, `ByValue_EventSubscriberParameter` :964-978). [`EventArg::Measured`],
    /// every fact by equality: the publisher's own facts are the cell's ([`EventPublisher`]); its parameter is a Record of a
    /// table that is not TableType = Temporary, passed by value (:294); `s` is the event's only subscriber, of a codeunit that
    /// states no property (:299-300), a `local` procedure with exactly one parameter, a Record of the same table, by value
    /// (:302), under `[EventSubscriber(.., '', false, false)]`: no element, SkipOnMissingLicense and SkipOnMissingPermission
    /// false (:301).
    /// [`EventArg::PublisherByValue`]: the publisher's parameter is a by-value Record, another fact differs.
    /// [`EventArg::Main`]: every other parameter, and each one of an event the platform raises (`publisher` `None`); an
    /// empty list when no parameter is anything else.
    pub(crate) fn event_args(
        &self,
        publisher: Option<&EventPublisher>,
        s: &Subscriber,
        map: &[Option<usize>],
        subscribers: usize,
    ) -> Vec<EventArg> {
        let Some(publisher) = publisher else {
            return Vec::new();
        };
        let engine = &self.engine;
        let objects = &engine.program.objects;
        let p = &objects[publisher.object].routines[publisher.routine];
        let by_value_record =
            |j: usize| p.by_var.get(j) == Some(&false) && matches!(p.locals[j].ty, RtType::Record(..));
        if !map.iter().any(|m| m.is_some_and(by_value_record)) {
            return Vec::new();
        }
        let sub = &objects[s.object];
        let r = &sub.routines[s.routine];
        let measured_subscriber = subscribers == 1
            && sub.kind == ObjKind::Codeunit
            && sub.properties.is_empty()
            && r.local
            && r.params == 1
            && s.element.is_empty()
            && r.attributes.iter().any(cell_subscriber_attribute);
        map.iter()
            .enumerate()
            .map(|(i, m)| match *m {
                Some(j) if by_value_record(j) => {
                    let measured = publisher.measured
                        && measured_subscriber
                        && matches!(p.locals[j].ty, RtType::Record(t, false) if !engine.object_facts[t as usize].temporary())
                        && r.by_var.get(i) == Some(&false)
                        && r.locals[i].ty == p.locals[j].ty;
                    if measured { EventArg::Measured } else { EventArg::PublisherByValue }
                }
                _ => EventArg::Main,
            })
            .collect()
    }

    /// The evidence for `source` handed by value to a Record parameter of type `param` of the procedure of `call`, called by
    /// name; `None` when there is none. An interface call is not measured.
    /// - [`Evidence::Pinned`] (record/filters/ByValueRecordView.al): a codeunit's procedure (`BVV
    ///   Callee`, and `DescribeLocal`, a local procedure of the test codeunit), or a table's procedure whose parameter is a
    ///   record of that table, called from a codeunit on `source` itself (`CountSameStatus`, called as
    ///   `Row.CountSameStatus(Row)` from the test codeunit in `ByValue_TableProcedure_OwnRecByValue`).
    /// - `by-value-table-proc-other-table` (corpus PR 1451, record/filters/ByValueTableProcedureView.al, open): a table's
    ///   procedure whose parameter is a record of another table, called on a record of that table from a codeunit
    ///   (`Buffer.DescribeEntry(Entry)` from the test codeunit, :286-662, and from `BVT Caller`, a codeunit that is not a test
    ///   codeunit, :342-361) or from the procedure or trigger of a third table (`BVT Holder`.CollectFromTable and OnInsert,
    ///   :672-696; OnValidate, OnModify and OnDelete of `BVT Trigger Holder`, :785-833), on a record of a TableType =
    ///   Temporary table (`BVT Buffer`), of a normal table (`BVT Holder`, :632) and on a temporary record of it (:652), with a
    ///   filter on a FlowField too (:699); the source a global record of a codeunit (:718), a var parameter (:730), Rec of a
    ///   codeunit with TableNo (:748) and an event subscriber's var parameter, the subscriber the caller (:766). Every cell:
    ///   `f={} n=4 key=Entry No. asc=Yes grp=0`. Base
    ///   Application's codeunit 5530 "Calc. Item Availability" calls table 5530 "Inventory Event Buffer".TransferInventoryQty
    ///   this way (al-runner issue 8385). A call from the code of the callee's or the parameter's table, and a table
    ///   extension's procedure (an object of its own kind), are not measured.
    fn measured_callee(&self, call: &ByValueCall, param: &RtType, source: &Rec) -> Option<Evidence> {
        let objects = &self.engine.program.objects;
        let callee = &objects[call.callee];
        let RtType::Record(param_table, false) = param else {
            return None;
        };
        let caller = self.frames.last().map(|&o| (o, objects[o].kind));
        let from_codeunit = matches!(caller, Some((_, ObjKind::Codeunit)));
        match (call.dispatch, callee.kind) {
            (Dispatch::Direct, ObjKind::Codeunit) => Some(Evidence::Pinned),
            (Dispatch::Direct, ObjKind::Table) if *param_table as usize != call.callee => {
                // A table caller in the cells is a third table: neither the callee's nor the parameter's.
                let from_other_table = matches!(caller, Some((o, ObjKind::Table))
                    if o != call.callee && o != *param_table as usize);
                (from_codeunit || from_other_table).then_some(Evidence::Assumed(Assumption::ByValueTableProcOtherTable))
            }
            (Dispatch::Direct, ObjKind::Table) => (from_codeunit
                && callee.rec_slot.is_some_and(|s| {
                    matches!(call.instance.borrow().globals.get(s), Some(Value::Record(r)) if Rc::ptr_eq(r, source))
                }))
            .then_some(Evidence::Pinned),
            _ => None,
        }
    }

    /// Gives `copy`, the by-value copy of `source` for a parameter of type `param` of the procedure of `call`, the view BC
    /// hands it. The entry is one allowlist: `source` is not temporary and of no other company, the parameter is a
    /// Record of the same table, not temporary and of a table that is not TableType = Temporary, and `source` carries no
    /// link filter. Every other copy keeps main's view (`record_by_value`).
    /// - An event subscriber's copy: [`subscriber_view`], told what [`Session::event_args`] says of parameter `index`.
    ///   The cases below are for a call by name or through an interface.
    /// - Caller in filter group 0, ascending, no marks, MarkedOnly off, no filter, on the primary key or an ad hoc key
    ///   (`RecordState::adhoc`), any callee: the copy has the primary key, also after an ad hoc key on a normal field, on a
    ///   FlowField whose value depends on the row (whose `FLOWFIELD_KEY` mark it does not take) or on a FlowField and then
    ///   the primary key field (record/keys-sorting/PwFlowFieldKeyRule.al:249-252 `byValue=Id order=1 2 3`).
    /// - A callee [`Session::measured_callee`] names, no mark on `source`, a view [`measured_view`] names: the view of
    ///   [`primary_view`], and a SetLoadFields of a caller that read no row is dropped (`ByValue_SetLoadFields`
    ///   `amount loaded=Yes`). Under [`Evidence::Assumed`] the copy is marked (`RecordState::view_assumption`)
    ///   so that a read of its view marks the assumption (`by_value_view_read`, `by_value_view_handed`).
    /// - Another callee, or the callee of `by-value-table-proc-other-table` with a view [`measured_view`] does not name, no
    ///   filter and no mark on `source`, the caller in filter group 0, ascending, with no marks: main's view (a declared
    ///   secondary key stays).
    /// - Else, with no mark on `source`, the copy keeps main's view and is marked [`BY_VALUE_VIEW`]: a read of its view
    ///   refuses (`by_value_view_runs`, `by_value_view_handed`). A copy of a marked `source` keeps main's view and the mark.
    pub(crate) fn by_value_view(&self, source: &Rec, copy: &Rec, param: &RtType, call: &ByValueCall, index: usize) {
        let s = source.borrow();
        let entry = matches!(param, RtType::Record(t, false) if *t == s.table
                && !self.engine.object_facts[*t as usize].temporary())
            && !s.temporary
            && s.company.is_none()
            && !s.crm
            && s.run_page_link.is_none();
        if !entry {
            return;
        }
        if call.dispatch == Dispatch::Subscriber {
            subscriber_view(&s, copy, call.event_arg(index), self.cell_record(&s));
            return;
        }
        let measured = self.measured_callee(call, param, source);
        let plain_view =
            s.filter_group == 0 && s.ascending && s.field_descending == 0 && s.marks.is_empty() && !s.marked_only;
        let no_filter = s.filters.is_empty();
        let pinned_key = s.key == 0 || s.adhoc.as_ref().is_some_and(|(k, _)| *k == s.key);
        let load_dropped = measured.is_some() && s.load.request.is_some() && s.load.buffer.is_none();
        // The cells call from a codeunit, and `Buffer.Proc(Head)` also from a third table's OnInsert trigger
        // (record/filters/ByValueAutoCalcFields.al:598); a third table's procedure, and a codeunit procedure called from table
        // code, are not measured.
        let objects = &self.engine.program.objects;
        let caller = self.frames.last().zip(self.call_stack.last());
        let from_codeunit = caller.is_some_and(|(&o, _)| objects[o].kind == ObjKind::Codeunit);
        let from_third_table_insert = caller.is_some_and(|(&o, f)| {
            let r = &objects[o].routines[f.routine];
            objects[o].kind == ObjKind::Table && r.is_trigger && r.name.eq_ignore_ascii_case("OnInsert")
        });
        let auto_calc_caller = from_codeunit
            || (measured == Some(Evidence::Assumed(Assumption::ByValueTableProcOtherTable)) && from_third_table_insert);
        let auto_calc_dropped =
            measured.is_some() && auto_calc_caller && s.unpinned_buffer.is_none() && self.measured_auto_calc(&s);
        let mut c = copy.borrow_mut();
        if auto_calc_dropped {
            c.auto_calc.clear();
            c.auto_calc_dropped = true;
        }
        match s.unpinned_buffer {
            None | Some(FLOWFIELD_KEY) if plain_view && no_filter && pinned_key && !load_dropped => {
                c.key = 0;
                c.adhoc = None;
                c.unpinned_buffer = None;
            }
            None if measured.is_some() && measured_view(&s, auto_calc_dropped) => {
                primary_view(&mut c);
                if load_dropped {
                    c.load.request = None;
                }
                c.view_assumption = measured.and_then(Evidence::assumption);
            }
            // Another callee, or one of `by-value-table-proc-other-table` whose view the probe did not measure: main's view.
            None if measured != Some(Evidence::Pinned) && plain_view && no_filter => {}
            None => c.unpinned_buffer = Some(BY_VALUE_VIEW),
            Some(_) => {}
        }
    }

    /// Whether `s` is the caller's record of the subscriber cell (record/filters/ByValueRecordView.al:964-972), every fact by
    /// equality: a fresh record (it read no row, no field written, no Insert of its own, no ChangeCompany) with the view
    /// [`group0_view`] names and exactly one filter, `SetRange(Status, ..)` in group 0 on a field of FieldClass Normal (:969).
    fn cell_record(&self, s: &RecordState) -> bool {
        let normal = |field: usize| {
            self.engine.program.objects[s.table as usize]
                .table
                .as_ref()
                .and_then(|t| t.fields.get(field))
                .is_some_and(|f| f.stored)
        };
        matches!(s.filters.as_slice(), [f] if normal(f.field))
            && group0_view(s)
            && s.position.is_none()
            && !s.touched
            && !s.inserted
            && !s.change_company_called
    }

    /// Whether the caller's SetAutoCalcFields of `s` is one record/filters/ByValueAutoCalcFields.al measured, which BC does
    /// not hand a by-value copy (assumption `by-value-record-no-auto-calc`, corpus PR 1473, merged, and corpus PR 1485, open; not pinned yet; lines as in PR 1485): each cell's copy has the
    /// caller's field values, its calculated FlowField included, but a FindLast of it leaves the FlowFields 0
    /// (`lastTotal=0 lastCount=0`), while the caller still calculates them (:270-730: a codeunit procedure :289-382, through a
    /// Variant to RecordRef.GetTable :383-418, `Buffer.Proc(Head)` :419-456 and :475-499, `Head.Proc(Head)` :457-474 and
    /// :500-523; a Get of the copy :524, Copy of it :542, its own SetAutoCalcFields :560, a FlowFilter formula :578,
    /// `Buffer.Proc(Head)` from a third table's OnInsert :598). Every fact the cells share, by equality: one or
    /// one to three FlowFields with a CalcFormula of any method (Sum and Count, and Min, Max, Average, Exist and Lookup, one
    /// of them a Min a table extension declares, :627-730), every filter on a Normal field in
    /// group 0 or 2,
    /// the caller standing in group 0, the primary or a declared key, any order, marks but MarkedOnly off, no SetLoadFields
    /// and ReadIsolation at its default.
    fn measured_auto_calc(&self, s: &RecordState) -> bool {
        let Some(table) = self.engine.program.objects[s.table as usize].table.as_ref() else {
            return false;
        };
        let field = |i: usize| table.fields.get(i);
        matches!(s.auto_calc.len(), 1..=3)
            && s.auto_calc
                .iter()
                .all(|&i| field(i).is_some_and(|f| !f.stored && !f.flow_filter && f.calc.is_some()))
            && s.filters
                .iter()
                .all(|f| matches!(f.group, 0 | 2) && field(f.field).is_some_and(|d| d.stored))
            && s.filter_group == 0
            && s.adhoc.is_none()
            && s.field_descending == 0
            && !s.marked_only
            && s.load.request.is_none()
            && s.read_isolation == 0
    }

    /// Whether the call of publisher `routine` of `object` from routine `caller` of the same object, whose locals are
    /// `caller_locals`, hands the publisher's only parameter (`values`) the record a `var` parameter of `caller` holds
    /// (`Raise(var R)` calls `OnThing(R)`, record/filters/ByValueRecordView.al:288-290). `finish_call` asks it at each
    /// publisher call; [`Session::event_publisher`] reads the answer.
    pub(crate) fn raise_arg_var_param(
        &self,
        (object, routine): (usize, usize),
        values: &[Value],
        caller: (usize, usize),
        caller_locals: &[Value],
    ) -> bool {
        let objects = &self.engine.program.objects;
        let c = &objects[caller.0].routines[caller.1];
        let [Value::Record(arg)] = values else {
            return false;
        };
        caller.0 == object
            && objects[object].routines[routine].params == 1
            && (0..c.params).any(|l| {
                c.by_var.get(l) == Some(&true)
                    && matches!(caller_locals.get(l), Some(Value::Record(r)) if Rc::ptr_eq(r, arg))
            })
    }

    /// The copy a by-value parameter that is not declared temporary, of a table that is not TableType = Temporary, of the
    /// procedure of `call` gets of the temporary record `rc`: a record of the stored table
    /// (record/temporary/TempRecordByValue.al:TempRecord_ByValue_ToPersistentParameter, :TempRecord_ByValue_ModifyAndDeleteAll),
    /// with the record's current values. The corpus pins one view
    /// (record/filters/ByValueRecordView.al:980-1003 `ByValue_TemporaryToPlainParameter`: `temp=No f={}`): a codeunit's procedure
    /// called by name, the caller with at least one filter and the view [`group0_view`] names. That copy has no filters. The same shape through an interface call is marked [`BY_VALUE_VIEW`]. An event
    /// subscriber's copy is not measured: parameter `index`, whose publisher parameter is a by-value Record
    /// ([`EventArg::Measured`] or [`EventArg::PublisherByValue`]), without the view of a fresh record ([`fresh_view`]), is
    /// marked [`BY_VALUE_VIEW`]. Every other copy keeps the caller's view (main; not pinned).
    pub(crate) fn temporary_into_stored_param(&self, rc: &Rec, call: &ByValueCall, index: usize) -> Value {
        let mut st = rc.borrow().copy_as_new_variable();
        st.temporary = false;
        st.temp_id = None;
        let unmarked = st.unpinned_buffer.is_none();
        let shape = self.engine.program.objects[call.callee].kind == ObjKind::Codeunit
            && !st.filters.is_empty()
            && group0_view(&st)
            && st.company.is_none()
            && st.run_page_link.is_none()
            && unmarked;
        match (shape, call.dispatch) {
            (true, Dispatch::Direct) => {
                st.filters.clear();
            }
            (true, Dispatch::Interface) => st.unpinned_buffer = Some(BY_VALUE_VIEW),
            (_, Dispatch::Subscriber)
                if unmarked
                    && matches!(call.event_arg(index), EventArg::Measured | EventArg::PublisherByValue)
                    && !fresh_view(&st) =>
            {
                st.unpinned_buffer = Some(BY_VALUE_VIEW)
            }
            (_, Dispatch::Subscriber) | (false, _) => {}
        }
        Value::Record(Rc::new(RefCell::new(st)))
    }

    /// A record method `method` on `rec`, which holds `RecordState::view_assumption` `assumed`: a method that reads the view
    /// marks `assumed` (every method but [`reads_no_view`]); `Reset`, `SetView` and `Clear` give it a view of its own, which
    /// ends the mark.
    fn by_value_view_read(method: RecordOp, rec: &Rec, assumed: Assumption) {
        if matches!(method, RecordOp::Reset | RecordOp::SetView | RecordOp::Clear) {
            rec.borrow_mut().view_assumption = None;
            return;
        }
        if !reads_no_view(method) {
            assumptions::assume(assumed);
        }
    }

    /// The view marks `record_op_inner` checks before `method` runs on `rec` with `args`. Copy and CopyFilters give `rec` the
    /// view of their source, and with it the source's `view_assumption` (marked now) or [`BY_VALUE_VIEW`] mark, so a later read
    /// of `rec`'s view marks or refuses; `rec` holds no other mark here (`record_op_inner` sent it to `unpinned_buffer_op`).
    /// Another method with a record as its first argument is handed it (`by_value_view_handed`), except TransferFields, which
    /// reads its fields only. Else a read of `rec`'s own view marks the assumption (`by_value_view_read`).
    pub(crate) fn by_value_view_marks(method: RecordOp, rec: &Rec, args: &[Value]) -> Result<(), Signal> {
        match (method, args.first()) {
            (RecordOp::TransferFields, _) => {}
            (RecordOp::Copy | RecordOp::CopyFilters, Some(Value::Record(src))) if !Rc::ptr_eq(src, rec) => {
                let (assumed, unpinned, event_reads) = {
                    let s = src.borrow();
                    (
                        s.view_assumption,
                        s.unpinned_buffer == Some(BY_VALUE_VIEW),
                        s.unpinned_buffer == Some(BY_VALUE_EVENT_READS),
                    )
                };
                if event_reads {
                    return unsupported(format!(
                        "Record.{}() from a record: {BY_VALUE_EVENT_READS}",
                        method.name()
                    ));
                }
                if let Some(a) = assumed {
                    assumptions::assume(a);
                }
                let mut r = rec.borrow_mut();
                r.view_assumption = assumed;
                if unpinned {
                    r.unpinned_buffer = Some(BY_VALUE_VIEW);
                }
                return Ok(());
            }
            (_, Some(Value::Record(src))) if !Rc::ptr_eq(src, rec) => {
                Self::by_value_view_handed(src, || format!("Record.{}() from a record", method.name()))?;
            }
            _ => {}
        }
        let assumed = rec.borrow().view_assumption;
        if let Some(a) = assumed {
            Self::by_value_view_read(method, rec, a);
        }
        Ok(())
    }

    /// `rec` handed to code that reads its view (`to` names it): a copy that holds `RecordState::view_assumption` marks the
    /// assumption, one marked [`BY_VALUE_VIEW`] or [`BY_VALUE_EVENT_READS`] refuses. Callers: `by_value_view_marks` (the source of a record method other
    /// than Copy, CopyFilters and TransferFields), `refs.rs` `gettable_record_copy`, the record a page opens on
    /// (`pages/open/record_refusals.rs` `page_record_refusal`: Page.Run, RunModal, SetRecord or SetTableView), the view of a
    /// report (`reports.rs` `report_set_view`) and of an XmlPort (`xmlports/mod.rs` `xp_set_view`), and
    /// FilterPageBuilder.AddRecord (`filterpage.rs`).
    pub(crate) fn by_value_view_handed(rec: &Rec, to: impl FnOnce() -> String) -> Result<(), Signal> {
        let r = rec.borrow();
        if let Some(a) = r.view_assumption {
            assumptions::assume(a);
        }
        match r.unpinned_buffer {
            Some(why) if by_value_mark(why) => unsupported(format!("{}: {why}", to())),
            _ => Ok(()),
        }
    }

    /// Whether `method` runs on `rec` with its mark kept, by `record_op_inner`, when `rec` is marked [`BY_VALUE_VIEW`] or
    /// [`BY_VALUE_EVENT_READS`] (`false`: another mark, for `unpinned_buffer_op`). It runs when it reads neither the view
    /// nor the marks ([`reads_no_view`]). Under [`BY_VALUE_EVENT_READS`] only GetFilters and Count run besides
    /// (record/filters/ByValueRecordView.al:306). Under [`BY_VALUE_VIEW`] it runs when it is `CalcFields` and no filter of
    /// the copy is on a FlowFilter field (in no filter group: BC hands the copy none of group 0), and when it is `Reset`,
    /// which gives the copy the view of a reset record: the mark goes when the copy holds no marks. Every other method
    /// refuses. The triggers and events a method runs see the mark.
    pub(crate) fn by_value_view_runs(&self, method: RecordOp, rec: &Rec) -> Result<bool, Signal> {
        let mark = rec.borrow().unpinned_buffer;
        if mark == Some(BY_VALUE_EVENT_READS) {
            if reads_no_view(method) || matches!(method, RecordOp::GetFilters | RecordOp::Count) {
                return Ok(true);
            }
            return unsupported(format!("Record.{}(): {BY_VALUE_EVENT_READS}", method.name()));
        }
        if mark != Some(BY_VALUE_VIEW) {
            return Ok(false);
        }
        let runs = reads_no_view(method)
            || method == RecordOp::Reset
            || (method == RecordOp::CalcFields && !self.holds_flow_filter_filter(rec));
        if !runs {
            return unsupported(format!("Record.{}(): {BY_VALUE_VIEW}", method.name()));
        }
        if method == RecordOp::Reset && rec.borrow().marks.is_empty() {
            rec.borrow_mut().unpinned_buffer = None;
        }
        Ok(true)
    }

    /// Whether a filter of `rec`, in any filter group, is on a FlowFilter field.
    fn holds_flow_filter_filter(&self, rec: &Rec) -> bool {
        let r = rec.borrow();
        let Some(schema) = self.engine.program.objects[r.table as usize].table.as_ref() else {
            return true;
        };
        r.filters
            .iter()
            .any(|f| schema.fields.get(f.field).is_none_or(|field| field.flow_filter))
    }
}
