StrictAfterRefused => {
    id: "strict-after-refused",
    assumed: "In the sequential strict run (ADR-070), a test codeunit of a BC session that ran after a test that was refused while it ran, a test stopped by the time limit, or a codeunit that was not run as a whole, started from state that is not BC's: BC ran the refused test to its end and its effects on the SingleInstance state, the rows and the number sequences are not known here. The test's own result is what the runtime computed from the state it had, flagged because that state is not what BC's run would have given it.",
    tests: "Any codeunit after a refused one in a session of a --strict-order run.",
    probe: "None: this marks a gap in what the runtime ran, not a behavior of BC. It goes away for a session when no test of it is refused while it runs.",
}
