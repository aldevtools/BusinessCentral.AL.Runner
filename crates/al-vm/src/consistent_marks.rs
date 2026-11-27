//! The tables `Record.Consistent(false)` marks in the open transaction, and what ends such a transaction: a Commit, a
//! guarded `Codeunit.Run`, and the inconsistency error both raise.
//!
//! A mark belongs to the record variable that set it. `Consistent(true)` on that variable lifts it
//! (errors/CuCommitTests.al:393-401, :422-433). `Consistent(true)` on another variable of the table does not: a guarded
//! `Codeunit.Run` after it still raises the inconsistency error (errors/ConsistentGuardedRun.al:272-289).
//!
//! Beyond those pins, the shapes of corpus PR 1568 (errors/ConsistentGuardedRunEnd.al, codeunit 75788; the cells its CI
//! confirmed on BC 27.5, 28.5 and 29.0) run through the assumption `guarded-run-consistent-end`: a guarded run that fails
//! drops the marks set inside it (with no Commit since it started; inside another guarded run only from a CommitBehavior
//! Error procedure, without a record argument, and when that run then fails), a temporary record marks the database
//! table, a Commit after another variable's `Consistent(true)` raises, and the error names a table by its Caption. The
//! cells of corpus PR 1586 (errors/ConsistentGuardedRunFail.al, codeunit 88974) beyond those, and their record-argument
//! twins of corpus PR 1590 (errors/ConsistentGuardedRunRecFail.al, codeunit 88983), and the filtered record arguments of
//! corpus PR 1598 (errors/ConsistentGuardedRunRecFiltered.al, codeunit 88992), run through `guarded-run-consistent-fail`
//! (`fail_probe_end`): a mark marked again by another variable, a FieldError, a filtered argument, a Commit inside the run.
//! Every other shape is refused.
//!
//! Entry points: `Session::set_consistent`; `mark_snapshot`, `refuse_write_and_mark_at_run_start` and
//! `guarded_run_end_marks` (by `codeunit_run`); `commit_mark_error` (by `commit_builtin`); `inconsistent_error`; `refuse_inconsistent_end` (an unguarded run, a
//! TestPage operation); `inconsistent_table` (the test runner's commit at the end of a test).

use crate::Session;
use crate::records::Rec;
use crate::transactions::CommitBehavior;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::filter::FieldFilter;
use al_builtins::native::Signal;
use al_builtins::value::{RecordState, Value};
use al_ir::FieldFacts;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

mod page_args;
mod page_view;
pub(crate) use page_view::page_view_ended;

/// How the variables of a table have called `Consistent` on its mark.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum MarkState {
    /// Only the variable that set the mark called `Consistent` on it.
    Marked,
    /// Another database variable of the table called `Consistent(true)` once, the variable that set it nothing since.
    ReleasedByAnother,
    /// Another database variable called `Consistent(false)` on a database variable's mark, after the setter's mark or after a
    /// third variable's `Consistent(true)`; a later `Consistent(true)` by the setter or by the variable that marked it again
    /// leaves it so (ConsistentGuardedRunFail.al steps `fya,fyc`, `fyp,fyp`, `fxa,txb,fxc`, then `txa` or `txc`). Read only
    /// by `fail_probe_end`.
    MarkedAgainByAnother,
    /// Any other sequence of calls by another variable: not measured.
    Unmeasured,
}

/// One table `Consistent(false)` marked in the open transaction.
#[derive(Clone, Debug)]
pub(crate) struct ConsistentMark {
    /// Object index of the table.
    pub(crate) table: u32,
    /// The record variable that set the mark (gone when that variable is).
    by: Weak<RefCell<RecordState>>,
    /// The variable that set the mark is a database record (not a temporary one).
    by_database: bool,
    /// The variable whose `Consistent(true)` gave `ReleasedByAnother`.
    released_by: Weak<RefCell<RecordState>>,
    /// The variable whose `Consistent(false)` gave `MarkedAgainByAnother`.
    again_by: Weak<RefCell<RecordState>>,
    pub(crate) state: MarkState,
}

/// What the record argument of a guarded run set in the marks: nothing, the first mark (`_MarkRec_Fails`, step `frec`;
/// no other arm reads it with more than one mark), or the first of two marks whose second a database variable set.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ArgMarks {
    None,
    First,
    FirstThenDatabase,
}

/// The record a guarded `Codeunit.Run` hands to OnRun, as the end-of-run check reads it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RunArg {
    /// `Codeunit.Run(Id)` or `CodeunitVar.Run()`.
    None,
    /// A Record of the table with this object index, with its filters as the run started.
    Record {
        table: u32,
        temporary: bool,
        filter: ArgFilter,
    },
    /// Any other value the caller passes (for example a Variant that holds a RecordRef).
    Other,
}

