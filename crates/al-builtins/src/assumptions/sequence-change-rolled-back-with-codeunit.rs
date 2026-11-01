SequenceChangeRolledBackWithCodeunit => {
    id: "sequence-change-rolled-back-with-codeunit",
    assumed: "Under runner 130450 a test's NumberSequence.Restart of a sequence that existed when its codeunit started is rolled back when the codeunit ends, with the numbers taken after it (those taken before it stay taken); a test's Delete of such a sequence it took no numbers from, and an Insert or Next on that name after the Delete, are rolled back too. The later codeunits see the sequence as it was before the first Restart or Delete. Measured on SaaS BC 29 by corpus writer Y56; not on the pinned corpus yet.",
    tests: "No Microsoft test: al-runner issue 6638, a partner app whose test codeunits restart or delete a number sequence (Base App SequenceNoMgt.CreateSequence, NoSeriesSequenceImpl).",
    probe: "SeqCarry.al R1_Act/R2_Act/R3_Act and D1_Act/D2_Act/D3_Act on corpus branch al-runtime-rs-staging-y56 (observe commit f7159b00).",
    awaiting: Awaiting {
        issues: &[6638],
        files: &["tests/al-language/codeunit/lt-seq-carry/SeqCarry.al"],
        since: "2026-11-01",
    },
}
