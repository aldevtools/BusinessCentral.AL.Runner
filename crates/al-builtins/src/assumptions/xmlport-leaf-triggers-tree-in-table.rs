XmlportLeafTriggersTreeInTable => {
    id: "xmlport-leaf-triggers-tree-in-table",
    assumed: "An XmlPort export of a container text element right below a table element, with no trigger of its own and no trigger on a container below it, whose nodes are text attributes, leaf text elements (with or without OnBeforePassVariable), text elements of only text attributes and containers of the same kind, writes it as any container: its nodes in schema order, each leaf's OnBeforePassVariable before the leaf, Skip there leaves out only that leaf. Its own text stays refused.",
    tests: "Microsoft's PEPPOL Tests of the newest major, 48 tests of codeunits 139235 and 139236 that export Sales Invoice - PEPPOL BIS 3.0: TaxSubtotal > TaxCategory (ID, Percent, TaxExemptionReason with Skip when empty, TaxScheme > ID); they pass in Microsoft's CI.",
    probe: "R6924eTreeInTable.al on corpus branch al-runtime-rs-staging-r6924e: the PEPPOL TaxCategory shape below an inner Integer table element and below the root table element, reason filled and empty, an empty leaf of the inner TaxScheme, the trigger order.",
    awaiting: Awaiting {
        issues: &[6924],
        files: &["tests/al-language/xmlports/shapes-properties/R6924eTreeInTable.al"],
        since: "2026-11-04",
    },
}
