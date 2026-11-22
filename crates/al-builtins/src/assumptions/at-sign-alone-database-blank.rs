AtSignAloneDatabaseBlank => {
    id: "at-sign-alone-database-blank",
    assumed: "`SetFilter(F, '@')` (`@` alone) on a Code or a Text field keeps the rows whose value is empty when the database reads them, and every row of a temporary record (al-builtins `filter_at_sign`): a stored field of a record in the database or in a temporary record, a lookup FlowField's filter of a record in the database or in a temporary record, and a FlowFilter's `@` that CalcFields applies to a source field of either type (a Text FlowFilter over a Code field, a Code FlowFilter over a Text field); `GetFilter` returns `@`. A Text field of a temporary record keeps its pinned answer, every row (WcInvalidFilterExpr.al), unflagged. Not covered, refused as on main: `@` alone on a Code field read where the reader does not say whether its rows are in the database (a platform table, a query, a TableRelation), `<>@` alone on a Code field, `@` alone on any other type. A Text field read there keeps main's answer, every row.",
    tests: "No Microsoft test: al-runner issue 8637 (a page background task's lookup FlowField filtered on `@` alone next to the key; a code review's Text FlowFilter `@` over a Code source field).",
    probe: "Open corpus pull request 1495, tests/al-language/pages/background-tasks/BgTaskQueryCache.al:5654 (`@` alone and `@k` on a Code field of a record in the database, GetFilter), :5647 (`@` alone on a lookup FlowField), and BQ Tests 9b, :5791 and :5805 (a Text and a Code FlowFilter's `@` applied by CalcFields to Code and Text source fields, sum and count), :5818 (the source's Code and Text fields in the database and in a temporary record), :5847 (a Text and a Code lookup FlowField of a host in the database and in a temporary record); 142 of 142 green on a local Business Central 29.1 container (3 runs).",
    awaiting: Awaiting {
        issues: &[8637],
        files: &["tests/al-language/pages/background-tasks/BgTaskQueryCache.al"],
        since: "2026-11-22",
    },
}
