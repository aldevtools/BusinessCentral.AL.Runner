VariantIsMatrix => {
    id: "variant-is-matrix",
    assumed: "Variant.IsTransactionType, IsDataClassificationType, IsSecurityFiltering, IsJsonArray, IsJsonObject, IsJsonToken, IsJsonValue and the sixteen Variant.IsXml methods answer true only for a value of their own kind: a TransactionType value is IsTransactionType, a DataClassification value is also IsDataClassificationType, an XmlNode that holds an element is IsXmlElement and no IsXmlNode, IsSecurityFiltering is never true; for every other value the probe holds they answer false (al-builtins native/variant_is.rs). The methods refuse a value of a kind the probe does not hold (a DotNet value, a Media, an HttpClient, an XmlNode that holds no element, and so on).",
    tests: "No Microsoft test. Outside report: al-runner issue 7963 (the reporter's own repro, a Variant holding TransactionType::Update).",
    probe: "Corpus PR 1382, merged on corpus master as 23337cd2, not pinned yet (types/variant/VariantIsMatrix.al:135-493): every IsXxx method asked of a Variant holding each kind, the same on BC 27.5, 28.5 and 29.0.",
    awaiting: Awaiting {
        issues: &[7963],
        files: &["tests/al-language/types/variant/VariantIsMatrix.al"],
        since: "2026-11-13",
    },
}
