FormatLengthIntegerString => {
    id: "format-length-integer-string",
    assumed: "`Format(Value, Length, FormatString)` of an Integer or a BigInteger with a Length other than 0: the format string gives its text first, then a positive Length pads a shorter text on the left with blanks and cuts a longer one to its first Length characters, and a negative Length keeps a shorter text and cuts a longer one to its first |Length| characters (al-builtins `format::length::string_cell` over `INTEGER_STRING_CELLS`, the cut in `formatstr::format_codes`). Runs only for the cells the probe measured, each by type, the exact format string, the sign of the Length and whether the text without a Length is shorter, as long or longer: Integer under `<Integer>` (both signs), `<Sign><Integer>`, `<Integer,2><Filler Character,0>`, `<Integer,4>`, `<Integer,5><Filler Character,0>`, `N<Integer>`, `No. <Integer> of 9`, `<Integer>.<Integer>`, `abc`, the empty string, `<Integer Thousand>`; BigInteger under `<Integer>`, `<Integer,2><Filler Character,0>`, `<Integer Thousand>`, positive Length only. Refused: every other format string, a negative Length on a BigInteger or under any string but `<Integer>`, a negative Length as long as the text, and the fits the probe did not measure for a string. Decimal under a format string keeps main's pinned rule (format-evaluate/P86FormatDecimal.al:1078-1096).",
    tests: "No Microsoft test. An outside report: `Format(8, 2, '<Integer>')` refused (al-runner issue 8652).",
    probe: "Corpus PR 1505 (open), format-evaluate/format-length/FormatLengthIntegerString.al, codeunit 75960: Integer, BigInteger and Decimal with lengths -6 to 16 under the strings above, measured on two local majors with identical results.",
    awaiting: Awaiting {
        issues: &[8652],
        files: &["tests/al-language/format-evaluate/format-length/FormatLengthIntegerString.al"],
        since: "2026-11-22",
    },
}
