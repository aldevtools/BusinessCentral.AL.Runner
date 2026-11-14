BulkWriteNoMatchInDeleteTrigger => {
    id: "bulk-write-no-match-in-delete-trigger",
    assumed: "In a session whose permissions a test lowered (ADR-062), a ModifyAll without RunTrigger whose filter matches no row raises no permission error although the sets grant only R on the table and the table holds one row, in the reporter's shape only: the ModifyAll runs in the OnDelete trigger of another table, whose Permissions property gives no Modify on the target, called directly from procedures of a test codeunit (Delete(true)); the record is a Record variable, not marked-only, with exactly one filter, a SetRange to one value in filter group 0 on a Code field outside the primary key; the field written is that filter field and the value written is a blank text; the target table runs no trigger (no table or field trigger, no table extension, no subscriber); the assigned sets give R and nothing else on it. The pinned denial (record/Y19PermWrites.al:294-307: a filter on the primary key, another field written, two rows, a local procedure of the test codeunit) is unchanged. Every other no-match ModifyAll or DeleteAll on a table with rows is refused.",
    tests: "No Microsoft test. An outside report: al-runner issue 8068, the reporter's own repro on real BC (tables 50300 Repro Parent, 50301 Repro Other, test 50400 DeleteParentWhoseTriggerCallsModifyAllOnAnEmptyFilter) passes.",
    probe: "Written, corpus PR not opened yet (al-runner issue 8068): one cell per difference between the reporter's shape and Y19_P2: caller (test codeunit procedure, OnDelete trigger of another table, a codeunit of the app), filter on the primary key or on another field, field written outside the filter or the filter field, value written blank or not, one or two rows in the table, sets R or RI, ModifyAll with and without RunTrigger and DeleteAll. Observe the error text and code or none.",
    awaiting: Awaiting {
        issues: &[8068],
        files: &["tests/al-language/record/Y19PermNoMatchShapes.al"],
        since: "2026-11-14",
    },
}
