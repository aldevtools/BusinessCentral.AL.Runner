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
    types: Vec<RtType>,
}

impl Session {
    /// A RecordRef's record method of AL code runs `record_op` next: the gate checks it as a RecordRef's call.
    pub(crate) fn recordref_permission_call(&mut self) {
        if self.permission_checks() {
            self.permission_from_al = true;
            self.permission_from_recordref = true;
        }
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
                types: t.fields.iter().map(|f| f.ty.clone()).collect(),
            }
        };
        // Facts both shapes share: a Record variable (not a RecordRef), code whose object grants nothing on the table, a
        // table that runs no trigger (Y19PermObjects.al:9-27 "LT Y19 Row"; the reporter's "Repro Other").
        let table = rec.borrow().table as usize;
        if caller != RecordCaller::Record || indirect || !self.table_runs_no_trigger(table) {
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

    /// al-runner issue 8068 (the reporter's repro on BC 28.5, assumption `bulk-write-no-match-in-delete-trigger`): in the
    /// OnDelete trigger of table "Repro Parent", reached by `Delete(true)` from a test method, `ReproOther.SetRange("Parent
    /// Code", Code)` then `ReproOther.ModifyAll("Parent Code", '')`: a Code field outside the primary key, the filter field
    /// written blank, without RunTrigger. "Repro Other" holds one row; the sets give R on it.
    fn reported_no_match(&self, method: RecordOp, args: &[Value], access: Access, by: usize, t: &Target) -> bool {
        let Some((field, value)) = &t.filter else {
            return false;
        };
        let is_code = |p: usize| matches!(t.types.get(p), Some(RtType::Code(_)));
        method == RecordOp::ModifyAll
            && matches!(args, [Value::Integer(p), Value::Text(v) | Value::Code(v)] if *p as usize == *field && v.as_str().is_empty())
            && t.rows == 1
            && !t.key.contains(field)
            && is_code(*field)
            && matches!(value, Value::Code(_))
            && self.levels_are(access, &[[1, 0, 0, 0]])
            && self.delete_trigger_of(by)
            && self.only_test_codeunit_frames(by, 1)
    }

    /// Whether the session's levels Read, Insert, Modify and Delete on the accessed table are one of `allowed`.
    fn levels_are(&self, access: Access, allowed: &[[u8; 4]]) -> bool {
        let Some(eff) = &self.permissions else {
            return false;
        };
        let levels = [Op::Read, Op::Insert, Op::Modify, Op::Delete].map(|op| eff.level(Access { op, ..access }));
        allowed.contains(&levels)
    }

    /// Whether the running frame is the OnDelete trigger of table object `by`.
    fn delete_trigger_of(&self, by: usize) -> bool {
        let engine = self.engine.clone();
        let o = &engine.program.objects[by];
        o.kind == al_ir::ObjKind::Table
            && self.frames.last() == Some(&by)
            && self.call_stack.last().is_some_and(|f| {
                !f.event
                    && o.routines
                        .get(f.routine)
                        .is_some_and(|r| r.is_trigger && r.field.is_none() && r.name.eq_ignore_ascii_case("OnDelete"))
            })
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
