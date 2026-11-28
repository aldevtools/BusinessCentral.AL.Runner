//! The xRec of a procedure of a table or of one of its table extensions called on a record while a trigger of the same record
//! runs, by code that holds no xRec of it (`Session::running_call_xrec`, by `table_proc_xrec` in `nested_validate.rs`): the
//! running trigger's xRec in the chains of calls the corpus measured, else a blank xRec marked `XREC_OF_RUNNING_CALL` whose
//! read refuses. While a table extension's insert or delete trigger runs on the record, the blank xRec carries that trigger's
//! mark instead (`ext_trigger_xrec.rs`, not measured), checked before every chain.
//!
//! The chains a field OnValidate of the record starts, ending in a codeunit procedure that has the record as a `var` parameter
//! and calls the procedure on it (al-runner issues 8189 and 8284), the trigger's xRec shared:
//! - through a procedure of the table that is no trigger (`through_own_procedure`), on the buffers and Validate call sites
//!   `validate_of_own_procedure_chain` lists (record/validate/XRecTableProcThroughCodeunit.al);
//! - straight from the trigger (`straight_from_trigger`), or from the subscriber of an integration event the trigger raises
//!   (`through_subscriber`), record/validate/XRecTableProcThroughSubscriber.al. Under a Validate the table's own code nested,
//!   BC keeps one xRec for both Validates; this runner does too in the nests `nested_validate_xrec` shares. Where the inner
//!   Validate kept its own copy, the xRec is shared only while neither was assigned or filtered (`nested_xrecs_untouched`),
//!   and for reading only (`ProcXrec::SharedReadOnly`, `XREC_CHANGED_UNDER_NESTED_VALIDATE`).
//!
//! Every such chain also needs the facts its cells share (`cell_facts`); the Validate called by other code than a test
//! procedure, and an event with two subscribers, run through assumption `xrec-table-proc-callers-subscribers`
//! (`callers_assumed`, `subscribers_measured`). A record changed after its OnBeforeValidateEvent returned on the two chains of
//! al-runner issue 8284 runs through assumption `xrec-table-proc-routes` (al-runner issue 8974).
//!
//! Under a write trigger of `write_trigger_below` (the table's OnInsert, OnModify or OnDelete, or a table extension's
//! OnBeforeModify, record/validate/XRecTableProcThroughCodeunit.al): through a codeunit (`through_write_trigger`), the
//! trigger's xRec shared for reading only (`XREC_CHANGED_UNDER_WRITE_TRIGGER`); `Rec.Proc()` from the table's own code
//! (`write_trigger_own_call_xrec`), a copy of the trigger's xRec values (`ProcXrec::PlainWith`), so an assignment stays in
//! the procedure as on a plain call. A procedure the shared one calls on Rec: `shared_xrec_of_own_procedure`.

use super::nested_validate::{ProcXrec, ValidateRun, ValidateSite, XREC_OF_RUNNING_CALL};
use crate::records::Rec;
use crate::{Frame, Session};
use al_builtins::assumptions::{self, Assumption};
use al_builtins::value::{RecordState, RtType, Value};
use al_ir::{ObjKind, Routine};
use std::rc::Rc;

/// How many procedures of the table record/validate/XRecTableProcThroughSubscriber.al measured between the trigger that nests a
/// Validate and the Validate (none, one, two and three: `Subscriber_VarRec_NestedValidateInTrigger` :663,
/// `..._NestedValidateInFirstTableProcedure` :691, `..._NestedValidateInTableProcedure` :590,
/// `..._NestedValidateInThirdTableProcedure` :677).
pub(super) const NESTING_PROCEDURES_MEASURED: usize = 3;

/// The refusal when a procedure that shared the trigger's xRec under a Validate the table's own code nested (`nested_by_own_code`)
/// changed it: real BC keeps one xRec for both Validates, so an assignment shows to the trigger of the outer Validate too
/// (record/validate/XRecTableProcThroughSubscriber.al:750 `I after x=0/SET/I1`) and a filter in both triggers (:1078
/// `I filters=Series: F`), where this runner keeps the two triggers' xRecs apart: in every shape `nested_validate_xrec` does not
/// share.
const XREC_CHANGED_UNDER_NESTED_VALIDATE: &str = "a change to xRec (an assignment, a filter or another view of it) in a table procedure called through an event subscriber under a Validate that the table's own code nested: real BC shows it to the outer Validate's trigger too, which this runner keeps apart";

/// The refusal when a procedure that shared a running write trigger's xRec (`through_write_trigger`) changed it: the cells
/// read it only (record/validate/XRecTableProcThroughCodeunit.al:567, :597, :627, :712).
const XREC_CHANGED_UNDER_WRITE_TRIGGER: &str = "a change to xRec (an assignment, a filter or another view of it) in a table procedure called through a codeunit while the table's OnInsert, OnModify or OnDelete trigger or a table extension's OnBeforeModify trigger runs: whether real BC shows it to the trigger is not measured";

/// The chain of calls `measured_chain` found below a procedure call: the object whose running trigger's xRec the procedure
/// shares (the table, or the table extension whose trigger runs); when a Validate the table's own code nested runs, that
/// Validate's xRec; whether the procedure shares the xRec for reading only, and the refusal of a change; whether a fact of the
/// chain is only measured by the open probe of assumption `xrec-table-proc-callers-subscribers` (`cell_facts`,
/// `subscribers_measured`), and whether the record changed since its Validate started, only by assumption
/// `xrec-table-proc-routes` (`cell_facts`).
struct Chain {
    trigger_object: usize,
    nested: Option<Rec>,
    /// Under the nested Validate, when its two xRecs are two copies (`nested_validate_xrec` gave the inner Validate its own), and
    /// under a write trigger: the procedure shares the xRec for reading only.
    read_only: Option<&'static str>,
    assumed: bool,
    routes: bool,
}

impl Chain {
    /// A chain of pinned facts only, sharing the running trigger's xRec of `trigger_object` for reading and writing.
    fn pinned(trigger_object: usize) -> Chain {
        Chain {
            trigger_object,
            nested: None,
            read_only: None,
            assumed: false,
            routes: false,
        }
    }
}

