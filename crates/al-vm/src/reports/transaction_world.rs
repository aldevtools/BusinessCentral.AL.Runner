//! The platform's transaction-world decision for a report run: [`Session::report_enter_world`].

use super::Schema;
use crate::transactions::{TxModel, TxPhase};
use crate::{Session, Signal};
use al_builtins::native::unsupported;

impl Session {
    /// The platform's decision, made before the request page is shown: a run enters a transaction
    /// world when it shows the request page or its TransactionType differs from the session's
    /// (`UpdateNoLocks` excepted), and a test under `TransactionModel::AutoRollback` is exempt
    /// (record/TestTxModelReportRun.al:1-35, record/TestTxModelReportExec.al:1-30). Entering one
    /// is refused while the caller holds an uncommitted write (:117-148 Test04/05/08;
    /// TestTxModelReportExec.al:77-102, 138-150, 214-239). An Update report without its request page
    /// runs only at the moment its term is measured, the start of the test method (`OpenTx::at_test_start`).
    /// Returns whether a world is entered.
    pub(super) fn report_enter_world(&mut self, schema: &Schema, shows_page: bool) -> Result<bool, Signal> {
        // Every report run ends that moment, whatever it decides below.
        let at_test_start = std::mem::take(&mut self.tx.open.at_test_start);
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
        // Without the request page the TransactionType term decides. For an Update report the corpus measures it only at the
        // start of the test method: nothing before the run but, in some tests, one Insert. No read call, no
        // CurrentTransactionType set, no Codeunit.Run, no earlier report run, no Commit and no rollback
        // (`OpenTx::at_test_start`), so the transaction has not started or a write started it. Under AutoCommit:
        // record/TestTxModelReportRun.al:177-211 Test07/Test08, TestTxModelReportExec.al:137-171 Test06/Test07, :213-239
        // Test10/Test11. Under AutoRollback, with no write: TestTxModelReportExec.al:241-257 Test12. Every other moment, and
        // TransactionModel None, is not pinned.
        let measured_moment = at_test_start
            && matches!(self.tx.phase, TxPhase::Fresh | TxPhase::Write)
            && matches!(self.tx_model, TxModel::AutoCommit | TxModel::AutoRollback);
        if update_report && !shows_page && !measured_moment {
            return unsupported(format!(
                "TransactionType = Update report \"{}\" run without its request page after the start of the test method (a \
                 Commit, rollback, read call, transaction type set, Codeunit.Run or report run since), or under TransactionModel None (not pinned)",
                schema.name()
            ));
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
}
