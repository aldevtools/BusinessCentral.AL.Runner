UndefinedDateTimeDifference => {
    id: "undefined-datetime-difference",
    assumed: "0DT - 0DT is a 0 Duration (also SystemModifiedAt - SystemCreatedAt of a record never inserted); a DateTime minus 0DT, or 0DT minus a DateTime, raises 'The date is not valid.' (DateInvalid). Measured on SaaS and every corpus CI major by corpus writer Y62; on corpus master, not on the pinned corpus yet.",
    tests: "No Microsoft test: al-runner issue 6678 (System Application Apply Retention Policy, IsSystemEnabledPolicy, with a setup record never inserted).",
    probe: "Y62UndefinedDate.al DateTimeSub_BothUndefined and DateTimeSub_OneUndefined_AndDefinedPair (corpus PR 999, merged to master).",
    awaiting: Awaiting {
        issues: &[6678],
        files: &["tests/al-language/types/lt-y62-undefined-date/Y62UndefinedDate.al"],
        since: "2026-11-01",
    },
}
