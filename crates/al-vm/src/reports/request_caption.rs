//! The caption of a report's request page as the report's own code sets and reads it (`RequestOptionsPage.Caption`).

use super::*;

impl Session {
    /// Whether the request page's own OnInit of the report `inst` is on the call stack.
    fn request_page_in_on_init(&self, inst: &Inst) -> bool {
        let object = inst.borrow().object as usize;
        let engine = self.engine.clone();
        let Some(page) = engine.program.objects[object].page.as_ref() else {
            return false;
        };
        let Some(on_init) = page.triggers.iter().find(|(n, _)| n == "oninit").map(|(_, r)| *r) else {
            return false;
        };
        self.frames
            .iter()
            .zip(&self.call_stack)
            .any(|(&o, f)| o == object && !f.event && f.routine == on_init)
    }

    /// `RequestOptionsPage.Update(SaveRecord)` in a request page trigger. With `false` the controls show
    /// the variables as they are now, which they do on every read here, so nothing is left to do
    /// (reports/TestReportRequestPageUpdateObjectIdContracts.al:RequestPageUpdate_RefreshesTheOtherControls:
    /// a handler sets Source to 21 and reads Derived 42 that the trigger set before it called Update(false)).
    /// `RequestOptionsPage.Caption := x` in the code of a report. Set while the request page's own OnOpenPage runs (in it
    /// or in a procedure it calls), it is the caption a request page handler reads
    /// (reports/AqTests.al:313-330, Caption_SetInOnOpenPage_TestRequestPageCaption). Set in its OnInit it is the handler's
    /// caption too, and OnOpenPage wins over it (reports/request-page/Y86RequestCaption.al). Set anywhere else (Base
    /// Application's Financial Report also sets it from OnValidate) the code reads it back and the handler does not (pinned
    /// for OnValidate by the same file; a report trigger or report extension is the assumption request-page-caption-set).
    ///
    /// From the code of a report (a procedure, also called on a variable never run, and a report trigger) the getter reads the
    /// request page's `Caption`, and after a set the caption set, also inside a later run (reports/runtime/
    /// Y40BareReportRun.al:344, Y40_BareRequestOptionsPage_Caption, corpus master ac3499d6). The caption of a report that also
    /// declares its own Caption, or of a request page without one, is not pinned.
    pub(crate) fn request_page_caption(&mut self, args: &[Value], inst: &Inst) -> Result<Value, Signal> {
        self.report.request_captions.retain(|(w, ..)| w.strong_count() > 0);
        let own = self
            .report
            .request_captions
            .iter()
            .position(|(w, ..)| w.upgrade().is_some_and(|i| Rc::ptr_eq(&i, inst)));
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
                // A report with neither Caption: the request page's caption is the report's name (reports/request-page/
                // Y86RequestCaption.al Y86_Caption_OnInitAndOnOpenPage `init_readback`).
                (None, None) if o.kind == ir::ObjKind::Report => Ok(Value::Text(o.name.clone().into())),
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
        // Set in the request page's OnInit it is the caption a request page handler reads (reports/request-page/
        // Y86RequestCaption.al); the page takes it when it opens (`request_page_takes_init_caption`).
        let in_init = self.request_page_in_on_init(inst);
        match own {
            Some(i) => {
                self.report.request_captions[i].1 = text.clone();
                if in_init {
                    self.report.request_captions[i].2 = Some(text);
                }
            }
            None => {
                let init = in_init.then(|| text.clone());
                self.report.request_captions.push((Rc::downgrade(inst), text, init));
            }
        }
        // A set while the request page is open is the page's own code (a control's OnValidate, an action, a procedure of the page):
        // the code reads it back and the handler does not (Y86RequestCaption.al Y86_Caption_OnValidateSets,
        // _ProcedureFromOnValidateSets). A set before the page opens by other code (a report trigger or procedure) is the assumption
        // request-page-caption-set, marked when the page opens (`request_page_opens_after_caption_set`).
        Ok(Value::Null)
    }

    /// A request page opens on `inst`: a caption its code set before is the assumption `request-page-caption-set`.
    pub(crate) fn request_page_opens_after_caption_set(&self, inst: &Inst) {
        if self
            .report
            .request_captions
            .iter()
            .any(|(w, code, init)| w.upgrade().is_some_and(|i| Rc::ptr_eq(&i, inst)) && init.as_ref() != Some(code))
        {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::RequestPageCaptionSet);
        }
    }

    /// The caption the request page's OnInit set, for a page that opened without OnOpenPage setting one: what a handler reads
    /// (Y86RequestCaption.al; OnOpenPage wins over OnInit).
    pub(crate) fn request_page_takes_init_caption(&self, inst: &Inst, st: &Rc<RefCell<al_builtins::value::PageState>>) {
        if st.borrow().caption.is_some() {
            return;
        }
        let init = self
            .report
            .request_captions
            .iter()
            .find(|(w, ..)| w.upgrade().is_some_and(|i| Rc::ptr_eq(&i, inst)))
            .and_then(|(_, _, init)| init.clone());
        st.borrow_mut().caption = init;
    }
}
