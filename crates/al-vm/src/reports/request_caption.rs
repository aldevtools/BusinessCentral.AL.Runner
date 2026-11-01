//! The caption of a report's request page as the report's own code sets and reads it (`RequestOptionsPage.Caption`).

use super::*;

impl Session {
    /// `RequestOptionsPage.Update(SaveRecord)` in a request page trigger. With `false` the controls show
    /// the variables as they are now, which they do on every read here, so nothing is left to do
    /// (handlers/TestReportRequestPageUpdateObjectIdContracts.al:RequestPageUpdate_RefreshesTheOtherControls:
    /// a handler sets Source to 21 and reads Derived 42 that the trigger set before it called Update(false)).
    /// `RequestOptionsPage.Caption := x` in the code of a report. Set while the request page's own OnOpenPage runs (in it
    /// or in a procedure it calls), it is the caption a request page handler reads
    /// (report/lt-aq/AqTests.al:313-330, Caption_SetInOnOpenPage_TestRequestPageCaption). Set anywhere else (Base
    /// Application's Financial Report also sets it from OnValidate) it is dropped: whether a handler reads it back is
    /// not pinned (assumption request-page-caption-set).
    ///
    /// From the code of a report (a procedure, also called on a variable never run, and a report trigger) the getter reads the
    /// request page's `Caption`, and after a set the caption set, also inside a later run (report/lt-y40-bare-report-run/
    /// Y40BareReportRun.al:344, Y40_BareRequestOptionsPage_Caption, corpus master ac3499d6). The caption of a report that also
    /// declares its own Caption, or of a request page without one, is not pinned.
    pub(crate) fn request_page_caption(&mut self, args: &[Value], inst: &Inst) -> Result<Value, Signal> {
        self.report.request_captions.retain(|(w, _)| w.strong_count() > 0);
        let own = self
            .report
            .request_captions
            .iter()
            .position(|(w, _)| w.upgrade().is_some_and(|i| Rc::ptr_eq(&i, inst)));
        let [arg @ (Value::Text(_) | Value::Code(_))] = args else {
            if !args.is_empty() {
                return unsupported("RequestPage.Caption set with something that is not text");
            }
            if let Some(st) = self.request_page_in_on_open_page(inst) {
                if let Some(c) = st.borrow().caption.clone() {
                    return Ok(Value::Text(c.into()));
                }
            }
            if let Some(i) = own {
                return Ok(Value::Text(self.report.request_captions[i].1.clone().into()));
            }
            let engine = self.engine.clone();
            let o = &engine.program.objects[inst.borrow().object as usize];
            let prop = |name: &str| {
                o.properties
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(name))
                    .map(|(_, v)| v)
            };
            return match (prop(ir::REQUEST_PAGE_CAPTION_PROPERTY), prop("Caption")) {
                (Some(rp), Some(caption))
                    if rp == caption && o.kind == ir::ObjKind::Report && crate::pages::is_text_literal(rp) =>
                {
                    Ok(Value::Text(crate::pages::unquote(rp).into()))
                }
                _ => unsupported(
                    "RequestPage.Caption read in a report without a request page Caption, or that declares its own Caption (not pinned)",
                ),
            };
        };
        let text = al_builtins::native::text_of(arg)
            .map(|t| t.to_string())
            .unwrap_or_default();
        if let Some(st) = self.request_page_in_on_open_page(inst) {
            st.borrow_mut().caption = Some(text);
            return Ok(Value::Null);
        }
        match own {
            Some(i) => self.report.request_captions[i].1 = text,
            None => self.report.request_captions.push((Rc::downgrade(inst), text)),
        }
        // What a request page handler then reads is the assumption: marked here once the page was made, else when it opens.
        if self.request_page_open(inst) {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::RequestPageCaptionSet);
        }
        Ok(Value::Null)
    }

    /// Whether the request page of the report `inst` is an instance of was made (it opens, is open or was open): a caption
    /// set then may be what its handler reads.
    pub(super) fn request_page_open(&self, inst: &Inst) -> bool {
        let object = inst.borrow().object as usize;
        let Some(page) = self.engine.program.objects[object].page.as_ref() else {
            return false;
        };
        matches!(inst.borrow().globals.get(page.curr_page_slot), Some(Value::TestPage(_)))
    }

    /// A request page opens on `inst`: a caption its code set before is the assumption `request-page-caption-set`.
    pub(crate) fn request_page_opens_after_caption_set(&self, inst: &Inst) {
        if self
            .report
            .request_captions
            .iter()
            .any(|(w, _)| w.upgrade().is_some_and(|i| Rc::ptr_eq(&i, inst)))
        {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::RequestPageCaptionSet);
        }
    }
}
