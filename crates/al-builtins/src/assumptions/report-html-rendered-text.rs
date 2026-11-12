ReportHtmlRenderedText => {
    id: "report-html-rendered-text",
    assumed: "A whole text read (InStream.Read(Text) without a length and its return value unused, BigText.Read) at position 0 of the Html placeholder of a Word layout report gives a text that holds only the mark of rendered content: known to be not empty, nothing else known (ADR-040 amendment 2026-11-12, Part A). A body that only moves through Text values, TextBuilder, BigText, Variants, collection values and UTF-8 streams never changes what the test can see; every other use refuses.",
    tests: "al-runner issue 6924: the reporter's e-mail body tests save a Word layout report as Html through Report Selections.SaveReportAsHTML, read it with Email Item.GetBodyText and pass it to Email Message.Create (they pass on BC).",
    probe: "Corpus pull request 1362 (branch al-runtime-rs-staging-opaque-text, commit 928b6a93, open; measured on BC 27.5 and 28.5, BC 29.0 not yet), reports/layouts-html-text/HtRenderedTextTests.al: Html_SaveAs_ReadWhole :98-153 (the read is whole, not empty, EOS after it), Html_Body_TextMoves :155-203 (Format, TextBuilder, StrSubstNo %1, Variant, List, Dictionary, BigText.GetSubText keep the text; T = known text No), Html_Body_StreamAndBlobRoundTrips :222-272 (UTF-8 stream and Blob field round trips keep it), FormatBigText_* :690-736. The FieldValue_* tests :645-679 (TestField.Value of plain, MultiLine and RichContent controls) are still in observe form: not measured.",
    awaiting: Awaiting {
        issues: &[6924],
        files: &["tests/al-language/reports/layouts-html-text/HtRenderedTextTests.al"],
        since: "2026-11-12",
    },
}
