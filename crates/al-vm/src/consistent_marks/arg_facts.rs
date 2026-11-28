//! The record argument of a guarded `Codeunit.Run` as the end arms read it: one reading, taken once as the run ends
//! (`ArgFacts::of`), for every assumption of `consistent_marks.rs` (`guarded-run-consistent-end`, `-fail`, `-page`,
//! `-page-view`, `-two-key`): where its table stands among the marks, whether the run kept its filters, and what the
//! argument itself did to the marks. It also names the argument shapes those entries serve that no corpus cell measures
//! yet, which run through `guarded-run-consistent-arg` (al-runner issues 9151, 9157, 9184): `end_measured`,
//! `fail_awaits_probe` and `assume_if_lifted`.
//!
//! Entry point: `ArgFacts::of` (by `Session::guarded_run_end_marks`).

use super::page_args::PageArgShape;
use super::{ArgFilter, ArgRecord, ConsistentMark, MarkState, RunArg, RunEnd};
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::value::RecordState;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// Where the table of a record argument stands among the marks as the run ends, in the order they were set.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum ArgTable {
    /// No Record argument (none, or `RunArg::Other`).
    NoRecord,
    /// No mark names the table (the `RecZ_` cells).
    Unmarked,
    /// The table marked first (the `RecX_` and `RecTemp_` cells).
    First,
    /// The table marked second (the `RecY_` cells of corpus PR 1598).
    Second,
    /// The table of a later mark.
    Later,
}

/// What the record argument of a guarded run set in the marks: nothing, the first mark (`_MarkRec_Fails`, step `frec`;
/// no other arm reads it with more than one mark), or the first of two marks whose second a database variable set.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum ArgMarks {
    None,
    First,
    FirstThenDatabase,
}

/// The record argument of a guarded run as it ends.
pub(super) struct ArgFacts {
    pub(super) arg: RunArg,
    pub(super) table: ArgTable,
    /// The run left the argument's filters, current filter group, marked rows and MarkedOnly as they were (true without a
    /// record argument).
    pub(super) kept: bool,
    pub(super) sets: ArgMarks,
    /// Every mark was set by a database variable other than the argument, and the argument marked none again.
    pub(super) database_set: bool,
    /// The argument's `Consistent(true)` released a mark another variable set (`MarkState::ReleasedByAnother`, also when a
    /// third variable then marked it again).
    pub(super) releases: bool,
    /// The argument set, released or marked again a mark that a `Consistent(true)` then lifted during the run
    /// (`RunStart::lifted`). No corpus cell of these entries has the argument call `Consistent` on a mark that is lifted.
    pub(super) lifted_by_arg: bool,
}

