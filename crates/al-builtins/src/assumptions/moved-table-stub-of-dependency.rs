MovedTableStubOfDependency => {
    id: "moved-table-stub-of-dependency",
    assumed: "A table of a workspace app with `ObsoleteState = Moved` and `MovedTo` naming a direct dependency loaded as a dependency package that declares a table with the same id and name and no ObsoleteState compiles (no AL0264), and a name or id that finds both resolves to the dependency's table (al-sema `Workspace::moved_table_of_dependency`). Every test of a program with such a pair is flagged. Other obsolete states, a missing MovedTo, a transitive dependency, a `MovedTo` app that is a workspace app of the same run, another name and an obsolete dependency table keep AL0264. The stub's own triggers and procedures are not measured; the dependency's table is used.",
    tests: "No Microsoft test. An outside report: alc 18.1 compiles the pair (al-runner issue 7914); Microsoft's Base Application 28.4 declares tables 231, 308, 309, 310 and 1263 so against Business Foundation.",
    probe: "XApp A gets a table and XApp B its Moved stub (MovedTo XApp A); a main app test inserts a row by name and reads it with Get and RecordRef.Open by id (corpus PR 1363, tests/al-language/table-objects/moved-tables/MvMovedTableTests.al).",
    awaiting: Awaiting {
        issues: &[7914],
        files: &["tests/al-language/table-objects/moved-tables/MvMovedTableTests.al"],
        since: "2026-11-12",
    },
}
