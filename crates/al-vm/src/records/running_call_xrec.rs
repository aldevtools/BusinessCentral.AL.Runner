//! The xRec of a procedure of a table or of one of its table extensions that code other than the table's own calls on a record
//! while a trigger of the same record runs (`Session::running_call_xrec`, by `table_proc_xrec` in `nested_validate.rs`): the
//! running field trigger's xRec, shared, in the chains of calls the corpus measured, else a blank xRec marked
//! `XREC_OF_RUNNING_CALL` whose read refuses. While a table extension's insert or delete trigger runs on the record, the
//! blank xRec carries that trigger's mark instead (`ext_trigger_xrec.rs`, not measured), checked before every chain.
//!
//! Each chain starts in a field OnValidate trigger of the table and ends in a codeunit procedure that has the record as a `var`
//! parameter and calls the procedure on it (al-runner issues 8189 and 8284):
//! - through a procedure of the table that is no trigger (`through_own_procedure`, record/validate/XRecTableProcThroughCodeunit.al:365-414);
//! - straight from the trigger (`straight_from_trigger`, not through an interface), or from the subscriber of an integration
//!   event the trigger raises (`through_subscriber`), record/validate/XRecTableProcThroughSubscriber.al. Under a Validate the
//!   table's own code nested, BC keeps one xRec for both Validates; this runner does too in the nests
//!   `nested_validate_xrec` shares. Where the inner Validate kept its own copy, the xRec is shared only while neither was assigned
//!   or filtered (`nested_xrecs_untouched`), and for reading only (`ProcXrec::SharedReadOnly`).
//!
//! Every chain also needs the facts its cells share (`cell_facts`); the Validate called by other code than a test procedure, and
//! an event with two subscribers, run through assumption `xrec-table-proc-callers-subscribers` (`callers_assumed`,
//! `subscribers_measured`). A record changed after its OnBeforeValidateEvent returned on the two
//! chains of al-runner issue 8284 runs through assumption `xrec-table-proc-routes` (al-runner issue 8974).
//!
//! The xRec of a procedure of a table or of its table extensions that a codeunit procedure calls on
//! its `var` record while a field OnValidate of that record runs: the trigger's xRec, shared, in the
//! measured chains (through a procedure of the table,
//! record/validate/XRecTableProcThroughCodeunit.al; straight from the trigger, or from the sole
//! subscriber of an integration event the trigger raises, also under a Validate the table's own code
//! nested, record/validate/XRecTableProcThroughSubscriber.al; only with the facts every cell shares,
//! `cell_facts`: no SingleInstance codeunit, the record untouched since the Validate started, the
//! outer Validate called by a test codeunit, or, through assumption `xrec-table-proc-callers-
//! subscribers`, by the callers `callers_assumed` lists and with two subscribers,
//! `subscribers_measured`; under the nested Validate shared only while both Validates' xRecs hold the
//! outer before-image and one view, and for reading: a change of values or view refuses,
//! `XREC_CHANGED_UNDER_NESTED_VALIDATE`, `XrecView`), else a blank xRec marked `XREC_OF_RUNNING_CALL`
//! whose reads refuse, also in the triggers of a Validate it calls (`BeforeImage`), and in a
//! procedure the shared one calls on Rec (`shared_xrec_of_own_procedure`).

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
pub(crate) const XREC_CHANGED_UNDER_NESTED_VALIDATE: &str = "a change to xRec (an assignment, a filter or another view of it) in a table procedure called through an event subscriber under a Validate that the table's own code nested: real BC shows it to the outer Validate's trigger too, which this runner keeps apart";

/// The chain of calls `measured_chain` found below a procedure call: when a Validate the table's own code nested runs, that
/// Validate's xRec (the procedure's xRec is then shared for reading only), and whether a fact of the chain is only measured by
/// the open probe of assumption `xrec-table-proc-callers-subscribers` (`cell_facts`, `subscribers_measured`), and whether the
/// record changed since its Validate started, only by assumption `xrec-table-proc-routes` (`cell_facts`).
struct Chain {
    nested: Option<Rec>,
    /// Under the nested Validate, whether its two xRecs are two copies (`nested_validate_xrec` gave the inner Validate its own),
    /// so the procedure shares the inner one for reading only.
    read_only: bool,
    assumed: bool,
    routes: bool,
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
        let of_table = running.iter().find(|(o, _)| *o == table).map(|(_, x)| x.clone());
        if let Some(x) = of_table
            && not_blank(&x)
            && let Some(chain) = self.measured_chain(object, table, rc, f)
            && chain.nested.as_ref().is_none_or(|inner| Rc::ptr_eq(inner, &x))
            && (!chain.assumed || assumptions::hit(Assumption::XrecTableProcCallersSubscribers))
            && (!chain.routes || assumptions::hit(Assumption::XrecTableProcRoutes))
        {
            return if chain.read_only {
                ProcXrec::SharedReadOnly(x)
            } else {
                ProcXrec::Shared(x)
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
            return Some(Chain {
                nested: None,
                read_only: false,
                assumed: false,
                routes: false,
            });
        }
        let (assumed, routes) = (!subscriber)
            .then(|| self.straight_from_trigger(object, table, rc))
            .flatten()?;
        Some(Chain {
            nested: None,
            read_only: false,
            assumed,
            routes,
        })
    }

