XmlportRootTableNested => {
    id: "xmlport-root-table-nested",
    assumed: "XmlPort import (Format = Xml, Direction not set, FormatEvaluate = Xml, no namespaces) of a port whose root is a table element (MinOccurs and MaxOccurs Once, no triggers, fields or LinkTable of its own, temporary or real) holding one temporary header table element (MaxOccurs = Unbounded, only OnAfterInitRecord and OnAfterInsertRecord) of leaf field and text elements without triggers and, last, at most one text element that wraps a temporary line table element (only OnBeforeInsertRecord, leaf field elements). Imported as any nested table element: the header's OnAfterInitRecord before its elements are read, text element variables never cleared between headers, the wrapper's variable stays empty, the lines inserted before the header row and its OnAfterInsertRecord, the root row (key blank) stored last, schema errors after the header's OnAfterInitRecord, a failed guarded import returns false without OnPostXmlPort and with no row stored, a Decimal without whole digits (`.5`) imports. A failure after a row was stored in a temporary table is refused.",
    tests: "No corpus test in the pinned oracle yet. al-runner issue 6924: the outside reporter's import of order messages through such a port (about 20 of his tests) passes on real BC. Measured in corpus PR 1308 (merged on master).",
    probe: "xmlports/shapes-properties/XpRootTableNested.al (corpus PR 1308, 11 tests, measured on every supported BC major): remove this entry and cite the file once the corpus bump brings it into the oracle.",
    awaiting: Awaiting {
        issues: &[6924],
        files: &["tests/al-language/xmlports/shapes-properties/XpRootTableNested.al"],
        since: "2026-11-11",
    },
}
