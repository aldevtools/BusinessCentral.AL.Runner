HubFunctionSequence => {
    id: "hub-function-sequence",
    assumed: "Under BC's TestRunnerHub with RequiredTestIsolation = Function (al-runner --isolation required, not a test runner codeunit, not a scratch run), a number sequence missing when the test codeunit began and inserted by its OnRun is left this way by the first passing test of the codeunit that changes it (al-vm `hub_sequences.rs`): put back to its state after OnRun when OnRun inserted it and took Next twice and the test only restarted it or only deleted it, for all companies; kept as the test left it when OnRun inserted it and took Next twice and the test took Next once, per company, or for all companies followed by an AL Commit() statement; kept when OnRun only inserted it and the test took Next once, for all companies or per company. Only the NumberSequence calls Insert, Next, Range, Restart and Delete and the transaction ends count; reads do not. Each later NumberSequence call of the codeunit on such a sequence passes through this entry. Not covered, refused at the later call on the sequence: a failed test, a sequence there before the codeunit began, OnRun calls other than these two, a sequence an earlier test changed, any other call or transaction end in the test (Range, Insert, a second Next, a Restart or Delete with Next, a commit after a Restart or Delete, a commit by anything but Commit(), a rollback), the restart or delete per company, and Next for all companies without a commit (cell E1, which BC reads as missing). The seed and increment OnRun inserts with are not read.",
    tests: "No Microsoft test.",
    probe: "Corpus PR 1580 (merged as 44a4cb76, not yet pinned), tests/al-language/codeunits/run-onrun/SeqNextHubFunction.al, cells E2-E5, O1 and O2 asserted, E1 observed only, measured on the TestRunnerHub legs of corpus CI (two supported majors, the same on both): Next kept (E2, E3, O1, O2), Restart and Delete put back (E4, E5).",
    awaiting: Awaiting {
        issues: &[],
        files: &["tests/al-language/codeunits/run-onrun/SeqNextHubFunction.al"],
        since: "2026-11-27",
    },
}
