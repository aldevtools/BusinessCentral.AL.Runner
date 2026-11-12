ReportHtmlRenderedText => {
    id: "report-html-rendered-text",
    assumed: "A whole text read (InStream.Read(Text) without a length and its return value unused, BigText.Read) at position 0 of the Html placeholder of a Word layout report gives a text that holds only the mark of rendered content: known to be not empty, nothing else known (ADR-040 amendment 2026-11-12, Part A). A body that only moves through Text values, TextBuilder, BigText, Variants, collection values and UTF-8 streams never changes what the test can see; every other use refuses.",
    tests: "al-runner issue 6924: the reporter's e-mail body tests save a Word layout report as Html through Report Selections.SaveReportAsHTML, read it with Email Item.GetBodyText and pass it to Email Message.Create (they pass on BC).",
    probe: "Windows tier: a Word layout report saved as Html to a Temp Blob, InStream.Read(Text), then T <> '', Format(T) = T, TextBuilder Append/ToText = T, a UTF-8 stream round trip = T, a Blob field stored and read back = T; Email Message.Create(.., T, true) then GetBody() <> ''; TestField.Value of a MultiLine and a RichContent control over a long Text; the same read after a run that ends by CurrReport.Quit or has no dataset row.",
    awaiting: Awaiting {
        issues: &[6924],
        files: &["tests/al-language/reports/layouts-saveas/RenderedHtmlText.al"],
        since: "2026-11-12",
    },
}
