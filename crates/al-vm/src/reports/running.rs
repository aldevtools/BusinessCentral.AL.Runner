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
            .any(|r| r.by_run && r.inst.upgrade().is_some_and(|i| Rc::ptr_eq(&i, inst)))
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
    /// The `UseRequestPage()` getter. In a trigger of the run that is running now (Run, RunModal or Execute), read through
    /// `CurrReport.`, bare with or without parentheses, it answers whether this run showed its request page: OnPreReport,
    /// OnPreDataItem, OnAfterGetRecord, OnPostDataItem, OnPostReport and the request page's OnOpenPage
    /// (reports/bare-report-member/UseRequestPageReadInTriggers.al:244-460, BC 27, 28 and 29; al-runner issues 7413, 7414):
    /// - a variable's `UseRequestPage(false)` with Run and RunModal, static Run and RunModal with RequestWindow false, and
    ///   `Execute` with empty parameters answer false (:244-305, :396-410); so does a report declaring `UseRequestPage = false`
    ///   run by a variable never set (:432-445);
    /// - a run that showed its page answers true, also the variable's `UseRequestPage(true)` on such a report (:307-394, :464-475);
    /// - `Execute` with the parameters RunRequestPage returned answers false in OnOpenPage, which it runs (:412-430).
    ///
    /// Anywhere else (OnInitReport, a report variable that is not running, other triggers) it answers what the next Run does
    /// (`report_shows_page`; OnInitReport reads the property: reports/request-page/P147UseRequestPageInitGet.al:79, the probe
    /// above `init Yes/Yes/Yes`). A static `RunModal(id, false)` keeps its OnPreReport answer false there
    /// (reports/request-page/P147UseRequestPagePreGet.al:93-131).
    pub(super) fn use_request_page_getter(&self, inst: &Inst, schema: &Schema) -> Result<Value, Signal> {
        if let Some(shows_page) = self.running_page_answer(inst) {
            return Ok(Value::Boolean(shows_page));
        }
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

    /// Whether the innermost running report is `inst` and the code runs in one of the triggers `use_request_page_getter`
    /// pins: then whether that run showed its request page.
    fn running_page_answer(&self, inst: &Inst) -> Option<bool> {
        let run = self.report.report_running.last()?;
        if !run.inst.upgrade().is_some_and(|i| Rc::ptr_eq(&i, inst)) {
            return None;
        }
        let report_trigger = matches!(
            self.report.report_trigger,
            Some((
                _,
                TrigKind::Pre | TrigKind::PreItem | TrigKind::Row | TrigKind::PostItem | TrigKind::PostReport
            ))
        );
        let open_page = matches!(
            self.pages.trigger_kinds.last(),
            Some((_, crate::pages::TriggerKind::OpenPage))
        );
        (report_trigger || open_page).then_some(run.shows_page)
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
