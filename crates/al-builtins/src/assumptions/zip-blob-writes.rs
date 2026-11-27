ZipBlobWrites => {
    id: "zip-blob-writes",
    assumed: "Three .NET writes into a Blob through the stream BC hands .NET for an AL OutStream. (1) ZipArchive(OutStream, ZipArchiveMode.Update), with or without leaveOpen false and Encoding.UTF8, over an OutStream that CreateOutStream made on a Blob that holds content and that nothing wrote to since: the archive reads that content, and its Dispose makes the Blob the archive's whole content, when AL did not change the Blob before the Dispose. (2) ZipArchive(OutStream, Update, false, UTF8) over an OutStream that CreateOutStream opened on a Blob field of a temporary record (declared temporary, or of a TableType = Temporary table) and that AL copied a zip into, when no read or Insert gave the record that Blob: its Dispose makes the Blob the archive's whole content also after AL changed the Blob, in four states only: Clear of the Blob; Clear and a rewrite of the Blob with as many bytes as the archive read; that rewrite and then bytes .NET appended through another stream of the Blob; the Dispose of another such archive over the same OutStream. (3) A System.IO.StreamWriter made with an OutStream alone, over a Blob that was empty when the stream was handed to .NET: after AL set the stream's Position to 0, or while AL calls its SetLength, a write over the bytes it had handed back goes over them in the Blob, and SetLength cuts the Blob.",
    tests: "None of Microsoft's tests; al-runner issues 9246, 9291 and 7947.",
    probe: "Corpus pull request 1633 (merged, not pinned yet), tests/al-language-onprem-dotnet/dotnet/zip-blob-writes/ZipBlobUpdateFreshOutStream.al:49-223 (part 1), ZipBlobUpdateBlobChanged.al:48-152 (part 2), ZipBlobDotNetOverwrite.al:25-96 (part 3); the same values on BC 27.5, 28.5 and 29.0.",
    awaiting: Awaiting {
        issues: &[9246, 9291, 7947],
        files: &[
            "tests/al-language-onprem-dotnet/dotnet/zip-blob-writes/ZipBlobUpdateFreshOutStream.al",
            "tests/al-language-onprem-dotnet/dotnet/zip-blob-writes/ZipBlobUpdateBlobChanged.al",
            "tests/al-language-onprem-dotnet/dotnet/zip-blob-writes/ZipBlobDotNetOverwrite.al",
        ],
        since: "2026-11-27",
    },
}
