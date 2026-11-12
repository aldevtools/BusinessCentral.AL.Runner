XmlportAfterAssignBeforeValidate => {
    id: "xmlport-after-assign-before-validate",
    assumed: "A Format = Xml XmlPort import (DefaultFieldsValidation left on) of a field attribute or field element with OnAfterAssignField into a Code field with one plain TableRelation, no table extension validate trigger and no validate event subscriber, below a table element right below the root text element without LinkTable, one row per document: OnAfterAssignField runs first and sees the imported value, then the field is validated with the value the trigger left (the TableRelation check, unless ValidateTableRelation = false, then OnValidate); with FieldValidate = No nothing is validated; on a temporary table element (UseTemporary) nothing is validated; a blank field attribute runs neither the trigger nor the validation, a blank field element runs both. Measured shapes: attribute and element with the field validated, attribute with FieldValidate = No, attribute into a field with ValidateTableRelation = false, each on a temporary and a stored table element, the field with an OnValidate trigger; the reporter's shape (attribute, temporary table element, a field without OnValidate) is assumed from the temporary measurement. The three stored ports with FieldValidate = No, ValidateTableRelation = false and the field element were measured on two majors only. Every other shape keeps main's result or refuses.",
    tests: "al-runner issue 7863: an Intercompany import reads the IC partner code in OnAfterAssignField of a field attribute on a temporary table element.",
    probe: "xmlports/import/XpAfterAssignTableRelation.al:578-676 (attribute, element, FieldValidate = No, ValidateTableRelation = false; temporary and stored table element; six values each). Corpus pull request 1366. Not measured: DefaultFieldsValidation = false, two rows in one document, FieldValidate = Yes, a field element with FieldValidate = No or into a field with ValidateTableRelation = false, other field types and relation shapes.",
    awaiting: Awaiting {
        issues: &[7863],
        files: &["tests/al-language/xmlports/import/XpAfterAssignTableRelation.al"],
        since: "2026-11-12",
    },
}
