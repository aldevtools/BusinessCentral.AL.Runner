ActionRefreshHoldsReadRows => {
    id: "action-refresh-holds-read-rows",
    assumed: "After an action changes the view of a list with OnAfterGetRecord and no OnFindRecord, and the refresh reads more rows than a window holds (the way back to the first row and a window from the current row), the client holds all the rows it read: First lands on the first row and Next walks on from them. (Pinned for a view that fits a window by pages/lt-y5/Y5GoToKeyAfterActionKeyAndTriggers.al:273-342.)",
    tests: "No Microsoft test: al-runner issue 6469, the reporter's run on real BC (25 bins, the page on A02, an action removes the Room filter: First gives A01, Next walks 25 rows; 21 rows the same).",
    probe: "The reporter's page with 21 and 25 rows: the OnAfterGetRecord log of the action, of First and of each Next, and Last.",
    awaiting: Awaiting {
        issues: &[6469],
        files: &["tests/al-language/pages/lt-y15/Y15ActionWidensView.al"],
        since: "2026-10-31",
    },
}