impl Session {
    /// A procedure of table `object` (or of one of its table extensions) called on `rc` by code that holds no xRec of it (frame
    /// `f`, no case of `frame_xrec`).
    /// - Called from the code of the record's table or of one of its table extensions (a trigger or procedure of any of them): a
    ///   plain call, as `table_proc_xrec` lists (record/PWTableGlobals.al:196-260; al-test-cli suite `xrec_proc_call_scope.rs`),
    ///   unless the caller shares a running call's xRec (`shared_xrec_of_own_procedure`).
    /// - Called from other code while triggers or procedures on the same record run further up: the instances of the record
    ///   variable (ADR-021) of its table and of its table extensions that hold `rc` as Rec, and those calls' xRecs
    ///   (`running_xrecs`); while a validate event of `rc` is raised, also the xRec it hands its subscribers
    ///   (`raised_validate_event_xrec`, records/validate_event_xrec.rs).
    /// - A chain the corpus measured (`measured_chain`): the xRec of the call running on the table's own instance (the field
    ///   trigger's), shared, so an assignment to it shows in the trigger after the call
    ///   (record/validate/XRecTableProcThroughCodeunit.al `Way3_ProcAssignsXRec_TriggerReadsAfter`,
    ///   record/validate/XRecTableProcThroughSubscriber.al:736, :750, :780).
    /// - Any other shape, when one of those xRecs holds other values than the blank record a plain call gets: the blank xRec
    ///   marked `XREC_OF_RUNNING_CALL`, so a read of it refuses.
    /// - Else, and when no call on the record runs: a plain call (record/PWTableGlobals.al:196-260 XRecPlainProc_*).
    pub(super) fn running_call_xrec(&mut self, object: usize, rc: &Rec, f: &Frame) -> ProcXrec {
        let table = rc.borrow().table as usize;
        // A table extension's insert or delete trigger runs on the record: its xRec is not measured, nor is the procedure's.
        if self.ext_trigger_xrec_running(table, rc) {
            return ProcXrec::PlainUnpinned(super::XREC_OF_EXT_INSERT_DELETE_TRIGGER);
        }
        if self.table_or_its_extension(f.object, table) {
            if let Some(x) = self.write_trigger_own_call_xrec(object, table, rc, f) {
                return ProcXrec::PlainWith(x.borrow().fields.clone());
            }
            let Some(x) = self.shared_xrec_of_own_procedure(rc, f) else {
                return ProcXrec::Plain;
            };
            let blank = self.initial_fields(table);
            return if Self::same_values(&x.borrow().fields, &blank) {
                ProcXrec::Plain
            } else {
                ProcXrec::PlainUnpinned(XREC_OF_RUNNING_CALL)
            };
        }
        let blank = self.initial_fields(table);
        let not_blank = |x: &Rec| !Self::same_values(&x.borrow().fields, &blank);
        let running = self.running_xrecs(table, rc);
        let event = self.raised_validate_event_xrec(rc);
        if !running.iter().any(|(_, x)| not_blank(x)) && !event.as_ref().is_some_and(not_blank) {
            return ProcXrec::Plain;
        }
        if let Some(chain) = self.measured_chain(object, table, rc, f)
            && let Some((_, x)) = running.iter().find(|(o, _)| *o == chain.trigger_object)
            && not_blank(x)
            && chain.nested.as_ref().is_none_or(|inner| Rc::ptr_eq(inner, x))
            && (!chain.assumed || assumptions::hit(Assumption::XrecTableProcCallersSubscribers))
            && (!chain.routes || assumptions::hit(Assumption::XrecTableProcRoutes))
        {
            return match chain.read_only {
                Some(reason) => ProcXrec::SharedReadOnly(x.clone(), reason),
                None => ProcXrec::Shared(x.clone()),
            };
        }
        ProcXrec::PlainUnpinned(XREC_OF_RUNNING_CALL)
    }

    /// The xRec of the procedure in frame `f`, of the table or of one of its table extensions, when it runs on `rc` with an xRec
    /// it shares with its caller or a running call (`shared_proc_xrec`; not a trigger, nor a procedure the table's own trigger
    /// calls unqualified, whose instance holds the trigger's xRec). A procedure
    /// it calls on `rc` (`Rec.OtherProc()`) refuses a read of xRec when that xRec holds other values than a blank record
    /// (`running_call_xrec`): BC gives the unqualified call the caller's xRec, as here (both run on one instance); the call on
    /// Rec from such a procedure is not measured.
    fn shared_xrec_of_own_procedure(&self, rc: &Rec, f: &Frame) -> Option<Rec> {
        let objects = &self.engine.program.objects;
        if objects[f.object].routines[f.routine].is_trigger {
            return None;
        }
        let fi = f.instance.borrow();
        let o = &objects[fi.object as usize];
        let slot = |s: Option<usize>| s.and_then(|s| fi.globals.get(s));
        match (slot(o.rec_slot), slot(o.xrec_slot)) {
            (Some(Value::Record(r)), Some(Value::Record(x)))
                if Rc::ptr_eq(r, rc) && self.shared_proc_xrec.iter().any(|p| Rc::ptr_eq(p, x)) =>
            {
                Some(x.clone())
            }
            _ => None,
        }
    }

    /// The xRecs of the calls running on the record variable `rc` of table `table`: of each instance (ADR-021) of the table
    /// and of its table extensions whose Rec slot holds `rc` (`enter_record` fills it while the call runs, `leave_record`
    /// empties it), with the instance's object.
    pub(super) fn running_xrecs(&self, table: usize, rc: &Rec) -> Vec<(usize, Rec)> {
        let instances = rc.borrow().globals.instances();
        let objects = &self.engine.program.objects;
        let mut running = Vec::new();
        for (object, inst) in instances {
            let object = object as usize;
            if !self.table_or_its_extension(object, table) {
                continue;
            }
            let o = &objects[object];
            let (Some(rs), Some(xs)) = (o.rec_slot, o.xrec_slot) else {
                continue;
            };
            let i = inst.borrow();
            if let (Value::Record(r), Value::Record(x)) = (&i.globals[rs], &i.globals[xs])
                && Rc::ptr_eq(r, rc)
            {
                running.push((object, x.clone()));
            }
        }
        running
    }

