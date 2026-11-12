XmlportTextElemFieldAttrExport => {
    id: "xmlport-text-elem-field-attr-export",
    assumed: "A Format = Xml XmlPort export (Direction = Export) of a text element with MaxOccurs = Once, MinOccurs = Zero or not set, holding one field attribute of a Boolean or Text field of its table element (no Occurrence, no trigger) and nothing else, right below a temporary table element (UseTemporary) right below the root text element, with no trigger or only OnBeforePassVariable: the element is always written, with the attribute and, when the text is not empty, the text (`<MinValue equalsValid=\"true\">5</MinValue>`, `<MinValue equalsValid=\"true\" />`); the attribute value is read after OnBeforePassVariable. Skip, Break or Quit in that trigger, another attribute type or count, the attribute's own OnBeforePassField, and import refuse.",
    tests: "al-runner issue 7865: a device screen XmlPort of an app the reporter depends on.",
    probe: "xmlports/shapes-properties/XpTextElemFieldAttrExport.al:468-533 (attribute with and without the trigger, MinOccurs Zero and not set, the trigger changing the attribute's field, a blank Text attribute, the text never assigned; one and three rows). Corpus pull request 1366.",
    awaiting: Awaiting {
        issues: &[7865],
        files: &["tests/al-language/xmlports/shapes-properties/XpTextElemFieldAttrExport.al"],
        since: "2026-11-12",
    },
}