impl RunArg {
    /// The argument of a run whose record argument is `v`; `field_facts` are the facts of each table's fields, `tables` the
    /// schema of each table.
    pub(crate) fn of(
        v: Option<&Value>,
        field_facts: &[Vec<FieldFacts>],
        tables: &[Option<std::sync::Arc<al_data::TableDef>>],
    ) -> RunArg {
        match v {
            None => RunArg::None,
            Some(Value::Record(r)) => {
                let r = r.borrow();
                let fields = page_args::ArgFields {
                    facts: field_facts.get(r.table as usize).map_or(&[][..], Vec::as_slice),
                    table: tables.get(r.table as usize).and_then(Option::as_deref),
                };
                RunArg::Record {
                    table: r.table,
                    temporary: r.temporary,
                    filter: ArgFilter::of(&r, &fields),
                }
            }
            Some(_) => RunArg::Other,
        }
    }
}

/// The filters of a record argument, by the shapes of corpus PR 1598 (errors/ConsistentGuardedRunRecFiltered.al, codeunit
/// 88992 "GRQ Tests"; the argument is prepared by `PrepareX`/`PrepareY`/`PrepareZ`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ArgFilter {
    /// No filter, no marked row and all rows shown (the `_NoFilter_` cells).
    None,
    /// One of the filters the probe set, each without a marked row: `SetRange` to one value on one field of FieldClass
    /// Normal, in filter group 0 (the `_SetRangeKey_` cells on the primary key, the `_SetRangeName_` cells on a field outside
    /// it); `SetFilter(Name, 'call*|zzz')` on a field of FieldClass Normal, in filter group 0 (the `_SetFilter_` cells); no
    /// filter, one row marked and `MarkedOnly(true)` (the `_MarkedOnly_` cells). `key`: the filter is the `SetRange` to one
    /// value on the one field of a one-field primary key (also the Key shape of corpus PR 1600, read by `page_probe_end`).
    Measured { key: bool },
    /// One of the other page-style filter shapes of corpus PR 1600 (`page_args`).
    Page(page_args::PageArgShape),
    /// Any other filters.
    Other,
}

impl ArgFilter {
    /// The filters of `r`, whose table has the field facts `fields`.
    fn of(r: &RecordState, fields: &page_args::ArgFields) -> ArgFilter {
        use al_builtins::filter::FilterExpr::{Eq, Like, Or};
        let one_filter = match r.filters.as_slice() {
            [f] if f.group == 0 && !r.marked_only && r.marks.is_empty() && fields.normal(f) => Some(f),
            _ => None,
        };
        let one_value = one_filter.filter(|f| fields.one_value(f));
        let measured = match one_filter.map(|f| &f.expr) {
            _ if one_value.is_some() => true,
            Some(Or(alternatives)) => matches!(
                alternatives.as_slice(),
                [Like(pattern, false), Eq(Value::Text(value))] if pattern == "call*" && value.as_str() == "zzz"
            ),
            Some(_) => false,
            None => r.filters.is_empty() && r.marked_only && r.marks.len() == 1,
        };
        // The probe's arguments start in filter group 0 (the prepare codeunits never change it).
        let unmarked = !r.marked_only && r.marks.is_empty();
        match (r.filters.is_empty() && unmarked, measured) {
            _ if r.filter_group != 0 => ArgFilter::Other,
            (true, _) => ArgFilter::None,
            (false, true) => ArgFilter::Measured {
                key: one_value.is_some_and(|f| fields.key_field(f)),
            },
            (false, false) if unmarked => page_args::page_shape(r, fields).map_or(ArgFilter::Other, ArgFilter::Page),
            (false, false) => ArgFilter::Other,
        }
    }
}

/// The record argument of a guarded run, kept to compare as the run ends: corpus PR 1598 measured only an inner codeunit
/// that leaves the argument's filters, current filter group, marked rows and MarkedOnly as the caller set them, and a mark
/// the argument itself sets
/// (`_MarkRec_Fails`, step `frec`).
pub(crate) struct ArgRecord {
    rec: Rc<RefCell<RecordState>>,
    filters: Vec<FieldFilter>,
    marks: Vec<Vec<Value>>,
    marked_only: bool,
    filter_group: i32,
}

impl ArgRecord {
    /// The record argument `v` as the run starts; `None` without a Record.
    pub(crate) fn of(v: Option<&Value>) -> Option<ArgRecord> {
        let Some(Value::Record(rec)) = v else { return None };
        let r = rec.borrow();
        let (filters, marks, marked_only, filter_group) =
            (r.filters.clone(), r.marks.clone(), r.marked_only, r.filter_group);
        drop(r);
        Some(ArgRecord {
            rec: rec.clone(),
            filters,
            marks,
            marked_only,
            filter_group,
        })
    }

