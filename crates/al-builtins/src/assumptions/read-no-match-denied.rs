ReadNoMatchDenied => {
    id: "read-no-match-denied",
    assumed: "In a session whose permissions a test lowered (ADR-062), a read whose filter matches no row of a table that holds rows and that the sets do not grant: FindSet(), FindLast(), Find('-') and Count() raise BC's `Read` denial (`Sorry, the current permissions prevented the action. (TableData <id> <name> Read: <app>)`, DB:ClientReadDenied), and IsEmpty() raises nothing and answers true, on every BC major, in the one shape corpus PR 1403 measured: a Record variable (not a RecordRef) in a procedure of the test codeunit without attributes, with only its procedures below (the test method with the one attribute [Test]), while no record method runs; a table per company with one key, its primary key one Integer field, its first, of FieldClass Normal, holding two rows; exactly one filter in filter group 0 that is a range of one value, the primary key to 100 (the gate reads the filter, not the call that set it); the methods without arguments (Find with the one argument '-'); the assigned sets are exactly what Permissions Mock.Set(X) assigns, All Objects and then one set X, declared by the app that declares the table, and give nothing on the table; the test codeunit has TestPermissions = Disabled; its object grants nothing on the table. Not checked: the record's key, sort order and lock, and whether the return value is used. Every other no-match read but FindFirst is refused.",
    tests: "No Microsoft test. Corpus PR 1403 observed the cells on each major its CI runs (corpus CI run 43637000726).",
    probe: "Corpus PR 1403 (open; permissions/testpermissions/Y19PermExtAndCompose.al, test PC_C_NoMatchReadsAndDelete, ops 42-46). Wanted next: other tables, row counts, filters and filter values, FindSet with arguments, Next, a RecordRef, code outside the test codeunit, a set that grants Indirect.",
    awaiting: Awaiting {
        issues: &[7854],
        files: &["tests/al-language/permissions/testpermissions/Y19PermExtAndCompose.al"],
        since: "2026-11-15",
    },
}
