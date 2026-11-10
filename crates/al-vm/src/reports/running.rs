//! A Run or RunModal of a report instance that is running now.

use super::*;

impl Session {
    /// A RunModal of the instance that is running now (the bare `RunModal()` in a procedure its OnPreReport calls) raises the
    /// error and runs nothing; the run goes on (reports/runtime/Y40BareReportRun.al:362,
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
        Err(already_run())
    }
}

impl Session {
    /// The `UseRequestPage()` getter: what the next Run does (`report_shows_page`). Read in OnPreReport, or in a procedure it
    /// calls, of a report a static `RunModal(id, false)` runs: false, for the two and the three argument form (corpus reports/request-page/P147UseRequestPagePreGet.al:93-131
    /// PW_Urp_PreGet_RunModalWithoutRequestPage `trace 1` and `trace 2` = `OnInitReport;OnPreReport;get=No;`; BC 27, 28 and 29;
    /// al-runner issue 7413). The static Run and a variable's `UseRequestPage(false)` answer false through the flag itself
    /// (:65-80, `trace 3`; reports/bare-report-member/Y36BareReportMember.al:220-282). In every other trigger of that run the
    /// read is not pinned (corpus probe reports/bare-report-member/UseRequestPageReadInTriggers.al).
    pub(super) fn use_request_page_getter(&self, inst: &Inst, schema: &Schema) -> Result<Value, Signal> {
        let slot = schema.report().request_flag_slot;
        if matches!(self.report.report_trigger, Some((_, TrigKind::Pre)))
            && matches!(
                inst.borrow().globals[slot],
                Value::Integer(REQUEST_PAGE_OFF_UNPINNED_GETTER)
            )
        {
            return Ok(Value::Boolean(false));
        }
        Ok(Value::Boolean(Self::report_shows_page(inst, schema)?))
    }
}

/// The error of a report variable that already ran (reports/request-page/P105TotArray.al:97-105; the code:
/// reports/runtime/Y40BareReportRun.al:252, text/PnBReportSaveAs.al:749-750).
pub(super) fn already_run_error() -> al_builtins::native::AlError {
    al_builtins::native::AlError {
        message: "The object has been run. Use the function CLEAR(Report).".into(),
        code: "ObjectHasAlreadyBeenRun".into(),
        call_stack: String::new(),
        ..Default::default()
    }
}

pub(super) fn already_run() -> Signal {
    Signal::Error(Box::new(already_run_error()))
}
