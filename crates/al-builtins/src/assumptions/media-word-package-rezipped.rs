MediaWordPackageRezipped => {
    id: "media-word-package-rezipped",
    assumed: "A Word package (a zip whose first entry is `[Content_Types].xml`) imported by Media.ImportFile from a file named `.dotx` or `.docm` (lower case) into a field that holds no media, or by MediaSet.ImportFile from a file named `.docx` (lower case) into an empty set, gives a media (a set of one member) with the MIME type of its extension (`.dotx`: the Word template type, `.docm`: `application/vnd.ms-word.document.macroEnabled.12`, `.docx`: the Word document type), width and height 0, stored re-zipped: a read of its bytes or of its Tenant Media row is refused (al-builtins `media_word_package`). Main's answers that take no assumption: Media.ImportFile of a `.docx` in any letter case holding a zip (MediaFiles.al:470-526 pins the import), Media.ImportStream and MediaSet.ImportStream of a zip with a Word MIME type (both now refuse a read of the bytes). Refused: another letter case of `.dotx`, `.docm` or of `.docx` into a MediaSet, a zip with another first entry, a field or set that holds a media.",
    tests: "No Microsoft test. Measured on Windows BC of every supported major (sandbox containers, 2026-11-19) by corpus PR 1469 (merged, not pinned yet).",
    probe: "Corpus PR 1469 (merged), streams/media/MediaDocxRezip.al:821 Docx_MediaImportFile_DotxExtension, :857 _DocmExtension, :996 Docx_MediaSetImportFile_Stored; facts header :10-14.",
    awaiting: Awaiting {
        issues: &[8519],
        files: &["tests/al-language-onprem/streams/media/MediaDocxRezip.al"],
        since: "2026-11-20",
    },
}
