//! A Run or RunModal of a report instance that is running now.

use super::*;

impl Session {
    /// A RunModal of the instance that is running now (the bare `RunModal()` in a procedure its OnPreReport calls) raises the
    /// error and runs nothing; the run goes on (report/lt-y40-bare-report-run/Y40BareReportRun.al:362,
    /// Y40_BareRunModal_InsidePreReport, corpus master ac3499d6). A Run there is not pinned.
    pub(super) fn run_of_running_instance(&self, id: al_ir::BuiltinId, inst: &Inst) -> Result<(), Signal> {
        if !self
            .report
            .report_running
            .iter()
            .any(|w| w.upgrade().is_some_and(|i| Rc::ptr_eq(&i, inst)))
        {
            return Ok(());
        }
        if id == ids::REPORT_RUN_1 {
            return unsupported("Report.Run of the report instance that is running (not pinned)");
        }
        Err(crate::transactions::al_error_code(
            "The object has been run. Use the function CLEAR(Report).",
            "ObjectHasAlreadyBeenRun",
        ))
    }
}
