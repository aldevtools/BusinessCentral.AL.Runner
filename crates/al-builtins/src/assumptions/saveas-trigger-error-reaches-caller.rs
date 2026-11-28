SaveasTriggerErrorReachesCaller => {
    id: "saveas-trigger-error-reaches-caller",
    assumed: "An error a report trigger raises inside Report.SaveAs reaches the caller: the statement form raises it, and the Boolean form answers false with that error's code and text in GetLastErrorCode and GetLastErrorText. Only for the static Report.SaveAs(Id, '', Format, OutStream) as Pdf, Word or Xml, of a report that is not processing-only, has one data item and declares one layout in its rendering section, named by DefaultRenderingLayout, of Type Word, or of Type RDLC with one report extension that only adds columns (a Word layout: with no extension or that one extension), or, as Xml only, of Type Excel with no extension; the error is Error('literal') (code Dialog) or a TestField (code TestField), raised by the report's own OnPreReport, OnPreDataItem, OnAfterGetRecord or OnPostReport (also through a procedure the trigger calls). Any other error, trigger, layout, extension, format, target or call form keeps its earlier answer or is refused.",
    tests: "No Microsoft test: al-runner issue 8486, item 3 (an error inside SaveAs of a report with a layout and an extension). The probe's cells gave the same texts on every supported major in the corpus pull request's CI.",
    probe: "tests/al-language/reports/layouts-saveas/SaveAsTriggerErrors.al (corpus pull request 1579, merged, not in the pinned corpus yet): codeunit 77388 SaveAs_Word_* (Word layout), the Xml calls of SaveAs_Excel_* (Excel layout), and codeunit 77395 SaveAs_RdlcExtColumn_*, SaveAs_WordExtColumn_* (a report extension that only adds a column), each site in six calls: Pdf, Word and Xml, as a statement inside Codeunit.Run and in the Boolean form, each followed by GetLastErrorCode and GetLastErrorText.",
    awaiting: Awaiting {
        issues: &[8486],
        files: &["tests/al-language/reports/layouts-saveas/SaveAsTriggerErrors.al"],
        since: "2026-11-27",
    },
}
