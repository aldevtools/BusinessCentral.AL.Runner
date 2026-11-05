InstreamDotnetSeekBack => {
    id: "instream-dotnet-seek-back",
    assumed: "An InStream handed to .NET code after earlier .NET code (or AL) moved its position is handed over whole, standing at that position: .NET can seek back before it and read the stream again, and the AL position follows where .NET left the stream. Excel Buffer's SelectSheetsNameStream then OpenBookStream on one InStream read the workbook twice.",
    tests: "No Microsoft test: al-runner issue 7134 (the reporter's real-BC run of the same-stream Excel Buffer probes on the three supported majors, corpus pull request 1152).",
    probe: "TestExcelStream.Codeunit.al ExcelBuffer_SelectSheetThenRead_TempBlob_SameStream_PreservesCell and _StoredBlob_SameStream_PreservesCell (corpus pull request 1152); R7134ExcelStreamPosition.al (EOS after each call) on corpus branch al-runtime-rs-staging-r7132.",
    awaiting: Awaiting {
        issues: &[7134],
        files: &["tests/al-language/streams/excel-buffer-roundtrip/R7134ExcelStreamPosition.al"],
        since: "2026-11-05",
    },
}