impl ArgFacts {
    /// The argument `arg` (`record` its Record) of a run that ends with the marks `marks`, and during which the marks
    /// `lifted` were lifted.
    pub(super) fn of(
        arg: RunArg,
        record: Option<&ArgRecord>,
        marks: &[ConsistentMark],
        lifted: &[ConsistentMark],
    ) -> ArgFacts {
        let is_arg = |w: &Weak<RefCell<RecordState>>| {
            record.is_some_and(|a| w.upgrade().is_some_and(|b| Rc::ptr_eq(&b, &a.rec)))
        };
        let table = match arg {
            RunArg::Record { table, .. } => match marks.iter().position(|m| m.table == table) {
                None => ArgTable::Unmarked,
                Some(0) => ArgTable::First,
                Some(1) => ArgTable::Second,
                Some(_) => ArgTable::Later,
            },
            RunArg::None | RunArg::Other => ArgTable::NoRecord,
        };
        let sets = match marks {
            // Corpus errors/ConsistentGuardedRunRecFail.al (corpus 9f7e6bb4) RecZ_ModifyRec_MarkRec_MarkYByA_Fails:1029 and
            // RecX_ModifyRec_MarkRec_MarkYByA_Fails:1121 (steps `modrec,frec,fya,err`): the argument sets the first mark, which
            // nobody marks again or releases, and one other database variable (not the argument) sets one more mark.
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
        ArgFacts {
            arg,
            table,
            kept: record.is_none_or(ArgRecord::filters_kept),
            sets,
            database_set: marks
                .iter()
                .all(|m| m.by_database && !is_arg(&m.by) && !is_arg(&m.again_by)),
            releases: marks.iter().any(|m| is_arg(&m.released_by)),
            lifted_by_arg: lifted
                .iter()
                .any(|m| is_arg(&m.by) || is_arg(&m.released_by) || is_arg(&m.again_by)),
        }
    }

    /// The arguments `guarded-run-consistent-end` takes: none, or a database record of a table no mark names.
    pub(super) fn end_shape(&self) -> bool {
        match self.arg {
            RunArg::None => true,
            RunArg::Record { temporary, .. } => !temporary && self.table == ArgTable::Unmarked,
            RunArg::Other => false,
        }
    }

    /// Whether a corpus cell measures this argument in an end of `guarded-run-consistent-end` (a run that starts with the
    /// marks `before` and ends with the marks `after` as `end`; `page_view` is `Session::page_view_arg`); every other end
    /// shape it takes runs through `guarded-run-consistent-arg` too. Measured with a record argument the run leaves as it
    /// was, each mark set by a database variable other than the argument, no lifted mark the argument touched
    /// (`lifted_by_arg`), by these lists of mark states:
    /// - without a filter: no mark at the start, then one held mark (ConsistentGuardedRunEnd.al
    ///   GuardedRunIdRec_InnerStartsStateAndFails :415, GuardedRunInstanceRec_InnerStartsStateAndFails :484) or two
    ///   (ConsistentGuardedRunRecFiltered.al RecZ_NoFilter_StartX_MarkYByA_Fails :466) and a run that fails with code
    ///   Dialog; one held mark (ConsistentGuardedRunEnd.al GuardedRunIdRec_InnerStartsState_Succeeds :815), two
    ///   (ConsistentGuardedRunRecFail.al RecZ_MarkYByA_MarkXByA_Ends :590, ConsistentGuardedRunRecFiltered.al
    ///   RecZ_NoFilter_StartX_MarkYByA_Ends :496), released then held (RecFail.al RecZ_StartX_TrueXByB_MarkYByA_Ends :485)
    ///   or one released (RecZ_StartX_TrueXByB_Ends :500) and a run that succeeds; one mark the caller holds, which the run
    ///   leaves as it was, and a run that fails with code Dialog or succeeds (ConsistentGuardedRunEnd.al
    ///   GuardedRunIdRec_CallerMarks_InnerFails :892, _InnerSucceeds :911, GuardedRunInstanceRec_CallerMarks_InnerFails
    ///   :930);
    /// - with one of [`ArgFilter::Measured`], no mark at the start and two held marks at the end, a run that fails with code
    ///   Dialog or succeeds (ConsistentGuardedRunRecFiltered.al `_StartX_MarkYByA_Fails`, `_StartX_MarkYByA_Ends`);
    /// - with a page view, only in the cells' page view shape (`page_view`), no mark at the start and one held mark at the
    ///   end, a run that fails with code Dialog (ConsistentGuardedRunPageConst.al Page_OrderDirect_Fails :1125,
    ///   Page_OrderCardDirect_Fails :1170).
    pub(super) fn end_measured(
        &self,
        before: &[(u32, MarkState)],
        after: &[(u32, MarkState)],
        end: RunEnd,
        page_view: bool,
    ) -> bool {
        use MarkState::{Marked as M, ReleasedByAnother as R};
        use RunEnd::{Failed, Succeeded};
        let RunArg::Record { filter, .. } = self.arg else {
            return true;
        };
        let start: Vec<MarkState> = before.iter().map(|(_, s)| *s).collect();
        let states: Vec<MarkState> = after.iter().map(|(_, s)| *s).collect();
        let unfiltered = match (start.as_slice(), states.as_slice(), end) {
            ([], [M] | [M, M], Failed("Dialog")) | ([], [M] | [M, M] | [R, M] | [R], Succeeded) => true,
            ([M], [M], Failed("Dialog") | Succeeded) => before == after,
            _ => false,
        };
        let filtered = start.is_empty() && states == [M, M] && matches!(end, Failed("Dialog") | Succeeded);
        self.kept
            && self.database_set
            && !self.lifted_by_arg
            && match filter {
                ArgFilter::None => unfiltered,
                ArgFilter::Measured { .. } => filtered,
                ArgFilter::Page(PageArgShape::PageView) => {
                    page_view && start.is_empty() && states == [M] && end == Failed("Dialog")
                }
                ArgFilter::Page(_) | ArgFilter::Other => false,
            }
    }

    /// Whether an end `fail_probe_end` takes runs through `guarded-run-consistent-arg` too: the argument set the first mark
    /// and a database variable the second (corpus PR 1590 measures it only after the run modified the argument, step
    /// `modrec`, which the session does not track; al-runner issue 9157), or the argument released a mark (every cell
    /// releases through another variable, step `txb`; al-runner issue 9184), or it set, released or marked again a mark that
    /// was then lifted (`lifted_by_arg`).
    pub(super) fn fail_awaits_probe(&self) -> bool {
        self.sets == ArgMarks::FirstThenDatabase || self.releases || self.lifted_by_arg
    }

    /// An end of `guarded-run-consistent-page`, `-page-view` or `-two-key` runs through `guarded-run-consistent-arg` too
    /// when the argument set, released or marked again a mark that was then lifted (`lifted_by_arg`): no cell of corpus PR
    /// 1600, 1616 or 1622 has the argument call `Consistent`.
    pub(super) fn assume_if_lifted(&self) {
        if self.lifted_by_arg {
            assume(Assumption::GuardedRunConsistentArg);
        }
    }

    /// The run left the argument as it was, and the argument set, released and marked again no mark: every cell of corpus
    /// PR 1600 (`page_probe_end`) releases through a variable other than the argument (step `txb`, variable XB).
    pub(super) fn kept_and_passive(&self) -> bool {
        self.kept && self.database_set && !self.releases && self.sets == ArgMarks::None
    }

    /// No mark names the argument's table, or it is the table marked first.
    pub(super) fn unmarked_or_first(&self) -> bool {
        matches!(self.table, ArgTable::Unmarked | ArgTable::First)
    }

    /// Whether the argument is a record of corpus PR 1598 (ConsistentGuardedRunRecFiltered.al, `RunRec`) that the run left
    /// as it was: a database record of a table no mark names (the `RecZ_` cells), of the table marked first (`RecX_`) or of
    /// the table marked second (`RecY_`), or a temporary record of the table marked first (`RecTempX_`), each without a
    /// filter or with one of [`ArgFilter::Measured`].
    pub(super) fn filtered_probe(&self) -> bool {
        let RunArg::Record {
            temporary,
            filter: ArgFilter::None | ArgFilter::Measured { .. },
            ..
        } = self.arg
        else {
            return false;
        };
        self.kept
            && match temporary {
                false => matches!(self.table, ArgTable::Unmarked | ArgTable::First | ArgTable::Second),
                true => self.table == ArgTable::First,
            }
    }
}