    /// Whether the argument has the filters, current filter group, marked rows and MarkedOnly it started with.
    fn filters_kept(&self) -> bool {
        let r = self.rec.borrow();
        let same_key =
            |a: &Vec<Value>, b: &Vec<Value>| a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.identical(y));
        r.marked_only == self.marked_only
            && r.filter_group == self.filter_group
            && r.marks.len() == self.marks.len()
            && r.marks.iter().zip(&self.marks).all(|(a, b)| same_key(a, b))
            && r.filters.len() == self.filters.len()
            && r.filters.iter().zip(&self.filters).all(|(a, b)| a.identical(b))
    }
}

/// What the end-of-run check reads of a guarded `Codeunit.Run` as it started, and of the runs nested in it.
pub(crate) struct RunStart<'a> {
    /// The marks at the start.
    pub(crate) marks: &'a [(u32, MarkState)],
    /// The session's commit count at the start (`Session::commits`).
    pub(crate) commits: u64,
    /// The record argument.
    pub(crate) arg: RunArg,
    /// The record argument itself, when it is a Record.
    pub(crate) arg_record: Option<&'a ArgRecord>,
    /// A guarded run nested in this one failed and dropped its marks (`GuardedBody::marks_dropped_inside`).
    pub(crate) marks_dropped_inside: bool,
    /// A guarded run started inside this one (`GuardedBody::runs_inside`).
    pub(crate) runs_inside: bool,
    /// The frames below the run: the caller of `Codeunit.Run` is at index `caller_frame - 1` of `Session::frames`.
    pub(crate) caller_frame: usize,
}

/// How a guarded `Codeunit.Run` ended.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RunEnd<'a> {
    Succeeded,
    /// OnRun raised an error with this code, which the guard traps.
    Failed(&'a str),
}

impl RunEnd<'_> {
    /// How a run that gave `result` ended; `None` for a refusal or another signal, which keeps its own reason.
    pub(crate) fn of<T>(result: &Result<T, Signal>) -> Option<RunEnd<'_>> {
        match result {
            Ok(_) => Some(RunEnd::Succeeded),
            Err(Signal::Error(e)) => Some(RunEnd::Failed(&e.code)),
            Err(_) => None,
        }
    }
}

/// The codes of the inner errors after which a guarded run that started with no mark returns false and the marks set
/// inside it are gone (ConsistentGuardedRunEnd.al, GuardedRunId_InnerStartsStateAndFails `Error('inner failed')`,
/// GuardedRunId_InnerStartsStateAndErrorsEmpty `Error('')`, GuardedRunId_InnerStartsStateAndGetFails,
/// GuardedRunId_InnerStartsStateAndTestFieldFails).
const DROPPED_BY: [&str; 3] = ["Dialog", "DB:RecordNotFound", "TestField"];

