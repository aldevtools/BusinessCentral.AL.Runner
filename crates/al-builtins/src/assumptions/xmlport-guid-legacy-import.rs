XmlportGuidLegacyImport => {
    id: "xmlport-guid-legacy-import",
    assumed: "A Format = Xml XmlPort without FormatEvaluate imports into a Guid field the text its own export writes for a Guid: the 8-4-4-4-12 digits in upper case inside braces (export pinned by xmlports/shapes-properties/BdXmlPort.al:425, :436), as the Guid it names. Every other text (no braces, lower case, white space around it, empty, invalid) and the text formats keep main's result.",
    tests: "System Application tests, Permission Import Export Tests (ExportImportSingleSystemPermissions and six more): the Import Permission Sets XmlPort (Format = Xml, no FormatEvaluate) reads back the App ID attributes its export wrote, and the tests pass on real BC (al-runner issue 7854).",
    probe: "xmlports/import/XpLegacyGuidImport.al: a Legacy Xml port importing a Guid as field attribute and field element in the export form, the null Guid, lower case, no braces, parentheses, 32 digits, empty, missing, and an invalid text; the same through VariableText. Corpus pull request 1340.",
    awaiting: Awaiting {
        issues: &[7854],
        files: &["tests/al-language/xmlports/import/XpLegacyGuidImport.al"],
        since: "2026-11-12",
    },
}
