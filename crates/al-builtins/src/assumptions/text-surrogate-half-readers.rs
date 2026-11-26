TextSurrogateHalfReaders => {
    id: "text-surrogate-half-readers",
    assumed: "A Text or Code that holds one half of a UTF-16 surrogate pair (55357 or 56832 alone, from CopyStr of a pair or a Char) keeps the half, unit by unit, through the readers the probe measures: TextBuilder Append, AppendLine, Insert, Replace and ToText; the comparison operators, ordinal by UTF-16 code unit; the text functions of the probe. TextBuilder.Insert of a low half at position 0 raises the measured error.",
    tests: "No Microsoft test. An owner report (al-runner issue 9153) names TextBuilder.Append and the comparison of such a text.",
    probe: "Corpus PR 1605, tests/al-language/text/LtSurrogateHalves.al (codeunit \"LT SH Ops\", 37 tests); measured identical on the three supported majors.",
    awaiting: Awaiting {
        issues: &[9153],
        files: &["tests/al-language/text/LtSurrogateHalves.al"],
        since: "2026-11-26",
    },
}