/// The run ends of corpus PR 1586 (ConsistentGuardedRunFail.al, codeunit 88974 "GRF Tests", section (2)) that the
/// shapes of `guarded-run-consistent-end` do not take: `Some(true)` the run raises the inconsistency error naming the
/// table marked first, `Some(false)` it returns false with the inner error and the marks set inside it are gone, `None`
/// not measured. Every cell is a run not nested in another guarded run, with no Commit since it started
/// and no mark at its start (`top_level_no_commit`; a Commit under CommitBehavior::Ignore commits nothing). The
/// argument is one of [`FailProbeArg`]: none (ConsistentGuardedRunFail.al), or a record of corpus PR 1590, which gives
/// every cell of a shape below the same result as without it, except the FieldError, which it does not measure. The marks it ends with, in the order they were set, equal a cell's (M held by
/// the setter, R released by another variable, A marked again by another):
/// - fails with code Dialog (`Error('inner failed')`) with M A (Inner_StartX_MarkYByA_MarkYByC_Fails,
///   Inner_StartX_InsertY_MarkYByA_MarkYByC_Fails, Inner_StartX_MarkYInProcTwice_Fails), R M
///   (Inner_StartX_TrueXByB_MarkYByA_Fails, Inner_MarkXByA_TrueXByB_MarkYByA_Fails) or A M
///   (Inner_MarkXByA_TrueXByB_MarkXByC_MarkYByA_Fails);
/// - fails with code NCLCSRTS:TableErrorStr (a FieldError) with M (Inner_MarkGlobal_FieldError,
///   Inner_MarkGlobal_InsertX_FieldError);
/// - succeeds with M A (Inner_StartX_MarkYByA_MarkYByC_Ends), A M (Inner_MarkXByA_TrueXByB_MarkXByC_MarkYByA_Ends) or
///   A (Inner_MarkXByA_TrueXByB_MarkXByC_TrueXByA_TrueYByA_Ends, _TrueXByC_TrueYByA_Ends).
///
/// A record argument of corpus PR 1598 ([`filtered_probe_arg`]) whose filters the run kept (`arg_kept`) also takes the
/// cells of ConsistentGuardedRunRecFiltered.al, each the same for every filter shape of [`ArgFilter`] and every table
/// kind there: fails with code Dialog with M M (`_StartX_MarkYByA_Fails`), M A (`_StartX_MarkYByA_MarkYByC_Fails`, and the
/// `Plain_` cells) or M set by the argument itself (`_MarkRec_Fails`, `marked_by_arg`); fails with code
/// DB:TableInConsistent, a Commit inside it raising, with M M (`_MarkXByA_MarkYByA_PlainCommit`; inside a run only a Commit
/// raises that code: a nested guarded run never raises it, it refuses): the run returns false;
/// succeeds with M M (`_StartX_MarkYByA_Ends`): the error.
///
/// Every mark comes from a database variable and none from the argument (`database_set`), except the one the argument itself
/// sets: a temporary argument of the table marked first sets it in `RecTempX_*_MarkRec_Fails`.
fn fail_probe_end(
    top_level_no_commit: bool,
    arg: RunArg,
    arg_kept: bool,
    arg_marks: ArgMarks,
    database_set: bool,
    after: &[(u32, MarkState)],
    end: RunEnd,
) -> Option<bool> {
    if !top_level_no_commit || after.is_empty() {
        return None;
    }
    use FailProbeArg::{Database, NoArg, Temporary};
    use MarkState::{Marked as M, MarkedAgainByAnother as A, ReleasedByAnother as R};
    let states: Vec<MarkState> = after.iter().map(|(_, s)| *s).collect();
    let filtered = arg_kept && filtered_probe_arg(arg, after);
    if filtered && end == RunEnd::Failed("Dialog") {
        match (arg_marks, states.as_slice()) {
            (ArgMarks::First, [M]) | (ArgMarks::FirstThenDatabase, [M, M]) => return Some(false),
            _ => {}
        }
    }
    if !database_set {
        return None;
    }
    if filtered {
        match (end, states.as_slice()) {
            (RunEnd::Failed("Dialog"), [M, M] | [M, A]) => return Some(false),
            (RunEnd::Failed("DB:TableInConsistent"), [M, M]) => return Some(false),
            (RunEnd::Succeeded, [M, M]) => return Some(true),
            _ => {}
        }
    }
    let arg = FailProbeArg::of(arg, arg_kept, after)?;
    match (arg, end, states.as_slice()) {
        (NoArg | Database, RunEnd::Failed("Dialog"), [M, A] | [R, M] | [A, M]) => Some(false),
        (Temporary, RunEnd::Failed("Dialog"), [M, A]) => Some(false),
        (NoArg, RunEnd::Failed("NCLCSRTS:TableErrorStr"), [M]) => Some(false),
        (NoArg | Database, RunEnd::Succeeded, [M, A] | [A, M] | [A]) => Some(true),
        (Temporary, RunEnd::Succeeded, [M, A]) => Some(true),
        _ => None,
    }
}

/// Whether `arg` is a record argument of corpus PR 1598 (ConsistentGuardedRunRecFiltered.al, `RunRec`) when the run ends
/// with the marks `after`: a database record of a table no mark names (the `RecZ_` cells), of the table marked first
/// (`RecX_`) or of the table marked second (`RecY_`), or a temporary record of the table marked first (`RecTempX_`), each
/// without a filter or with one of [`ArgFilter::Measured`].
fn filtered_probe_arg(arg: RunArg, after: &[(u32, MarkState)]) -> bool {
    let RunArg::Record {
        table,
        temporary,
        filter: ArgFilter::None | ArgFilter::Measured { .. },
    } = arg
    else {
        return false;
    };
    let marked = |i: usize| after.get(i).map(|(t, _)| *t) == Some(table);
    let unmarked = after.iter().all(|(t, _)| *t != table);
    match temporary {
        false => unmarked || marked(0) || marked(1),
        true => marked(0),
    }
}

/// The arguments of the guarded runs `fail_probe_end` takes. Each record argument of corpus PR 1590
/// (ConsistentGuardedRunRecFail.al, codeunit 88983 "GRR Tests") is a record without a filter that the caller read or
/// inserted, and the run leaves without one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum FailProbeArg {
    /// `Codeunit.Run(Id)` (ConsistentGuardedRunFail.al).
    NoArg,
    /// A database record of a table no mark names (RecZ_StartX_MarkYByA_MarkYByC_Fails :410,
    /// RecZ_StartX_InsertY_MarkYByA_MarkYByC_Fails :425, RecZ_StartX_MarkYInProcTwice_Fails :440,
    /// RecZ_StartX_MarkYByA_MarkYByC_Ends :455, RecZ_StartX_TrueXByB_MarkYByA_Fails :470,
    /// RecZ_MarkXByA_TrueXByB_MarkYByA_Fails :515, RecZ_MarkXByA_TrueXByB_MarkXByC_MarkYByA_Fails :530, _Ends :545,
    /// RecZ_MarkXByA_TrueXByB_MarkXByC_TrueXByA_TrueYByA_Ends :560, _TrueXByC_TrueYByA_Ends :575), or of the table
    /// marked first (the RecX_ twins, :697-:862).
    Database,
    /// A temporary record of the table marked first (RecTemp_StartX_MarkYByA_MarkYByC_Fails :1168, _Ends :1183).
    Temporary,
}

