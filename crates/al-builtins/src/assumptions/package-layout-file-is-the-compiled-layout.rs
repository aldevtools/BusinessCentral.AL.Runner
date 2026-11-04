PackageLayoutFileIsTheCompiledLayout => {
    id: "package-layout-file-is-the-compiled-layout",
    assumed: "Media.ExportStream of the Layout of a Report Layout List row of a Microsoft app writes the layout file of the app's source archive unchanged. The corpus pins the bytes only for an app whose folder holds the file the compiler wrote at build (reports/layouts-saveas/Y17LayoutExportTests.al:70); BC may export a rewrite of the source file (the data set part of a Word file, the Fields list of an RDLC).",
    tests: "Microsoft tests that call Custom Report Layout.InitBuiltInLayout and need a non-empty layout: Layers/W1/Tests/Integration-Internal/MailServiceTest.Codeunit.al (CreateCustomReportSelection, Type::Word), Layers/*/Tests/Report/ReportLayoutTest.Codeunit.al. None compares the bytes.",
    probe: "Export the Layout of a Report Layout List row of a Base Application report (an RDLC and a Word one) and assert the length and SHA-256; compare with the file in the source archive of the same build.",
}
