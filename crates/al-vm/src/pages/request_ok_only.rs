//! The request page that al-sema `lower/ext_request_page.rs` admits with triggers on the request pages of two run-changing
//! reportextensions of independent apps (`ir::Report::request_page_ok_only`). The corpus measured that page for one call
//! only: the static `Report.RunRequestPage(id)` with no parameters and no SubstituteReport subscriber, whose
//! `[RequestPageHandler]` calls `OK().Invoke()` and nothing else, on a report no event subscriber listens to
//! (reports/extensions/IndependentAppsReportExtensions.al:73-81 `ObserveRequestPage`, :103-111, :133-141, handlers :143-153).
//! Every other call that opens the page, every other handler step, every other close and every error raised while the page
//! is open is refused here, each at its own step.

use super::request::RequestClosed;
use super::*;
use al_builtins::page_op::PageOp;

/// What every refusal of this module ends with.
const PINNED: &str = "pinned only: Report.RunRequestPage(id) closed by OK";

impl Session {
    /// Whether report `object`'s request page is the one `ir::Report::request_page_ok_only` marks.
    fn request_ok_only(&self, object: usize) -> bool {
        self.engine.program.objects[object]
            .report
            .as_ref()
            .is_some_and(|r| r.request_page_ok_only)
    }

    fn refuse_ok_only<T>(&self, object: usize, what: &str) -> Result<T, Signal> {
        let name = &self.engine.program.objects[object].name;
        unsupported(format!(
            "{what} on the request page of report \"{name}\" that two reportextensions of independent apps extend with triggers ({PINNED})"
        ))
    }

    /// The `RunRequestPage` builtin on report `object`: `pinned_call` says it is the static form with the report id as its only
    /// argument, and no SubstituteReport subscriber swapped the report (IndependentAppsReportExtensions.al:78).
    pub(crate) fn request_ok_only_call(&self, object: usize, pinned_call: bool) -> Result<(), Signal> {
        if self.request_ok_only(object) && !pinned_call {
            return self.refuse_ok_only(
                object,
                "RunRequestPage of a report variable, with parameters or substituted",
            );
        }
        Ok(())
    }

    /// Opens report `object`'s request page through `open` (`want_parameters`: `RunRequestPage` opened it, not a run) and
    /// refuses on the marked page what the pin does not have: a run that shows the page, an event subscriber of the report,
    /// a close other than OK and an error raised while the page is open (its triggers, the procedures they call, the handler).
    pub(super) fn request_ok_only_open(
        &mut self,
        object: usize,
        want_parameters: bool,
        open: impl FnOnce(&mut Self) -> Result<(RequestClosed, Option<String>), Signal>,
    ) -> Result<(RequestClosed, Option<String>), Signal> {
        if !self.request_ok_only(object) {
            return open(self);
        }
        if !want_parameters {
            return self.refuse_ok_only(object, "a report run that shows the page");
        }
        if self.engine.events.any_subscriber_of(object) {
            return self.refuse_ok_only(object, "an event subscriber of the report");
        }
        match open(self) {
            Ok((RequestClosed::Ok, parameters)) => Ok((RequestClosed::Ok, parameters)),
            Ok((closed, _)) => self.refuse_ok_only(object, &format!("a close by {closed:?}")),
            Err(Signal::Error(_)) => self.refuse_ok_only(object, "an error raised while the page is open"),
            Err(other) => Err(other),
        }
    }

    /// A step of the test client on page `st` (`method`, on control `target` when the receiver is a control): on the marked
    /// page only `OK()` and `Invoke` of the OK action pass.
    pub(super) fn request_ok_only_step(
        &self,
        st: &St,
        method: PageOp,
        target: Option<&ControlTarget>,
    ) -> Result<(), Signal> {
        let object = st.borrow().object as usize;
        if !self.request_ok_only(object) {
            return Ok(());
        }
        let ok = match target {
            None => method == PageOp::TestPageOK,
            Some(ControlTarget::BuiltIn(name)) => method == PageOp::TestActionInvoke && name == "OK",
            Some(_) => false,
        };
        if ok {
            return Ok(());
        }
        self.refuse_ok_only(object, &format!("{method} in the handler"))
    }

    /// `TP.Ctl` on page `st`: no control of the marked page is touched in the pin.
    pub(super) fn request_ok_only_control(&self, st: &St) -> Result<(), Signal> {
        let object = st.borrow().object as usize;
        if self.request_ok_only(object) {
            return self.refuse_ok_only(object, "a control read or written in the handler");
        }
        Ok(())
    }
}
