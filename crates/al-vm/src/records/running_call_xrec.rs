//! The xRec of a procedure of a table or of one of its table extensions that code other than the table's own calls on a record
//! while a trigger of the same record runs (`Session::running_call_xrec`, by `table_proc_xrec` in `nested_validate.rs`): the
//! running field trigger's xRec, shared, in the chains of calls the corpus measured, else a blank xRec marked
//! `XREC_OF_RUNNING_CALL` whose read refuses.
//!
//! Each chain starts in a field OnValidate trigger of the table and ends in a codeunit procedure that has the record as a `var`
//! parameter and calls the procedure on it (al-runner issues 8189 and 8284):
//! - through a procedure of the table that is no trigger (`through_own_procedure`, assumption `xrec-table-proc-through-codeunit`);
//! - straight from the trigger (`straight_from_trigger`, not through an interface), or from the subscriber of an integration
//!   event the trigger raises (`through_subscriber`), assumption `xrec-table-proc-through-subscriber`. Under a Validate the
//!   table's own code nested, BC keeps one xRec for both Validates where this runner keeps two: the xRec is shared only while
//!   neither was assigned or filtered (`nested_xrecs_untouched`), and for reading only (`ProcXrec::SharedReadOnly`).

use super::nested_validate::{ProcXrec, ValidateSite};
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
const NESTING_PROCEDURES_MEASURED: usize = 3;

/// The refusal when a procedure that shared the trigger's xRec under a Validate the table's own code nested (`nested_by_own_code`)
/// changed it: real BC keeps one xRec for both Validates, so an assignment shows to the trigger of the outer Validate too
/// (record/validate/XRecTableProcThroughSubscriber.al:750 `I after x=0/SET/I1`) and a filter in both triggers (:1070
/// `I filters=Series: F`), where this runner keeps the two triggers' xRecs apart.
pub(crate) const XREC_CHANGED_UNDER_NESTED_VALIDATE: &str = "a change to xRec (an assignment, a filter or another view of it) in a table procedure called through an event subscriber under a Validate that the table's own code nested: real BC shows it to the outer Validate's trigger too, which this runner keeps apart";

/// The chain of calls `measured_chain` found below a procedure call: the assumption it runs through, and, when a Validate the
/// table's own code nested runs, that Validate's xRec (the procedure's xRec is then shared for reading only).
struct Chain {
    assumption: Assumption,
    nested: Option<Rec>,
}

impl Session {
    /// A procedure of table `object` (or of one of its table extensions) called on `rc` by code that holds no xRec of it (frame
    /// `f`, no case of `frame_xrec`).
    /// - Called from the code of the record's table or of one of its table extensions (a trigger or procedure of any of them): a
    ///   plain call, as `table_proc_xrec` lists (record/PWTableGlobals.al:196-260; al-test-cli suite `xrec_proc_call_scope.rs`).
    /// - Called from other code while triggers or procedures on the same record run further up: the instances of the record
    ///   variable (ADR-021) of its table and of its table extensions that hold `rc` as Rec, and those calls' xRecs
    ///   (`running_xrecs`).
    /// - A chain the corpus measured (`measured_chain`): the xRec of the call running on the table's own instance (the field
    ///   trigger's), shared, so an assignment to it shows in the trigger after the call
    ///   (record/validate/XRecTableProcThroughCodeunit.al `Way3_ProcAssignsXRec_TriggerReadsAfter`,
    ///   record/validate/XRecTableProcThroughSubscriber.al:736, :750, :780).
    /// - Any other shape, when one of those xRecs holds other values than the blank record a plain call gets: the blank xRec
    ///   marked `XREC_OF_RUNNING_CALL`, so a read of it refuses.
    /// - Else, and when no call on the record runs: a plain call (record/PWTableGlobals.al:196-260 XRecPlainProc_*).
    pub(super) fn running_call_xrec(&mut self, object: usize, rc: &Rec, f: &Frame) -> ProcXrec {
        let table = rc.borrow().table as usize;
        if self.table_or_its_extension(f.object, table) {
            return ProcXrec::Plain;
        }
        let blank = self.initial_fields(table);
        let not_blank = |x: &Rec| !Self::same_values(&x.borrow().fields, &blank);
        let running = self.running_xrecs(table, rc);
        if !running.iter().any(|(_, x)| not_blank(x)) {
            return ProcXrec::Plain;
        }
        let of_table = running.iter().find(|(o, _)| *o == table).map(|(_, x)| x.clone());
        if let Some(x) = of_table
            && not_blank(&x)
            && let Some(chain) = self.measured_chain(object, table, rc, f)
            && chain.nested.as_ref().is_none_or(|inner| Rc::ptr_eq(inner, &x))
            && assumptions::hit(chain.assumption)
        {
            return if chain.nested.is_some() {
                ProcXrec::SharedReadOnly(x)
            } else {
                ProcXrec::Shared(x)
            };
        }
        ProcXrec::PlainUnpinned
    }

