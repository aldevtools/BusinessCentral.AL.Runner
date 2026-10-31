FilterEmptyTextDateFormula => {
    id: "filter-empty-text-dateformula",
    assumed: "In a filter on a DateFormula field the empty text is the empty formula, so a filter of not-equal and the empty text selects the rows that have a formula.",
    tests: "Library - ERM GetDiscountPaymentTerm (PaymentTerms.SetFilter(Due Date Calculation, not-equal and the empty text)): about 170 Tests-ERM tests.",
    probe: "Payment Terms rows with and without Due Date Calculation; SetFilter on it with the empty text, not-equal and the empty text, then Count. Asked in corpus #644.",
}
