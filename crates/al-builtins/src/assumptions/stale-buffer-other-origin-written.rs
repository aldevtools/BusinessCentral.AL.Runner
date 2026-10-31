StaleBufferOtherOriginWritten => {
    id: "stale-buffer-other-origin-written",
    assumed: "A Modify or Delete through a buffer that is older than the last Commit's row, whose row version came neither from a Get nor from the variable's own Insert (a Find, an assignment, TransferFields), is written when the open transaction already wrote the row: the last writer wins. (A buffer read by Get is refused there, record/TestRecordStaleBufferWrite.al:604-660; one from the Insert of the transaction before is written, :495-584.)",
    tests: "No. Series - Batch Impl.UpdateNoSeriesLine (Get, TransferFields from the batch's temporary line, Modify(true)) in Gen. Jnl.-Post Batch; 110 Tests-ERM tests (for example Payment Export Validation UT CustLedgEntryBankAccountMissingFormat).",
    probe: "A buffer read by FindFirst, by assignment from another variable and by TransferFields from a temporary copy, made stale by a Commit, then the row written in the new transaction (changed and unchanged Modify), then a Modify through the buffer: accepted or DB:RecordChanged, and the stored row.",
}
