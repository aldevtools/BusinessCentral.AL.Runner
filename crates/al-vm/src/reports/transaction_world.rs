//! The platform's transaction-world decision for a report run: [`Session::report_enter_world`], and the moments of the open
//! transaction it is measured at ([`Moment`]).

use super::Schema;
use crate::transactions::{TRANSACTION_TYPE_UPDATE_NO_LOCKS as UPDATE_NO_LOCKS, TxModel, TxPhase};
use crate::{Session, Signal};
use al_builtins::assumptions::{self, Assumption};
use al_builtins::native::unsupported;

/// Members of the TransactionType option (session-system/ProbeSystemEnumMembers.al:60-75).
const UPDATE: i32 = 1;
const SNAPSHOT: i32 = 2;
const BROWSE: i32 = 3;
const REPORT: i32 = 4;

/// Where the open transaction began, and which of the steps the corpus cells vary have run in it since: the moment an
/// Update report without its request page runs at (`OpenTx::moment`). The start of a test method and an AL `Commit` in the
/// code of a test (not in a Codeunit.Run, a report, a TryFunction or a page) open it; a step no cell has ends it, and so do a
/// Codeunit.Run, a report run that enters a world and the end of the transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Moment {
    start: MomentStart,
    /// The AL call depth of the code that committed or set the type: the set and the report run come from that code, its
    /// database calls from it or the procedures it calls; anything else is a step no cell has.
    depth: Option<usize>,
    /// The `CurrentTransactionType` set that took effect since, before any database call.
    set: Option<i32>,
    /// A read call ran since.
    read: bool,
    /// Report runs since that entered no transaction world; nothing ran between them.
    runs: u8,
    /// Nothing but the set ran before the first of them.
    first_run_clean: bool,
}

/// What began a [`Moment`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MomentStart {
    TestStart,
    Commit,
}

impl Moment {
    pub(crate) fn at(start: MomentStart) -> Self {
        Moment {
            start,
            depth: None,
            set: None,
            read: false,
            runs: 0,
            first_run_clean: false,
        }
    }

    /// The moment at the test's start the corpus pins with no set (record/TestTxModelReportRun.al:177-211).
    fn pinned_test_start(&self) -> bool {
        self.start == MomentStart::TestStart && self.set.is_none() && self.runs == 0
    }
}

/// What the platform does with an Update report run without its request page at a measured moment.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Cell {
    NoWorld,
    World,
}

impl Session {
    /// Whether a step of `m` runs in the code its steps run in; the first step after the test's start decides it.
    fn moment_depth(&self, m: &mut Moment) -> bool {
        *m.depth.get_or_insert(self.depth) == self.depth
    }

    /// An AL `Commit` ran (`transactions.rs`): it opens a moment in the code of a test, outside a Codeunit.Run (its OnRun
    /// is deeper than the code the report later runs in), a report, a TryFunction and a page operation.
    pub(crate) fn moment_after_commit(&mut self) {
        let plain = self.report.report_running.is_empty()
            && self.tx.tx_world == 0
            && self.tx.try_depth == 0
            && self.tx.guarded_runs.is_empty();
        self.tx.open.moment = plain.then(|| Moment {
            depth: Some(self.depth),
            ..Moment::at(MomentStart::Commit)
        });
    }

    /// A database call of AL code on a record that is not temporary (`records/op_inner.rs`), before a write sets the phase.
    /// A report's own calls are not steps of the moment its run was decided at. At the test's start without a set a read
    /// ends the pinned moment and writes keep it; after a set or a Commit only the calls of the cells keep it:
    /// record/TxTypeUpdateReport.al TxU01 to TxU06, TxU27 to TxU32 (after the Update set: Get, FindFirst, FindSet and Next,
    /// Count and IsEmpty, Insert, Modify, Delete, reads before writes), TxU10 and TxU33 (one Insert or one FindFirst after
    /// the set at the test's start), TxU08, TxU12 and TxU20 (Inserts at UpdateNoLocks).
    pub(crate) fn moment_db_call(&mut self, method: al_builtins::record_op::RecordOp, write: bool) {
        use al_builtins::record_op::RecordOp::{Count, Delete, FindFirst, FindSet, Get, Insert, IsEmpty, Modify, Next};
        if !self.report.report_running.is_empty() {
            return;
        }
        let Some(mut m) = self.tx.open.moment else { return };
        let kept = if m.pinned_test_start() {
            true
        } else {
            let after_write = self.tx.phase == TxPhase::Write;
            // The code of the moment and the procedures it calls (TxU06 inserts through a local procedure); not code it
            // returned to, and not a page operation's triggers.
            m.depth.is_none_or(|d| self.depth >= d)
                && self.tx.tx_world == 0
                && m.runs == 0
                && match (m.start, m.set) {
                    (MomentStart::Commit, Some(UPDATE)) => {
                        if write {
                            matches!(method, Insert | Modify | Delete)
                        } else {
                            !after_write && matches!(method, Get | FindFirst | FindSet | Next | Count | IsEmpty)
                        }
                    }
                    // One call: nothing ran before it (a read call has already started the transaction here).
                    (MomentStart::TestStart, Some(UPDATE)) => {
                        !m.read && !after_write && matches!(method, Insert | FindFirst)
                    }
                    (MomentStart::Commit, None | Some(UPDATE_NO_LOCKS)) => write && method == Insert,
                    _ => false,
                }
        };
        m.read |= !write;
        self.tx.open.moment = kept.then_some(m);
    }