    /// The chain of calls below a call of a procedure of `object` on `rc` (a record of table `table`) from frame `f`, when it is
    /// one the corpus measured, every fact by equality; `None` for any other chain. The innermost frame `f` runs a procedure of a
    /// codeunit, called directly, that has `rc` as a `var` parameter.
    fn measured_chain(&self, object: usize, table: usize, rc: &Rec, f: &Frame) -> Option<Chain> {
        let n = self.frames.len();
        if n < 2 || self.call_stack.len() != n {
            return None;
        }
        let objects = &self.engine.program.objects;
        let (cu, cu_frame) = (self.frames[n - 1], self.call_stack[n - 1]);
        if cu != f.object || cu_frame.routine != f.routine || cu_frame.event || objects[cu].kind != ObjKind::Codeunit {
            return None;
        }
        let callee = &objects[cu].routines[cu_frame.routine];
        let subscriber = callee
            .attributes
            .iter()
            .any(|a| a.name.eq_ignore_ascii_case("EventSubscriber"));
        if subscriber && let Some(chain) = self.through_subscriber(object, table, rc, f) {
            return Some(chain);
        }
        let var_param = (0..callee.params).any(|i| {
            callee.by_var.get(i) == Some(&true)
                && matches!(f.locals.get(i), Some(Value::Record(p)) if Rc::ptr_eq(p, rc))
        });
        if !var_param {
            return None;
        }
        // A subscriber of a table event right above the trigger (a Modify in the trigger) is not the codeunit called from it.
        if self.through_own_procedure(object, table, rc) {
            return Some(Chain::pinned(table));
        }
        if subscriber {
            return None;
        }
        if let Some(trigger_object) = self.through_write_trigger(object, table, rc) {
            return Some(Chain {
                read_only: Some(XREC_CHANGED_UNDER_WRITE_TRIGGER),
                ..Chain::pinned(trigger_object)
            });
        }
        let (trigger_object, assumed, routes) = self.straight_from_trigger(object, table, rc)?;
        Some(Chain {
            assumed,
            routes,
            ..Chain::pinned(trigger_object)
        })
    }

    /// The chain record/validate/XRecTableProcThroughCodeunit.al pins (`Way3_*` and `Way4_*`, :365-414): `object` is the table
    /// itself; below the codeunit procedure (plain or reached through an interface, as on main also a subscriber of a table event
    /// the procedure raised; `XTC Mover.Handle` :294-296, `XTC Iface Mover.Handle` :322-324), a procedure of the table that is no
    /// trigger (`CallPlain` :141-145, `CallIface` :148-154), then the table's field OnValidate (:46-58); the Validate is
    /// `validate_of_own_procedure_chain`.
    fn through_own_procedure(&self, object: usize, table: usize, rc: &Rec) -> bool {
        let n = self.frames.len();
        if n < 3 {
            return false;
        }
        let (proc_o, proc_frame) = (self.frames[n - 2], self.call_stack[n - 2]);
        let proc = &self.engine.program.objects[proc_o].routines[proc_frame.routine];
        object == table
            && proc_o == table
            && !proc_frame.event
            && !proc.is_trigger
            && self.field_on_validate_at(table, n - 3)
            && self.validate_of_own_procedure_chain(rc, n - 3)
    }

    /// The Validate whose trigger runs in frame `trigger`, in the chain `through_own_procedure` (record/validate/
    /// XRecTableProcThroughCodeunit.al), is the one Validate of `rc` that runs, and one the file measured:
    /// - `Record.Validate` from AL code (:358), on a buffer of `measured_buffer`;
    /// - `FieldRef.Validate(Value)` straight from the test procedure, on the RecordRef `GetTable` filled from the buffer `AfterGet`
    ///   (`Way3_FieldRefValidate` :469-486);
    /// - `SetValue` of a TestPage control straight from the test procedure, on the Card that `OpenEdit` and `GoToKey` positioned on
    ///   the row `AfterInsert` inserted (`Way3_TestPageSetValue` :447-467; the site's `after_go_to_key_on_card`).
    fn validate_of_own_procedure_chain(&self, rc: &Rec, trigger: usize) -> bool {
        let Some(run) = self.sole_validate_run(Rc::as_ptr(rc) as usize) else {
            return false;
        };
        let from_test =
            || run.depth == trigger && run.with_value && self.only_test_codeunit_frames(0, self.frames.len() - trigger);
        let r = rc.borrow();
        match run.site {
            Some(ValidateSite::AlRecordValidate) => Self::measured_buffer(&r),
            Some(ValidateSite::FieldRef) => from_test() && Self::buffer_after_get(&r),
            Some(ValidateSite::TestPageControl {
                new_line_of_trapped_static_card: false,
                after_go_to_key_on_card: true,
            }) => from_test() && Self::buffer_of_test_page(&r),
            _ => false,
        }
    }

    /// The codeunit procedure, no event subscriber and not reached through an interface (`iface_frames`), called straight from the
    /// table's field OnValidate calls a procedure of the table or of one of its table extensions: the trigger's xRec, the
    /// Validate being the one `Record.Validate` AL code called on the record, whose trigger is that
    /// frame, on a buffer of `straight_chain_buffer` (record/validate/XRecTableProcThroughCodeunit.al
    /// `CodeunitFromTriggerDirectly_AfterGet`, `BaseField_Codeunit_ExtProc`; record/validate/XRecTableProcThroughSubscriber.al:647,
    /// :764, :780). Which facts are
    /// only assumed (`cell_facts`). Or the OnValidate of a field of a table extension calls the codeunit, which calls a
    /// procedure of the table, the facts all pinned (`ExtField_Codeunit_BaseProc` :669, `ValidateExtAndObserve` :657): the
    /// extension's xRec. The object whose trigger runs, and which facts are only assumed.
    fn straight_from_trigger(&self, object: usize, table: usize, rc: &Rec) -> Option<(usize, bool, bool)> {
        let n = self.frames.len();
        let trigger_object = self.frames[n - 2];
        let ext_field = object == table && self.ext_field_on_validate_at(table, n - 2);
        let shape = self.table_or_its_extension(object, table)
            && !self.iface_frames.contains(&(n - 1))
            && (self.field_on_validate_at(table, n - 2) || ext_field)
            && self.sole_validate_from_al_at(Rc::as_ptr(rc) as usize, n - 2)
            && Self::straight_chain_buffer(&rc.borrow(), ext_field);
        if !shape {
            return None;
        }
        let (assumed, routes) = self.cell_facts(rc, self.frames[n - 1], n - 2, n - 2, false)?;
        if ext_field && (assumed || routes) {
            return None;
        }
        Some((trigger_object, assumed, routes))
    }

