//! The platform's transaction-world decision for a report run: [`Session::report_enter_world`].

use super::Schema;
use crate::transactions::TxModel;
use crate::{Session, Signal};
use al_builtins::native::unsupported;

impl Session {
    /// The platform's decision, made before the request page is shown: a run enters a transaction
    /// world when it shows the request page or its TransactionType differs from the session's
    /// (`UpdateNoLocks` excepted; for an Update report only a session at UpdateNoLocks is measured), and a test under `TransactionModel::AutoRollback` is exempt
    /// (record/TestTxModelReportRun.al:1-35, record/TestTxModelReportExec.al:1-30). Entering one
    /// is refused while the caller holds an uncommitted write (:117-148 Test04/05/08;
    /// TestTxModelReportExec.al:77-102, 138-150, 214-239). Returns whether a world is entered.
    pub(super) fn report_enter_world(&mut self, schema: &Schema, shows_page: bool) -> Result<bool, Signal> {
        let differs = match schema.report().transaction_type.as_deref() {
            None => false,
            Some("update") => true,
            // Snapshot runs when no write is pending (reports/TestReportRuntimeContracts.al:
            // 468-482); with a pending write what BC does is not pinned.
            Some("snapshot") => {
                if self.in_write_transaction()? {
                    return unsupported(format!(
                        "TransactionType = snapshot on report \"{}\" with a pending write (not pinned)",
                        schema.name()
                    ));
                }
                true
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
        let update_report = schema.report().transaction_type.as_deref() == Some("update");
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
        // Without the request page the TransactionType term decides. The corpus measures it for an Update report only
        // against a session at UpdateNoLocks, a test's start type (record/TestTxModelReportRun.al:177-211 Test07/Test08,
        // TestTxModelReportExec.al:138-171 Test06/Test07). After a Commit() the type is Update
        // (session-system/ProbeSystemEnumMembers.al:94-112): whether the run then enters a world is not pinned.
        if update_report && !shows_page {
            match self.tx.transaction_type {
                crate::transactions::TRANSACTION_TYPE_UPDATE_NO_LOCKS => {}
                _ => {
                    return unsupported(format!(
                        "TransactionType = Update report \"{}\" run without its request page in a session whose transaction type is not UpdateNoLocks (not pinned)",
                        schema.name()
                    ));
                }
            }
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
