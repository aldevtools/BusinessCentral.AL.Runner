TablefilterDecimalAtLeastKept => {
    id: "tablefilter-decimal-at-least-kept",
    assumed: "Evaluate into a TableFilter field stores the filter `>=<digits>` on a Decimal field as written, so Format gives `<field caption>=>=40` back. Pinned: the stored text, fed to the Base Application page Table Filter and FieldRef.SetFilter, selects the row at 42.5 and not the one at 17.5 (format-evaluate/tablefilter-neutral/TableFilterEnumForms.al:539). The stored text and every other Decimal filter are not pinned.",
    tests: "No Microsoft test: al-runner issue 7164 (the reporter's real-BC run of TableFilter_Evaluate_CaptionClassFieldName_SelectsPricedRow on the three supported majors, corpus pull request 1157).",
    probe: "format-evaluate/tablefilter-neutral/R7164CaptionClassStored.al: what Evaluate returns and Format gives back for Decimal filters `>=40`, `>=40.5`, `40..50`, `<>0` and `'40'` on a Decimal field.",
    awaiting: Awaiting {
        issues: &[7164],
        files: &["tests/al-language/format-evaluate/tablefilter-neutral/R7164CaptionClassStored.al"],
        since: "2026-11-06",
    },
}
