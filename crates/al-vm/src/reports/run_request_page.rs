//! `Rep.RunRequestPage` and `Report.RunRequestPage`: OnInitReport first (a quit there is `quit_in_init.rs`), then the request
//! page alone, whose OK answers the parameters.

use super::*;

impl Session {
    /// `Rep.RunRequestPage` (`REPORT_RUNREQUESTPAGE_1`) and `Report.RunRequestPage` (`REPORT_RUNREQUESTPAGE_2`).
    pub(super) fn report_run_request_page(
        &mut self,
        id: al_ir::BuiltinId,
        receiver: Option<Value>,
        args: &[Value],
    ) -> Result<Value, Signal> {
        let (inst, schema, seed, pinned_call) = if id == ids::REPORT_RUNREQUESTPAGE_1 {
            let (inst, quit) = self.report_instance(receiver, InitQuit::Stops)?;
            if quit {
                return self.request_page_after_quit(&self.report_schema(&inst)?, false, args);
            }
            // After a RunModal of the variable: the error, and no page opens
            // (reports/request-page/RptRequestPage.al:990-1009 UseRequestPageFalse_RunModalAndRunRequestPage).
            if self.report_run_modal_done_on(&inst) {
                return Err(running::already_run());
            }
            let schema = self.report_schema(&inst)?;
            (inst, schema, args.first(), false)
        } else {
            // The static form raises SubstituteReport with RunMode ParametersOnly first, once, with an empty
            // RequestPageXml also when old parameters are given (events/platform-triggers/PeVTests.al:465, :660:
            // `SubstituteReport(69860,ParametersOnly,in=-1)`; reports/Y14ReportSubstitute.al:276, :447).
            // A subscriber that sets another report swaps it: that report's request page opens, its handler runs and
            // the parameters are that report's (Y14ReportSubstitute.al:225 platform event, :261 OnAfterSubstituteReport,
            // :447 with old parameters; corpus master).
            let number = self.substitute_static(args.first(), 1, "")?;
            // SetupPrinters follows, once, with an empty Printers (events/platform-triggers/PeVTests.al:450
            // RunRequestPage_RaisedEvents `SubstituteReport(69860,ParametersOnly,in=-1);SetupPrinters(in=0);`, :651).
            self.raise_setup_printers()?;
            let (inst, schema) = self.report_by_number(id, number.as_ref().or(args.first()))?;
            // OnInitReport runs first, so the views the data items get next are what the handler sees
            // (reports/W3ReportRequestPageRun.al:280, RunRequestPage_OrderOfReportAndRequestPageTriggers: `view=[1..3]`).
            if self.report_init(&inst)? {
                return self.request_page_after_quit(&schema, true, args);
            }
            (inst, schema, args.get(1), args.len() == 1 && number.is_none())
        };
        self.request_ok_only_call(schema.object, pinned_call)?;
        // Only the empty string and the text this runtime produced are understood; how BC
        // loads a parameters string into the page is pinned only by its result
        // (reports/TestReportInstanceRunReqPage.al:SeededWithParameters_StillRunsTheHandler).
        let seed = match seed {
            Some(Value::Text(s)) if !s.is_empty() => Some(s.clone()),
            None | Some(Value::Text(_)) => None,
            Some(other) => return unsupported(format!("{id} seeded with {}", other.describe())),
        };
        // RunRequestPage always shows the page, so it always enters a transaction world
        // (record/TestTxModelReportExec.al:77-102, Test02 and Test03).
        let enters = self.report_enter_world(&schema, true)?;
        self.report_in_world(enters, |s| s.report_request_only(&inst, &schema, seed.as_deref()))
            .map(|s| Value::Text(s.into()))
    }
}