    /// Whether frame `i` runs the OnValidate trigger of a field of a table extension of table `table`, not as an event.
    fn ext_field_on_validate_at(&self, table: usize, i: usize) -> bool {
        let r = self.routine_at(i).1;
        self.objects_extension_of(self.frames[i], table)
            && !self.call_stack[i].event
            && r.is_trigger
            && r.field.is_some()
            && r.name.eq_ignore_ascii_case("OnValidate")
    }

    /// The chains record/validate/XRecTableProcThroughCodeunit.al pins with a write trigger running on the record
    /// (`write_trigger_below`) in which the trigger calls a codeunit procedure (not SingleInstance, called directly) with the
    /// record as `var`, which calls a procedure of the table on it: through two procedures of the table that are no triggers
    /// from the table's OnInsert, OnModify or OnDelete (`RunWriteTrigger` :108, `CallPlain` :141; `OnInsert_Codeunit` :567,
    /// `OnModify_Codeunit` :597, `OnDelete_Codeunit` :627), straight from a table extension's OnBeforeModify (:197-213,
    /// `ExtOnBeforeModify_Codeunit` :712). The procedure reads the trigger's xRec. The object whose trigger runs.
    fn through_write_trigger(&self, object: usize, table: usize, rc: &Rec) -> Option<usize> {
        let n = self.frames.len();
        let cu = self.frames[n - 1];
        if object != table || self.engine.object_facts[cu].single_instance || self.iface_frames.contains(&(n - 1)) {
            return None;
        }
        self.write_trigger_below(table, rc, n - 1, [2, 0])
    }

    /// The xRec a procedure of the table gets when the code of the table or of a table extension calls it on Rec (`Rec.Proc()`)
    /// while a write trigger of `write_trigger_below` runs, in the chains record/validate/XRecTableProcThroughCodeunit.al
    /// pins: from a procedure of the table the table's OnInsert, OnModify or OnDelete called unqualified (`RunWriteTrigger`
    /// :108-124; `OnInsert_RecProc` :577, `OnModify_RecProc` :607, `OnDelete_RecProc` :637, and the procedure validating a field
    /// whose trigger reads xRec, :587, :617, :647), and straight from a table extension's OnBeforeModify (:197-213;
    /// `ExtOnBeforeModify_RecProc` :722, :732): the trigger's xRec, which the caller's instance holds. `running_call_xrec` hands
    /// the procedure a copy of its values.
    fn write_trigger_own_call_xrec(&self, object: usize, table: usize, rc: &Rec, f: &Frame) -> Option<Rec> {
        let n = self.frames.len();
        if object != table || n == 0 || self.frames[n - 1] != f.object || self.call_stack[n - 1].routine != f.routine {
            return None;
        }
        let trigger_object = self.write_trigger_below(table, rc, n, [1, 0])?;
        let fi = f.instance.borrow();
        let o = &self.engine.program.objects[fi.object as usize];
        let slot = |s: Option<usize>| s.and_then(|s| fi.globals.get(s));
        match (slot(o.rec_slot), slot(o.xrec_slot)) {
            (Some(Value::Record(r)), Some(Value::Record(x)))
                if fi.object as usize == trigger_object && Rc::ptr_eq(r, rc) =>
            {
                Some(x.clone())
            }
            _ => None,
        }
    }

    /// The write trigger record/validate/XRecTableProcThroughCodeunit.al measured running on `rc` (a record of table `table`)
    /// below frame `end`, with `procedures[0]` procedures of the table that are no triggers between it and `end` when it is the
    /// table's own trigger, `procedures[1]` when it is a table extension's: AL code of the test codeunit (`InsertWith` :527,
    /// `ModifyWith` :542, `DeleteWith` :554, called from the test procedure) called `Insert(true)` on a record never inserted,
    /// or `Modify(true)` or `Delete(true)` on the buffer `AfterGet`; its trigger is the table's OnInsert, OnModify or OnDelete,
    /// or a table extension's OnBeforeModify. The object whose trigger runs.
    fn write_trigger_below(&self, table: usize, rc: &Rec, end: usize, procedures: [usize; 2]) -> Option<usize> {
        use al_builtins::record_op::RecordOp;
        let r = rc.borrow();
        let after_get = Self::buffer_after_get(&r);
        // (procedures between, the trigger's object is the table, its name, the write, the buffer)
        let measured = [
            (
                procedures[0],
                true,
                "OnInsert",
                RecordOp::Insert,
                r.row_origin.is_none() && !r.temporary,
            ),
            (procedures[0], true, "OnModify", RecordOp::Modify, after_get),
            (procedures[0], true, "OnDelete", RecordOp::Delete, after_get),
            (procedures[1], false, "OnBeforeModify", RecordOp::Modify, after_get),
        ];
        measured.into_iter().find_map(|(between, own, name, op, buffer)| {
            let t = end.checked_sub(between + 1)?;
            let (trigger_object, routine) = (self.frames[t], self.routine_at(t).1);
            let object_ok = if own {
                trigger_object == table
            } else {
                self.objects_extension_of(trigger_object, table)
            };
            let shape = object_ok
                && buffer
                && !self.call_stack[t].event
                && routine.is_trigger
                && routine.field.is_none()
                && routine.name.eq_ignore_ascii_case(name)
                && self.is_al_record_call(op, rc)
                && self.own_procedures_between(table, t, end)
                && self.only_test_codeunit_frames(0, self.frames.len() - t);
            shape.then_some(trigger_object)
        })
    }

