TablefilterCodeChainedKept => {
    id: "tablefilter-code-chained-kept",
    assumed: "Evaluate into a TableFilter field stores Code alternatives joined by `|` whose values hold `-` or `/` as written (`Code Field=BIN-A|BIN/B`). Pinned: the stored text, fed to the Base Application page Table Filter and FieldRef.SetFilter, selects BIN-A and BIN/B and not BIN/C (format-evaluate/tablefilter-neutral/TableFilterEnumForms.al:943, corpus PR 1199). The stored text, lower case and other characters are not pinned. Exclusions joined by `&` are pinned with their stored text by format-evaluate/tablefilter-neutral/TableFilterSpacedChains.al and no longer assumed.",
    tests: "No Microsoft test. The corpus pins only what the stored text selects: format-evaluate/tablefilter-neutral/TableFilterEnumForms.al:943 TableFilter_Evaluate_CodeAlternatives_SelectsBothMatchingRows (corpus PR 1199).",
    probe: "format-evaluate/tablefilter-neutral/TableFilterEnumForms.al: what Format gives back after Evaluate of `Code Field=BIN-A|BIN/B` and `Code Field=bin-a|bin/b`.",
}
