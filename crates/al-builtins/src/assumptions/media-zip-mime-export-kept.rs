MediaZipMimeExportKept => {
    id: "media-zip-mime-export-kept",
    assumed: "The measured Word package (a zip whose central directory lists exactly `[Content_Types].xml`, `_rels/.rels` and `word/document.xml`, in that order, each deflated) imported by a three-argument Media.ImportStream as `application/zip` into a Media field that held no media is stored as it is and ExportStream writes its bytes back (al-builtins `media_export`). Refused: other content with that MIME type, a fourth argument (a blank one too), a field that held a media, a MediaSet.",
    tests: "No Microsoft test. Measured on Windows BC of every supported major (sandbox containers, 2026-11-19) by corpus PR 1469 (merged, not pinned yet).",
    probe: "Corpus PR 1469 (merged), streams/media/MediaDocxRezip.al:677 Docx_MediaImportStream_ZipMime (the call at :690, into a new row); facts header :19-20.",
    awaiting: Awaiting {
        issues: &[8519],
        files: &["tests/al-language-onprem/streams/media/MediaDocxRezip.al"],
        since: "2026-11-20",
    },
}
