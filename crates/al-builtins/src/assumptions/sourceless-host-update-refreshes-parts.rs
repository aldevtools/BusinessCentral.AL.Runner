SourcelessHostUpdateRefreshesParts => {
    id: "sourceless-host-update-refreshes-parts",
    assumed: "CurrPage.Update on a page without a source table (a Worksheet of variables) refreshes its parts as on a page with one: each part reads its rows again and runs OnAfterGetRecord and OnAfterGetCurrRecord.",
    tests: "No Microsoft test: al-runner issue 6465, the reporter's run on real BC (a Worksheet field's OnValidate hands a value to its unlinked temporary ListPart and calls CurrPage.Update(false); the part then shows `(A)`). Measured by corpus writer Y15 (Y15WorksheetPartRefresh.al, on the SaaS sandbox, newest major).",
    probe: "Y15WorksheetPartRefresh.al on branch al-runtime-rs-staging-y15: the part's trigger log and value after SetValue, with one and two rows, OpenEdit, and without CurrPage.Update.",
    awaiting: Awaiting {
        issues: &[6465],
        files: &["tests/al-language/pages/lt-y15/Y15WorksheetPartRefresh.al"],
        since: "2026-10-31",
    },
}
