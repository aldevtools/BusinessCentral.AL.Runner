ReportParamsWrittenDeclaration => {
    id: "report-params-written-declaration",
    assumed: "Report.SaveAs reads a parameters string that starts with the declaration XmlDocument.WriteTo(Text) writes (`<?xml version=\"1.0\" encoding=\"utf-16\" standalone=\"yes\"?>`) and holds a root without name and id with one Options element and no DataItems, either on one line with nothing between the elements or in exactly the layout WriteTo writes (CR LF before every element, two spaces per depth), as it reads the same string with the plain declaration and no whitespace: a Field sets its variable, an empty one blanks it. Every other break or space between the elements, other declarations, a named root, DataItems, Execute and RunRequestPage keep their path.",
    tests: "Outside reporter's repro on BC (al-runner issue 7908): WriteTo(Text) of a read and filled template gives StartDate 2026-01-15 and NoteText blank; the one-line utf-16 declaration gives StartDate 2026-01-15 and NoteText 'init'. Corpus probe branch al-runtime-rs-writer-reports6 (pull request 1368), reports/request-page/ReportParametersStrings.al.",
    probe: "Corpus probe branch al-runtime-rs-writer-reports6 (pull request 1368), reports/request-page/ReportParametersStrings.al: Report.SaveAs with the utf-16 declaration on one line, the WriteTo(Text) result of a filled template, an utf-8 declaration, no standalone, hand-indented text, the same strings to Execute and RunRequestPage, and a named root.",
    awaiting: Awaiting {
        issues: &[7908],
        files: &["tests/al-language/reports/request-page/ReportParametersStrings.al"],
        since: "2026-11-12",
    },
}
