//! `CurrReport.Quit` in OnInitReport of the report corpus PR 1382 measured, for the ways to start it that a quit stopped only
//! by refusing before (al-runner issue 7964 item 3; reports/runtime/ReportQuitInOnInit.al:235-355, not pinned yet, assumption
//! `report-quit-in-oninit-request-page`): RunRequestPage on a variable and the static one answer '' and open no page, an Xml
//! SaveAs to a stream answers false and writes nothing, and `UseRequestPage(false)` as the first use of a variable quits so
//! that its next Run or RunModal runs nothing. A Run, RunModal or Execute that runs OnInitReport itself already stopped there
//! (`REPORT_RUN_1 | REPORT_RUNMODAL_1`, `REPORT_EXECUTE_1`, the static forms) and is not changed here.

use super::*;
use al_builtins::assumptions::{Assumption, assume};

/// How a caller of [`Session::report_instance`] takes a quit in the OnInitReport its first use runs.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum InitQuit {
    /// The method cannot stop the report: a quit refuses.
    Refuses,
    /// The method stops: it gets the quit back.
    Stops,
    /// Run and RunModal of a variable: they stop, and also take the quit a `UseRequestPage(false)` deferred.
    StopsTakingDeferred,
    /// `UseRequestPage(false)`: a quit in the measured report is kept for the next Run or RunModal.
    Defers,
}

impl Session {
    /// The report the probe measured (ReportQuitInOnInit.al:35-102): ProcessingOnly (:37), a request page (:57-80) without
    /// an OnInit trigger, no UseRequestPage property, no report extension.
    pub(super) fn quit_in_init_measured(&self, schema: &Schema) -> bool {
        let r = schema.report();
        let page = self.engine.program.objects[schema.object].page.as_ref();
        r.processing_only
            && r.has_request_page
            && r.use_request_page.is_none()
            && r.extensions.is_empty()
            && page.is_some_and(|p| p.ext_triggers.is_empty() && p.triggers.iter().all(|(n, _)| n != "oninit"))
    }

    /// Whether `inst` holds a quit that `UseRequestPage(false)` deferred.
    pub(super) fn quit_deferred_on(&self, inst: &Inst) -> bool {
        self.report
            .report_quit_deferred
            .iter()
            .any(|w| w.upgrade().is_some_and(|i| Rc::ptr_eq(&i, inst)))
    }

    /// The quit of the OnInitReport that `UseRequestPage(false)` ran as the first use of `inst` (:265, :305, :355): kept for the
    /// next Run or RunModal. Any other report than the measured one refuses.
    pub(super) fn defer_quit(&mut self, inst: &Inst) -> Result<(), Signal> {
        let schema = self.report_schema(inst)?;
        if !self.quit_in_init_measured(&schema) {
            return unsupported("CurrReport.Quit in OnInitReport before a method that does not run the report (not pinned)");
        }
        assume(Assumption::ReportQuitInOninitRequestPage);
        self.report.report_quit_deferred.retain(|w| w.strong_count() > 0);
        self.report.report_quit_deferred.push(Rc::downgrade(inst));
        Ok(())
    }

    /// Takes the deferred quit of `inst` for its Run or RunModal (:265 Run, :305 RunModal, :355 the first of two Runs).
    pub(super) fn take_deferred_quit(&mut self, inst: &Inst) -> bool {
        let had = self.quit_deferred_on(inst);
        if had {
            assume(Assumption::ReportQuitInOninitRequestPage);
            self.report
                .report_quit_deferred
                .retain(|w| w.upgrade().is_some_and(|i| !Rc::ptr_eq(&i, inst)));
        }
        had
    }

    /// RunRequestPage after a quit in OnInitReport (:235 variable, :245 static, :255 static with a parameters string): '' and no
    /// page, for the measured report; any other refuses.
    pub(super) fn request_page_after_quit(&mut self, schema: &Schema) -> Result<Value, Signal> {
        if !self.quit_in_init_measured(schema) {
            return unsupported("CurrReport.Quit in OnInitReport of a RunRequestPage (not pinned)");
        }
        assume(Assumption::ReportQuitInOninitRequestPage);
        Ok(Value::Text("".into()))
    }

    /// An Xml SaveAs to a stream with its answer used after a quit in OnInitReport (:325 variable, :335 static): false and
    /// nothing written, for the measured report; any other refuses.
    pub(super) fn save_as_after_quit(&mut self, schema: &Schema, stream: bool, result_used: bool) -> Result<Value, Signal> {
        if !(stream && result_used && self.quit_in_init_measured(schema)) {
            return unsupported("CurrReport.Quit in OnInitReport of a SaveAs of a processing-only report (not pinned)");
        }
        assume(Assumption::ReportQuitInOninitRequestPage);
        Ok(Value::Boolean(false))
    }
}
