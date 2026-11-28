ZipBlobWrites => {
    id: "zip-blob-writes",
    assumed: "Three .NET writes into a Blob through the stream BC hands .NET for an AL OutStream. (1) ZipArchive(OutStream, ZipArchiveMode.Update), made with two arguments, with leaveOpen false, or with leaveOpen false and Encoding.UTF8, over the OutStream made by the CreateOutStream that is the newest change of a Blob that holds content: the archive reads that content, and its Dispose makes the Blob the archive's whole content, when AL did not change the Blob before the Dispose. (2) ZipArchive(OutStream, Update, false, Encoding.UTF8) over an OutStream that CreateOutStream opened on a Blob field of a temporary record (declared temporary, or of a TableType = Temporary table) and that AL copied a zip into, when no read or Insert had given the record that Blob when the archive was made: its Dispose makes the Blob the archive's whole content also after AL changed the Blob, in four states only: a Clear of the Blob as AL's only change; a Clear, then one CreateOutStream and one CopyStream(OutStream, InStream) through that new OutStream of as many bytes as the archive read, as AL's only changes; that rewrite and then bytes .NET appended through another stream of the Blob; the Dispose of another such archive over the same OutStream with no AL change before it. (3) A System.IO.StreamWriter made with an OutStream alone, the only .NET object made with that stream, over a Blob that was empty when the stream was handed to .NET, once per stream, while the stream holds just the bytes it handed back: a write of that StreamWriter inside AL's call of its Write(Text) or Flush() that starts at 0 right after AL set the stream's Position to 0 and ends at the stream's end goes over them in the Blob; AL's SetLength(10) on the stream standing at its end cuts the Blob to 10 bytes.",
    tests: "None of Microsoft's tests; al-runner issues 9246, 9291 and 7947.",
    probe: "Corpus pull request 1633 (merged, not pinned yet), tests/al-language-onprem-dotnet/dotnet/zip-blob-writes/ZipBlobUpdateFreshOutStream.al:49-223 (part 1), ZipBlobUpdateBlobChanged.al:48-152 (part 2), ZipBlobDotNetOverwrite.al:25-96 (part 3); the same values on every BC version the corpus runs.",
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
