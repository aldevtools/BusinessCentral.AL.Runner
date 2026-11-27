BlobNeverLoadedNotStored => {
    id: "blob-never-loaded-not-stored",
    assumed: "A Blob whose stored content the record never loaded (read with Get, FindFirst or FindSet and no CalcFields, also after SetLoadFields, or a copy of such a record by :=, Record.Copy, a by-value parameter, a Variant or RecordRef.GetTable) and that no stream wrote since is not stored by Insert: a database table's new row holds no Blob content (also after Delete, Insert of the same key, and Insert(true)); a temporary table's new row answers HasValue Yes until CalcFields, which loads it empty. The inserting record keeps answering HasValue Yes and reads empty. A Modify of such a record into a database row whose Blob holds other content (its key names another stored row) leaves that row's Blob as it was. Not covered, so as before: a record of another company (ChangeCompany), a Modify of a temporary record.",
    tests: "No Microsoft test: al-runner issue 9286, the reporter's repro measured on Business Central 28.4 (the copy inserted under a new key has no Blob). Corpus PR 1631 (aldevtools/BusinessCentral.AL.Language.Tests, open), record/lt-blob-field-assign/BlobInsertUnloaded.al, 19 cells measured on a local Business Central 29.0 W1 for a Compressed, a Compressed = false and a table extension Blob field.",
    probe: "record/lt-blob-field-assign/BlobInsertUnloaded.al of corpus PR 1631 (open), not yet pinned: cells C01 to C11, T12, T13 and the controls K1 to K4. Still to measure: ChangeCompany, a Modify of a temporary record, Rename, a stale buffer whose row another variable changed.",
    awaiting: Awaiting {
        issues: &[9286],
        files: &["tests/al-language/record/lt-blob-field-assign/BlobInsertUnloaded.al"],
        since: "2026-11-27",
    },
}
