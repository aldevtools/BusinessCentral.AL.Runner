DotNetFileDeletePlaceholder => {
    id: "dotnet-file-delete-placeholder",
    assumed: "DotNet System.IO.File.Delete of a file that holds a placeholder Html or Pdf report output, written to the file by File.Close from a stream copy (File Management InstreamExportToServerFile, BLOBExportToServerFile) and closed again after any File.Open, deletes the file as any file: no error, File.Exists answers No afterwards, and the bytes copied out of the file before the delete stay (ADR-040). This is the last step of Base Application's e-mail body round trip through a server file (Report Selections GetEmailBodyForVend / GetEmailBodyTextForCust, `#if not CLEAN27`: File Management TempBlobToServerFile, BLOBImportFromServerFile, DeleteServerFile). A Word or Excel placeholder, a file the rendering save wrote itself, and a file an AL File holds open refuse.",
    tests: "Outside reporter's suite (al-runner issue 6924, comment of 2026-11-12 18:46 UTC): 218 + 8 tests stop at DeleteServerFile; they pass on BC. Corpus probe branch al-runtime-rs-writer-reports6 (pull request 1368), al-language-onprem/reports/layouts-saveas/HtmlServerFileRoundTrip.al.",
    probe: "Corpus probe branch al-runtime-rs-writer-reports6 (pull request 1368), al-language-onprem/reports/layouts-saveas/HtmlServerFileRoundTrip.al: a Word layout report saved as Html and as Pdf into a Temp Blob, exported to a server file, imported back, deleted with DotNet System.IO.File.Delete; HasValue, File.Exists and byte identity with a direct copy.",
    awaiting: Awaiting {
        issues: &[6924],
        files: &["tests/al-language-onprem/reports/layouts-saveas/HtmlServerFileRoundTrip.al"],
        since: "2026-11-12",
    },
}
