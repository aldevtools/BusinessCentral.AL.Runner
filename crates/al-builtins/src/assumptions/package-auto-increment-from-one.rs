PackageAutoIncrementFromOne => {
    id: "package-auto-increment-from-one",
    assumed: "A table built from a RapidStart package whose rows hold no value for the table's AutoIncrement field (Dimension Value.\"Dimension Value ID\" is not in the shipped package) gets 1, 2, 3 in the order of the package's rows, as the database numbers rows it inserts, and the counter continues after them.",
    tests: "Sales-Calc. Discount Test LcyDiscountTest and ERM Sales Quotes SalesQuoteWithContact (a dimension set entry must carry a nonzero Dimension Value ID); Production & Assembly Costing AssemblyOrderWithDimensionShouldPostWithoutError.",
    probe: "Not measurable from the corpus (it depends on the import tool): import a RapidStart package into Dimension Value and read the field. Until then the rule rests on the field being AutoIncrement and absent from Microsoft's package.",
}
