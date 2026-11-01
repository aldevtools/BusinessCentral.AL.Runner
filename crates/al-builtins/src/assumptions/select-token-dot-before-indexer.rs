SelectTokenDotBeforeIndexer => {
    id: "select-token-dot-before-indexer",
    assumed: "In a SelectToken path a `.` right before a filter indexer (`$.[?(@.id==2)]`) is ignored: the path selects what it selects without the dot. A dot before another indexer stays refused. The indexer without the dot is pinned (json/lt-dc-json-leads/DcJsonLeads.al:286-345); an unquoted union `$.[A,B]` answers false whether or not the dot is there (json/lt-db-json-leads/DbJPathLeads.al:89-122).",
    tests: "No Microsoft test: al-runner issue 6588, the reporter's suite uses `$.[?(@.id==N)]` on a JsonArray; measured on real BC by corpus writer Y47 (DbJPathLeads.al Y47_* tests on corpus branch al-runtime-rs-staging-y47).",
    probe: "SelectToken on a JsonArray of objects with `$.[?(@.id==2)]`, `$[?(@.id==2)]`, `$.[0]` and `$.[*]` (result used and as a statement): the Boolean, the token and the last error.",
    awaiting: Awaiting {
        issues: &[6588],
        files: &[
            "tests/al-language/json/lt-db-json-leads/DbJPathLeads.al::Y47_ArrayFilter_ReporterShapes",
            "tests/al-language/json/lt-db-json-leads/DbJPathLeads.al::Y47_ArrayFilter_Neighbours",
        ],
        since: "2026-11-01",
    },
}
