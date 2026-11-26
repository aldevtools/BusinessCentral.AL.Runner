MovedTableStubOfDependency => {
    id: "moved-table-stub-of-dependency",
    assumed: "A table of a workspace app with `ObsoleteState = Moved` and `MovedTo` naming a direct dependency loaded as a dependency package that declares a table with the same id and name and no ObsoleteState, whose `fields` or `keys` differ from that table's, compiles (no AL0264), and a name or id that finds both resolves to the dependency's table (al-sema `Workspace::moved_table_of_dependency`). Every test of a program with such a pair is flagged. The pair with the same fields and keys is pinned (corpus al-language-xapp-b/MvXbMovedStub.al, MvMovedTableTests.al) and not flagged. Other obsolete states, a missing MovedTo, a transitive dependency, another name, an obsolete dependency table, and a `MovedTo` app of the same run with other fields or keys keep AL0264. The stub's own triggers and procedures are not measured; the dependency's table is used.",
    tests: "No Microsoft test. An outside report: alc 18.1 compiles the pair (al-runner issue 7914); Microsoft's Base Application 28.4 declares tables 231, 308, 309, 310 and 1263 so against Business Foundation, with other field properties (DataClassification) than Business Foundation's tables.",
    probe: "Still to write: the Moved stub of XApp B gets other field properties than XApp A's live table (the Base Application shape); a main app test inserts a row by name and reads it with Get and RecordRef.Open by id. The same-fields pair is pinned by corpus PR 1363 (MvMovedTableTests.al).",
    awaiting: Awaiting {
        issues: &[7914],
        files: &["tests/al-language/table-objects/moved-tables/MvMovedTableTests.al::MovedStub_OtherFieldProperties"],
        since: "2026-11-12",
    },
}
