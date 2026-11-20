MediaZipMimeExportKept => {
    id: "media-zip-mime-export-kept",
    assumed: "A Word package imported with Media.ImportStream as `application/zip` with no file name (the measured call has three arguments; a blank fourth argument is answered alike, the stored media does not tell them apart) is stored as it is and ExportStream writes its bytes back (al-builtins `media_export`). Refused: other content with that MIME type, a file name, a MediaSet.",
    tests: "No Microsoft test. Measured on Windows BC of every supported major (sandbox containers, 2026-11-19) by corpus PR 1469 (merged, not pinned yet).",
    probe: "Corpus PR 1469 (merged), streams/media/MediaDocxRezip.al:678 Docx_MediaImportStream_ZipMime; facts header :19-20.",
    awaiting: Awaiting {
        issues: &[8519],
        files: &["tests/al-language-onprem/streams/media/MediaDocxRezip.al"],
        since: "2026-11-20",
    },
}
