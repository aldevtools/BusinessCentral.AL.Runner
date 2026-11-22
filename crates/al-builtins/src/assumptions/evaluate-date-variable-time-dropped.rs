EvaluateDateVariableTimeDropped => {
    id: "evaluate-date-variable-time-dropped",
    assumed: "`Evaluate` into a Date or DateTime local variable or parameter, without a number or with number 0, in an en-US session, reads the date part of `mm/dd/yy` and leaves out a time after it (al-builtins `evaluate/short_us_date.rs`). Into a Date: one blank and `h:mm AM` or `h:mm PM` (also with two blanks), `hh:mm:ss`, `noon`, or `T` and `hh:mm` give the date. The date parts `m/d/yy` and `mm/dd/yyyy` are measured with `12:00 PM` and `1:20 AM` only. Into a DateTime: `T` and `hh:mm` give the date at midnight. A rejected date with a time (`13/04/25 12:00 PM`) into a Date also marks this entry. Every other target (a field, a global, an array element), number, language and text keeps the readers of `evaluate.rs`.",
    tests: "No Microsoft test. An outside report (al-runner issue 8644): the app author's test notes that on a newer BC `Evaluate(D, '01/04/25 12:00 PM')` into a Date takes the text. The probe of corpus PR 1503 on three local BC versions gives the date.",
    probe: "Corpus PR 1503 (open), format-evaluate/EvalShortUsDate.al: Date and DateTime targets, locals and parameters, `mm/dd/yy` with each time tail, statement form and result used, en-US, three local BC versions.",
    awaiting: Awaiting {
        issues: &[8644],
        files: &["tests/al-language/format-evaluate/EvalShortUsDate.al"],
        since: "2026-11-22",
    },
}
