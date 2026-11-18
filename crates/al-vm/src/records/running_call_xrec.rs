//! The xRec of a procedure of a table or of one of its table extensions that code other than the table's own calls on a record
//! while a trigger of the same record runs (`Session::running_call_xrec`, by `table_proc_xrec` in `nested_validate.rs`): the
//! running field trigger's xRec, shared, in the chains of calls the corpus measured, else a blank xRec marked
//! `XREC_OF_RUNNING_CALL` whose read refuses.
//!
//! Each chain starts in a field OnValidate trigger of the table and ends in a codeunit procedure that has the record as a `var`
//! parameter and calls the procedure on it (al-runner issues 8189 and 8284):
//! - through a procedure of the table that is no trigger (`through_own_procedure`, assumption `xrec-table-proc-through-codeunit`);
//! - straight from the trigger (`straight_from_trigger`, not through an interface), or from the subscriber of an integration
//!   event the trigger raises
//!   (`through_subscriber`), assumption `xrec-table-proc-through-subscriber`. Under a Validate the table's own code nested, the
//!   xRec is shared for reading only (`ProcXrec::SharedReadOnly`): each Validate here has its own xRec, where BC shows an
//!   assignment to the outer trigger too.

use super::nested_validate::{ProcXrec, ValidateSite};
use crate::records::Rec;
use crate::{Frame, Session};
use al_builtins::assumptions::{self, Assumption};
use al_builtins::value::{RecordState, RtType, Value};
use al_ir::{ObjKind, Routine};
use std::rc::Rc;

/// How many procedures of the table record/validate/XRecTableProcThroughSubscriber.al measured between the trigger that nests a
/// Validate and the Validate (none, one, two and three: `Subscriber_VarRec_NestedValidateInTrigger` :486,
/// `..._NestedValidateInFirstTableProcedure` :514, `..._NestedValidateInTableProcedure` :413,
/// `..._NestedValidateInThirdTableProcedure` :500).
const NESTING_PROCEDURES_MEASURED: usize = 3;

/// The refusal when a procedure that shared the trigger's xRec under a Validate the table's own code nested (`nested_by_own_code`)
/// assigned to it: real BC shows the assignment to the trigger of the outer Validate too
/// (record/validate/XRecTableProcThroughSubscriber.al:573 `I after x=0/SET/I1`), where this runner keeps that trigger's xRec apart.
pub(crate) const XREC_ASSIGNED_UNDER_NESTED_VALIDATE: &str = "an assignment to xRec in a table procedure called through an event subscriber under a Validate that the table's own code nested: real BC shows it to the outer Validate's trigger too, which this runner keeps apart";

/// The chain of calls `measured_chain` found below a procedure call: the assumption it runs through, and whether a Validate
/// the table's own code nested runs (the procedure's xRec is then shared for reading only).
struct Chain {
    assumption: Assumption,
    nested: bool,
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
    ///   record/validate/XRecTableProcThroughSubscriber.al:559, :573, :603).
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
            && assumptions::hit(chain.assumption)
        {
            return if chain.nested {
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
        let assumption = Assumption::XrecTableProcThroughSubscriber;
        let subscriber = callee
            .attributes
            .iter()
            .any(|a| a.name.eq_ignore_ascii_case("EventSubscriber"));
        if subscriber && let Some(nested) = self.through_subscriber(object, table, rc, f) {
            return Some(Chain { assumption, nested });
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
            assumption
        } else {
            return None;
        };
        Some(Chain {
            assumption,
            nested: false,
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
    /// `BaseField_Codeunit_ExtProc`; record/validate/XRecTableProcThroughSubscriber.al:470, :587, :603).
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
    /// (record/validate/XRecTableProcThroughSubscriber.al:351-399, :545, :619, :709-745). Every fact by equality:
    /// - frame `f` runs a subscriber of a codeunit that is not bound manually; below it the publisher's event frame, a procedure
    ///   of the table marked `[IntegrationEvent(false, false)]`, whose one or two parameters are all `var`, the first being `rc`
    ///   and the second a record of another table; below that the table's field OnValidate;
    /// - the subscriber declares the publisher's parameters, all `var`;
    /// - either the Validate is the one `Record.Validate` AL code called on the record, whose trigger is that frame; or
    ///   `nested_by_own_code` nested it (record/validate/XRecTableProcThroughSubscriber.al:413-441, :486-541, :573, :636, :655).
    ///
    /// Whatever the buffer: the xRec is the record before the Validate, not the stored row, on every buffer the probes tried
    /// (here: after Insert, Get, FindFirst, FindLast, FindSet, Find('='), Next, FindLast then Init with a new key, on a fresh and
    /// on a temporary record; record/validate/XRecTableProcThroughCodeunit.al, for the codeunit chain: Get of a committed row and
    /// of a row the open transaction modified).
    ///
    /// `Some(nested)` for the chain, `nested` telling the second case.
    fn through_subscriber(&self, object: usize, table: usize, rc: &Rec, f: &Frame) -> Option<bool> {
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
        if self.sole_validate_from_al_at(handle, n - 3) {
            return Some(false);
        }
        self.nested_by_own_code(table, handle, n - 3).then_some(true)
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

    /// Whether the Validate of the record at `handle` whose trigger runs in frame `inner` was nested by the table's own code in
    /// one other Validate of the record, the shape record/validate/XRecTableProcThroughSubscriber.al:413-441, :486-541 measured:
    /// two Validates of the record run, both `Record.Validate` from AL code; the outer one's trigger is a field OnValidate of the
    /// table, of another field; between the two triggers run at most [`NESTING_PROCEDURES_MEASURED`] procedures of the table
    /// that are no triggers, none entered through a call on a record value (`plain_proc_xrec` notes such a call on another
    /// record), so each runs on the outer trigger's record.
    fn nested_by_own_code(&self, table: usize, handle: usize, inner: usize) -> bool {
        let mut running = self.validate_sites.iter().filter(|(h, ..)| *h == handle);
        let (Some(&(_, outer_site, outer)), Some(&(_, inner_site, inner_at)), None) =
            (running.next(), running.next(), running.next())
        else {
            return false;
        };
        let from_al = Some(ValidateSite::AlRecordValidate);
        if outer_site != from_al || inner_site != from_al || inner_at != inner || outer >= inner {
            return false;
        }
        if inner - outer - 1 > NESTING_PROCEDURES_MEASURED || !self.field_on_validate_at(table, outer) {
            return false;
        }
        let objects = &self.engine.program.objects;
        let field = |i: usize| objects[self.frames[i]].routines[self.call_stack[i].routine].field;
        field(outer) != field(inner)
            && ((outer + 1)..inner).all(|i| {
                let (o, cf) = (self.frames[i], self.call_stack[i]);
                o == table
                    && !cf.event
                    && !objects[o].routines[cf.routine].is_trigger
                    && !self.plain_proc_xrec.iter().any(|(_, d, _)| *d == i + 1)
            })
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
        let last = self.tx.writes.iter().rev().find(|w| {
            w.table == t.object
                && w.company == r.company
                && (crate::records::same_key(&w.pk, &pk)
                    || !matches!(w.kind, TxWriteKind::Insert | TxWriteKind::Modify | TxWriteKind::Delete))
        });
        self.writes_all_noted()
            && last.is_some_and(|w| w.kind == TxWriteKind::Insert && crate::records::same_key(&w.pk, &pk))
    }
}
