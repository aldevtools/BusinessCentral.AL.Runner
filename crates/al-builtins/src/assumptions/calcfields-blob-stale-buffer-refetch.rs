CalcfieldsBlobStaleBufferRefetch => {
    id: "calcfields-blob-stale-buffer-refetch",
    assumed: "CalcFields of a Blob field whose buffer holds no unsaved stream write, came from an Insert, Modify or an earlier CalcFields of this variable, and no longer equals the stored row (another variable of the same row modified the Blob since) reads the stored content. The pinned cases stay as they are: an unsaved write is kept (streams/TestBlobCalcFieldsUncommittedWrite.al), and so is a buffer that equals the row.",
    tests: "Tests-ERM ERM Copy Purch/Sales Doc.CopyWorkDescriptionSalesArchiveShipment (ERMCopyPurchSalesDoc.Codeunit.al:2929-2972): the test inserts a Sales Header, hands it by value to the Copy Sales Document report, which modifies the row through its own copy, then calls CalcFields and GetWorkDescription on its own record and expects the copied text (al-runner issue 6924).",
    probe: "An inserted record with an empty Blob, a second variable (a Get of the row, or a by-value copy) that writes and modifies the Blob, then CalcFields on the first variable: the content read; and the same after the first variable's own Modify.",
    awaiting: Awaiting {
        issues: &[6924],
        files: &["tests/al-language/streams/CalcFieldsBlobStaleBuffer.al"],
        since: "2026-11-06",
    },
}
