SaveasPlatformErrorBeforeTriggers => {
    id: "saveas-platform-error-before-triggers",
    assumed: "The platform's own check of a Report.SaveAs comes before any trigger runs, in both call forms, for these shapes of the static Report.SaveAs(Id, '', Format, OutStream) with no layout selection row and no custom layout: as Pdf or Word of a report with one data item, no report extension and one Excel layout in its rendering section named by DefaultRenderingLayout, the statement raises ReportUnsupportedFormat ('The Pdf output format is not supported by the current report layout format.', Word likewise), as the Boolean form answers false with it (pinned for the Boolean form); as Word of a processing-only report with no layout and no report extension, the statement raises ReportProcessingOnly ('Report <id> is a processing-only report. Output cannot be rendered and saved to a file.'), as the Boolean form answers false with it, and no trigger runs (the Boolean form is pinned, streams/PnH6SaveAs.al:196). A subscriber the test can see to a layout event other than SubstituteReport and OnDocumentReady refuses. Any other layout, extension, format, target or call form keeps its earlier answer or is refused.",
    tests: "No Microsoft test: al-runner issue 8486, item 3. The probe's cells gave the same texts on every supported major in the corpus pull request's CI.",
    probe: "tests/al-language/reports/layouts-saveas/SaveAsTriggerErrors.al (corpus pull request 1579, merged, not in the pinned corpus yet): codeunit 77388 SaveAs_Excel_* (:604-683) and SaveAs_ProcessingOnly_* (:684-763), each error site and the control in six calls: Pdf, Word and Xml, as a statement inside Codeunit.Run and in the Boolean form.",
    awaiting: Awaiting {
        issues: &[8486],
        files: &["tests/al-language/reports/layouts-saveas/SaveAsTriggerErrors.al"],
        since: "2026-11-27",
    },
}
