XmlportTemporaryImportNotValidated => {
    id: "xmlport-temporary-import-not-validated",
    assumed: "An XmlPort import declaring exactly Direction = Import, Encoding = UTF8 and FormatEvaluate = Xml (Format = Xml, UseRequestPage = false and inert properties allowed) of a table element right below the root text element that declares only UseTemporary = true, with no triggers besides OnBeforeInsertRecord and OnAfterInsertRecord, a port without triggers, a root text element without properties or triggers that holds only that table element, and exactly two nodes in the table element, both field nodes of it without triggers and without nodes below: a field attribute without properties and the measured node (an XmlName equal to a node's own name counts as no property): a field node into a Code[20] field with one TableRelation to the Code[20] one-field primary key of a table, without conditions or where(), no table extension validate trigger and no validate event subscriber, is not validated (no TableRelation check, no OnValidate); the imported value is stored as it is. Measured combinations only: a field with OnValidate (attribute without properties or with FieldValidate = Yes; element without properties) and a field without OnValidate (attribute without properties). A second row in one document refuses. Every other shape keeps main's result, which validates.",
    tests: "None known in the Microsoft buckets; the layout of al-runner issue 7863 without OnAfterAssignField.",
    probe: "xmlports/import/XpAfterAssignTableRelation.al:1667, :1700, :1720, :1766 (an existing, a missing and a blank value each). Corpus pull request 1366. Not measured: two rows in one document, Xmlport.Import with a record argument, a field declared by a table extension, other field types and lengths, an element with FieldValidate = Yes, an element into the field without OnValidate, ValidateTableRelation = false, DefaultFieldsValidation = false, other formats and table element properties.",
    awaiting: Awaiting {
        issues: &[7863],
        files: &["tests/al-language/xmlports/import/XpAfterAssignTableRelation.al"],
        since: "2026-11-12",
    },
}