    /// The facts every cell of the two chains of al-runner issue 8284 shares beyond its frames (review of PR 8304, 2026-11-20):
    /// - the codeunit in `cu` (the subscriber's or the one the trigger calls) is not SingleInstance;
    /// - the Validate whose trigger runs in frame `trigger` finds the record as it was when its triggers started: no record
    ///   method ran on it since (`ValidateRun::touched`: a Modify, a Get, a Validate of another field), its fields are the same
    ///   (`ValidateRun::entered`), and its buffer came from no TransferFields. Or, through assumption
    ///   `xrec-table-proc-routes`, its fields changed only after its OnBeforeValidateEvent returned
    ///   (`changed_after_before_event`), that Validate being the outermost one (`trigger == caller`) and called by a test
    ///   codeunit's own procedures (record/validate/XRecTableProcRoutes.al, corpus PR 1583: `Codeunit_TriggerAssigns_TableProc`
    ///   and `IntegrationEvent_TriggerAssigns`, the trigger assigning a field before it calls the codeunit or raises the event);
    /// - the outermost Validate, whose trigger runs in frame `caller`, was called by a test codeunit's own procedures, as every
    ///   cell's (`ValidateModeAndObserve` and the like): pinned. Or by the callers `callers_assumed` lists (`subscriber`: the
    ///   subscriber chain's): assumed.
    ///
    /// Whether a fact is only assumed by `xrec-table-proc-callers-subscribers`, and whether by `xrec-table-proc-routes`.
    fn cell_facts(&self, rc: &Rec, cu: usize, trigger: usize, caller: usize, subscriber: bool) -> Option<(bool, bool)> {
        let handle = Rc::as_ptr(rc) as usize;
        let run = self
            .validate_sites
            .iter()
            .find(|v| v.handle == handle && v.depth == trigger)?;
        let untouched = self.untouched_since(run, rc);
        let routes = !untouched && trigger == caller && self.changed_after_before_event(run, rc);
        if self.engine.object_facts[cu].single_instance || !(untouched || routes) || caller == 0 {
            return None;
        }
        if self.only_test_codeunit_frames(0, self.frames.len() - caller) {
            return Some((false, routes));
        }
        (!routes && self.callers_assumed(caller, subscriber)).then_some((true, false))
    }

    /// Whether the frames below frame `caller` (the outermost Validate's trigger) are the callers record/validate/
    /// XRecTableProcCallers.al measured (corpus PR 1482, merged at 5ad2970e, not pinned yet; assumption `xrec-table-proc-callers-subscribers`): from the test
    /// procedure (`[Test]`, also naming handlers, `TwoSubscribers_MaintainLineFromReport_TestWithHandler`, also carrying
    /// `[Scope('OnPrem')]`, `scope_on_prem`) through procedures of
    /// its test codeunit and procedures of other codeunits (`Sub_FromCodeunitProcedure`, `Codeunit_FromCodeunitProcedure`),
    /// every codeunit not SingleInstance, called directly (not through an event or an interface). For the subscriber chain
    /// (`subscriber`) also a `[TryFunction]`, its result used or not (`Sub_FromTryFunction`, `Sub_FromTryFunctionAsStatement`;
    /// no other try scope runs: the try depth counts only those), a
    /// codeunit's OnRun as a statement (`Sub_FromCodeunitRun`, `Nested_FromCodeunitRun`) and a report's OnAfterGetRecord that
    /// runs one, as a statement or guarded (`Sub_FromReportThroughCodeunitRun`, `Sub_FromReportThroughGuardedCodeunitRun`,
    /// `Nested_MaintainLineFromReportThroughGuardedCodeunitRun`).
    fn callers_assumed(&self, caller: usize, subscriber: bool) -> bool {
        let base = self.test_frame_base();
        if caller <= base || caller > self.frames.len() || self.call_stack.len() != self.frames.len() {
            return false;
        }
        let objects = &self.engine.program.objects;
        let tries = (base..caller)
            .filter(|&i| Self::has_attribute(self.routine_at(i).1, "TryFunction"))
            .count();
        Self::is_test_codeunit(&objects[self.frames[base]])
            && self.tx.try_depth == tries
            && (base..caller).all(|i| self.caller_frame_measured(i, base, caller, subscriber))
    }

    /// Whether frame `i`, between the test procedure in frame `base` and the outermost Validate's trigger in frame `caller`, is
    /// one `callers_assumed` lists.
    fn caller_frame_measured(&self, i: usize, base: usize, caller: usize, subscriber: bool) -> bool {
        let objects = &self.engine.program.objects;
        let (o, cf) = (self.frames[i], self.call_stack[i]);
        let r = &objects[o].routines[cf.routine];
        let named = |a: &al_ir::Attribute, names: &[&str]| names.iter().any(|n| a.name.eq_ignore_ascii_case(n));
        let only = |names: &[&str]| r.attributes.iter().all(|a| named(a, names));
        if cf.event || self.iface_frames.contains(&i) {
            return false;
        }
        if i == base {
            let measured = |a: &al_ir::Attribute| named(a, &["Test", "HandlerFunctions"]) || Self::scope_on_prem(a);
            return Self::has_attribute(r, "Test") && r.attributes.iter().all(measured);
        }
        if o == self.frames[base] {
            return !r.is_trigger && r.attributes.is_empty();
        }
        let trigger_at = |j: usize, kind: ObjKind, name: &str| {
            j < caller && {
                let (ob, t) = self.routine_at(j);
                ob.kind == kind && !self.call_stack[j].event && t.is_trigger && t.name.eq_ignore_ascii_case(name)
            }
        };
        match objects[o].kind {
            ObjKind::Codeunit
                if !Self::is_test_codeunit(&objects[o]) && !self.engine.object_facts[o].single_instance =>
            {
                if r.is_trigger {
                    let guarded = self.tx.guarded_runs.contains(&i);
                    subscriber
                        && r.name.eq_ignore_ascii_case("OnRun")
                        && (!guarded || trigger_at(i - 1, ObjKind::Report, "OnAfterGetRecord"))
                } else {
                    only(&[]) || (subscriber && only(&["TryFunction"]))
                }
            }
            ObjKind::Report => {
                subscriber
                    && trigger_at(i, ObjKind::Report, "OnAfterGetRecord")
                    && trigger_at(i + 1, ObjKind::Codeunit, "OnRun")
            }
            _ => false,
        }
    }

