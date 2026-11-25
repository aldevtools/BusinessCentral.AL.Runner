PartUpdateFirstRowFindForward => {
    id: "part-update-first-row-find-forward",
    assumed: "The exact Document-host/ListPart bodies, properties, tables, calls and values of the issue 7820 probes, with OnFindRecord and no OnNextRecord; object, page and local variable names are free through the measured slots. The original-properties host/part admits OpenEdit, GoToRecord, First and a Qty write on the first of 1, 2 or 5 O1 rows, and First, one Next and that write on the second of exactly 3 O1 rows. The standalone host/part with those three page-level properties absent admits only the first of 1 or 5 O1 rows. The selected Qty is seeded as its row number and changes to 100 in OnValidate after CurrPage.Update(true), without UpdatePropagation Both. The exact refresh reads `=>`, the measured rows and trigger order, then `=`, the selected row and OnAfterGetCurrRecord; calls after it are reads and Close only.",
    tests: "Issue 7820's real-BC reporter and merged corpus R7820PartUpdateFindForward.al:169, :181, :193 and :205 run the original 1-row, 2-row, 5-row and 3-row second-cursor cells. Merged corpus PR 1574 runs the standalone shape at TestPartUpdateFindRecord.al:143 and :154.",
    probe: "Corpus PR 1333 is merged at 5e90c6db4e47886493317422b3d39f279555a918 for tests/al-language/pages/parts/R7820PartUpdateFindForward.al. Corpus PR 1574 is merged at dabfe441b7d1b5c46ce071a2d68a18da01627e4e for the exact standalone shape at tests/al-language/pages/parts/TestPartUpdateFindRecord.al:143 and :154.",
    awaiting: Awaiting {
        issues: &[7820],
        files: &[
            "tests/al-language/pages/parts/R7820PartUpdateFindForward.al",
            "tests/al-language/pages/parts/TestPartUpdateFindRecord.al",
        ],
        since: "2026-11-12",
    },
}