    /// The xRecs of the calls running on the record variable `rc` of table `table`: of each instance (ADR-021) of the table
    /// and of its table extensions whose Rec slot holds `rc` (`enter_record` fills it while the call runs, `leave_record`
    /// empties it), with the instance's object.
    fn running_xrecs(&self, table: usize, rc: &Rec) -> Vec<(usize, Rec)> {
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
        let assumption = if self.through_own_procedure(object, table, rc) {
            Assumption::XrecTableProcThroughCodeunit
        } else if !subscriber && self.straight_from_trigger(object, table, rc) {
            Assumption::XrecTableProcThroughSubscriber
        } else {
            return None;
        };
        Some(Chain {
            assumption,
            nested: None,
        })
    }

    /// The chain al-runner issue 8189 measured (ways 3 and 4 of the reporter's repro): `object` is the table itself; below the
    /// codeunit procedure (plain or reached through an interface, as on main also a subscriber of a table event the procedure
    /// raised), a procedure of the table that is no trigger, then the table's field OnValidate; the Validate is the one
    /// `Record.Validate` AL code called on the record (`sole_validate_from_al`), on a buffer of `measured_buffer`.
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
    /// `BaseField_Codeunit_ExtProc`; record/validate/XRecTableProcThroughSubscriber.al:647, :764, :780).
    fn straight_from_trigger(&self, object: usize, table: usize, rc: &Rec) -> bool {
        let n = self.frames.len();
        self.table_or_its_extension(object, table)
            && !self.iface_frames.contains(&(n - 1))
            && self.field_on_validate_at(table, n - 2)
            && self.sole_validate_from_al_at(Rc::as_ptr(rc) as usize, n - 2)
            && self.measured_buffer(&rc.borrow())
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
        let handle = Rc::as_ptr(rc) as usize;
        let assumption = Assumption::XrecTableProcThroughSubscriber;
        if self.sole_validate_from_al_at(handle, n - 3) {
            return Some(Chain {
                assumption,
                nested: None,
            });
        }
        let (outer, inner) = self.nested_by_own_code(table, handle, n - 3)?;
        self.nested_xrecs_untouched(handle, &outer, &inner).then_some(Chain {
            assumption,
            nested: Some(inner),
        })
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
    fn field_on_validate_at(&self, table: usize, i: usize) -> bool {
        let (o, cf) = (self.frames[i], self.call_stack[i]);
        let r = &self.engine.program.objects[o].routines[cf.routine];
        o == table && !cf.event && r.is_trigger && r.field.is_some() && r.name.eq_ignore_ascii_case("OnValidate")
    }

    /// The xRecs of the outer and the inner Validate when the Validate of the record at `handle` whose trigger runs in frame
    /// `inner` was nested by the table's own code in one other Validate of the record, the shape
    /// record/validate/XRecTableProcThroughSubscriber.al:590-618, :663-718 measured: two Validates of the record run, both
    /// `Record.Validate` from AL code (:1098 with one argument); the outer one's trigger is a field OnValidate of the table, of
    /// another field; between the two triggers run at most [`NESTING_PROCEDURES_MEASURED`] procedures of the table that are no
    /// triggers, each called on the outer trigger's own record: unqualified (:591), as `Rec.Proc()` (:1084) or as
    /// `this.Proc()`, which the runner does not tell apart from the two (`plain_proc_xrec` notes a call on another record).
    fn nested_by_own_code(&self, table: usize, handle: usize, inner: usize) -> Option<(Rec, Rec)> {
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
        let objects = &self.engine.program.objects;
        let field = |d: usize| objects[self.frames[d]].routines[self.call_stack[d].routine].field;
        let own_procedures = ((o + 1)..i).all(|d| {
            let (ob, cf) = (self.frames[d], self.call_stack[d]);
            ob == table
                && !cf.event
                && !objects[ob].routines[cf.routine].is_trigger
                && !self.plain_proc_xrec.iter().any(|(_, depth, _)| *depth == d + 1)
        });
        (field(o) != field(i) && own_procedures).then(|| (outer.xrec.clone(), inner_run.xrec.clone()))
    }

    /// Whether the two xRecs of a nested Validate (`nested_by_own_code`) still hold what the outer Validate handed its trigger:
    /// the outer Validate's before-image, and the same filters (each xRec takes the record's filters when its Validate starts).
    /// Real BC keeps one xRec for both Validates: what the outer trigger assigns before it nests shows in the inner trigger and
    /// the procedures (:1042), what the inner trigger assigns shows in the outer one (:1056), and a filter the procedure sets
    /// shows in both triggers (:1070). This runner keeps two, so a difference refuses. An assignment of the value a field
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
    /// The two buffers al-runner issue 8189 measured (the reporter's `PrepareAfterInsert` and `PrepareAfterGet`): a record that is
    /// not temporary, whose buffer came from its own `Insert` (no Modify, Rename or read after it), or from a `Get` of the row
    /// that its own transaction inserted and wrote nothing to since (`inserted_unwritten_in_tx`). A Find, a GetBySystemId, a
    /// Get of a committed row, a Get of a row the open transaction modified or a TransferFields refuse.
    fn measured_buffer(&self, r: &RecordState) -> bool {
        let Some(o) = r.row_origin else {
            return false;
        };
        let own_insert = !o.read_in_write_tx && !o.by_get;
        let get_of_own_insert = o.read_in_write_tx && o.by_get && self.inserted_unwritten_in_tx(r);
        !r.temporary
            && o.own_write
            && !o.own_modified
            && !o.lock_only
            && !o.from_find
            && (own_insert || get_of_own_insert)
    }

    /// Whether the open transaction inserted the row of the buffer `r` and wrote nothing to it since: every store write since
    /// the last Commit or rollback is noted (`writes_all_noted`), and the last of them that is to the row's key or to its whole
    /// table (a Rename, ModifyAll, DeleteAll, Truncate or other write whose rows are not told apart) is an Insert of the row.
    fn inserted_unwritten_in_tx(&self, r: &RecordState) -> bool {
        use crate::state::TxWriteKind;
        let Some(t) = self.engine.tables.get(r.table as usize).and_then(|t| t.as_deref()) else {
            return false;
        };
        let pk = t.primary_key(&r.fields);
        let last = self.tx.open.writes.iter().rev().find(|w| {
            w.table == t.object
                && w.company == r.company
                && (crate::records::same_key(&w.pk, &pk)
                    || !matches!(w.kind, TxWriteKind::Insert | TxWriteKind::Modify | TxWriteKind::Delete))
        });
        self.writes_all_noted()
            && last.is_some_and(|w| w.kind == TxWriteKind::Insert && crate::records::same_key(&w.pk, &pk))
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
