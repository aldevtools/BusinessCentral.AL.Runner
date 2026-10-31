UpdateRefreshListOnFindRecord => {
    id: "update-refresh-list-onfindrecord",
    assumed: "CurrPage.Update on a list with OnFindRecord (with or without OnNextRecord) reads the rows as the pinned table of 3 rows with the cursor on the middle one does (misc/W2FindRecordUpdate.al), for any number of rows and any cursor position below a window.",
    tests: "ERM Sales Subform (InvoiceAddingLinesUpdatesTotals) and about 120 more Tests-ERM tests whose sales and purchase subforms refresh.",
    probe: "The same list with 1, 2, 5 and 25 rows and the cursor on the first, last and a middle row; a list with OnFindRecord only; a list part of a card.",
}
