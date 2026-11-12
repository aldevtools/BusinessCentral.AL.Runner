ReportHtmlPlaceholder => {
    id: "report-html-placeholder",
    assumed: "Report.SaveAs to Html and Report.SaveAsHtml of a report whose default layout is Word run the report as the Xml path does, answer true and write a placeholder html page instead of BC's rendering (owner decision 2026-11-11, ADR-040 amendment). Only 'it exists, it is markup, it is not empty' is meant to match BC; reading the Html into a Text (an e-mail body) is refused. The layout events BC raises around the render are not raised, and a subscriber the test can see refuses the save.",
    tests: "reports/layouts-saveas/AMReportOutputTests.al:205 WordLayout_SaveAsHtml and reports/CiReportRunLeads.al:606-610 (true, length > 0) for the static stream form on the Linux tier; an outside reporter's e-mail body reports that copy the Html into an attachment.",
    probe: "A corpus probe on the Windows nightly: SaveAsHtml(FileName) of a Word layout report, then File.Exists and the size; the variable form.",
}