    /// `[Scope('OnPrem')]` on the test procedure (record/validate/XRecTableProcRoutes.al, corpus PR 1583, merged on 2026-11-26,
    /// not pinned yet: `Caller_ScopeOnPremTest_IntegrationEvent`, `Caller_ScopeOnPremTestWithHandler_Nested`).
    fn scope_on_prem(a: &al_ir::Attribute) -> bool {
        a.name.eq_ignore_ascii_case("Scope") && a.args == [al_ir::Const::Text("OnPrem".to_string())]
    }

    /// The object and the routine frame `d` runs.
    pub(super) fn routine_at(&self, d: usize) -> (&al_ir::Object, &Routine) {
        let ob = &self.engine.program.objects[self.frames[d]];
        (ob, &ob.routines[self.call_stack[d].routine])
    }

    fn has_attribute(r: &Routine, name: &str) -> bool {
        r.attributes.iter().any(|a| a.name.eq_ignore_ascii_case(name))
    }

    /// Whether object `o` is a codeunit with `Subtype = Test`.
    pub(crate) fn is_test_codeunit(o: &al_ir::Object) -> bool {
        o.kind == ObjKind::Codeunit
            && o.properties
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case("Subtype") && v.trim().eq_ignore_ascii_case("Test"))
    }

    /// The subscriber of an integration event of the table, raised by the table's field OnValidate, calls a procedure of the
    /// table or of one of its table extensions on the record the event hands it by `var`: the trigger's xRec
    /// (record/validate/XRecTableProcThroughSubscriber.al:528-576, :722, :796, :886-922). Every fact by equality:
    /// - frame `f` runs a subscriber of a codeunit that is not bound manually; below it the publisher's event frame, a procedure
    ///   of the table marked `[IntegrationEvent(false, false)]`, whose one or two parameters are all `var`, the first being `rc`
    ///   and the second a record of another table; below that the table's field OnValidate;
    /// - the subscriber declares the publisher's parameters, all `var`;
    /// - either the Validate is the one `Record.Validate` AL code called on the record, whose trigger is that frame; or
    ///   `nested_by_own_code` nested it (record/validate/XRecTableProcThroughSubscriber.al:590-618, :663-718, :750, :813, :832,
    ///   :1084, :1098, :1112, :1126) and `nested_xrecs_untouched` holds.
    ///
    /// Whatever the buffer: the xRec is the record before the Validate, not the stored row, on every buffer the probes tried
    /// (here: after Insert, Get, FindFirst, FindLast, FindSet, Find('='), Next, FindLast then Init with a new key, on a fresh and
    /// on a temporary record, also never inserted; record/validate/XRecTableProcThroughCodeunit.al, for the codeunit chain: Get
    /// of a committed row and of a row the open transaction modified).
    ///
    /// The chain, with the nested Validate's xRec in the second case.
    fn through_subscriber(&self, object: usize, table: usize, rc: &Rec, f: &Frame) -> Option<Chain> {
        let n = self.frames.len();
        if n < 3 || !self.table_or_its_extension(object, table) {
            return None;
        }
        let objects = &self.engine.program.objects;
        let (cu, sub_frame) = (self.frames[n - 1], self.call_stack[n - 1]);
        let (pub_o, pub_frame) = (self.frames[n - 2], self.call_stack[n - 2]);
        let subscriber = &objects[cu].routines[sub_frame.routine];
        let publisher = &objects[pub_o].routines[pub_frame.routine];
        let all_var = |r: &Routine| (0..r.params).all(|i| r.by_var.get(i) == Some(&true));
        let second_is_other_table = publisher.params < 2
            || matches!(publisher.locals.get(1).map(|s| &s.ty), Some(RtType::Record(t, _)) if *t as usize != table);
        let shape = pub_o == table
            && pub_frame.event
            && Self::integration_event_measured(publisher)
            && (1..=2).contains(&publisher.params)
            && all_var(publisher)
            && second_is_other_table
            && !self.engine.object_facts[cu].manual_subscriber
            && subscriber.params == publisher.params
            && all_var(subscriber)
            && matches!(f.locals.first(), Some(Value::Record(p)) if Rc::ptr_eq(p, rc))
            && self.field_on_validate_at(table, n - 3);
        if !shape {
            return None;
        }
        let two_subscribers = self.subscribers_measured(pub_o, publisher)?;
        let handle = Rc::as_ptr(rc) as usize;
        if self.sole_validate_from_al_at(handle, n - 3) {
            let (assumed, routes) = self.cell_facts(rc, cu, n - 3, n - 3, true)?;
            // The probe's events with a record changed since the Validate started have one subscriber.
            if routes && two_subscribers {
                return None;
            }
            return Some(Chain {
                assumed: assumed || two_subscribers,
                routes,
                ..Chain::pinned(table)
            });
        }
        let (outer, inner) = self.nested_by_own_code(table, handle, n - 3)?;
        let (assumed, routes) = self.cell_facts(rc, cu, n - 3, outer.depth, true)?;
        // One xRec for both Validates (`nested_validate_xrec`): shared as a non-nested one is.
        let one_xrec = Rc::ptr_eq(&outer.xrec, &inner.xrec);
        (one_xrec || self.nested_xrecs_untouched(handle, &outer.xrec, &inner.xrec)).then(|| Chain {
            nested: Some(inner.xrec.clone()),
            read_only: (!one_xrec).then_some(XREC_CHANGED_UNDER_NESTED_VALIDATE),
            assumed: assumed || two_subscribers,
            routes,
            ..Chain::pinned(table)
        })
    }

    /// Whether the event `publisher` of object `publisher_o` has the subscribers the cells measured: one (each event of
    /// record/validate/XRecTableProcThroughSubscriber.al, `Some(false)`), or two, each of a codeunit neither bound manually nor
    /// SingleInstance, all its parameters `var`, declaring all of the publisher's parameters (the reading one, checked by
    /// `through_subscriber`, and the other in `TwoSubscribers_*`) or only one, a record of the table (`Wide_*`); parameters
    /// bind by name, so a subscriber that declares only another parameter is not measured (review of PR 8304, 2026-11-22).
    /// record/validate/XRecTableProcCallers.al, corpus PR 1482, merged at 5ad2970e, not pinned yet: assumed, `Some(true)`.
    fn subscribers_measured(&self, publisher_o: usize, publisher: &Routine) -> Option<bool> {
        let subscribers = self
            .engine
            .events
            .subscribers_at(publisher_o, &publisher.name.to_lowercase())?;
        let measured = |s: &crate::events::Subscriber| {
            let r = &self.engine.program.objects[s.object].routines[s.routine];
            let only_the_record = r.params == 1
                && matches!(r.locals.first().map(|l| &l.ty), Some(RtType::Record(t, _)) if *t as usize == publisher_o);
            !s.manual
                && !self.engine.object_facts[s.object].manual_subscriber
                && !self.engine.object_facts[s.object].single_instance
                && (r.params == publisher.params || only_the_record)
                && (0..r.params).all(|i| r.by_var.get(i) == Some(&true))
        };
        match subscribers.len() {
            1 => Some(false),
            2 if subscribers.iter().all(measured) => Some(true),
            _ => None,
        }
    }

    /// Whether `publisher` is marked `[IntegrationEvent(false, false)]`, the publisher record/validate/
    /// XRecTableProcThroughSubscriber.al measured (no sender, not global).
    fn integration_event_measured(publisher: &Routine) -> bool {
        use al_ir::Const::Boolean;
        publisher
            .attributes
            .iter()
            .any(|a| a.name.eq_ignore_ascii_case("IntegrationEvent") && a.args == [Boolean(false), Boolean(false)])
    }

    /// Whether frame `i` (from the outside in) runs the field OnValidate trigger of table `table`, not as an event.
    pub(super) fn field_on_validate_at(&self, table: usize, i: usize) -> bool {
        let r = self.routine_at(i).1;
        self.frames[i] == table
            && !self.call_stack[i].event
            && r.is_trigger
            && r.field.is_some()
            && r.name.eq_ignore_ascii_case("OnValidate")
    }

    /// The outer and the inner Validate when the Validate of the record at `handle` whose trigger runs in frame
    /// `inner` was nested by the table's own code in one other Validate of the record, the shape
    /// record/validate/XRecTableProcThroughSubscriber.al:590-618, :663-718 measured: two Validates of the record run, both
    /// `Record.Validate` from AL code (:1098 with one argument); the outer one's trigger is a field OnValidate of the table, of
    /// another field; between the two triggers run at most [`NESTING_PROCEDURES_MEASURED`] procedures of the table that are no
    /// triggers, each called on the outer trigger's own record: unqualified (:591), as `Rec.Proc()` (:1084) or as
    /// `this.Proc()`, which the runner does not tell apart from the two (BC gives all three forms the trigger's xRec,
    /// record/validate/XRecTableProcCalls.al:66; `plain_proc_xrec` notes a call on another record).
    fn nested_by_own_code(&self, table: usize, handle: usize, inner: usize) -> Option<(&ValidateRun, &ValidateRun)> {
        let mut running = self.validate_sites.iter().filter(|v| v.handle == handle);
        let (Some(outer), Some(inner_run), None) = (running.next(), running.next(), running.next()) else {
            return None;
        };
        let from_al = Some(ValidateSite::AlRecordValidate);
        let (o, i) = (outer.depth, inner_run.depth);
        if outer.site != from_al || inner_run.site != from_al || i != inner || o >= i {
            return None;
        }
        if i - o - 1 > NESTING_PROCEDURES_MEASURED || !self.field_on_validate_at(table, o) {
            return None;
        }
        let field = |d: usize| self.routine_at(d).1.field;
        (field(o) != field(i) && self.own_procedures_between(table, o, i)).then_some((outer, inner_run))
    }

    /// Whether every frame between frame `o` and frame `i` runs a procedure of `table` that is no trigger, called directly (not
    /// through an event) on the record of the frame below (`plain_proc_xrec` notes a call on another record).
    pub(super) fn own_procedures_between(&self, table: usize, o: usize, i: usize) -> bool {
        ((o + 1)..i).all(|d| {
            self.frames[d] == table
                && !self.call_stack[d].event
                && !self.routine_at(d).1.is_trigger
                && !self.plain_proc_xrec.iter().any(|(_, depth, _)| *depth == d + 1)
        })
    }

    /// Whether the two xRecs of a nested Validate (`nested_by_own_code`) still hold what the outer Validate handed its trigger:
    /// the outer Validate's before-image, and the same filters (each xRec takes the record's filters when its Validate starts).
    /// Real BC keeps one xRec for both Validates: what the outer trigger assigns before it nests shows in the inner trigger and
    /// the procedures (:1042), what the inner trigger assigns shows in the outer one (:1056), and a filter the procedure sets
    /// shows in both triggers (:1078). This runner keeps two, so a difference refuses. An assignment of the value a field
    /// already holds changes nothing either reads.
    fn nested_xrecs_untouched(&self, handle: usize, outer: &Rec, inner: &Rec) -> bool {
        let Some((_, (before, _))) = self.validating.iter().find(|(h, _)| *h == handle) else {
            return false;
        };
        let (o, i) = (outer.borrow(), inner.borrow());
        Self::same_values(&o.fields, before)
            && Self::same_values(&i.fields, before)
            && XrecView::of(&o).same(&XrecView::of(&i))
    }
    /// The buffers record/validate/XRecTableProcThroughCodeunit.al tried the chain on, each by the facts of its origin
    /// (own write, read in the write transaction, lock rule, own Modify, Find of a committed version, the read; `None`: no row
    /// version): every one hands on the trigger's xRec (the file's facts, :26-28). Any other buffer (a GetBySystemId, a Find
    /// other than an AL FindFirst, a committed FindFirst, an own Modify, a TransferFields, a temporary record read back) refuses.
    /// The read is told apart by `RowOrigin::read_by`; the row's writes before a Get are not read: the cells Get a row no write
    /// of the open transaction touched, one it inserted, and one another variable modified after a Commit or after its Insert.
    fn measured_buffer(r: &RecordState) -> bool {
        use al_builtins::value::ReadBy::{AlFindFirst, Get, NoRead};
        match (r.temporary, Self::origin_facts(r)) {
            // `AfterInsert` :337-345 (`Way3_PlainCodeunit_AfterInsert` :365).
            (false, Some((true, false, false, false, false, NoRead))) => true,
            // `AfterGet` :348-353, a Get of the row the open transaction inserted (`Way3_PlainCodeunit_AfterGet` :378), also
            // after another variable modified it (`Way3_GetOfCommittedRowModifiedByOther` :790,
            // `Way3_GetOfInsertedRowModifiedByOther` :810).
            (false, Some((true, true, false, false, false, Get))) => true,
            // A Get of a committed row (`Way3_GetOfCommittedRow` :774).
            (false, Some((false, false, false, false, false, Get))) => true,
            // FindFirst of the row the open transaction inserted (`Way3_FindFirst` :759).
            (false, Some((true, true, false, false, false, AlFindFirst))) => true,
            // A record that was never inserted (`Way3_FreshRecord` :430).
            (false, None) => true,
            // A temporary record after its own Insert (`Way3_TemporaryRecord` :743).
            (true, Some((true, false, false, false, false, NoRead))) => true,
            _ => false,
        }
    }

    /// The buffers the chain straight from the trigger (`straight_from_trigger`) was measured on: `AfterInsert`
    /// (record/validate/XRecTableProcThroughSubscriber.al:498-507; `Codeunit_BaseProc_AfterInsert` :764) and `AfterGet`
    /// (record/validate/XRecTableProcThroughCodeunit.al `CodeunitFromTriggerDirectly_AfterGet` :417). The other buffers of
    /// `measured_buffer` were measured on the chain through a procedure of the table only. From a table extension's field
    /// OnValidate (`ext_field`): `AfterGet` only (`ExtField_Codeunit_BaseProc` :669, `ValidateExtAndObserve` :657).
    fn straight_chain_buffer(r: &RecordState, ext_field: bool) -> bool {
        Self::buffer_after_get(r) || (!ext_field && Self::buffer_of_own_insert(r))
    }

    /// The buffer `AfterGet` (record/validate/XRecTableProcThroughCodeunit.al:348-353): not temporary, a Get of the row the
    /// open transaction wrote.
    fn buffer_after_get(r: &RecordState) -> bool {
        !r.temporary
            && Self::origin_facts(r) == Some((true, true, false, false, false, al_builtins::value::ReadBy::Get))
    }

    /// The page's record of `Way3_TestPageSetValue` (record/validate/XRecTableProcThroughCodeunit.al:447-467): not temporary,
    /// the row the open transaction inserted, read by the page (`GoToKey`).
    fn buffer_of_test_page(r: &RecordState) -> bool {
        !r.temporary
            && Self::origin_facts(r) == Some((true, true, false, false, false, al_builtins::value::ReadBy::OtherRead))
    }

    /// The facts of a buffer's origin that tell the measured buffers apart: own write, read in the write transaction, lock rule,
    /// own Modify, Find of a committed version, and the read (`None`: no row version).
    fn origin_facts(r: &RecordState) -> Option<(bool, bool, bool, bool, bool, al_builtins::value::ReadBy)> {
        r.row_origin.map(|o| {
            (
                o.own_write,
                o.read_in_write_tx,
                o.lock_only,
                o.own_modified,
                o.from_find,
                o.read_by,
            )
        })
    }

    /// The buffer `AfterInsert` (record/validate/XRecTableProcThroughCodeunit.al:337-344, XRecTableProcThroughSubscriber.al:498-507):
    /// a record that is not temporary, whose buffer came from its own `Insert`, with no Modify, Rename or read after it.
    pub(super) fn buffer_of_own_insert(r: &RecordState) -> bool {
        r.row_origin.as_ref().is_some_and(|o| {
            !r.temporary
                && o.own_write
                && !o.own_modified
                && !o.lock_only
                && !o.from_find
                && !o.read_in_write_tx
                && o.read_by == al_builtins::value::ReadBy::NoRead
        })
    }
}