/// Whether no mark of `after` names `table`, or `table` is the one marked first.
fn unmarked_or_first(table: u32, after: &[(u32, MarkState)]) -> bool {
    after.iter().all(|(t, _)| *t != table) || after.first().map(|(t, _)| *t) == Some(table)
}

impl FailProbeArg {
    /// The kind of `arg` when the run ends with the marks `after` (`arg_kept`: the run left the argument's filters as they
    /// were); `None` for any other argument.
    fn of(arg: RunArg, arg_kept: bool, after: &[(u32, MarkState)]) -> Option<FailProbeArg> {
        let first_marked = |table: u32| after.first().map(|(t, _)| *t) == Some(table);
        match arg {
            RunArg::None => Some(FailProbeArg::NoArg),
            RunArg::Record {
                table,
                temporary: false,
                filter: ArgFilter::None,
            } if arg_kept && unmarked_or_first(table, after) => Some(FailProbeArg::Database),
            RunArg::Record {
                table,
                temporary: true,
                filter: ArgFilter::None,
            } if arg_kept && first_marked(table) => Some(FailProbeArg::Temporary),
            _ => None,
        }
    }
}

impl Session {
    /// `Record.Consistent(Value)` on `rec`: marks, lifts or changes the mark of its table for the open transaction
    /// (errors/CuCommitTests.al:382-433). A temporary record marks the database table, and only `Consistent(true)` on
    /// the same temporary variable lifts it (ConsistentGuardedRunEnd.al Commit_CallerMarksTemporary,
    /// Commit_TemporaryMarkedThenUnmarked, Commit_TemporaryMarked_DatabaseVariableUnmarks).
    pub(crate) fn set_consistent(&mut self, rec: &Rec, consistent: bool) -> Result<(), Signal> {
        let (table, temporary) = {
            let r = rec.borrow();
            (r.table, r.temporary)
        };
        if temporary {
            assume(Assumption::GuardedRunConsistentEnd);
        }
        let Some(i) = self.tx.open.inconsistent.iter().position(|m| m.table == table) else {
            if !consistent {
                self.tx.open.inconsistent.push(ConsistentMark {
                    table,
                    by: Rc::downgrade(rec),
                    by_database: !temporary,
                    released_by: Weak::new(),
                    again_by: Weak::new(),
                    state: MarkState::Marked,
                });
            }
            return Ok(());
        };
        let mark = &mut self.tx.open.inconsistent[i];
        let is = |w: &Weak<RefCell<RecordState>>| w.upgrade().is_some_and(|b| Rc::ptr_eq(&b, rec));
        let (same, releaser, again) = (is(&mark.by), is(&mark.released_by), is(&mark.again_by));
        match (mark.state, same, consistent) {
            (MarkState::Marked, true, true) => {
                self.tx.open.inconsistent.remove(i);
            }
            (MarkState::Marked, true, false) => {}
            (MarkState::Marked, false, true) if !temporary => {
                mark.state = MarkState::ReleasedByAnother;
                mark.released_by = Rc::downgrade(rec);
            }
            // A database setter's mark marked again by another database variable: one that did not release it (steps
            // `fya,fyc`, `fyp,fyp`, and `fxa,txb,fxc` with a third variable).
            (MarkState::Marked | MarkState::ReleasedByAnother, false, false)
                if !temporary && mark.by_database && !releaser =>
            {
                mark.state = MarkState::MarkedAgainByAnother;
                mark.again_by = Rc::downgrade(rec);
            }
            // Consistent(true) by the setter or by the variable that marked it again (steps `txa`, `txc`).
            (MarkState::MarkedAgainByAnother, _, true) if !temporary && (same || again) => {}
            // Consistent(true) on the database variable that set the mark, after another one's: lifted
            // (ConsistentGuardedRunEnd.al Commit_FalseOnOneVariable_TrueOnAnother_ThenTrueOnFirst).
            (MarkState::ReleasedByAnother, true, true) if !temporary => {
                assume(Assumption::GuardedRunConsistentEnd);
                self.tx.open.inconsistent.remove(i);
            }
            _ => mark.state = MarkState::Unmeasured,
        }
        Ok(())
    }

    /// The marks of the open transaction (table, state), which a guarded `Codeunit.Run` takes as it starts and as it ends.
    pub(crate) fn mark_snapshot(&self) -> Vec<(u32, MarkState)> {
        self.tx.open.inconsistent.iter().map(|m| (m.table, m.state)).collect()
    }

