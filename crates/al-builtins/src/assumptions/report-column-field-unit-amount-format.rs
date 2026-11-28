ReportColumnFieldUnitAmountFormat => {
    id: "report-column-field-unit-amount-format",
    assumed: "A Decimal report column whose source is a field of its own data item written as `Item.Field`, with the item's name (dotted or not) and the field's name as declared, takes the decimalformatter `#,##0.00###` when the field states AutoFormatType 2 and no DecimalPlaces, as a column's own AutoFormatType 2 does (al-sema `reports::field_unit_amount_measured`, al-vm `column_formatter.rs`). Measured only for: a normal field the table itself declares, whose caption no tableextension changes, that states a Caption, no CaptionML, and no AutoFormatExpression or AutoFormatExpression ''; a column that states exactly IncludeCaption = true; in the report itself, not a reportextension. Every other column over such a field keeps the formatter of DecimalPlaces (`#,##0.00` without them): a bare column, a column of another data item, a qualifier or field name in another case, a reportextension column, a column with no property or another property, a field with a currency expression or DecimalPlaces.",
    tests: "No Microsoft test. al-runner issue 9330: report 99000763 \"Prod. Order - List\" (`\"Prod. Order Line\".\"Unit Cost\"`, SCM 137035 PrintMultipleProductionOrdersWhenUsingCarryOutActionMessage); issue 9392 (first row).",
    probe: "Corpus pull request 1652 (merged, not yet pinned) at cf6884b9, tests/al-language/reports/dataset/DataItemNamesDottedQualifier.al: field :34 (AutoFormatType 2), columns :78 (dotted) and :114 (undotted), lines :236 and :286; field :354 (AutoFormatType 2, AutoFormatExpression ''), columns :389 (dotted) and :426 (undotted), lines :630 and :692. Measured on BC on Linux 27.5, 28.5 and 29.0.",
    awaiting: Awaiting {
        issues: &[9330, 9392],
        files: &["tests/al-language/reports/dataset/DataItemNamesDottedQualifier.al"],
        since: "2026-11-28",
    },
}
