SequenceDeleteRolledBackWithCodeunit => {
    id: "sequence-delete-rolled-back-with-codeunit",
    assumed: "Under runner 130450 a test's NumberSequence.Delete of a sequence that existed when its codeunit started, and an Insert or Next on that name after the Delete, are rolled back when the codeunit ends: the later codeunits see the sequence as it was. Measured on SaaS BC 29 by corpus writer Y56; not on the pinned corpus yet.",
    tests: "No Microsoft test: al-runner issue 6638, a partner app whose test codeunits delete or restart a number sequence (Base App SequenceNoMgt, NoSeriesSequenceImpl).",
    probe: "SeqCarry.al D1_Act/D2_Act/D3_Act on corpus branch al-runtime-rs-staging-y56 (observe commit f55c353d).",
    awaiting: Awaiting {
        issues: &[6638],
        files: &["tests/al-language/codeunit/lt-seq-carry/SeqCarry.al"],
        since: "2026-11-01",
    },
}
