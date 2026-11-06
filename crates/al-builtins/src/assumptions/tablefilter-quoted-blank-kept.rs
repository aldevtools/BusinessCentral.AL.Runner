TablefilterQuotedBlankKept => {
    id: "tablefilter-quoted-blank-kept",
    assumed: "Evaluate into a TableFilter field stores the quoted blank filters `''` on an Enum field with a blank member, `<>''` on a Code field and `''` on a Code field as written, so Format gives `Choice=''`, `Code=<>''` and `Service Type=''` back. Pinned: these stored texts, fed to the Base Application page Table Filter and FieldRef.SetFilter, select the blank member, the nonempty code and the blank codes, alone and beside another condition (format-evaluate/tablefilter-neutral/TableFilterEnumForms.al:468, :485, :579 and :587). The stored text itself is not pinned.",
    tests: "No Microsoft test: al-runner issue 7132 (the reporter's real-BC run of TableFilter_Evaluate_QuotedBlankEnum_SelectsBlankMember and TableFilter_Evaluate_NotEqualQuotedBlankCode_SelectsNonemptyCode on the three supported majors, corpus pull request 1152); al-runner issue 7165 (TableFilter_Evaluate_QuotedBlankCode10_SelectsBlankService and _CompoundQuotedBlankCode10_SelectsBlankService, corpus pull request 1157).",
    probe: "format-evaluate/tablefilter-neutral/R7132QuotedBlankForms.al TableFilterEvaluate_QuotedBlankForms (corpus branch al-runtime-rs-staging-r7132): what Evaluate returns and what Format gives back for `''` and `<>''` on Code, Text, an Enum with and without a blank member and an Option.",
    awaiting: Awaiting {
        issues: &[7132, 7165],
        files: &["tests/al-language/format-evaluate/tablefilter-neutral/R7132QuotedBlankForms.al"],
        since: "2026-11-05",
    },
}
