TestEndClosesOpenTestPages => {
    id: "test-end-closes-open-testpages",
    assumed: "A TestPage local to the test method, of a page that declares OnQueryClosePage or OnClosePage, opened with OpenEdit or OpenView and still open when the test method returns, closes then as Close() closes it: OnQueryClosePage with the action OK, then OnClosePage, a Confirm in it handled by the test's ConfirmHandler, and a veto leaves the test passing; several such pages close in the order of their declaration. A global TestPage stays open. A page without close triggers, and one opened with OpenNew, is not closed at the end of the test (unchanged: what its close saves is not measured). Measured on real BC by corpus writer Y47 for one page.",
    tests: "No Microsoft test: al-runner issue 6587 (12 tests of the reporter's app).",
    probe: "A test that leaves two local TestPages open, and one opened with OpenNew: the order of their OnQueryClosePage and OnClosePage, and what OpenNew saves.",
    awaiting: Awaiting {
        issues: &[6587],
        files: &["tests/al-language/testpageclose/Y47CloseAtTestEnd.al"],
        since: "2026-11-01",
    },
}