    /// What a guarded `Codeunit.Run` does with the marks as it ends: `Some` the inconsistency error, which the guard
    /// does not trap; `None` nothing (no mark, or the marks set inside a failed run, which its rollback drops). `before`
    /// are the marks at the start, `arg` the record argument.
    ///
    /// Pinned (errors/ConsistentGuardedRun.al), without a record argument: one table marked before the run by the caller
    /// (and released or not by another variable before it), unchanged by the run, which succeeds or fails with an `Error`
    /// (:170-236, :272-289); or no mark before the run and one the inner codeunit set and kept, when the run succeeds
    /// (:238-252). The error is the one of a Commit: the pin reads its text (:181).
    ///
    /// Assumed (`guarded-run-consistent-end`, ConsistentGuardedRunEnd.al), outside another guarded run and with no Commit
    /// since the run started (every cell; the cells with an inner Commit commit under the mark and are refused), unless noted:
    /// - no mark before, a failed run with a code of [`DROPPED_BY`], every mark left held by the variable that set it:
    ///   nothing (GuardedRunId_InnerMarks*AndFails, GuardedRunInstance*_*AndFails). Inside another guarded run too, but
    ///   only without a record argument, called in a [CommitBehavior(CommitBehavior::Error)] procedure, and when the outer
    ///   run then fails with an `Error` (Preview_OuterGuardedRun_InnerStartsStateAndFails): the drop marks the outer run,
    ///   whose end checks that shape (`outer_end_after_nested_drop`);
    /// - no mark before, a successful run that leaves marks held or released by another variable: the error naming the
    ///   table marked first (GuardedRunId_InnerMarksRowThenOther_Succeeds, _InnerMarksOtherThenRow_Succeeds,
    ///   _InnerStartsAndStopsState_Succeeds, _InnerMarksTemporary_Succeeds, GuardedRunIdRec_InnerStartsState_Succeeds);
    /// - the pinned caller shape with a record argument (GuardedRunIdRec_CallerMarks_InnerFails, _InnerSucceeds,
    ///   GuardedRunInstanceRec_CallerMarks_InnerFails).
    /// - one mark the caller holds, no record argument, and a run that fails with DB:RecordNotFound and leaves the marks
    ///   unchanged, or with an `Error` after the inner codeunit marked one other table: the error naming the caller's table
    ///   (GuardedRunId_CallerMarks_InnerGetFails, GuardedRunId_CallerMarksOther_InnerStartsStateAndFails).
    ///
    /// A record argument is measured only as a database record of a table no mark names. Any other shape is refused.
    pub(crate) fn guarded_run_end_marks(&mut self, start: RunStart, end: RunEnd) -> Result<Option<Signal>, Signal> {
        let (before, arg) = (start.marks, start.arg);
        let after = self.mark_snapshot();
        // Whether no other guarded run is still running, and whether nothing committed since this run started.
        let top_level = self.tx.guarded_runs.is_empty();
        let no_commit = self.commits == start.commits;
        if start.marks_dropped_inside {
            self.outer_end_after_nested_drop(top_level && no_commit, before, &after, end)?;
        }
        if before.is_empty() && after.is_empty() {
            return Ok(None);
        }
        if let Some(r) = self.page_view_run_end(&start, end, &after, top_level, no_commit) {
            return Ok(r);
        }
        let caller_unchanged =
            before == after.as_slice() && matches!(before, [(_, MarkState::Marked | MarkState::ReleasedByAnother)]);
        let caller_shape = caller_unchanged && matches!(end, RunEnd::Succeeded | RunEnd::Failed("Dialog"));
        let inner_kept = before.is_empty() && end == RunEnd::Succeeded;
        if arg == RunArg::None && (caller_shape || (inner_kept && matches!(after.as_slice(), [(_, MarkState::Marked)])))
        {
            return Ok(Some(self.inconsistent_error(after[0].0 as usize)));
        }
        let arg_measured = match arg {
            RunArg::None => true,
            RunArg::Record { table, temporary, .. } => !temporary && !after.iter().any(|(t, _)| *t == table),
            RunArg::Other => false,
        };
        let held = |states: &[MarkState]| after.iter().all(|(_, s)| states.contains(s));
        let dropped = before.is_empty()
            && matches!(end, RunEnd::Failed(code) if DROPPED_BY.contains(&code))
            && held(&[MarkState::Marked]);
        // One mark the caller set and still holds, and a run without a record argument that fails: an inner Get that fails
        // (GuardedRunId_CallerMarks_InnerGetFails), or an inner codeunit that marks one other table and fails with an `Error`
        // (GuardedRunId_CallerMarksOther_InnerStartsStateAndFails).
        let caller_held = arg == RunArg::None && matches!(before, [(_, MarkState::Marked)]);
        let caller_get_failed = caller_held && before == after.as_slice() && end == RunEnd::Failed("DB:RecordNotFound");
        let caller_inner_marked = caller_held
            && end == RunEnd::Failed("Dialog")
            && after.len() == 2
            && after[0] == before[0]
            && after[1..].iter().all(|(_, s)| *s == MarkState::Marked);
        let raised = top_level
            && (caller_shape
                || caller_get_failed
                || caller_inner_marked
                || (inner_kept && held(&[MarkState::Marked, MarkState::ReleasedByAnother])));
        // A drop inside another guarded run marks that run, once (one cell has one nested drop). That cell's nested run has
        // no record argument and is called in a [CommitBehavior(CommitBehavior::Error)] procedure that the outer run's OnRun
        // calls (GRC Outer.PreviewStart): the scope is Error at the drop and was not Error when the outer run started.
        let outer = self.tx.guarded_bodies.last_mut().filter(|_| dropped && !top_level);
        let drop_scope_measured = outer.as_ref().map_or(top_level, |o| {
            arg == RunArg::None
                && self.tx.commit_behavior == CommitBehavior::Error
                && o.commit_behavior_at_start != CommitBehavior::Error
        });
        let first_drop = outer.as_ref().is_none_or(|o| !o.marks_dropped_inside);
        if arg_measured && no_commit && first_drop && drop_scope_measured && (dropped || raised) {
            assume(Assumption::GuardedRunConsistentEnd);
            if let Some(o) = outer {
                o.marks_dropped_inside = true;
            }
            return Ok(raised.then(|| self.inconsistent_error(after[0].0 as usize)));
        }
        let arg_kept = start.arg_record.is_none_or(ArgRecord::filters_kept);
        let is_arg = |w: &Weak<RefCell<RecordState>>| {
            start
                .arg_record
                .is_some_and(|a| w.upgrade().is_some_and(|b| Rc::ptr_eq(&b, &a.rec)))
        };
        let marks = &self.tx.open.inconsistent;
        let arg_marks = match marks.as_slice() {
            // Corpus errors/ConsistentGuardedRunRecFail.al (corpus 9f7e6bb4) RecZ_ModifyRec_MarkRec_MarkYByA_Fails:1029 and
            // RecX_ModifyRec_MarkRec_MarkYByA_Fails:1121 (steps `modrec,frec,fya,err`): the argument sets the first mark, which
            // nobody marks again or releases, and one other database variable (not the argument) sets one more mark. The
            // session does not track the Modify of the argument before the run: `frec,fya,err` without it runs as the measured
            // `modrec,frec,fya,err` shape (follow-up).
            [a, b]
                if is_arg(&a.by)
                    && a.state == MarkState::Marked
                    && b.by_database
                    && !is_arg(&b.by)
                    && b.state == MarkState::Marked =>
            {
                ArgMarks::FirstThenDatabase
            }
            [a, ..] if is_arg(&a.by) => ArgMarks::First,
            _ => ArgMarks::None,
        };
        let top_level_no_commit = top_level && no_commit && before.is_empty();
        // Corpus PR 1598 measures the argument as a setter only alone (`_MarkRec_Fails`, step `frec`): no mark of another
        // shape is set or marked again by it.
        let database_set = self
            .tx
            .open
            .inconsistent
            .iter()
            .all(|m| m.by_database && !is_arg(&m.by) && !is_arg(&m.again_by));
        let probe_end = fail_probe_end(top_level_no_commit, arg, arg_kept, arg_marks, database_set, &after, end);
        if let Some(raises) = probe_end {
            assume(Assumption::GuardedRunConsistentFail);
            return Ok(raises.then(|| self.inconsistent_error(after[0].0 as usize)));
        }
        // Every cell of corpus PR 1600 releases through a variable other than the argument (step `txb`, variable XB).
        let released_by_other = self.tx.open.inconsistent.iter().all(|m| !is_arg(&m.released_by));
        let page_end =
            (top_level_no_commit && arg_kept && database_set && released_by_other && arg_marks == ArgMarks::None)
                .then(|| page_args::page_probe_end(arg, || self.work_date(), &after, end))
                .flatten();
        if let Some(entry) = page_end {
            assume(entry);
            return Ok(None);
        }
        Err(Signal::unsupported(format!(
            "A guarded Codeunit.Run ends while Consistent(false) marks table {} in a shape no probe measured (marks, outcome, \
             record argument, Commit or nesting)",
            self.table_name(after.first().or(before.first()).expect("a mark").0)
        )))
    }

