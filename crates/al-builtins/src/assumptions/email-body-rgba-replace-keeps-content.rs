EmailBodyRgbaReplaceKeepsContent => {
    id: "email-body-rgba-replace-keeps-content",
    assumed: "The static DotNet Regex.Replace(Body, Pattern, Replacement) with exactly the pattern and replacement of Email Message Impl. ReplaceRgbaColorsWithRgb (System Application) maps a text that holds rendered content to a text that is not empty; every mark in the result gets the changed form (ADR-040 amendment 2026-11-12, P12).",
    tests: "al-runner issue 6924: Email Message.Create calls it on every body the reporter's tests pass (they pass on BC).",
    probe: "Corpus pull request 1362 (branch al-runtime-rs-staging-opaque-text, commit 928b6a93, open; measured on the two older BC majors, the newest not yet), reports/layouts-html-text/HtRenderedTextTests.al:274-317 Html_Body_EmailMessageAndEmailItem: Email Message.Create(.., T, true) then GetBody() <> '' and = T, a body with a signature appended, Email Item.SetBody and GetBodyTextFromBlob = T.",
    awaiting: Awaiting {
        issues: &[6924],
        files: &["tests/al-language/reports/layouts-html-text/HtRenderedTextTests.al"],
        since: "2026-11-12",
    },
}
