BgTaskInsertQueue => {
    id: "bg-task-insert-queue",
    assumed: "A page background task's worker does not see the rows the test session's insert queue holds (statement-form Inserts of AL code into a table without a Blob or AutoIncrement field, as the queue that sends at its fifth row, at a Commit and at the next access to the table models them): every record method of the worker on that table, and a FlowField of the worker over it, runs as if those rows were not there, and the worker's methods send nothing of the queue. A FlowField of the test session over the table lets the rows through. Refused: a worker's read of a table whose Insert error is deferred, whose queue holds rows under another company value, or whose queued keys an OnAfterInsert subscriber may have changed, a worker's Query, CountApprox, GetBySystemId or TableRelation check of a table whose queue holds rows, and a worker's read of a table after the test session read it by one of those since its last queued insert.",
    tests: "No Microsoft test: al-runner issue 8491 (the reporter's repro, green on real BC 3/3).",
    probe: "Corpus pull request 1459 (merged, not pinned yet), tests/al-language/pages/background-tasks/BgTaskInsertQueue.al:234-481, 16 tests, green on a local Business Central 29.1 container.",
    awaiting: Awaiting {
        issues: &[8491],
        files: &["tests/al-language/pages/background-tasks/BgTaskInsertQueue.al"],
        since: "2026-11-19",
    },
}
