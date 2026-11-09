BlobCopiedWriteKeptByCalcfields => {
    id: "blob-copied-write-kept-by-calcfields",
    assumed: "A record assignment (Copy := Rec, also through a by-value parameter and a var parameter) copies an unsaved stream write into a Blob of a stored row as unsaved: CalcFields on the copy keeps the write instead of reading the row again. A copy of a row that is not stored still reads empty (ProbeTypeFormsAndFilters.al FormatRecord_AssignmentCopiesBlobBytes).",
    tests: "No Microsoft test: al-runner issue 7360 (the reporter's ISV test suite passes on real BC with this sequence).",
    probe: "Insert a row with an empty Blob, write text through CreateOutStream (with and without Clear first), copy the record with := once, and through a codeunit that stores a by-value parameter and hands it back through a var parameter; CalcFields on each copy and read the text back. Also Modify of the copy: does it store the write?",
    awaiting: Awaiting {
        issues: &[7360],
        files: &["tests/al-language/streams/BlobWriteThenRecordCopy.al"],
        since: "2026-11-09",
    },
}
