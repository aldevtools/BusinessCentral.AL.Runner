//! The request page triggers that run while its page is not shown: OnInit when a procedure first uses the report and in a
//! run that does not show the page, and for an Execute with parameters OnOpenPage and OnClosePage.

use super::*;

/// Refuses the OnInit a reportextension adds to the request page `page` of report `report`: when it runs there, and in which
/// order with the report's own, is not pinned. `when` says what runs it.
fn refuse_ext_on_init(page: &ir::Page, report: &str, when: &str) -> Result<(), Signal> {
    if page.ext_triggers.iter().any(|(n, _, _)| n == "oninit") {
        return unsupported(format!(
            "the OnInit a reportextension adds to the request page of report \"{report}\", {when} (not pinned)"
        ));
    }
    Ok(())
}

impl Session {
    /// The request page's OnInit of a report that a procedure uses first (see `first_use`): it runs right after OnInitReport and
    /// before the procedure's body, and the run does not run it again, with or without the request page
    /// (reports/request-page/RequestPageOnInitFirstUse.al:104-185). Refused: an OnInit a reportextension adds, also when no
    /// run follows the call.
    pub(super) fn request_page_init_at_first_use(&mut self, inst: &Inst) -> Result<(), Signal> {
        let object = inst.borrow().object as usize;
        let engine = self.engine.clone();
        let o = &engine.program.objects[object];
        if o.report.is_none() {
            return Ok(());
        }
        let Some(page) = o.page.as_ref() else {
            return Ok(());
        };
        refuse_ext_on_init(page, &o.name, "when a procedure first uses the report")?;
        let Some(r) = page.triggers.iter().find(|(n, _)| n == "oninit").map(|(_, r)| *r) else {
            return Ok(());
        };
        self.report.rp_init_done.push(Rc::downgrade(inst));
        self.call(object, r, inst.clone(), Vec::new()).map(|_| ())
    }

    /// Whether the request page's OnInit of `inst` already ran at a procedure's first use.
    pub(crate) fn request_page_init_done(&self, inst: &Inst) -> bool {
        self.report
            .rp_init_done
            .iter()
            .any(|w| w.upgrade().is_some_and(|i| Rc::ptr_eq(&i, inst)))
    }

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
        refuse_ext_on_init(page, schema.name(), "which runs without its page")?;
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
    /// the page (`report_body`). A SaveAs with parameters runs OnOpenPage too (reports/runtime/ExecuteParametersWithRecordRef.al:
    /// 556-617 `request page opened;pre opt=8;..`, static and variable SaveAs, BC 27, 28 and 29); its OnClosePage there is not
    /// pinned and refused. Refused as well: the triggers a reportextension adds to the page (not pinned).
    pub(super) fn request_page_open_close_unshown(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        call: ParamsCall,
    ) -> Result<(), Signal> {
        let engine = self.engine.clone();
        let Some(page) = engine.program.objects[schema.object].page.as_ref() else {
            return Ok(());
        };
        let what = if call == ParamsCall::SaveAs {
            "a SaveAs"
        } else {
            "an Execute"
        };
        for name in ["onopenpage", "onclosepage"] {
            if page.ext_triggers.iter().any(|(n, _, _)| n == name) {
                return unsupported(format!(
                    "the {name} trigger a reportextension adds to the request page of report \"{}\", for {what} with parameters (not pinned)",
                    schema.name()
                ));
            }
        }
        if call == ParamsCall::SaveAs && page.triggers.iter().any(|(n, _)| n == "onclosepage") {
            return unsupported(format!(
                "the OnClosePage trigger of the request page of report \"{}\", for a SaveAs with parameters (not pinned)",
                schema.name()
            ));
        }
        for name in ["onopenpage", "onclosepage"] {
            if let Some(r) = page.triggers.iter().find(|(n, _)| n == name).map(|(_, r)| *r) {
                let kind = match name {
                    "onopenpage" => crate::pages::TriggerKind::OpenPage,
                    _ => crate::pages::TriggerKind::Other,
                };
                self.pages.trigger_kinds.push((std::ptr::null(), kind, None));
                let called = self.call(schema.object, r, inst.clone(), Vec::new());
                self.pages.trigger_kinds.pop();
                called?;
            }
        }
        Ok(())
    }
}
