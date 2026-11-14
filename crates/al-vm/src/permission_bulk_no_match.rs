//! A denied ModifyAll or DeleteAll whose filter matches no row of a table that holds rows (ADR-062 probe P2).
//!
//! BC denies it as it denies one that matches rows: record/Y19PermWrites.al:294-307 pins one shape, and corpus PR 1403
//! (open; record/Y19PermNoMatchShapes.al, record/Y19PermNoMatchTriggers.al) measured the denial in each shape it changed,
//! on each major its CI runs (assumption `bulk-write-no-match-denied`). The measured shapes are allowlists of their facts
//! here; every other no-match shape is refused.

use crate::Session;
use crate::permissions::{Access, Op};
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::filter::FilterExpr;
use al_builtins::native::Signal;
use al_builtins::record_op::RecordOp;
use al_builtins::value::{RtType, Value};

/// Who calls a record method that the permission gate checks.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RecordCaller {
    /// The runtime, on behalf of a page, report, xmlport or validation.
    Runtime,
    /// AL code, through a Record variable.
    Record,
    /// AL code, through a RecordRef.
    RecordRef,
}

impl RecordCaller {
    /// AL code calls the method (a Record or RecordRef method).
    pub(crate) fn al(self) -> bool {
        self != RecordCaller::Runtime
    }
}

/// A record method that runs while the permission gate is on: the facts of the call that started a trigger.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct RecordCall {
    method: RecordOp,
    caller: RecordCaller,
    /// The table object of the record.
    table: usize,
    temporary: bool,
    /// The arguments are exactly `true` (RunTrigger).
    run_trigger: bool,
    /// The call is a statement: its return value is not used.
    statement: bool,
}

/// The rows of the target table and the facts of the record that the measured shapes compare.
struct Target {
    rows: usize,
    /// The one filter: (field position, value), a SetRange to one value in filter group 0.
    filter: Option<(usize, Value)>,
    /// The field positions of each key, the primary key first.
    keys: Vec<Vec<usize>>,
    types: Vec<RtType>,
}

impl Target {
    /// The primary key's field positions.
    fn key(&self) -> &[usize] {
        &self.keys[0]
    }
}

impl Session {
    /// A RecordRef's record method of AL code runs `record_op` next: the gate checks it as a RecordRef's call.
    pub(crate) fn recordref_permission_call(&mut self) {
        if self.permission_checks() {
            self.permission_from_al = true;
            self.permission_from_recordref = true;
        }
    }

    /// The permission gate of a record method `record_op` runs, while the gate is on: it checks the call and opens it in
    /// `record_calls`. Returns whether it opened it; the caller closes it with [`Session::close_record_call`] when the method
    /// returns, also with an error.
    pub(crate) fn permission_gate_and_open(
        &mut self,
        method: RecordOp,
        rec: &crate::records::Rec,
        args: &[Value],
    ) -> Result<bool, Signal> {
        let from_recordref = std::mem::take(&mut self.permission_from_recordref);
        if !self.permission_checks() {
            return Ok(false);
        }
        let caller = match (std::mem::take(&mut self.permission_from_al), from_recordref) {
            (false, _) => RecordCaller::Runtime,
            (true, false) => RecordCaller::Record,
            (true, true) => RecordCaller::RecordRef,
        };
        self.permission_gate_record(method, rec, args, caller)?;
        let (table, temporary) = {
            let r = rec.borrow();
            (r.table as usize, r.temporary)
        };
        self.record_calls.push(RecordCall {
            method,
            caller,
            table,
            temporary,
            run_trigger: matches!(args, [Value::Boolean(true)]),
            statement: !self.ret_used,
        });
        Ok(true)
    }

    /// Closes the record method [`Session::permission_gate_and_open`] opened.
    pub(crate) fn close_record_call(&mut self) {
        self.record_calls.pop();
    }