/// What a reader can tell apart on an xRec besides its field values: filters, filter group, marks, MarkedOnly, key and sort.
/// Under a nested Validate the two xRecs this runner keeps must agree on all of it at the call, and the shared one must keep
/// it to the end of the call (`interp.rs`, `ProcXrec::SharedReadOnly`).
pub(crate) struct XrecView {
    filters: Vec<al_builtins::filter::FieldFilter>,
    filter_group: i32,
    marks: usize,
    marked_only: bool,
    key: usize,
    adhoc: bool,
    ascending: bool,
    field_descending: u32,
}

impl XrecView {
    pub(crate) fn of(r: &RecordState) -> Self {
        XrecView {
            filters: r.filters.clone(),
            filter_group: r.filter_group,
            marks: r.marks.len(),
            marked_only: r.marked_only,
            key: r.key,
            adhoc: r.adhoc.is_some(),
            ascending: r.ascending,
            field_descending: r.field_descending,
        }
    }

    /// The view of a record no filter, mark, key or sort was set on: what the buffer `AfterInsert` has.
    pub(crate) fn untouched(&self) -> bool {
        self.filters.is_empty()
            && (
                self.filter_group,
                self.marks,
                self.marked_only,
                self.key,
                self.adhoc,
                self.ascending,
                self.field_descending,
            ) == (0, 0, false, 0, false, true, 0)
    }

    pub(crate) fn same(&self, o: &Self) -> bool {
        crate::records::same_filters(&self.filters, &o.filters)
            && (
                self.filter_group,
                self.marks,
                self.marked_only,
                self.key,
                self.adhoc,
                self.ascending,
                self.field_descending,
            ) == (
                o.filter_group,
                o.marks,
                o.marked_only,
                o.key,
                o.adhoc,
                o.ascending,
                o.field_descending,
            )
    }
}
