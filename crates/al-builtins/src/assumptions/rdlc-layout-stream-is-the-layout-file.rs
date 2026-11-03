RdlcLayoutStreamIsTheLayoutFile => {
    id: "rdlc-layout-stream-is-the-layout-file",
    assumed: "Report.RdlcLayout, as a statement, of a report whose DefaultRenderingLayout is an RDLC layout with a file the app holds gives a stream of the file's bytes unchanged. MEASURED WRONG by corpus writer Y86 (measured on two BC majors, corpus probe report/lt-y86-request-page/Y86LayoutStreams.al, not on corpus master yet): the stream is the file with its <Fields> list replaced by the report's data set columns (a 2316-byte file with four fields gives 1917 bytes and <Fields /> for a report without columns); implement that when the probe lands. Pinned only that the stream has a value and that a Tenant Report Layout row with it as its Layout inserts (report/lt-sc-report-metadata-views/Y6TenantReportLayoutRows.al:35-89, corpus master).",
    tests: "No Microsoft test: corpus PR 891 (Y6TenantReportLayoutRows.al) reads the stream only through Media.ImportStream.",
    probe: "Report.RdlcLayout of a report with an RDLC layout file, the stream read with ReadText and compared with the file's text (and its length).",
}