    /// Whether BC denies `method` (ModifyAll or DeleteAll) on `rec` whose filters match no row while the table holds `rows`
    /// rows, as it denies one that matches rows: true in the measured shapes, false (the caller refuses) in every other.
    /// `by` is the object whose code makes the access; `indirect` whether its Permissions property grants it.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn bulk_no_match_denied(
        &mut self,
        method: RecordOp,
        rec: &crate::records::Rec,
        args: &[Value],
        access: Access,
        indirect: bool,
        by: usize,
        caller: RecordCaller,
        rows: usize,
    ) -> Result<bool, Signal> {
        let t = self.table(rec)?;
        let target = {
            let r = rec.borrow();
            let filter = match r.filters.as_slice() {
                [f] if f.group == 0 && !r.marked_only => match &f.expr {
                    FilterExpr::Range(Some(a), Some(b))
                        if matches!(al_builtins::ops::compare_key_value(a, b), Ok(std::cmp::Ordering::Equal)) =>
                    {
                        Some((f.field, a.clone()))
                    }
                    _ => None,
                },
                _ => None,
            };
            Target {
                rows,
                filter,
                keys: t.keys.clone(),
                types: t.fields.iter().map(|f| f.ty.clone()).collect(),
            }
        };
        // Facts every measured shape shares: a Record variable (not a RecordRef), code whose object grants nothing on the
        // table, a table that runs no trigger (Y19PermObjects.al:9-27 "LT Y19 Row"; Y19PermNoMatchShapes.al:15-26 "LT NM
        // Target", Y19PermNoMatchTriggers.al:19-34 "LT NT Target", corpus PR 1403).
        let table = rec.borrow().table as usize;
        if caller != RecordCaller::Record || indirect || !self.table_runs_no_trigger(table) {
            return Ok(false);
        }
        if self.pinned_no_match(method, args, access, by, &target) {
            return Ok(true);
        }
        if probed_no_match_write(method, args, &target)
            && self.levels_are(access, &[[1, 0, 0, 0], [1, 1, 0, 0]])
            && self.probed_no_match_caller(by)
        {
            assume(Assumption::BulkWriteNoMatchDenied);
            return Ok(true);
        }
        Ok(false)
    }

    /// record/Y19PermWrites.al:294-307 (Y19_P2_SetR, Y19_P2_SetRI; BC 27, 28, 29): `Row.SetRange(Id, 100)` then
    /// `DeleteAll()`, `DeleteAll(true)`, `ModifyAll(Value, 99)` or `ModifyAll(Value, 99, true)` in local procedure `Op` of
    /// test codeunit 91950, called by the test method (RunCase, :257-265). The table "LT Y19 Row" has an Integer primary
    /// key Id and an Integer field Value (Y19PermObjects.al:9-18) and two rows (Seed, Y19PermObjects.al:443-462); the sets
    /// are R and RI on it (Y19_P2_SetR :57, Y19_P2_SetRI :85).
    fn pinned_no_match(&self, method: RecordOp, args: &[Value], access: Access, by: usize, t: &Target) -> bool {
        let Some((field, value)) = &t.filter else {
            return false;
        };
        let written_ok = match method {
            RecordOp::DeleteAll => matches!(args, [] | [Value::Boolean(_)]),
            RecordOp::ModifyAll => match args {
                [Value::Integer(p), v] | [Value::Integer(p), v, Value::Boolean(_)] => {
                    let p = *p as usize;
                    !t.key().contains(&p) && t.types.get(p) == Some(&RtType::Integer) && matches!(v, Value::Integer(_))
                }
                _ => false,
            },
            _ => false,
        };
        written_ok
            && t.rows == 2
            && t.key() == [*field]
            && t.types.get(*field) == Some(&RtType::Integer)
            && matches!(value, Value::Integer(_))
            && self.levels_are(access, &[[1, 0, 0, 0], [1, 1, 0, 0]])
            && self.only_test_codeunit_frames(by, 0)
    }

    /// The code that makes the access in a shape corpus PR 1403 measured (assumption `bulk-write-no-match-denied`):
    /// - a procedure of the test codeunit, with only procedures of it below (Y19PermNoMatchShapes.al:342-371, ops 1-6);
    /// - a procedure of a codeunit of the app without properties, called by procedures of the test codeunit while no record
    ///   method runs (Y19PermNoMatchShapes.al:93-110, :392-395, ops 21 and 22);
    /// - the OnDelete trigger of a table, called by procedures of the test codeunit through one of the record calls in
    ///   [`probed_delete_start`] (Y19PermNoMatchShapes.al:372-391, ops 11-14; Y19PermNoMatchTriggers.al:386-458, ops 31-34,
    ///   51-102, 131);
    /// - a subscriber of a codeunit without properties to the OnBeforeDeleteEvent of a table and to no other event, raised by
    ///   the statement `Delete(true)` of a Record variable of that table in a procedure of the test codeunit
    ///   (Y19PermNoMatchTriggers.al:146-156, :447-451, op 122).
    fn probed_no_match_caller(&self, by: usize) -> bool {
        if self.only_test_codeunit_frames(by, 0) {
            return true;
        }
        let engine = self.engine.clone();
        let o = &engine.program.objects[by];
        let (Some(&last), Some(f)) = (self.frames.last(), self.call_stack.last()) else {
            return false;
        };
        let Some(routine) = o.routines.get(f.routine) else {
            return false;
        };
        let start = self.record_calls.last();
        let app_procedure = o.kind == al_ir::ObjKind::Codeunit
            && o.properties.is_empty()
            && !routine.is_trigger
            && self.record_calls.is_empty();
        let trigger = self.delete_trigger_of(by).is_some() && start.is_some_and(|c| probed_delete_start(c, by));
        // A subscriber runs right above the frame of the code whose Delete raised the event (no publisher frame).
        let subscriber = o.kind == al_ir::ObjKind::Codeunit
            && o.properties.is_empty()
            && start.is_some_and(|c| {
                *c == probed_subscriber_start(c.table)
                    && engine.events.subscriptions_of(by, f.routine) == [(c.table, "onbeforedeleteevent")]
            });
        last == by && !f.event && (app_procedure || trigger || subscriber) && self.only_test_codeunit_frames(by, 1)
    }

    /// Whether the session's levels Read, Insert, Modify and Delete on the accessed table are one of `allowed`.
    fn levels_are(&self, access: Access, allowed: &[[u8; 4]]) -> bool {
        let Some(eff) = &self.permissions else {
            return false;
        };
        let levels = [Op::Read, Op::Insert, Op::Modify, Op::Delete].map(|op| eff.level(Access { op, ..access }));
        allowed.contains(&levels)
    }

    /// The routine of the running frame when it is the OnDelete trigger of table object `by`.
    fn delete_trigger_of(&self, by: usize) -> Option<usize> {
        let engine = self.engine.clone();
        let o = &engine.program.objects[by];
        let f = self.call_stack.last()?;
        let is_delete = o.kind == al_ir::ObjKind::Table
            && self.frames.last() == Some(&by)
            && !f.event
            && o.routines
                .get(f.routine)
                .is_some_and(|r| r.is_trigger && r.field.is_none() && r.name.eq_ignore_ascii_case("OnDelete"));
        is_delete.then_some(f.routine)
    }

    /// Whether every frame of the test, but the last `skip`, is a procedure (not a trigger, not an event) of one test
    /// codeunit, which is `by` when `skip` is 0.
    fn only_test_codeunit_frames(&self, by: usize, skip: usize) -> bool {
        let base = self.test_frame_base();
        let end = self.frames.len().saturating_sub(skip);
        if end <= base || self.call_stack.len() != self.frames.len() {
            return false;
        }
        let engine = self.engine.clone();
        let first = self.frames[base];
        let o = &engine.program.objects[first];
        let test_codeunit = o.kind == al_ir::ObjKind::Codeunit
            && o.properties
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case("Subtype") && v.trim().eq_ignore_ascii_case("Test"));
        test_codeunit
            && (skip != 0 || first == by)
            && (base..end).all(|i| {
                let f = &self.call_stack[i];
                self.frames[i] == first && !f.event && o.routines.get(f.routine).is_some_and(|r| !r.is_trigger)
            })
    }
}