    /// A `CurrentTransactionType` set to `new` (`transactions.rs`); `fresh` when no database call has run in the
    /// transaction, so the set takes effect. The cells set the type once, before any database call, in the code of the
    /// moment: any other set ends it.
    pub(crate) fn moment_type_set(&mut self, new: i32, fresh: bool) {
        self.tx.open.moment = match self.tx.open.moment {
            Some(mut m) if fresh && m.set.is_none() => {
                self.moment_depth(&mut m).then_some(Moment { set: Some(new), ..m })
            }
            _ => None,
        };
    }

    /// The platform's decision, made before the request page is shown: a run enters a transaction world when it shows the
    /// request page or its TransactionType differs from the session's (`UpdateNoLocks` excepted), and a test under
    /// `TransactionModel::AutoRollback` is exempt (record/TestTxModelReportRun.al:1-35, record/TestTxModelReportExec.al:1-30).
    /// Entering one is refused while the caller holds an uncommitted write (:117-148 Test04/05/08;
    /// TestTxModelReportExec.al:77-102, 138-150, 214-239). An Update report without its request page runs only at a moment a
    /// cell measures ([`Self::update_report_cell`]); `variable_run` says a report variable's Run or RunModal runs it, the only
    /// calls the cells after a Commit or a set make. Returns whether a world is entered.
    pub(super) fn report_enter_world(
        &mut self,
        schema: &Schema,
        shows_page: bool,
        variable_run: bool,
    ) -> Result<bool, Signal> {
        // Every report run ends the moment it ran at; a run that enters no world keeps it, counted (`Moment::runs`).
        let moment = self.tx.open.moment.take();
        let (differs, update_report) = match schema.report().transaction_type.as_deref() {
            None => (false, false),
            Some("update") => (true, true),
            // Snapshot runs when no write is pending (reports/TestReportRuntimeContracts.al:
            // 509-525); with a pending write what BC does is not pinned.
            Some("snapshot") => {
                if self.in_write_transaction()? {
                    return unsupported(format!(
                        "TransactionType = snapshot on report \"{}\" with a pending write (not pinned)",
                        schema.name()
                    ));
                }
                (true, false)
            }
            Some(other) => {
                return unsupported(format!(
                    "TransactionType = {other} on report \"{}\" (only Update and Snapshot are pinned)",
                    schema.name()
                ));
            }
        };
        if !(shows_page || differs) {
            return Ok(false);
        }
        if update_report && !shows_page {
            match self.update_report_cell(moment, variable_run) {
                None => {
                    return unsupported(format!(
                        "TransactionType = Update report \"{}\" run without its request page at a moment no corpus cell \
                         measures (they run it at the test method's start, or after a Commit with at most one \
                         CurrentTransactionType set, under AutoCommit)",
                        schema.name()
                    ));
                }
                Some(Cell::NoWorld) => {
                    let clean = self.tx.phase == TxPhase::Fresh && moment.is_some_and(|m| !m.read);
                    self.tx.open.moment = moment.map(|m| Moment {
                        runs: m.runs + 1,
                        first_run_clean: if m.runs == 0 { clean } else { m.first_run_clean },
                        ..m
                    });
                    return Ok(false);
                }
                Some(Cell::World) => {}
            }
        }
        if self.tx_model == TxModel::AutoRollback {
            // A report declaring a TransactionType needs a type change, which BC refuses once a
            // transaction has started; AutoRollback exempts the world, not that rule
            // (record/TestTxModelReportExec.al:243-257, Test12).
            if differs && self.in_write_transaction()? {
                return unsupported(
                    "a TransactionType = Update report under AutoRollback with a pending write (not pinned)",
                );
            }
            return Ok(false);
        }
        // A run with the request page after an uncommitted insert raises before the page opens (reports/bare-report-member/
        // Y36BareReportMember.al:236-249, Report.Run(Id, true) in a test; the handler is not called).
        if self.in_write_transaction()? {
            return Err(crate::transactions::transaction_stopped());
        }
        // Inside a TryFunction, with nothing to commit, the run goes on as outside one (misc/lt-y14-triage/
        // Y14ReportSubstitute.al:276 RunRequestPage, :371 Run and RunModal, after a Commit, corpus master). A run that
        // writes there is refused when its world would commit ([`Self::report_in_world`]).
        Ok(true)
    }

