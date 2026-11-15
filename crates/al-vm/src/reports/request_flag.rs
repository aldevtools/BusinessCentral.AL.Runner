//! The hidden flag slot `Report.UseRequestPage(x)` sets on a report instance (`ir::Report::request_flag_slot`), and the
//! value a static `Report.Run(Id)` or `Report.RunModal(Id)` gives it.
use crate::clientless::RequestWindow;

/// Values of a report's request-page flag slot (`UseRequestPage`); any other value means "never set": the report's own
/// property decides.
pub(super) const REQUEST_PAGE_ON: i32 = 1;
pub(super) const REQUEST_PAGE_OFF: i32 = 2;
/// The page is hidden by a static `RunModal(id, false)`, or skipped by the test session under `--no-client`: the getter is
/// pinned in OnPreReport only for the first, and refused elsewhere.
pub(super) const REQUEST_PAGE_OFF_UNPINNED_GETTER: i32 = 3;
/// After an `Execute` of a variable whose `UseRequestPage` was set: what the getter answers is not pinned
/// (reports/PnH2UseRp.al:389 pins only the variable never set), so it is refused.
pub(super) const REQUEST_PAGE_UNPINNED_GETTER: i32 = 4;

/// The RequestWindow a static run goes on with, and the flag it sets (`None`: the slot stays as it is), from what a session
/// without a client made of the call (`Session::clientless_report`). An explicit RequestWindow sets the flag; `RunModal(id,
/// false)` hides the page with the getter pinned in OnPreReport only. A request page the test session under `--no-client`
/// skips runs as `false`, and the getter is not measured there.
pub(super) fn static_run(window: RequestWindow, run_modal: bool) -> (Option<bool>, Option<i32>) {
    match window {
        RequestWindow::As(None) => (None, None),
        RequestWindow::As(Some(true)) => (Some(true), Some(REQUEST_PAGE_ON)),
        RequestWindow::As(Some(false)) if run_modal => (Some(false), Some(REQUEST_PAGE_OFF_UNPINNED_GETTER)),
        RequestWindow::As(Some(false)) => (Some(false), Some(REQUEST_PAGE_OFF)),
        RequestWindow::Skipped => (Some(false), Some(REQUEST_PAGE_OFF_UNPINNED_GETTER)),
    }
}
