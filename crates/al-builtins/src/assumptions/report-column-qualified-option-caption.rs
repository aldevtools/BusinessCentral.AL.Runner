ReportColumnQualifiedOptionCaption => {
    id: "report-column-qualified-option-caption",
    assumed: "A report column whose source is an Option field of its own data item written as `Item.Field` (an item name that holds no dot, the item's and the field's names as declared) writes the member's OptionCaption, as a bare Option field does (al-sema `reports/column_source.rs` `option_caption_measured`, al-vm `reports.rs`). Measured only for: a normal field the table itself declares, whose Caption, CaptionML, OptionCaption and OptionCaptionML no tableextension changes, that states a Caption, an OptionCaption with one caption that is not empty for each member, no OptionCaptionML, DecimalPlaces, AutoFormatType or AutoFormatExpression; a column that states exactly IncludeCaption = true; in the report itself, not a reportextension. Every other qualified Option column keeps main's answer (the member name): a field without Caption or OptionCaption, an empty caption or fewer captions than members, a column with no property or another property, a dotted item name (the report's dotted columns turn off, `report-column-dotted-item-qualifier`), a qualifier or field name in another case, a column of another data item, a reportextension column, a field whose caption or OptionCaption a tableextension changes.",
    tests: "No Microsoft test. al-runner issue 9411.",
    probe: "Corpus pull request 1652 (merged, not yet pinned) at cf6884b9, tests/al-language/reports/dataset/DataItemNamesDottedQualifier.al: field :356 (`Kind`, OptionMembers \" \",Open,Closed, OptionCaption ' ,DQ Open cap,DQ Closed cap', Caption), undotted column :428 (`DqLine.Kind`, IncludeCaption = true), line :678 (`DQ Open cap` for Open, ' ' for the blank member); the same with a dotted item name at :391 and :616. Measured on BC on Linux, on every supported major (the latest minor of each in corpus CI).",
    awaiting: Awaiting {
        issues: &[9411],
        files: &["tests/al-language/reports/dataset/DataItemNamesDottedQualifier.al"],
        since: "2026-11-28",
    },
}
