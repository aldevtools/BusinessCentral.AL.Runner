MediaWordImportNotAPackage => {
    id: "media-word-import-not-a-package",
    assumed: "Plain text (bytes 0x20 to 0x7E only, not empty) imported as a Word document gives no media and raises nothing: the call returns the null Guid, the field has no value, no Tenant Media row is made and ExportStream writes nothing (al-builtins `media_word_package`). Covered: Media.ImportFile of a lower-case `.docx` file and Media.ImportStream with the lower-case Word document MIME type and no file name, each into an empty field. Refused: other content that is no zip, a field that holds a media, a MediaSet, a file name, another letter case.",
    tests: "No Microsoft test. Measured on Windows BC of every supported major (sandbox containers, 2026-11-19) by corpus PR 1469 (merged, not pinned yet).",
    probe: "Corpus PR 1469 (merged), streams/media/MediaDocxRezip.al:789 Docx_MediaImportFile_NotAPackage, :959 Docx_MediaImportStream_NotAPackageDocxMime (both with the text `hello docx`); facts header :21-22.",
    awaiting: Awaiting {
        issues: &[8519],
        files: &["tests/al-language-onprem/streams/media/MediaDocxRezip.al"],
        since: "2026-11-20",
    },
}
