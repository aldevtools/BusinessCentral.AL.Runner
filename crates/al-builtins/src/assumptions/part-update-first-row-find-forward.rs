PartUpdateFirstRowFindForward => {
    id: "part-update-first-row-find-forward",
    assumed: "Two exact Document-host/ListPart shapes with OnFindRecord (no OnNextRecord), standing on the first row: the original issue 7820 reporter with page-level ApplicationArea=All and the host UsageCategory=None, and the standalone reporter/probe with those page-level properties absent. The tested field OnValidate calls CurrPage.Update(true) without UpdatePropagation Both. After OpenEdit, host GoToRecord and part First, changing that field reads `=>`, all one or five rows, the first row again, `=`, OnAfterGetRecord and OnAfterGetCurrRecord.",
    tests: "Issue 7820's real-BC reporter runs OneRowSetValueOnFirst and FiveRowsSetValueOnFirst. Corpus PR 1574 measures the standalone shape in pages/parts/TestPartUpdateFindRecord.al:143 with one row and :154 with five rows on supported versions.",
    probe: "Corpus PR 1574: the exact standalone shape at tests/al-language/pages/parts/TestPartUpdateFindRecord.al:143 and :154. The original merged probe remains listed in tests/al-language/pages/parts/R7820PartUpdateFindForward.al.",
    awaiting: Awaiting {
        issues: &[7820],
        files: &[
            "tests/al-language/pages/parts/R7820PartUpdateFindForward.al",
            "tests/al-language/pages/parts/TestPartUpdateFindRecord.al",
        ],
        since: "2026-11-12",
    },
}
