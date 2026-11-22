EvaluateDateVariableTimeDropped => {
    id: "evaluate-date-variable-time-dropped",
    assumed: "`Evaluate` into a Date or DateTime local variable or parameter, without a number or with number 0, in an en-US session in which no report set `CurrReport.FormatRegion`, reads the date part of `mm/dd/yy` and leaves out a time after it (al-builtins `evaluate/short_us_date.rs`). Into a Date, exactly these tails give the date: ` 12:00 PM`, ` 12:00 AM`, ` 1:20 AM`, ` 13:00 PM`, `  12:00 PM` (two blanks), ` 23:59:59`, ` noon` and `T12:00`. The date parts `m/d/yy` and `mm/dd/yyyy` (digit lengths 1, 1, 2 and 2, 2, 4 only) are measured with ` 12:00 PM` and ` 1:20 AM` only. Into a DateTime: `T12:00` gives the date at midnight. A rejected date with ` 12:00 PM` (`13/04/25 12:00 PM`) into a Date also marks this entry. Every other tail, hour, minute, date part, target (a field, a global, an array element), number, language and region keeps the readers of `evaluate.rs` or refuses.",
    tests: "No Microsoft test. An outside report (al-runner issue 8644): the app author's test notes that on a newer BC `Evaluate(D, '01/04/25 12:00 PM')` into a Date takes the text. The probe of corpus PR 1503 on three local BC versions gives the date.",
    probe: "Corpus PR 1503 (open), format-evaluate/EvalShortUsDate.al: Date and DateTime targets, locals and parameters, `mm/dd/yy` with each time tail, statement form and result used, en-US, three local BC versions.",
    awaiting: Awaiting {
        issues: &[8644],
        files: &["tests/al-language/format-evaluate/EvalShortUsDate.al"],
        since: "2026-11-22",
    },
}
