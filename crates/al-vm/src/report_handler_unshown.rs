//! Which report runs a test's `[ReportHandler]` takes over, and the hand-over itself.
//!
//! A run that shows the request page goes to `Session::report_handler_answers` (reports/reporthandler/AeReportHandler.al:31-158).
//! A plain Run or RunModal without the page of a report that renders output (not processing-only) calls the handler bound for
//! it too, and the run ends there: Run is pinned by reports/layouts-saveas/Y43SystemPrinterRequestPage.al:313
//! Y43_G411_Layout_Run_ReqFalse_Sp_ReportHandler (`ok / handler;`, no OnPreReport, no data item); RunModal by
//! reports/reporthandler/AeReportHandler.al:237 Report_RunModal_RdlcReportCallsRegisteredHandler (corpus pull request 1199,
//! al-runner issue 7269: `Report.RunModal(Report::"Assembly Order", false, false)` logs only what the handler adds). Execute and SaveAs keep not calling it (AeReportHandler.al:121,
//! :133). A processing-only report run without its page is left as it was: whether the handler is called there is not pinned.

use std::cell::RefCell;
use std::rc::Rc;

use al_builtins::value::{Instance, Value};

use crate::reports::Schema;
use crate::ui::UiKind;
use crate::{Session, Signal};

type Inst = Rc<RefCell<Instance>>;

impl Session {
    /// Whether the test's `[ReportHandler]` took this run over (and ran): see the module comment.
    pub(crate) fn report_handler_takes_run(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        shows_page: bool,
        save_as: bool,
    ) -> Result<bool, Signal> {
        if shows_page {
            return self.report_handler_answers(inst, schema);
        }
        let plain_run = !save_as && self.report.execute_parameters.is_none();
        if !plain_run || schema.report().processing_only || !self.has_page_handler(UiKind::Report, schema.object as u32)
        {
            return Ok(false);
        }
        if self.has_page_handler(UiKind::RequestPage, schema.object as u32) {
            return Err(Signal::Unsupported(format!(
                "a [ReportHandler] and a [RequestPageHandler] for report \"{}\" in one test, run without its request page (not pinned)",
                schema.name()
            )));
        }
        self.hand_run_to_report_handler(inst, schema)
    }

    /// The request page's OnInit, then the handler gets the report, and the run ends there.
    pub(crate) fn hand_run_to_report_handler(&mut self, inst: &Inst, schema: &Schema) -> Result<bool, Signal> {
        self.request_page_init_unshown(inst, schema)?;
        self.report.rp_init_by_handler.push(Rc::downgrade(inst));
        let (o, r, hi) = self.bind_page_handler(UiKind::Report, schema.object as u32)?;
        // The handler takes the run over: its Run or RunModal of the instance runs it (AeReportHandlerCalls.al:79-99), so the
        // instance does not count as running while the handler runs.
        let running = std::mem::take(&mut self.report.report_running);
        let called = self.call(o, r, hi, vec![Value::Codeunit(inst.clone())]);
        self.report.report_running = running;
        called?;
        Ok(true)
    }
}
