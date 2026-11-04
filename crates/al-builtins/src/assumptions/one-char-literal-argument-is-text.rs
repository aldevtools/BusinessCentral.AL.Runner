OneCharLiteralArgumentIsText => {
    id: "one-char-literal-argument-is-text",
    assumed: "A text literal of one character handed to a by-value Variant parameter of an AL procedure is a Text in the Variant (IsText, not IsChar), unlike V := 'a', which is a Char. Measured on the newest major by the corpus writer's observe round for al-runner issue 6924; the pin is not on master yet.",
    tests: "No Microsoft test: a partner app's JSON helper that switches on IsChar, called with ',' and '.' (2 tests, al-runner issue 6924).",
    probe: "VaTIsChecks.al Is_OneCharacter_PassedToVariantParameter and JsonObjectBasics.al QW_Add_CharAndText_ThroughVariant on branch al-runtime-rs-staging-p6924c.",
    awaiting: Awaiting {
        issues: &[6924],
        files: &[
            "tests/al-language/types/variant/VaTIsChecks.al::Is_OneCharacter_PassedToVariantParameter",
            "tests/al-language/json/JsonObjectBasics.al::QW_Add_CharAndText_ThroughVariant",
        ],
        since: "2026-11-04",
    },
}
