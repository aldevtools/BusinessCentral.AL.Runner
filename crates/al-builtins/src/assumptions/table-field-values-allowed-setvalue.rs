TableFieldValuesAllowedSetValue => {
    id: "table-field-values-allowed-setvalue",
    assumed: "TestPage SetValue on a control over a table field with ValuesAllowed (Enum, Option, Integer, Decimal) on a Card, NavigatePage or List with a row: a listed Enum value or ordinal is taken; another is a TestValidation error naming the ordinal the control holds; a member name as Text in a Variant is an Evaluate:Format error; an Integer or Decimal outside the list (a Decimal also when a whole number of the list) names the value and the list; a control's own ValuesAllowed on an Enum replaces the field's; a NavigatePage adds no Refresh hint and shows the ordinal of a value outside the field's list. Measured on SaaS (newest major) by corpus writer Y65; not on the pinned corpus yet.",
    tests: "No Microsoft test: al-runner issue 6672, a partner wizard (NavigatePage over a temporary table) and card that set an Enum field with ValuesAllowed.",
    probe: "Y65ValuesAllowedTableField.al on corpus branch al-runtime-rs-staging-y65 (commit eefaf01b), the Wizard, Card and List tests and the two Reporter tests.",
    awaiting: Awaiting {
        issues: &[6672],
        files: &["tests/al-language/pages/lt-y65-values-allowed-table-field/Y65ValuesAllowedTableField.al"],
        since: "2026-11-01",
    },
}
