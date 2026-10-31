RecordRefIsDirtyAgainstStoredRow => {
    id: "record-ref-is-dirty-against-stored-row",
    assumed: "RecordRef.IsDirty of a buffer that did not come from a read or write through the ref (after GetTable, Init or Clear) compares the buffer with the row stored under its primary key (a buffer with no stored row is compared with a fresh buffer). After a read or write through the ref the pinned snapshot is used and this does not apply.",
    tests: "Item Card OnAssistEdit and the other pages that ask whether the record changed: about 345 Microsoft tests (Tests-SCM, Tests-ERM).",
    probe: "IsDirty of a RecordRef right after GetTable of a Record that was read, modified without Modify, or Init: is it compared with what the Record read, or with the stored row. Asked in corpus #644.",
}
