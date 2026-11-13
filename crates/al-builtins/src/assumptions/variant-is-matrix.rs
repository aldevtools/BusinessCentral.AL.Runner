VariantIsMatrix => {
    id: "variant-is-matrix",
    assumed: "Variant.IsTransactionType is true for a value whose declared type is TransactionType (a TransactionType variable, the literal `TransactionType::X`, `CurrentTransactionType()`; al-sema marks it where it goes into a Variant), Variant.IsDataClassificationType for a DataClassification value, Variant.IsSecurityFiltering for none; for every other value the probe holds they answer false (al-builtins native/variant_is.rs). The three methods refuse a value of a kind the probe does not hold (a DotNet value, a Media, a Blob, an HttpClient, an XmlNode that holds no element, an Option whose members are a platform option's but whose declared type is not TransactionType, and so on). Variant.IsOption of a value whose declared type is TransactionType is false; an Option with the same members (an app Option, the TransactionType field of XmlPort Metadata or Report Metadata) keeps IsOption true, unflagged. The default TransactionType value's IsTransactionType is pinned (types/TestVariantExtended.al:238) and not flagged.",
    tests: "No Microsoft test. Outside report: al-runner issue 7963 (the reporter's own repro, a Variant holding TransactionType::Update).",
    probe: "Corpus PR 1382, merged on corpus master as 23337cd2, not pinned yet (types/variant/VariantIsMatrix.al:135-493): every IsXxx method asked of a Variant holding each kind, the same on every BC major of the probe.",
    awaiting: Awaiting {
        issues: &[7963],
        files: &["tests/al-language/types/variant/VariantIsMatrix.al"],
        since: "2026-11-13",
    },
}
