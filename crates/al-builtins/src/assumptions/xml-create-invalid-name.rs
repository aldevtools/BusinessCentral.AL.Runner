XmlCreateInvalidName => {
    id: "xml-create-invalid-name",
    assumed: "XmlElement.Create(Name), XmlElement.Create(Name, Ns), XmlAttribute.Create(Name, Value) and XmlAttribute.Create(Name, Ns, Value) with a name that is not a plain XML name raise, for the cells the probe measured (al-builtins xml/create_names.rs): an empty name the NavNCLArgumentException error, code Argument; a name that begins with '1' (every entry point), '9' (both forms without a namespace), '.' or ':' (XmlElement.Create(Name) only), and a name whose first bad character is ':' or ' ' after ASCII letters (every entry point), the XmlException text with that character and its position, code Xml:Xml. The namespace forms count only with the namespace 'urn:x', and XmlElement.Create('p:name', '') with the empty one. The cells that xml/XmlElement.al:98-141 and xml/XmlAttributes.al:104-131 pin for the forms without a namespace (also '-a', 'a<b', 'a/b', 'a&b' for XmlElement.Create and 'a=b' for XmlAttribute.Create) raise the same way unflagged. Every other name that is not a plain name stays refused; a call with content arguments is not covered.",
    tests: "No Microsoft test. Outside report: al-runner issue 7964 item 1 (XmlElement.Create with '1.0' and a URN as the name).",
    probe: "Corpus PR 1382, merged on corpus master as 23337cd2, not pinned yet (xml/XmlNameValidity.al:113-241), the same on BC 27.5, 28.5 and 29.0.",
    awaiting: Awaiting {
        issues: &[7964],
        files: &["tests/al-language/xml/XmlNameValidity.al"],
        since: "2026-11-13",
    },
}
