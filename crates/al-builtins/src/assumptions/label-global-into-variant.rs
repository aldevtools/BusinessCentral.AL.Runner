LabelGlobalIntoVariant => {
    id: "label-global-into-variant",
    assumed: "A global Label declared as just `Label 'text'` (no Locked, MaxLength or Comment) assigned with `:=` to a non-array Variant variable that the routine's `var` section declares makes the Variant hold a TextConstant: IsTextConstant is true and every other Is... test false (al-sema lower/label_variant.rs). A local Label and a TextConst in the same assignment are pinned (types/TestTernaryContracts.al:564, :572; types/variant/VaTTextConst.al:287, :289) and not flagged. A global Label handed to a Variant parameter, assigned to a global Variant, a parameter or an array element keeps the Text conversion.",
    tests: "No Microsoft test. Outside report: al-runner issue 7987 (a global Label in a Variant answered IsText).",
    probe: "Corpus PR 1382, merged on corpus master as 23337cd2, not pinned yet (types/variant/VariantIsMatrix.al:224 with :242: `V := SomeLabel` with a global Label answers only IsTextConstant, on every BC major of the probe).",
    awaiting: Awaiting {
        issues: &[7987],
        files: &["tests/al-language/types/variant/VariantIsMatrix.al"],
        since: "2026-11-13",
    },
}
