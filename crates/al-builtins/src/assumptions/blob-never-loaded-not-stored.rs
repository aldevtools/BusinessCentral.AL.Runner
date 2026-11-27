BlobNeverLoadedNotStored => {
    id: "blob-never-loaded-not-stored",
    assumed: "A Blob whose stored content the record never loaded (read with Get, FindFirst or FindSet and no CalcFields, also after SetLoadFields, or a copy of such a record by :=, Record.Copy, a by-value parameter, a Variant or RecordRef.GetTable, also between a database and a temporary record and between companies) and that no stream wrote since is never written by Insert or Modify. Insert into a database table stores no Blob content (also Delete then Insert of the same key, and Insert(true), whose OnInsert sees HasValue Yes before CalcFields and No after); into a temporary table the new row answers HasValue Yes until CalcFields loads it empty. The inserting record keeps answering HasValue Yes and reads empty, also after its own CalcFields. A Modify of a database record leaves the row's Blob as it is: under the key it was read under, under another stored row's key, with a stale buffer whose row another variable changed, and after an Insert of the same variable. Not measured, so as before: a Modify of a temporary record, Rename.",
    tests: "No Microsoft test: al-runner issue 9286, the reporter's repro measured on Business Central 28.4 (the copy inserted under a new key has no Blob). Corpus PR 1631 (aldevtools/BusinessCentral.AL.Language.Tests, open), record/lt-blob-field-assign/BlobInsertUnloaded.al: 28 tests (20 in codeunit 73781, 8 in codeunit 73782) measured on a local Business Central 29.0 W1 for a Compressed, a Compressed = false and a table extension Blob field; its CI is green on BC 27.5, 28.5 and 29.0 (codeunit 73782 asserted on BC 28 and later, an observe round on 27.5).",
    probe: "record/lt-blob-field-assign/BlobInsertUnloaded.al of corpus PR 1631 (open), not yet pinned: cells C01 to C11, T12, T13, the controls K1 to K4, and M1 to M3, I1 to I4 and G1. Still to measure: a Modify of a temporary record, Rename, the 73782 cells asserted on BC 27.5.",
    awaiting: Awaiting {
        issues: &[9286],
        files: &["tests/al-language/record/lt-blob-field-assign/BlobInsertUnloaded.al"],
        since: "2026-11-27",
    },
}
