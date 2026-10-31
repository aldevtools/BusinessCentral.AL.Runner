SelectTokenDotBeforeIndexer => {
    id: "select-token-dot-before-indexer",
    assumed: "In a SelectToken path a `.` right before an indexer (`$.[?(@.id==2)]`, `$.[0]`) is ignored: the path selects what it selects without the dot. The indexer without the dot is pinned (json/lt-dc-json-leads/DcJsonLeads.al:286-345); an unquoted union `$.[A,B]` answers false whether or not the dot is there (json/lt-db-json-leads/DbJPathLeads.al:89-122).",
    tests: "No Microsoft test: al-runner issue 6588, the reporter's suite uses `$.[?(@.id==N)]` on a JsonArray and passes on real BC of the newest major.",
    probe: "SelectToken on a JsonArray of objects with `$.[?(@.id==2)]`, `$[?(@.id==2)]`, `$.[0]` and `$.[*]` (result used and as a statement): the Boolean, the token and the last error.",
    awaiting: Awaiting {
        issues: &[6588],
        files: &["tests/al-language/json/lt-y47/Y47SelectTokenRootFilter.al"],
        since: "2026-11-01",
    },
}
