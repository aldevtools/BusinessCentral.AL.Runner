TablefilterCodeChainedKept => {
    id: "tablefilter-code-chained-kept",
    assumed: "Evaluate into a TableFilter field stores two Code filter forms as written: alternatives joined by `|` whose values hold `-` or `/` (`Code Field=BIN-A|BIN/B`), and exclusions joined by `&` (`Code Field=<>CORP01&<>CORP02`). Pinned: the stored text, fed to the Base Application page Table Filter and FieldRef.SetFilter, selects BIN-A and BIN/B and not BIN/C, and only OTHER beside CORP01 and CORP02 (format-evaluate/tablefilter-neutral/TableFilterEnumForms.al:936 and :943, corpus pull request 1199). The stored text, lower case and other characters are not pinned.",
    tests: "No Microsoft test. The corpus pins only what the stored text selects: format-evaluate/tablefilter-neutral/TableFilterEnumForms.al:936 TableFilter_Evaluate_CodeExclusions_LeavesOtherRow and :943 TableFilter_Evaluate_CodeAlternatives_SelectsBothMatchingRows (corpus master, PR 1199).",
    probe: "format-evaluate/tablefilter-neutral/TableFilterEnumForms.al: what Format gives back after Evaluate of `Code Field=BIN-A|BIN/B`, `Code Field=bin-a|bin/b` and `Code Field=<>CORP01&<>CORP02`.",
}