/// The record calls corpus PR 1403 measured as the start of the OnDelete trigger of table `table`, each a statement on that
/// table with RunTrigger true: `Delete(true)` of a Record variable (Y19PermNoMatchShapes.al:372-391,
/// Y19PermNoMatchTriggers.al:407-458), also of a temporary one (Y19PermNoMatchTriggers.al:401-406, op 34); `DeleteAll(true)`
/// of a Record variable, filtered or not (Y19PermNoMatchTriggers.al:387-393, ops 31 and 32); `Delete(true)` of a RecordRef
/// (Y19PermNoMatchTriggers.al:394-400, op 33).
fn probed_delete_start(c: &RecordCall, table: usize) -> bool {
    let call = |method, caller, temporary| RecordCall {
        method,
        caller,
        table,
        temporary,
        run_trigger: true,
        statement: true,
    };
    [
        call(RecordOp::Delete, RecordCaller::Record, false),
        call(RecordOp::Delete, RecordCaller::Record, true),
        call(RecordOp::DeleteAll, RecordCaller::Record, false),
        call(RecordOp::Delete, RecordCaller::RecordRef, false),
    ]
    .contains(c)
}

/// The record call corpus PR 1403 measured as the start of an OnBeforeDeleteEvent subscriber: the statement `Delete(true)`
/// of a Record variable, not temporary, of table `table` (Y19PermNoMatchTriggers.al:447-451, op 122).
fn probed_subscriber_start(table: usize) -> RecordCall {
    RecordCall {
        method: RecordOp::Delete,
        caller: RecordCaller::Record,
        table,
        temporary: false,
        run_trigger: true,
        statement: true,
    }
}

