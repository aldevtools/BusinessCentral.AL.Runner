NewLineSwitchToEmptyView => {
    id: "new-line-switch-to-empty-view",
    assumed: "A list that stands on its untouched new-row line over a view without rows, whose control's OnValidate set the filters of Rec (to another view without rows, or to the ones it had) and called CurrPage.Update: after the OnValidate the page shows the new-row line of the new view, OnNewRecord and OnAfterGetCurrRecord run twice with the values of the new filters in Rec, no row is read or stored, and an action then sees the new view in Rec. Observed on BC 27.5, 28.5 and 29.0 for a Worksheet with DelayedInsert, with and without AutoSplitKey, whose OnValidate calls SaveRecord, sets the filters in filter group 2, calls Find('-') and Update(false); not measured: BelowxRec and xRec in those OnNewRecord runs, Update(true), a List, a page with parts, an OnValidate without the SaveRecord or the Find.",
    tests: "Tests-SCM 137293 RequisitionLineForItemWithSKU, NoPlanningLinesCreatedWhenVariantForecastingIsOFFAndNoForecastEntriesWithVariantsExist, PlanningLinesForNonDemandLocationWithPoductionForcast. No corpus test on the pinned master yet: the probe is corpus pull request 1282 (pages/journal-batch-switch/EmptyToEmptyBatchSwitch.al), observed on all three majors of the matrix and pinned on that branch.",
    probe: "pages/journal-batch-switch/EmptyToEmptyBatchSwitch.al; still to measure: BelowxRec and xRec in the OnNewRecord runs, Update(true), the same on a List, an OnValidate that only calls CurrPage.Update.",
    awaiting: Awaiting {
        issues: &[6924],
        files: &["tests/al-language/pages/journal-batch-switch/EmptyToEmptyBatchSwitch.al"],
        since: "2026-11-10",
    },
}
