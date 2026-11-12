XmlportGuidLegacyImport => {
    id: "xmlport-guid-legacy-import",
    assumed: "An XmlPort without FormatEvaluate imports a text into a Guid field as its probe measured: in a Format = Xml field attribute the 8-4-4-4-12 digits in braces (upper case, lower case, with spaces around in upper case, the null Guid), without braces (upper and lower case), in parentheses (upper case) and the 32 digits alone (upper case) give the Guid, and the empty text the null Guid; in a Format = Xml field element and a VariableText field the form in braces (upper case, lower case, the null Guid) and without braces (upper case) give the Guid, and the empty text fails. A text with a character that is no hex digit, dash or bracket fails at all three places, and in an attribute the form in braces one digit short fails, with the InvalidGuidFormat error. Every other text, and FixedText, keep main's refusal.",
    tests: "System Application tests, Permission Import Export Tests (ExportImportSingleSystemPermissions and six more): the Import Permission Sets XmlPort (Format = Xml, no FormatEvaluate) reads back the App ID attributes its export wrote, and the tests pass on real BC (al-runner issue 7854). The probe's values come from a local BC run and its green corpus CI on every supported major.",
    probe: "xmlports/import/XpLegacyGuidImport.al: a Legacy Xml port importing a Guid as field attribute and field element in the export form, the null Guid, lower case, no braces, parentheses, 32 digits, empty, missing, and an invalid text; the same through VariableText. Corpus pull request 1340.",
    awaiting: Awaiting {
        issues: &[7854],
        files: &["tests/al-language/xmlports/import/XpLegacyGuidImport.al"],
        since: "2026-11-12",
    },
}
