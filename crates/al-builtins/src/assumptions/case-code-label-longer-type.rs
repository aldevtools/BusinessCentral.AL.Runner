CaseCodeLabelLongerType => {
    id: "case-code-label-longer-type",
    assumed: "A case label of type Code[n] with a selector of type Code[m], n > m, is converted to the selector's Code as a Text label is: a value of at most m characters compares as it is, a longer one raises StringLengthExceeded (text in upper case) when the label is reached, also when a later label would match; it is never cut.",
    tests: "No Microsoft test: al-runner issue 6491, an ISV test app (147 tests refused at compile time). Measured on BC by corpus writer Y20 (Y20CaseCodeLabel.al, SaaS sandbox, newest major).",
    probe: "Y20CaseCodeLabel.al on branch al-runtime-rs-staging-y20: values that fit, a 15-character label before and after the matching one, an 11-character label against its 10-character prefix, a Code[20] selector with Code[10] labels.",
    awaiting: Awaiting {
        issues: &[6491],
        files: &["tests/al-language/types/lt-y20-case-code-label/Y20CaseCodeLabel.al"],
        since: "2026-10-31",
    },
}
