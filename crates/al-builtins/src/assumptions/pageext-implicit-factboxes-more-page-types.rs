PageExtImplicitFactBoxesMorePageTypes => {
    id: "pageext-implicit-factboxes-more-page-types",
    assumed: "A page extension may add parts to area(FactBoxes) of a Worksheet, a Document or a ListPlus page that declares none, as on a List: the page opens, the extension's triggers run and a part added with Visible = false is not on the test page (TestPartNotFound).",
    tests: "No Microsoft test: al-runner issue 6490, an ISV app adds a document archive factbox to a Worksheet page (195 tests refused). Measured on BC by corpus writer Y20 (Y20ImplicitFactBoxes.al, SaaS sandbox, newest major); part pages do not compile (must-not-compile 2351-2354).",
    probe: "Y20ImplicitFactBoxes.al on branch al-runtime-rs-staging-y20: a Worksheet, a Document and a ListPlus page with an added linked part and an added hidden part loaded in OnAfterGetCurrRecord.",
    awaiting: Awaiting {
        issues: &[6490],
        files: &["tests/al-language/pageext/lt-bc-implicit-areas/Y20ImplicitFactBoxes.al"],
        since: "2026-10-31",
    },
}
