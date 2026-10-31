RdlcLayoutStreamIsTheLayoutFile => {
    id: "rdlc-layout-stream-is-the-layout-file",
    assumed: "Report.RdlcLayout, as a statement, of a report whose DefaultRenderingLayout is an RDLC layout with a file the app holds gives a stream of the file's bytes unchanged. Pinned only that the stream has a value and that a Tenant Report Layout row with it as its Layout inserts (report/lt-sc-report-metadata-views/Y6TenantReportLayoutRows.al:35-89, corpus master).",
    tests: "No Microsoft test: corpus PR 891 (Y6TenantReportLayoutRows.al) reads the stream only through Media.ImportStream.",
    probe: "Report.RdlcLayout of a report with an RDLC layout file, the stream read with ReadText and compared with the file's text (and its length).",
}
