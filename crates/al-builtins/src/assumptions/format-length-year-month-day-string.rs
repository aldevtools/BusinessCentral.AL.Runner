FormatLengthYearMonthDayString => {
    id: "format-length-year-month-day-string",
    assumed: "Format(D, L, '<Year4>-<Month,2>-<Day,2>') of a defined Date that is no closing date keeps its 10-character text for L = 10 and -10, pads it on the left for L = 11 and 12, keeps it for L = -12, and cuts it for L = 3, 8 and 9. Under '<Year4>-<Month,2>' the cut is pinned (types/date-time/Format.al:537) and is not this entry's. Every other length (-8, -9, -11, 13 and up, below -12), a closing date and every other format string with a length refuse (owner al-builtins format/length.rs).",
    tests: "Corpus PR 1460 (merged, commit 79c5192c; not yet in the pinned corpus), measured on a local BC of the first supported major and green in its CI on all three: tests/al-language/format-evaluate/format-length/FormatLengthDates.al:206-214, :219-223 (the same with WorkDate()), and the cuts at :207 (L = 8, '[2024-01-]'), :209 (L = 9, '[2024-01-1]'), :445 (L = 3, '[202]'). Microsoft: Tests-ERM codeunit ERM PostRecurringJournal (PostRecurringJnlWithFiltering, PostRecurringJournalFromBlankLine) through Format(WorkDate(), 10, '<Year4>-<Month,2>-<Day,2>') in the Base App.",
    probe: "Corpus PR 1460 (merged, not yet pinned by a corpus bump), tests/al-language/format-evaluate/format-length/FormatLengthDates.al. Not measured: other format strings (Tests-Data Exchange uses '<Day,2><Month,2><Year,2>' in 6), a closing date with lengths other than 10.",
    awaiting: Awaiting {
        issues: &[8389],
        files: &["tests/al-language/format-evaluate/format-length/FormatLengthDates.al"],
        since: "2026-11-19",
    },
}
