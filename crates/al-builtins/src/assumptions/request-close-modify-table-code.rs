RequestCloseModifyTableCode => {
    id: "request-close-modify-table-code",
    assumed: "A request page with a source table that closes with OK or Cancel on a row the test client changed writes the row (Modify) after the page's OnModifyRecord and its extensions' ones, and the table's OnModify, its extensions' and the table events run as on any Modify. The corpus pins the page triggers' order and the stored row (report/lt-y80-reportext-request-page/Y80ReportExtRequestPage.al Y80_RX_SetTxt_OK, _SetTxt_Cancel) but never observes the table code.",
    tests: "None known; the corpus cases whose table code writes to a log the case does not read.",
    probe: "Y80_RX_SetTxt_OK and _SetTxt_Cancel with the table's OnModify and the page's OnModifyRecord writing to the one observed log.",
}
