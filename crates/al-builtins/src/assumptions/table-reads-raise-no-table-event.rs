TableReadsRaiseNoTableEvent => {
    id: "table-reads-raise-no-table-event",
    assumed: "Install code that could not run to its end leaves unknown the tables it and the event subscribers it reaches touch (al-testrun install.rs touched). Where a routine of that reach is a codeunit without TableNo and only reads a table in a measured shape, the subscribers of that table's ten table events (On(Before|After)(Insert|Modify|Delete|Rename|Validate)Event) are left out of the reach (al-testrun runtime/table_reads.rs reads_only): the read raises none of them. Measured shapes: every slot of the table in the routine is a record that is not temporary, and each use is a field read or one of these calls: on an app table (id below 2000000000, TableType absent or Normal, no table trigger, no table extension) IsEmpty, Get, Find, FindFirst, FindLast, FindSet with at most one argument, Next, Count, CountApprox, SetRange, SetFilter, Reset, SetCurrentKey, SetAscending, Init, SetLoadFields, LockTable, GetFilters and HasFilter; on the platform table Company IsEmpty, Count, FindFirst, Get, FindSet, Next, SetRange and Reset. The tables only the left-out subscribers add are held apart (al-vm SkippedInstall::assumed). A test method or the code that runs when the test session opens the company that reads or writes one runs, marked; the test codeunit's OnRun and later install code that read or write one are refused, as before. Every other shape (another table type, a temporary record, a field assignment, Validate, Copy, TransferFields, any write, a var or value argument, a codeunit with TableNo, install code in another declared company) keeps the full reach.",
    tests: "No Microsoft test. The outside reporter's repro of al-runner issue 8238 (an install routine stopped by an outbound HTTP request, with a procedure that only calls Company.IsEmpty()).",
    probe: "Written: corpus PR 1431 (open), measured on BC 27.5 and 28.5; its corpus CI passed these tests on 27.5, 28.5 and 29.0: tests/al-language/record/table-events-reads/TiReadEventTests.al (:47-92, :95-113 and :116-169 reads raise no table event; :204-258 writes raise their pairs) and TiCompanyReadEventTests.al (:26-70 Company reads raise no table event; :74-111 Validate and writes to a temporary Company raise their pairs).",
    awaiting: Awaiting {
        issues: &[8238],
        files: &[
            "tests/al-language/record/table-events-reads/TiReadEventTests.al",
            "tests/al-language/record/table-events-reads/TiCompanyReadEventTests.al",
        ],
        since: "2026-11-18",
    },
}
