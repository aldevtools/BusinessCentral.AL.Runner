PageVarValidateCodeChangeSaved => {
    id: "page-var-validate-code-change-saved",
    assumed: "A change a page variable control's OnValidate makes to Rec in code is part of the row's edit: the next page variable SetValue, an action, Next or Close saves it (on a Worksheet, Card and List). Measured on SaaS (newest major) by corpus writer Y65; not on the pinned corpus yet.",
    tests: "No Microsoft test: al-runner issue 6669 (a Secret field's OnValidate writes Rec.\"Secret Key\", the next field's OnValidate reads the row with Get).",
    probe: "Y65CodeChangeNextField.al (corpus PR 1012).",
    awaiting: Awaiting {
        issues: &[6669],
        files: &["tests/al-language/pages/lt-y65-code-change-next-field/Y65CodeChangeNextField.al"],
        since: "2026-11-01",
    },
}
