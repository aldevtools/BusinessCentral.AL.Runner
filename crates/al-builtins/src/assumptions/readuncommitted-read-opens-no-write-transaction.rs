ReadUncommittedReadOpensNoWriteTransaction => {
    id: "readuncommitted-read-opens-no-write-transaction",
    assumed: "A read under ReadIsolation ReadUncommitted does not open a write transaction (IsInWriteTransaction No, a guarded Codeunit.Run allowed), as a ReadCommitted read does not (codeunit/TestCodeunitRunWriteTransaction.al:254-278 `readcommitted_read` No).",
    tests: "Tests-ERM Manual Payment Registration.PostPaymentForSalesInvoiceBalAccountBank and about 25 more Tests-ERM tests whose posting reads under ReadUncommitted before a guarded Codeunit.Run or a write-transaction check.",
    probe: "Commit, then a FindFirst under ReadIsolation ReadUncommitted on a database table: IsInWriteTransaction, and a guarded Codeunit.Run afterwards.",
}
