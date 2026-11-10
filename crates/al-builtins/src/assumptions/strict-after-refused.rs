StrictAfterRefused => {
    id: "strict-after-refused",
    assumed: "In the sequential strict run (ADR-070), a test codeunit of a BC session that ran after a stopped one started from state that is not BC's: a test was refused while it ran, a time limit stopped a test, no test of a codeunit ran, or a codeunit panicked. BC ran that work to its end, and its effects on the SingleInstance state, the rows and the number sequences are not known here. Only a codeunit whose static reach can read what the stopped codeunit's reach could have changed is flagged (a stopped codeunit with an unknown reach flags every later one; a flagged codeunit counts as changed for the ones after it). The test's own result is what the runtime computed from the state it had.",
    tests: "Any codeunit that can read state a refused codeunit could have changed, in a session of a --strict-order run.",
    probe: "None: this marks a gap in what the runtime ran, not a behavior of BC. It goes away for a session when no test of it is refused while it runs.",
}
