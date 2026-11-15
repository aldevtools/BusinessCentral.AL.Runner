//! The answer of a `Report.SaveAs` of a processing-only report (moved out of `report_save_core`).

use super::*;

impl Session {
    /// reports/layouts-saveas/TestReportSaveAsColumnTypesContracts.al:372-388: false with the text and the code
    /// `ReportProcessingOnly`, no trigger runs and nothing is written. With the answer dropped
    /// BC's behaviour is not pinned.
    pub(super) fn save_of_processing_only(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        format: &str,
        at: usize,
        quit: bool,
        result_used: bool,
    ) -> Result<Value, Signal> {
        // The Xml SaveAs of a processing-only report runs OnInitReport before it answers false
        // (reports/request-page/RptTriggerOrder.al:308,:324 SaveAs_NestedItems_Order, SaveAs_Siblings_Order;
        // reports/PnH5aExt.al:380). For the other formats that is not pinned and no trigger runs.
        if format == "Xml" && at != 0 && self.report_init(inst)? {
            return unsupported("CurrReport.Quit in OnInitReport of a SaveAs of a processing-only report (not pinned)");
        }
        // The request page's OnInit follows, as in a run without the page (reports/reporthandler/AeReportHandler.al:
        // 133-145 SaveAs_WithReportHandler `OnInitReport;RP OnInit ...;` with an empty parameters string; with parameters the
        // same error follows and OnPreReport does not run, Y68SelfClosingParams.al:407-418).
        if format == "Xml" && at != 0 {
            self.request_page_init_unshown(inst, schema)?;
        }
        if format == "Xml" && at == 0 && quit {
            return unsupported("CurrReport.Quit in OnInitReport of a SaveAs of a processing-only report (not pinned)");
        }
        let error = al_builtins::native::AlError {
            message: format!(
                "Report {} is a processing-only report. Output cannot be rendered and saved to a file.",
                self.object_id_of(schema.object as u32)
            ),
            code: "ReportProcessingOnly".into(),
            call_stack: String::new(),
            ..Default::default()
        };
        // The statement form raises the same error (streams/TestReportSaveAsColumnDetailsContracts.al:
        // ProcessingOnlyReport_StatementFormRaises, streams/PWReportSaveAsColumns.al:
        // ProcessingOnly_AnswerDropped_AsserterrorForm); with the answer used it is returned as false.
        if !result_used {
            return Err(Signal::Error(Box::new(error)));
        }
        self.last_error = error;
        Ok(Value::Boolean(false))
    }
}
