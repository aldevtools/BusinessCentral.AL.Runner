CloseActionOkNavigateAndParts => {
    id: "close-action-ok-navigate-and-parts",
    assumed: "TestPage.Close() of a NavigatePage, a CardPart or a ListPart that is not run modally, and the close of such a page left open at the end of the test, report CloseAction OK to OnQueryClosePage (a NavigatePage with a temporary or a real source table, after a footer action, opened with OpenEdit or OpenView). Measured on a SaaS sandbox (newest major) by corpus writer Y65; not on the pinned corpus yet.",
    tests: "No Microsoft test: al-runner issue 6670, a NavigatePage wizard whose OnQueryClosePage asks to confirm only on Action::OK.",
    probe: "Y65ClosePageTypes.al Y65_Close_Navigate, _AfterFooterAction, _OpenView, _NavigateRealTable, _CardPart, _ListPart, Y65_Open01/02/05/06 and the wizard Y65_Close_Wizard_Registered / Y65_Open11 on corpus branch al-runtime-rs-staging-y65 (eefaf01b).",
    awaiting: Awaiting {
        issues: &[6670],
        files: &["tests/al-language/testpageclose/lt-y65-close-page-types/Y65ClosePageTypes.al"],
        since: "2026-11-01",
    },
}
