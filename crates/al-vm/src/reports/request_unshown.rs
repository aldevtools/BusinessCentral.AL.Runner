//! The request page triggers of a run that does not show its page: OnInit, and for an Execute with parameters OnOpenPage and
//! OnClosePage.

use super::*;

impl Session {
    /// The request page's `OnInit` runs although the page is not shown, after OnInitReport and before OnPreReport, and
    /// OnOpenPage does not: a SaveAs (text/PnBReportSaveAs.al:348 TriggerOrder, :262 Rows_SkipAndBreak),
    /// an Execute and a Run with UseRequestPage(false) (reports/reporthandler/AeReportHandler.al:85, :121). The values of
    /// an Execute's or a SaveAs's parameters land after it (`report_body`). Not pinned, and refused: an OnInit a reportextension
    /// adds to the page. The values of a SaveAs's parameters landing after it: reports/runtime/
    /// Y68SelfClosingParams.al:420-444 (Q1_SaveAs_SelfClosing_DataReport, a control they do not name keeps OnInit's value).
    /// Returns whether the trigger ran here.
    pub(crate) fn request_page_init_unshown(&mut self, inst: &Inst, schema: &Schema) -> Result<bool, Signal> {
        let engine = self.engine.clone();
        let Some(page) = engine.program.objects[schema.object].page.as_ref() else {
            return Ok(false);
        };
        if page.ext_triggers.iter().any(|(n, _, _)| n == "oninit") {
            return unsupported(format!(
                "the OnInit a reportextension adds to the request page of report \"{}\", which runs without its page (not pinned)",
                schema.name()
            ));
        }
        let Some(r) = page.triggers.iter().find(|(n, _)| n == "oninit").map(|(_, r)| *r) else {
            return Ok(false);
        };
        // It ran before the [ReportHandler] that runs the report now.
        if self
            .report
            .rp_init_by_handler
            .iter()
            .any(|w| w.upgrade().is_some_and(|i| Rc::ptr_eq(&i, inst)))
        {
            return Ok(false);
        }
        // It ran at the procedure that used the variable first and does not run again
        // (reports/request-page/RequestPageOnInitFirstUse.al:104-185).
        if self.request_page_init_done(inst) {
            return Ok(false);
        }
        self.call(schema.object, r, inst.clone(), Vec::new()).map(|_| true)
    }

    /// The request page's OnOpenPage, then its OnClosePage, for the parameters of an Execute whose page's OnInit ran without
    /// the page (`report_body`). Refused: the triggers a reportextension adds to the page (not pinned).
    pub(super) fn request_page_open_close_unshown(&mut self, inst: &Inst, schema: &Schema) -> Result<(), Signal> {
        let engine = self.engine.clone();
        let Some(page) = engine.program.objects[schema.object].page.as_ref() else {
            return Ok(());
        };
        for name in ["onopenpage", "onclosepage"] {
            if page.ext_triggers.iter().any(|(n, _, _)| n == name) {
                return unsupported(format!(
                    "the {name} trigger a reportextension adds to the request page of report \"{}\", for an Execute with parameters (not pinned)",
                    schema.name()
                ));
            }
        }
        for name in ["onopenpage", "onclosepage"] {
            if let Some(r) = page.triggers.iter().find(|(n, _)| n == name).map(|(_, r)| *r) {
                let kind = match name {
                    "onopenpage" => crate::pages::TriggerKind::OpenPage,
                    _ => crate::pages::TriggerKind::Other,
                };
                self.pages.trigger_kinds.push((std::ptr::null(), kind));
                let called = self.call(schema.object, r, inst.clone(), Vec::new());
                self.pages.trigger_kinds.pop();
                called?;
            }
        }
        Ok(())
    }
}
