UndefinedDateTimeDifference => {
    id: "undefined-datetime-difference",
    assumed: "0DT - 0DT is a 0 Duration (also SystemModifiedAt - SystemCreatedAt of a record never inserted); a DateTime minus 0DT, or 0DT minus a DateTime, raises 'The date is not valid.' (DateInvalid), and so does 0DT plus or minus a whole number or a Duration. Measured on SaaS and every corpus CI major by corpus writers Y62 and Y65; on corpus master (Y62) and writer Y65's branch, not on the pinned corpus yet.",
    tests: "No Microsoft test: al-runner issue 6678 (System Application Apply Retention Policy, IsSystemEnabledPolicy, with a setup record never inserted).",
    probe: "Y62UndefinedDate.al DateTimeSub_BothUndefined and DateTimeSub_OneUndefined_AndDefinedPair (corpus PR 999, merged to master); Y65UndefinedArith.al Y65_UndefinedDateTime_Arithmetic (corpus PR 1012).",
    awaiting: Awaiting {
        issues: &[6678],
        files: &[
            "tests/al-language/types/lt-y62-undefined-date/Y62UndefinedDate.al",
            "tests/al-language/types/lt-y65-undefined-arith/Y65UndefinedArith.al",
        ],
        since: "2026-11-01",
    },
}
