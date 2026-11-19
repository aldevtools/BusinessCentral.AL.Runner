FormatLengthUndefinedDate => {
    id: "format-length-undefined-date",
    assumed: "An undefined Date (0D) or DateTime (0DT) with a length formats as that many blanks for a positive length and as the empty text for a negative one: Format(0D, L) and Format(0D, L, 0 | 9) for L from 1 to 30 and -1 to -30, Format(0D, 10, 1 | 2); Format(0DT, L) for L from 1 to 30 and -1 to -30, Format(0DT, L, 9) for L = 3, 10, 30, -3, -30; Format(0D, L, '<Year4>-<Month,2>-<Day,2>') for L = 3, 10, 30, -3, -10 and Format(0D, 10, '<Closing><Day>'). Every other length, standard format and format string of 0D and 0DT refuses (owner al-builtins format/length.rs).",
    tests: "Corpus PR 1460 (merged, commit 79c5192c; not yet in the pinned corpus), measured on a local BC of the first supported major and green in its CI on all three: tests/al-language/format-evaluate/format-length/FormatLengthDates.al:62-71, :99-113, :132-137, :216-217, :162-171, :244-249, :352-366. Microsoft: Tests-Misc codeunit 139323 Onboarding Signal Tests (TestGetOnboardingSignal, TestInvalidOnboardingSignal, OnboardingSignalTestSignal, TestCompanySignal) through Format(Rec.Date, 10, 9) of a 0D field in the Base App codeunit Onboarding Signal.",
    probe: "Corpus PR 1460 (merged, not yet pinned by a corpus bump), tests/al-language/format-evaluate/format-length/FormatLengthDates.al. Not measured: lengths beyond 30, 0DT under a format string with a length, other standard formats.",
    awaiting: Awaiting {
        issues: &[8389],
        files: &["tests/al-language/format-evaluate/format-length/FormatLengthDates.al"],
        since: "2026-11-19",
    },
}
