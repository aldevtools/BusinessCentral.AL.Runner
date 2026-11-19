FormatLengthStandardFormatString => {
    id: "format-length-standard-format-string",
    assumed: "Format(V, L, '<Standard Format,N>') with a length gives the text of Format(V, L, N) where that call runs and the format string gives the same text: for N = 0 on a Date (undefined, closing or defined), the undefined DateTime, a Duration, a Char, a Time, a Boolean, a Guid, a BigInteger, a Byte, an Integer, a Decimal and a Code, and for N = 9 on a Date that is no closing date, with a text shorter or longer than the length, either sign. The Decimal shorter than a positive length in formats 0, 1 and 9 is pinned (format-evaluate/P86FormatDecimal.al:1083, :1084, :1086). A text as long as the length, other types and other numbers refuse (owner al-builtins format/length.rs).",
    tests: "Corpus PR 1460 (open, commit 79c5192c), measured on a local BC of the first supported major and green in its CI on all three: tests/al-language/format-evaluate/format-length/FormatLengthNeighbors.al:300-318, :357-375, :407-420, :452-465 (lengths 10, 3, -3, -10 against Format(V, 10, 0)), :486-492 (standard format 9 on a Date). No Microsoft test needs it.",
    probe: "Corpus PR 1460 (open), tests/al-language/format-evaluate/format-length/FormatLengthNeighbors.al. Not measured: a text as long as the length, standard formats 1, 2 and 9 on other types.",
    awaiting: Awaiting {
        issues: &[8389],
        files: &["tests/al-language/format-evaluate/format-length/FormatLengthNeighbors.al"],
        since: "2026-11-19",
    },
}
