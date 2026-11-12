XmlportTextElemFieldAttrExport => {
    id: "xmlport-text-elem-field-attr-export",
    assumed: "An XmlPort export declaring exactly Direction = Export and FormatEvaluate = Xml (Format = Xml, UseRequestPage = false and inert properties allowed; no Encoding, no namespace) of a text element declaring MaxOccurs = Once and at most MinOccurs = Zero, holding one field attribute without properties or triggers of a Normal Boolean field (with the trigger also Text) of its table element and nothing else, the only such text element of a temporary table element that declares exactly MinOccurs = Zero and UseTemporary = true and stands right below the root text element, with no trigger or only OnBeforePassVariable: the element is always written, with the attribute and, when the text is not empty, the text (`<MinValue equalsValid=\"true\">5</MinValue>`, `<MinValue equalsValid=\"true\" />`); the attribute value is read after OnBeforePassVariable. Skip, Break or Quit in that trigger refuses. Every other shape keeps main's result: with the trigger, al-sema refuses running the port.",
    tests: "al-runner issue 7865: a device screen XmlPort of an app the reporter depends on.",
    probe: "xmlports/shapes-properties/XpTextElemFieldAttrExport.al:468-533 (attribute with and without the trigger, MinOccurs Zero and not set, the trigger changing the attribute's field, a blank Text attribute, the text never assigned; one and three rows). Corpus pull request 1366.",
    awaiting: Awaiting {
        issues: &[7865],
        files: &["tests/al-language/xmlports/shapes-properties/XpTextElemFieldAttrExport.al"],
        since: "2026-11-12",
    },
}
