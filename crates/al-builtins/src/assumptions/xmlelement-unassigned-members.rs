XmlElementUnassignedMembers => {
    id: "xmlelement-unassigned-members",
    assumed: "On an XmlElement nothing assigned (or cleared), these members in the probe's form raise \"Microsoft.Dynamics.Nav.Runtime.Nav<type> variable not initialized.\" (NotInitialized): InnerXml, GetChildElements(), GetChildNodes and IsEmpty with the result used name XmlElement; SetAttribute('a', 'b') as a statement and Attributes with the result used name XmlAttributeCollection; AsXmlNode with the result used and Remove as a statement name XmlNode. Other arguments and forms keep their answer before this assumption (a refusal).",
    tests: "Outside report: al-runner issue 7760 (the XmlElement cells of its comment of 2026-11-14). Measured on BC 27.5, 28.5 and 29.0 by corpus PR 1408 (merged, not yet pinned).",
    probe: "Corpus PR 1408: tests/al-language/xml/UhUnassignedXmlElement.al (codeunit 66992).",
    awaiting: Awaiting {
        issues: &[7760],
        files: &["tests/al-language/xml/UhUnassignedXmlElement.al"],
        since: "2026-11-15",
    },
}
