TextSurrogateHalfReaders => {
    id: "text-surrogate-half-readers",
    assumed: "A Text that holds one half of a UTF-16 surrogate pair keeps the half, unit by unit, through TextBuilder Append, AppendLine, Insert, Replace and ToText. These read no unit value, so any lone half is kept; the probe uses 55357 and 56832. An Insert position or a ToText range end between the two halves of a whole pair refuses. The operators `=` and `<>` of two Texts call them equal only for the same units, as the pinned case-sensitive exact equality of plain Text (types/TestFormatEvaluateTextContracts.al:171). `<` and `>` of two Texts are answered by UTF-16 unit only for these pairs, in either order: 55357 alone against 'a', 'z', the empty Text, 55357 alone, 56832 alone, U+FFFD alone, 'abc' and 'a' followed by 55357; 56832 alone against 55357 alone, 'a', U+FFFD alone, 'abc' and '~'. Every other order of such a Text refuses. `<=` and `>=` of two Texts refuse. Every operator of such a Text against an operand that is not a Text (a Char, a Code) refuses. Every other reader of the half refuses.",
    tests: "No Microsoft test. An owner report (al-runner issue 9153) names TextBuilder.Append and the comparison of such a text.",
    probe: "Corpus PR 1605, tests/al-language/text/LtSurrogateHalves.al, codeunit \"LT SH Ops\": Ops_tb_1, Ops_tb_2, Ops_tb_3 (TextBuilder) and Ops_cmp_1 to Ops_cmp_10 (comparison); measured identical on the three supported majors.",
    awaiting: Awaiting {
        issues: &[9153],
        files: &["tests/al-language/text/LtSurrogateHalves.al"],
        since: "2026-11-26",
    },
}
