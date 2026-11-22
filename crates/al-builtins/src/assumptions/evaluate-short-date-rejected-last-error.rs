EvaluateShortDateRejectedLastError => {
    id: "evaluate-short-date-rejected-last-error",
    assumed: "`Evaluate` into a Date or DateTime local variable or parameter, without a number or with number 0, in an en-US session in which no report set `CurrReport.FormatRegion`, with the result used, rejects `mm/dd/yy` with month 0, a month past 12 or a day from 1 to 31 past the end of its month (`00/01/25`, `13/01/25`, `02/30/25`), also with ` 12:00 PM` after it, and leaves a last error of code `Evaluate` (al-builtins `evaluate/short_us_date.rs`). Into a DateTime, exactly `01/04/25 13:00 PM` leaves code `DateInvalid` and exactly `01/04/25 25:00` leaves code `Evaluate`; other hours and minutes are not measured. The last error text is not measured: reading it refuses. The statement form of `13:00 PM` into a DateTime is not measured and refuses. Day 0, days past 31, every other tail and every other target, number, language, region and text keep the readers of `evaluate.rs` or refuse.",
    tests: "No Microsoft test. The probe of corpus PR 1503 on three local BC versions (the neighbors of al-runner issue 8643, a short US date into a Date, then a DateTime): `GetLastErrorCode` after the rejected texts.",
    probe: "Corpus PR 1503 (open), format-evaluate/EvalShortUsDate.al: rejected `mm/dd/yy` texts with and without a time, into a Date and a DateTime, result used, `GetLastErrorCode`, en-US, three local BC versions.",
    awaiting: Awaiting {
        issues: &[8643],
        files: &["tests/al-language/format-evaluate/EvalShortUsDate.al"],
        since: "2026-11-22",
    },
}