    /// The chain record/validate/XRecTableProcThroughCodeunit.al pins (`Way3_*` and `Way4_*`, :365-414): `object` is the table
    /// itself; below the codeunit procedure (plain or reached through an interface, as on main also a subscriber of a table event
    /// the procedure raised; `XTC Mover.Handle` :294-296, `XTC Iface Mover.Handle` :322-324), a procedure of the table that is no
    /// trigger (`CallPlain` :141-145, `CallIface` :148-154), then the table's field OnValidate (:46-58); the Validate is the one
    /// `Record.Validate` AL code called on the record (`sole_validate_from_al`, :358), on a buffer of `measured_buffer`.
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
            && self.sole_validate_from_al(Rc::as_ptr(rc) as usize)
            && self.measured_buffer(&rc.borrow())
    }

    /// The codeunit procedure, no event subscriber and not reached through an interface (`iface_frames`), called straight from the
    /// table's field OnValidate calls a procedure of the table or of one of its table extensions: the trigger's xRec on a buffer
    /// of `measured_buffer`, the Validate being the one `Record.Validate` AL code called on the record, whose trigger is that
    /// frame (record/validate/XRecTableProcThroughCodeunit.al `CodeunitFromTriggerDirectly_AfterGet`,
    /// `BaseField_Codeunit_ExtProc`; record/validate/XRecTableProcThroughSubscriber.al:647, :764, :780). Which facts are
    /// only assumed (`cell_facts`).
    fn straight_from_trigger(&self, object: usize, table: usize, rc: &Rec) -> Option<(bool, bool)> {
        let n = self.frames.len();
        let shape = self.table_or_its_extension(object, table)
            && !self.iface_frames.contains(&(n - 1))
            && self.field_on_validate_at(table, n - 2)
            && self.sole_validate_from_al_at(Rc::as_ptr(rc) as usize, n - 2)
            && self.measured_buffer(&rc.borrow());
        if !shape {
            return None;
        }
        self.cell_facts(rc, self.frames[n - 1], n - 2, n - 2, false)
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
                nested: None,
                read_only: false,
                assumed: assumed || two_subscribers,
                routes,
            });
        }
        let (outer, inner) = self.nested_by_own_code(table, handle, n - 3)?;
        let (assumed, routes) = self.cell_facts(rc, cu, n - 3, outer.depth, true)?;
        // One xRec for both Validates (`nested_validate_xrec`): shared as a non-nested one is.
        let one_xrec = Rc::ptr_eq(&outer.xrec, &inner.xrec);
        (one_xrec || self.nested_xrecs_untouched(handle, &outer.xrec, &inner.xrec)).then(|| Chain {
            nested: Some(inner.xrec.clone()),
            read_only: !one_xrec,
            assumed: assumed || two_subscribers,
            routes,
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
    /// The two buffers record/validate/XRecTableProcThroughCodeunit.al pins (`AfterInsert` :337-344 and `AfterGet` :348-352): a record that is
    /// not temporary, whose buffer came from its own `Insert` (no Modify, Rename or read after it), or from a `Get` of the row
    /// that its own transaction inserted and wrote nothing to since (`inserted_unwritten_in_tx`). A Find, a GetBySystemId, a
    /// Get of a committed row, a Get of a row the open transaction modified or a TransferFields refuse.
    fn measured_buffer(&self, r: &RecordState) -> bool {
        let get_of_own_insert =
            |o: &al_builtins::value::RowOrigin| o.read_in_write_tx && o.by_get && self.inserted_unwritten_in_tx(r);
        Self::buffer_of_own_insert(r) || Self::buffer_of_own_write(r, get_of_own_insert)
    }

    /// The buffer `AfterInsert` (record/validate/XRecTableProcThroughCodeunit.al:337-344, XRecTableProcThroughSubscriber.al:498-507):
    /// a record that is not temporary, whose buffer came from its own `Insert`, with no Modify, Rename or read after it.
    pub(super) fn buffer_of_own_insert(r: &RecordState) -> bool {
        Self::buffer_of_own_write(r, |o| !o.read_in_write_tx && !o.by_get)
    }

    fn buffer_of_own_write(r: &RecordState, origin: impl Fn(&al_builtins::value::RowOrigin) -> bool) -> bool {
        r.row_origin.as_ref().is_some_and(|o| {
            !r.temporary && o.own_write && !o.own_modified && !o.lock_only && !o.from_find && origin(o)
        })
    }

    /// Whether the open transaction inserted the row of the buffer `r` and wrote nothing to it since: every store write since
    /// the last Commit or rollback is noted (`writes_all_noted`), and the last of them that is to the row's key or to its whole
    /// table (a Rename, ModifyAll, DeleteAll, Truncate or other write whose rows are not told apart) is an Insert of the row.
    fn inserted_unwritten_in_tx(&self, r: &RecordState) -> bool {
        let Some(t) = self.engine.tables.get(r.table as usize).and_then(|t| t.as_deref()) else {
            return false;
        };
        self.writes_all_noted()
            && super::write_notes::inserted_unwritten(
                &self.tx.open.writes,
                t.object,
                &r.company,
                &t.primary_key(&r.fields),
            )
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
