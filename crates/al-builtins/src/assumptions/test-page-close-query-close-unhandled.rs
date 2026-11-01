TestPageCloseQueryCloseUnhandled => {
    id: "test-page-close-query-close-unhandled",
    assumed: "Under Microsoft's Test Runner codeunit: a TestPage left open at the end of the test whose OnQueryClosePage raises an error (a Confirm with no ConfirmHandler, or an AL Error) does not fail the test, and OnClosePage does not run; TestPage.Close() whose OnQueryClosePage asks a Confirm with no ConfirmHandler raises 'Unhandled UI: Confirm <question>' with the code TestWrapped, and the TestPage is closed after it. Measured by the corpus CI on every major; the corpus file still asserts the dev-endpoint runner's values and is not on the pinned corpus yet.",
    tests: "No Microsoft test: al-runner issue 6673, a Card whose OnQueryClosePage asks a Confirm, left open by a test with no ConfirmHandler.",
    probe: "Y65ClosePageTypes.al Y65_Open07/08/10/12/13 (_Read) and Y65_Close_Card_Ask_NoHandler, _Navigate_Ask_NoHandler, _AskDefaultFalse_NoHandler, _AskNoDefault_NoHandler on corpus branch al-runtime-rs-staging-y65 (eefaf01b), asserted with the Test Runner codeunit's values; plus a read of the TestPage after the failed Close() (open or not).",
    awaiting: Awaiting {
        issues: &[6673],
        files: &["tests/al-language/testpageclose/lt-y65-close-page-types/Y65ClosePageTypes.al"],
        since: "2026-11-01",
    },
}
