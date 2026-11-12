DotNetFileDeletePlaceholder => {
    id: "dotnet-file-delete-placeholder",
    assumed: "DotNet System.IO.File.Delete of a file that holds a placeholder report output (a rendering save of a report written to the file, or such a buffer copied into it) deletes the file as any file: no error, File.Exists answers No afterwards, and the bytes copied out of the file before the delete stay (ADR-040). This is the last step of Base Application's e-mail body round trip through a server file (Report Selections GetEmailBodyForVend / GetEmailBodyTextForCust, `#if not CLEAN27`: File Management TempBlobToServerFile, BLOBImportFromServerFile, DeleteServerFile).",
    tests: "Outside reporter's suite (al-runner issue 6924, comment of 2026-11-12 18:46 UTC): 218 + 8 tests stop at DeleteServerFile; they pass on BC. Corpus probe pull request PROBE_PR, group R.",
    probe: "Corpus probe pull request PROBE_PR, group R: a Word layout report saved as Html and as Pdf into a Temp Blob, exported to a server file, imported back, deleted with DotNet System.IO.File.Delete; HasValue, File.Exists and byte identity with a direct copy.",
    awaiting: Awaiting {
        issues: &[6924],
        files: &["PROBE_FILE"],
        since: "2026-11-12",
    },
}
