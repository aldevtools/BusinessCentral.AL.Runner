//! How the last close of a page ended (`CloseOutcome`, al-runner issue 7685): the answer of OnQueryClosePage, a save that
//! failed at `TestPage.Close()`, and what a `TestPage.Close()` inside the handler of a modal run left for `RunModal`, with
//! transitions named after what happened and questions named after what the caller needs. The corpus citations are at the
//! callers in `pages/open/close.rs`, `pages/rows.rs` (`answer_page`) and `pages/close_save_failed.rs`. Entry points:
//! `CloseOutcome`, field `PageState::close`.
#![deny(clippy::wildcard_enum_match_arm)]

/// What the last close of a page left. Every field starts false (`Default`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CloseOutcome {
    /// The last close was refused because OnQueryClosePage raised an error (shown as a message).
    error: bool,
    /// The last close was refused because the page's own OnQueryClosePage (or a part's, of a modal page) answered false.
    vetoed: bool,
    /// A `TestPage.Close()` of this Card failed on the save of its row.
    save_failed: bool,
    /// A `TestPage.Close()` in the modal page's handler was refused by OnQueryClosePage and closed the TestPage all the same.
    test_vetoed_in_handler: bool,
    /// A `TestPage.Close()` in the modal page's handler closed the page (no veto).
    test_closed_in_handler: bool,
}

impl CloseOutcome {
    /// A close starts: an earlier veto no longer counts; an earlier error stays until the query is asked again.
    pub fn close_started(&mut self) {
        self.vetoed = false;
    }

    /// A `TestPage.Close()` starts: an earlier error of OnQueryClosePage no longer counts.
    pub fn test_close_started(&mut self) {
        self.error = false;
    }

    /// OnQueryClosePage raised an error, shown as a message (or a Confirm no handler took): the close is refused. Only `error`
    /// changes; `vetoed` keeps what `close_started` or the last answer left.
    pub fn refused_by_error(&mut self) {
        self.error = true;
    }

    /// OnQueryClosePage of the page answered: `vetoed` when it answered false.
    pub fn query_answered(&mut self, vetoed: bool) {
        self.error = false;
        self.vetoed = vetoed;
    }

    /// A part's OnQueryClosePage refused the close of a modal page.
    pub fn vetoed_by_part(&mut self) {
        self.query_answered(true);
    }

    /// The save of the row failed at `TestPage.Close()`.
    pub fn save_failed_at_close(&mut self) {
        self.save_failed = true;
    }

    /// The failed save no longer holds the page.
    pub fn save_failure_cleared(&mut self) {
        self.save_failed = false;
    }

    /// A `TestPage.Close()` in the handler of a modal run was refused by OnQueryClosePage and closed the TestPage.
    pub fn test_close_vetoed_in_handler(&mut self) {
        self.test_vetoed_in_handler = true;
    }

    /// A `TestPage.Close()` in the handler of a modal run closed the page.
    pub fn test_close_closed_in_handler(&mut self) {
        self.test_closed_in_handler = true;
    }

    /// The page is handed to its handler: nothing a TestPage close in an earlier handler did counts.
    pub fn handler_started(&mut self) {
        self.test_vetoed_in_handler = false;
        self.test_closed_in_handler = false;
    }

    /// Whether a `TestPage.Close()` in the handler was vetoed; reading it clears it.
    pub fn take_test_vetoed_in_handler(&mut self) -> bool {
        std::mem::take(&mut self.test_vetoed_in_handler)
    }

    /// Whether a `TestPage.Close()` in the handler closed the page; reading it clears it.
    pub fn take_test_closed_in_handler(&mut self) -> bool {
        std::mem::take(&mut self.test_closed_in_handler)
    }

    /// The last close was refused by an error of OnQueryClosePage.
    pub fn error(&self) -> bool {
        self.error
    }

    /// The last close was refused by a false answer of OnQueryClosePage.
    pub fn vetoed(&self) -> bool {
        self.vetoed
    }

    /// The save of the row failed at `TestPage.Close()`.
    pub fn save_failed(&self) -> bool {
        self.save_failed
    }

    /// The last close was refused, by a false answer or an error, or a `TestPage.Close()` in a handler left an outcome.
    pub fn refused_or_handled(&self) -> bool {
        self.vetoed || self.error || self.test_vetoed_in_handler || self.test_closed_in_handler
    }
}

#[cfg(test)]
mod tests {
    use super::CloseOutcome;

    #[test]
    fn a_new_outcome_answers_nothing() {
        let c = CloseOutcome::default();
        assert!(!c.error() && !c.vetoed() && !c.save_failed() && !c.refused_or_handled());
    }

    #[test]
    fn a_close_start_clears_the_veto_and_keeps_the_error() {
        let mut c = CloseOutcome::default();
        c.query_answered(true);
        c.close_started();
        assert!(!c.vetoed());
        c.refused_by_error();
        c.close_started();
        assert!(c.error());
    }

    #[test]
    fn a_test_close_start_clears_the_error_and_keeps_the_veto() {
        let mut c = CloseOutcome::default();
        c.query_answered(true);
        c.refused_by_error();
        c.test_close_started();
        assert!(!c.error() && c.vetoed());
    }

    #[test]
    fn an_error_keeps_a_veto_set_in_between() {
        let mut c = CloseOutcome::default();
        c.vetoed_by_part();
        c.refused_by_error();
        assert!(c.error() && c.vetoed() && c.refused_or_handled());
    }

    #[test]
    fn an_answer_clears_the_error_and_sets_the_veto_both_ways() {
        let mut c = CloseOutcome::default();
        c.refused_by_error();
        c.query_answered(true);
        assert!(!c.error() && c.vetoed());
        c.query_answered(false);
        assert!(!c.error() && !c.vetoed() && !c.refused_or_handled());
    }

    #[test]
    fn a_failed_save_holds_until_cleared() {
        let mut c = CloseOutcome::default();
        c.save_failed_at_close();
        assert!(c.save_failed() && !c.refused_or_handled());
        c.save_failure_cleared();
        assert!(!c.save_failed());
    }

    #[test]
    fn a_handler_outcome_is_read_once_and_cleared_by_the_next_handler() {
        let mut c = CloseOutcome::default();
        c.test_close_vetoed_in_handler();
        assert!(c.refused_or_handled());
        assert!(c.take_test_vetoed_in_handler());
        assert!(!c.take_test_vetoed_in_handler());
        c.test_close_closed_in_handler();
        assert!(c.refused_or_handled());
        assert!(c.take_test_closed_in_handler());
        assert!(!c.take_test_closed_in_handler());
        c.test_close_vetoed_in_handler();
        c.test_close_closed_in_handler();
        c.handler_started();
        assert!(!c.refused_or_handled());
    }
}
