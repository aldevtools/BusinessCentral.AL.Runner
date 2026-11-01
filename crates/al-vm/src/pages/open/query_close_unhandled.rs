//! A Confirm with no ConfirmHandler in OnQueryClosePage while `TestPage.Close()` closes the page, under Microsoft's Test
//! Runner codeunit (the default runner mode): the Close raises `Unhandled UI: Confirm <question>` with the code TestWrapped,
//! not the message an AL error of the trigger is shown as (corpus CI run 41345957702 of corpus branch
//! al-runtime-rs-staging-y65, every major it runs: testpageclose/lt-y65-close-page-types/Y65ClosePageTypes.al:690
//! Y65_Close_Card_Ask_NoHandler, :704 Y65_Close_Navigate_Ask_NoHandler, :945 _AskDefaultFalse_NoHandler, :961
//! _AskNoDefault_NoHandler `close = error:Unhandled UI: Confirm Close the card page?|TestWrapped`; al-runner issue 6673).
//! Under `--dev-endpoint-ui` the Confirm answers its default and raises nothing. Not on the pinned corpus yet: assumption
//! test-page-close-query-close-unhandled. That the TestPage is closed after the error, as after an error of the trigger
//! shown as a message, is part of the assumption.

use super::*;

impl Session {
    /// The error `TestPage.Close()` raises for an error of OnQueryClosePage, when it is an unhandled Confirm.
    pub(super) fn unhandled_confirm_in_query_close(&self, e: &al_builtins::native::AlError) -> Option<Signal> {
        if e.code != "MissingUIHandler" || !e.message.starts_with("Unhandled UI: Confirm ") {
            return None;
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::TestPageCloseQueryCloseUnhandled);
        Some(error_code(e.message.clone(), "TestWrapped"))
    }
}