    /// The end of a guarded run inside which a nested guarded run failed and dropped its marks: measured only as the outer
    /// run of Preview_OuterGuardedRun_InnerStartsStateAndFails, which is not nested itself, has had no Commit (and no
    /// successful run, which commits) since it started, keeps no mark before or after, and fails with code Dialog (the cell
    /// raises `Error('')`; an `Error` with a text drops the same, GuardedRunId_InnerStartsStateAndFails). `top_level_no_commit`
    /// is whether no other guarded run is running and nothing committed since this run started.
    fn outer_end_after_nested_drop(
        &self,
        top_level_no_commit: bool,
        before: &[(u32, MarkState)],
        after: &[(u32, MarkState)],
        end: RunEnd,
    ) -> Result<(), Signal> {
        if top_level_no_commit && before.is_empty() && after.is_empty() && end == RunEnd::Failed("Dialog") {
            assume(Assumption::GuardedRunConsistentEnd);
            return Ok(());
        }
        Err(Signal::unsupported(
            "A guarded Codeunit.Run ends after a guarded run inside it dropped its Consistent(false) marks, in a shape no \
             probe measured (outcome, Commit, marks or nesting)",
        ))
    }

    /// A guarded `Codeunit.Run` that starts while the caller holds an uncommitted write and a Consistent(false) mark:
    /// refused, because BC's answer differs between CI runs of corpus PR 1568 on the same version, so it depends on an
    /// input the probe does not measure (ConsistentGuardedRunEnd.al:965 GuardedRunId_CallerStartsState_InnerFails, :983
    /// GuardedRunId_CallerStartsAndStopsState_InnerFails: the pending-write error in CI run 45353497570, the
    /// inconsistency error in some legs of run 45358869670). Without a mark the pending-write error stays
    /// (codeunits/TestCodeunitRunWriteTransaction.al:GuardedRun_WithUncommittedWrite_IsRefused_AndCodeunitNeverRuns).
    pub(crate) fn refuse_write_and_mark_at_run_start(&self) -> Result<(), Signal> {
        self.inconsistent_table().map_or(Ok(()), |name| {
            Err(Signal::unsupported(format!(
                "A guarded Codeunit.Run starts while the caller holds an uncommitted write and Consistent(false) marks table \
                 {name}: BC's answer differs between runs (not measured)"
            )))
        })
    }