    /// The cell of an Update report run without its request page at `moment`, `None` when no corpus cell measures it.
    /// At the start of the test method at UpdateNoLocks, with nothing before the run but writes, it enters a world:
    /// record/TestTxModelReportRun.al:177-211 Test07/Test08, TestTxModelReportExec.al:137-171 Test06/Test07, :213-239
    /// Test10/Test11 under AutoCommit, :241-257 Test12 under AutoRollback. Every other cell is the open probe of corpus pull
    /// request 1488 (`update-report-by-session-type`): no world while the session's type is Update, a world for every other
    /// type.
    fn update_report_cell(&self, moment: Option<Moment>, variable_run: bool) -> Option<Cell> {
        use MomentStart::{Commit, TestStart};
        use TxPhase::{Fresh, Read, Write};
        let mut m = moment?;
        let session_type = self.transaction_type_seen().ok()?;
        if m.pinned_test_start() && !m.read && session_type == UPDATE_NO_LOCKS {
            let model = matches!(self.tx_model, TxModel::AutoCommit | TxModel::AutoRollback);
            return (matches!(self.tx.phase, Fresh | Write) && model).then_some(Cell::World);
        }
        // Every cell of the probe runs a report variable under AutoCommit, in the code of the moment, with no lock.
        let plain = variable_run
            && self.tx_model == TxModel::AutoCommit
            && self.tx.tx_world == 0
            && self.tx.try_depth == 0
            && self.tx.guarded_runs.is_empty()
            && !self.tx.open.locks.any()
            && self.moment_depth(&mut m);
        if !plain || (m.runs > 0 && !m.first_run_clean) {
            return None;
        }
        let cell = match (m.start, m.set, self.tx.phase, m.read, m.runs) {
            // The type is Update: no world, whatever measured call ran after the set (`moment_db_call`).
            // TxTypeUpdateReport.al:269 TxU04 and :233 TxU02 (nothing), :218 TxU01, :247 TxU03 and :288 TxU05 (a Get), :633
            // TxU27, :652 TxU28 and :673 TxU29 (FindFirst, FindSet and Next, Count and IsEmpty), :308 TxU06, :693 TxU30 and
            // :714 TxU31 (Insert, Modify, Delete), :734 TxU32 (a Get, then an Insert); RunModal and Run.
            (Commit, Some(UPDATE), _, _, 0) => Cell::NoWorld,
            // :510 TxU18: a second run right after the first, with nothing before the first.
            (Commit, Some(UPDATE), _, _, 1) => Cell::NoWorld,
            // The set at the test's start: :357 TxU09 (nothing), :371 TxU10 (an Insert), :754 TxU33 (a FindFirst).
            (TestStart, Some(UPDATE), Fresh | Write, false, 0) | (TestStart, Some(UPDATE), Read, true, 0) => {
                Cell::NoWorld
            }
            // Update the test before left, at the test's start: session-system/TxTypeNextTestMethod.al TxN12.
            (TestStart, None, Fresh, false, 0) if session_type == UPDATE => Cell::NoWorld,
            // Another type: a world. :327 TxU07 and :531 TxU19 (the UpdateNoLocks a Commit kept), :341 TxU08
            // and :548 TxU20 (with an Insert, so the run is refused).
            (Commit, None, Fresh | Write, false, 0) if session_type == UPDATE_NO_LOCKS => Cell::World,
            // :389 TxU11, :404 TxU12 (UpdateNoLocks, also with an Insert).
            (Commit, Some(UPDATE_NO_LOCKS), Fresh | Write, false, 0) => Cell::World,
            // :420 TxU13, :435 TxU14, :450 TxU15 (Snapshot, Report, Browse).
            (Commit, Some(SNAPSHOT | REPORT | BROWSE), Fresh, false, 0) => Cell::World,
            _ => return None,
        };
        // The cells run a report that enters no world outside an asserterror; the world cells refuse inside one too.
        if cell == Cell::NoWorld && self.tx.assert_depth != 0 {
            return None;
        }
        assumptions::assume(Assumption::UpdateReportBySessionType);
        Some(cell)
    }
}