/// The written shape of a no-match bulk write corpus PR 1403 measured (Y19PermNoMatchShapes.al:15-26 "LT NM Target", one
/// key; Y19PermNoMatchTriggers.al:19-34 "LT NT Target", a second key on the Code[10] field "Parent Code"; both an Integer
/// primary key Id as first field), one or two rows, none matching (Seed, Y19PermNoMatchShapes.al:302-330), and one filter:
/// - `SetRange` of the primary key to an Integer (ops 1, 13, 22: `SetRange(Id, 100)`), or of a Code[10] or Code[20] field
///   outside the primary key to a Code value (the other ops; blank in Y19PermNoMatchTriggers.al op 61, Code[20] op 81);
/// - `DeleteAll()` (ops 6 and 14), or `ModifyAll` of a field outside the primary key: an Integer field to an Integer
///   (ops 1, 2, 13, 22: `ModifyAll(Value, 99)`), or a Code[10] or Code[20] field to '' or 'Y' (ops 3, 4, 11, 12, 21, 81);
///   with RunTrigger true as third argument (op 5) or no third argument.
fn probed_no_match_write(method: RecordOp, args: &[Value], t: &Target) -> bool {
    let Some((field, value)) = &t.filter else {
        return false;
    };
    let int = |p: usize| t.types.get(p) == Some(&RtType::Integer);
    let code = |p: usize| matches!(t.types.get(p), Some(RtType::Code(Some(10 | 20))));
    let outside_key = |p: usize| !t.key().contains(&p);
    let table_ok = t.key() == [0]
        && int(0)
        && match t.keys.as_slice() {
            [_] => true,
            [_, k2] => matches!(k2.as_slice(), [p] if t.types.get(*p) == Some(&RtType::Code(Some(10)))),
            _ => false,
        };
    let filter_ok = (t.key() == [*field] && matches!(value, Value::Integer(_)))
        || (outside_key(*field) && code(*field) && matches!(value, Value::Code(_)));
    let written_ok = match (method, args) {
        (RecordOp::DeleteAll, []) => true,
        (RecordOp::ModifyAll, [Value::Integer(p), v] | [Value::Integer(p), v, Value::Boolean(true)]) => {
            let p = *p as usize;
            outside_key(p)
                && ((int(p) && matches!(v, Value::Integer(_)))
                    || (code(p)
                        && matches!(v, Value::Text(s) | Value::Code(s) if s.as_str().is_empty() || s.as_str() == "Y")))
        }
        _ => false,
    };
    table_ok && filter_ok && written_ok && matches!(t.rows, 1 | 2)
}
