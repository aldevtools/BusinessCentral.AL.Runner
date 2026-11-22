RecordRefMetadataGlobalFilter => {
    id: "recordref-metadata-global-filter",
    assumed: "A RecordRef of a database table whose filter group 1 a Record variable set (SetRange of one Integer value on an Integer field outside the primary key, the variable out of scope, the entry in the form `group1_sessions.rs` pins) answers a call in the test session as it does on a table without such a filter, with the RecordRef in filter group 0, no filter, key, view or mark of its own and the session company: Number, Name, Caption, FieldCount, KeyCount, IsTemporary, RecordId before any read, CurrentKeyIndex, SecurityFiltering, ReadPermission, Close (then Open again and Count), IsEmpty and FindFirst with no argument, and FieldExist(2), FieldExist(9), KeyIndex(1), FieldIndex(1) and Field(1) with one Integer argument; a Count after them and the rows IsEmpty and FindFirst read follow the filter (al-vm group1_sessions.rs `rr_probe_call`, `rr_probe_args_measured`). Every other RecordRef call, a RecordRef that is temporary or whose filter another RecordRef set, and a worker session keep the refusal.",
    tests: "No Microsoft test. al-runner issue 7874 (the reporter's repro: a RecordRef opened after a Record variable set a filter in group 1).",
    probe: "Open corpus pull request 1525, tests/al-language/recordref-fieldref/RecordRefMetadataGlobalFilter.al (Filtered_* tests, Control_NoGlobalFilter_*), 49 tests of the three probe files green on all three pinned majors of a local Business Central with no version difference (corpus CI not run yet).",
    awaiting: Awaiting {
        issues: &[7874],
        files: &["tests/al-language/recordref-fieldref/RecordRefMetadataGlobalFilter.al"],
        since: "2026-11-22",
    },
}
