TextSurrogateHalfReaders => {
    id: "text-surrogate-half-readers",
    assumed: "A Text that holds one half of a UTF-16 surrogate pair (55357 or 56832 alone) keeps the half, unit by unit, through TextBuilder Append, AppendLine, Insert, Replace and ToText. The comparison operators order two such Texts by UTF-16 code unit. Every other reader of the half refuses.",
    tests: "No Microsoft test. An owner report (al-runner issue 9153) names TextBuilder.Append and the comparison of such a text.",
    probe: "Corpus PR 1605, tests/al-language/text/LtSurrogateHalves.al, codeunit \"LT SH Ops\": Ops_tb_1, Ops_tb_2, Ops_tb_3 (TextBuilder) and Ops_cmp_1 to Ops_cmp_10 (comparison); measured identical on the three supported majors.",
    awaiting: Awaiting {
        issues: &[9153],
        files: &["tests/al-language/text/LtSurrogateHalves.al"],
        since: "2026-11-26",
    },
}
