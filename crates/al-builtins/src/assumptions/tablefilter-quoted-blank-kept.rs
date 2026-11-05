TablefilterQuotedBlankKept => {
    id: "tablefilter-quoted-blank-kept",
    assumed: "Evaluate into a TableFilter field stores the quoted blank filters `''` on an Enum field with a blank member and `<>''` on a Code field as written, so Format gives `Choice=''` and `Code=<>''` back. Pinned: these stored texts, fed to the Base Application page Table Filter and FieldRef.SetFilter, select the blank member and the nonempty code (format-evaluate/tablefilter-neutral/TableFilterEnumForms.al:468 and :485). The stored text itself is not pinned.",
    tests: "No Microsoft test: al-runner issue 7132 (the reporter's real-BC run of TableFilter_Evaluate_QuotedBlankEnum_SelectsBlankMember and TableFilter_Evaluate_NotEqualQuotedBlankCode_SelectsNonemptyCode on BC 27.5, 28.5 and 29.0, corpus pull request 1152).",
    probe: "format-evaluate/tablefilter-neutral/R7132QuotedBlankForms.al TableFilterEvaluate_QuotedBlankForms (corpus branch al-runtime-rs-staging-r7132): what Evaluate returns and what Format gives back for `''` and `<>''` on Code, Text, an Enum with and without a blank member and an Option.",
    awaiting: Awaiting {
        issues: &[7132],
        files: &["tests/al-language/format-evaluate/tablefilter-neutral/R7132QuotedBlankForms.al"],
        since: "2026-11-05",
    },
}
