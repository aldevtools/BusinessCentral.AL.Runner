TimeRolledPastMidnight => {
    id: "time-rolled-past-midnight",
    assumed: "A Time of 23:59:59.999 in a database table, outside the primary key, is kept as a time after every time of the day: a filter or count compares it so, a read gives 00:00:00, a Modify that leaves the field as read keeps it, and a FlowField max, min or lookup gives 00:00:00. A filter value of 23:59:59.999 on such a field compares after the day too. A read in the order of a key that holds the field, a query over it and a Rename of such a row are refused. A Time primary key field still rounds 23:59:59.999 to 00:00:00 (a filter on 00:00:00 finds it, where BC finds none: still open).",
    tests: "No Microsoft test. Measured on a local service tier (corpus pull request 1364).",
    probe: "record/filters/TimeRolledPastMidnight.al: filters and counts (:99-158), sort order (:162-182), DeleteAll (:186-199), Modify and ModifyAll (:202-254), temporary table (:257-295), Time primary key (:298-338), FlowField and FieldRef (:341-368).",
    awaiting: Awaiting {
        issues: &[],
        files: &["tests/al-language/record/filters/TimeRolledPastMidnight.al"],
        since: "2026-11-12",
    },
}
