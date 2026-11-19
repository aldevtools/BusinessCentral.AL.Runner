OptionFieldRefToTextField => {
    id: "option-fieldref-to-text-field",
    assumed: "On BC 27 through 29, assigning an Option directly from FieldRef.Value() to a record's Text or Code field uses its caption when it fits. Text preserves a blank space; Code trims it and uppercases nonblank captions.",
    tests: "Corpus PR https://github.com/aldevtools/BusinessCentral.AL.Language.Tests/pull/1464, VariantEnumOptionToText.al:FieldRef_Value_OptionToTextAndCode_UsesCaption, measured on BC 27.5.46862.58266, 28.5.54976.58543 and 29.1.57326.58275.",
    probe: "Pin the direct Text/Code field assignments in corpus PR 1464. Length errors are not measured and stay refused.",
    awaiting: Awaiting {
        issues: &[],
        files: &["tests/al-language/types/variant/VariantEnumOptionToText.al::FieldRef_Value_OptionToTextAndCode_UsesCaption"],
        since: "2026-11-19",
    },
}
