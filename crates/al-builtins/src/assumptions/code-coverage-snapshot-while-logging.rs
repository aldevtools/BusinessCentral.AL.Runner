CodeCoverageSnapshotWhileLogging => {
    id: "code-coverage-snapshot-while-logging",
    assumed: "While CodeCoverageLog(true) is on, the Code Coverage tables hold the snapshot that CodeCoverageRefresh wrote and do not grow with later hits (a read before any refresh is empty, pinned by platform/lt-code-coverage-refresh/CodeCoverageRefreshLogged.al).",
    tests: "Tests that read the Code Coverage table after a refresh while logging is still on: none known in the buckets.",
    probe: "CodeCoverageLog(true, false), run code, CodeCoverageRefresh(), count the rows, run more code, count again (still logging), then CodeCoverageRefresh() and count. Asked in corpus #647.",
}
