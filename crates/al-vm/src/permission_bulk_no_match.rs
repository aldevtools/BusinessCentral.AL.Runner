//! A denied ModifyAll or DeleteAll whose filter matches no row of a table that holds rows (ADR-062 probe P2).
//!
//! BC denies it in the shape record/Y19PermWrites.al:294-307 pins, and does not deny it in the shape of al-runner issue
//! 8068 (the reporter's run on BC 28.5). Which of the differences between the two decides is not measured, so each shape is
//! an allowlist of its facts here, and every other no-match shape is refused.

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

/// What a denied no-match bulk write does.
pub(crate) enum NoMatch {
    /// BC's denial (the pinned shape).
    Deny,
    /// No error: the write runs over no row (the reporter's shape, flagged).
    Allow,
    /// Not measured.
    Refuse,
}

/// The rows of the target table and the facts of the record that both shapes compare.
struct Target {
    rows: usize,
    /// The one filter: (field position, value), a SetRange to one value in filter group 0.
    filter: Option<(usize, Value)>,
    /// The primary key's field positions.
    key: Vec<usize>,
    /// How many keys the table has.
    keys: usize,
    types: Vec<RtType>,
    table: u32,
    /// The record reads in primary key order, ascending, and has read no row.
    fresh_order: bool,
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