    /// The name of the marked table `table`, for a refusal text.
    fn table_name(&self, table: u32) -> &str {
        &self.engine.program.objects[table as usize].name
    }

    /// The error of a Commit while a mark is set, `None` without one (errors/CuCommitTests.al:405-419). After
    /// `Consistent(true)` on another variable the Commit raises it too (ConsistentGuardedRunEnd.al
    /// Commit_FalseOnOneVariable_TrueOnAnother, Commit_StateStartedAndStopped); any other call by another variable is
    /// refused.
    pub(crate) fn commit_mark_error(&self) -> Option<Signal> {
        let m = self.tx.open.inconsistent.first()?;
        match m.state {
            MarkState::Marked => {}
            MarkState::ReleasedByAnother => assume(Assumption::GuardedRunConsistentEnd),
            MarkState::MarkedAgainByAnother | MarkState::Unmeasured => {
                return Some(Signal::unsupported(format!(
                    "Commit after another variable of table {} called Consistent on its Consistent(false) mark (not measured)",
                    self.table_name(m.table)
                )));
            }
        }
        Some(self.inconsistent_error(m.table as usize))
    }

    /// The inconsistency error of table `table` (errors/CuCommitTests.al:405-419, code DB:TableInConsistent). It names
    /// the table by its own Caption (ConsistentGuardedRunEnd.al Commit_CallerMarksCaptioned,
    /// GuardedRunId_InnerMarksCaptioned_Succeeds); a caption a tableextension or a translation sets is refused.
    pub(crate) fn inconsistent_error(&self, table: usize) -> Signal {
        let t = &self.engine.program.objects[table];
        if self.caption_refused(table).is_some() || t.properties.iter().any(|(k, _)| k == al_ir::EXT_CAPTION_OVER_OWN) {
            return Signal::unsupported(format!(
                "the inconsistency error of table {}, whose caption a tableextension or a translation sets (not measured)",
                t.name
            ));
        }
        let caption = self.source_caption(self.engine.tables[table].as_deref().expect("table"));
        if caption != t.name {
            assume(Assumption::GuardedRunConsistentEnd);
        }
        crate::transactions::al_error_code(
            &format!(
                "The transaction cannot be completed because it will cause inconsistencies in the {caption} table. Check where and how \
                 the CONSISTENT function is used in the transaction to find the reason for the error.\\\\Contact your system \
                 administrator.\\\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This \
                 prevents data from being updated incorrectly."
            ),
            "DB:TableInConsistent",
        )
    }

    /// The name of a table `Consistent(false)` marks in the open transaction, for the test runner's commit at the end of a
    /// test (not pinned: refused there).
    pub fn inconsistent_table(&self) -> Option<String> {
        self.tx
            .open
            .inconsistent
            .first()
            .map(|m| self.table_name(m.table).to_string())
    }

    /// A transaction that ends other than by `Commit()` while `Consistent(false)` marks a table: what BC does is not
    /// pinned (only `Commit()` is, errors/CuCommitTests.al:405-419), so it is refused.
    pub(crate) fn refuse_inconsistent_end(&self, how: &str) -> Result<(), Signal> {
        self.inconsistent_table().map_or(Ok(()), |name| {
            Err(Signal::unsupported(format!(
                "{how} ends a transaction while Consistent(false) marks table {name} (only Commit() is pinned)"
            )))
        })
    }
}
