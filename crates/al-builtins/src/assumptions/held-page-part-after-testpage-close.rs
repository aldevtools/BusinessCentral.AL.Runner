HeldPagePartAfterTestPageClose => {
    id: "held-page-part-after-testpage-close",
    assumed: "A page the test opened with TestPage.OpenEdit, OpenView or OpenNew and the test client closed (TestPage.Close, or the close at the end of the test), held in a variable (a SingleInstance codeunit kept the sender of the page's own event), reaches its part through CurrPage.<part>.Page without an error: the call goes to the page's own part instance, with its old row and its globals as they were, also from the next test of the codeunit and while another page of the same object is open. A part a Visible expression hid answers the same. A shown or created part of a page AL code ran and a handler closed stays refused.",
    tests: "No corpus test on the pinned corpus yet. al-runner issue 6924: the reporter's 34 tests of COD ERM CopyPurchSalesDoc Test pass on real BC (a factbox's OnAfterGetCurrRecord event subscriber calls UpdateDataCheckFactboxDCIFC on a held Sales Credit Memo page, which reads CurrPage.DataCheckFactboxDCIFC.Page). Measured on the three supported BC majors by the corpus probe on branch al-runtime-rs-staging-probe-train-1110b.",
    probe: "pages/parts/HeldSenderPartAccess.al HS01-HS13 (staging branch al-runtime-rs-staging-probe-train-1110b): the held host reached while open, after TestPage.Close in the same test, in the next test, from a factbox event while a second TestPage of the host opens; part hidden or shown; direct call and TryFunction.",
    awaiting: Awaiting {
        issues: &[6924],
        files: &["tests/al-language/pages/parts/HeldSenderPartAccess.al"],
        since: "2026-11-10",
    },
}