    /// The verdict for a denied `method` (ModifyAll or DeleteAll) on `rec` whose filters match no row while the table holds
    /// `rows` rows. `by` is the object whose code makes the access; `indirect` whether its Permissions property grants it.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn bulk_no_match_verdict(
        &mut self,
        method: RecordOp,
        rec: &crate::records::Rec,
        args: &[Value],
        access: Access,
        indirect: bool,
        by: usize,
        caller: RecordCaller,
        rows: usize,
    ) -> Result<NoMatch, Signal> {
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
                key: t.keys[0].clone(),
                keys: t.keys.len(),
                types: t.fields.iter().map(|f| f.ty.clone()).collect(),
                table: r.table,
                fresh_order: r.key == 0
                    && r.adhoc.is_none()
                    && r.ascending
                    && r.field_descending == 0
                    && r.position.is_none(),
            }
        };
        // Facts both shapes share: a Record variable (not a RecordRef), code whose object grants nothing on the table, a
        // table that runs no trigger (Y19PermObjects.al:9-27 "LT Y19 Row"; the reporter's "Repro Other").
        let table = rec.borrow().table as usize;
        let shared = caller == RecordCaller::Record && !indirect && self.table_runs_no_trigger(table);
        if !shared {
            return Ok(NoMatch::Refuse);
        }
        if self.pinned_no_match(method, args, access, by, &target) {
            return Ok(NoMatch::Deny);
        }
        if self.reported_no_match(method, args, access, by, &target) {
            assume(Assumption::BulkWriteNoMatchInDeleteTrigger);
            return Ok(NoMatch::Allow);
        }
        Ok(NoMatch::Refuse)
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
                    !t.key.contains(&p) && t.types.get(p) == Some(&RtType::Integer) && matches!(v, Value::Integer(_))
                }
                _ => false,
            },
            _ => false,
        };
        written_ok
            && t.rows == 2
            && t.key.as_slice() == [*field]
            && t.types.get(*field) == Some(&RtType::Integer)
            && matches!(value, Value::Integer(_))
            && self.levels_are(access, &[[1, 0, 0, 0], [1, 1, 0, 0]])
            && self.only_test_codeunit_frames(by, 0)
    }

    /// al-runner issue 8068 (the reporter's repro on BC 28.5, assumption `bulk-write-no-match-in-delete-trigger`), its
    /// facts by equality: the running frame is the OnDelete trigger of table "Repro Parent", whose body is
    /// `ReproOther.SetRange("Parent Code", Code); ReproOther.ModifyAll("Parent Code", '');` on its one local Record (the
    /// SetRange value is the trigger record's own Code, the same field in both calls); the table "Repro Parent" runs no
    /// code but that OnDelete (no procedure, field trigger, table extension or subscriber) and its writes in the open
    /// transaction are exactly one Insert; the
    /// trigger was started by the statement `ReproParent.Delete(true)` of a Record variable of that table, not temporary,
    /// in a procedure of a test codeunit; the filter value is not blank; the filter field and the primary key field are
    /// Code[10] (the table's first field is the primary key, the second the filter field), the table has one key and one row, inserted in the open transaction (no Commit since), and nothing is
    /// locked; the sets give R on it.
    fn reported_no_match(&self, method: RecordOp, args: &[Value], access: Access, by: usize, t: &Target) -> bool {
        let Some((field, value)) = &t.filter else {
            return false;
        };
        let code10 = |p: usize| t.types.get(p) == Some(&RtType::Code(Some(10)));
        let started_by = RecordCall {
            method: RecordOp::Delete,
            caller: RecordCaller::Record,
            table: by,
            temporary: false,
            run_trigger: true,
            statement: true,
        };
        method == RecordOp::ModifyAll
            && self
                .engine
                .target
                .as_ref()
                .is_some_and(|v| al_version::bulk_write_no_match_in_delete_trigger_reported(v.bc.major))
            && matches!(args, [Value::Integer(p), Value::Text(v) | Value::Code(v)] if *p as usize == *field && v.as_str().is_empty())
            && matches!(value, Value::Code(v) if !v.as_str().is_empty())
            && t.rows == 1
            && t.keys == 1
            && t.key.as_slice() == [0]
            && *field == 1
            && code10(0)
            && code10(1)
            && t.fresh_order
            && self.levels_are(access, &[[1, 0, 0, 0]])
            && self.record_calls.last() == Some(&started_by)
            && self
                .delete_trigger_of(by)
                .is_some_and(|routine| self.reported_trigger_body(by, routine))
            && self.only_test_codeunit_frames(by, 1)
            && self.table_code_is(by, &["ondelete"])
            && self.rows_inserted_in_open_transaction(by as u32, 1)
            && self.rows_inserted_in_open_transaction(t.table, t.rows)
            && self.nothing_locked()
    }

    /// Whether routine `routine` of object `by` declares one local, a Record, and its body is a SetRange and then a
    /// ModifyAll of two arguments on that local, both statements, the same field in both and the SetRange value the trigger
    /// record's own field 1 (Code, the primary key; the reporter's OnDelete, al-runner issue 8068).
    fn reported_trigger_body(&self, by: usize, routine: usize) -> bool {
        use al_builtins::builtin_id::ids;
        use al_ir::{Arg, Const, Expr, Place, Stmt};
        let engine = self.engine.clone();
        let Some(r) = engine.program.objects[by].routines.get(routine) else {
            return false;
        };
        // The call statement `<local>.<id>(a, b)` as its two arguments.
        fn on_local_fn(s: &Stmt, id: al_builtins::builtin_id::BuiltinId) -> Option<(&Expr, &Expr)> {
            match s {
                Stmt::Expr(Expr::Builtin {
                    id: i,
                    receiver: Some(recv),
                    args,
                    ret_used: false,
                }) if *i == id
                    && matches!(
                        &**recv,
                        Arg::Ref(Place::Local(0)) | Arg::Value(Expr::Load(Place::Local(0)))
                    ) =>
                {
                    match args.as_slice() {
                        [Arg::Value(a), Arg::Value(b)] => Some((a, b)),
                        _ => None,
                    }
                }
                _ => None,
            }
        }
        let stmts: Vec<&Stmt> = r.body.stmts().iter().filter(|s| !matches!(s, Stmt::Pos(_))).collect();
        let [a, b] = stmts.as_slice() else {
            return false;
        };
        let (Some((set_field, set_value)), Some((write_field, _))) = (
            on_local_fn(a, ids::RECORD_SETRANGE_1),
            on_local_fn(b, ids::RECORD_MODIFYALL_1),
        ) else {
            return false;
        };
        // The trigger record `Rec` is global 0 of the table (the lowering of the reporter's trigger); its field 1 is the
        // primary key Code, field position 0 (`SetRange(<field>, Code)`).
        r.params == 0
            && r.locals.len() == 1
            && matches!(r.locals[0].ty, RtType::Record(..))
            && set_field == write_field
            && matches!(set_field, Expr::Const(Const::FieldPos(_)))
            && matches!(set_value, Expr::Load(Place::Field(rec, 0)) if **rec == Place::Global(0))
    }

    /// Whether the open transaction's writes of table `table` are exactly `rows` Inserts (no Commit since they were made).
    fn rows_inserted_in_open_transaction(&self, table: u32, rows: usize) -> bool {
        let mut writes = self.tx.writes.iter().filter(|w| w.table == table);
        let all_inserts = writes
            .clone()
            .all(|w| matches!(w.kind, crate::state::TxWriteKind::Insert));
        all_inserts && writes.by_ref().count() == rows
    }

    /// Whether the open transaction holds no lock (no LockTable, no RecordRef.LockTable, no locking read).
    fn nothing_locked(&self) -> bool {
        !self.tx.lock_tx
            && !self.tx.recref_lock_tx
            && self.tx.lock_unpinned.is_none()
            && self.tx.locked_tables.is_empty()
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
