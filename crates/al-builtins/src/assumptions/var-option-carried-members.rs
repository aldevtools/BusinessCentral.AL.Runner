VarOptionCarriedMembers => {
    id: "var-option-carried-members",
    assumed: "A `var` Option parameter declared without members, of a procedure (no attribute) of a codeunit or a table, handed on to a `var` Option parameter with members of a procedure (no attribute) of a codeunit or a table, or of an interface method a codeunit implements, while the value it holds carries exactly that parameter's member list: the callee reads, writes, evaluates and clears it as a parameter with its own members, and the write goes back through each hop to the first caller. The value comes from a variable with members of the `var` section of a codeunit procedure (no attribute, or `Test`), through one or more memberless `var` parameters of procedures (no attribute) of codeunits or tables, or through an `IntegrationEvent(true, false)` of a codeunit, not isolated, to a static subscriber of a codeunit that hands its memberless `var` parameter on. Refused: a value that carries another member list or none (the callee reads the value's list, which the run does not model), a value from any other place (a global, a field, an element, a parameter passed by value, a named return value, an interface method's memberless `var` parameter), another event contract, a hop or callee with an attribute or of another object kind.",
    tests: "Corpus PR 1569 (open): tests/al-language/types/enum-option/OptionVarMembersCallee.al and OptionVarMembersBatch.al, 56 of 56 on a local BC of the newest of the three supported majors. 51 Microsoft tests of the batch posting of purchase and sales documents (al-runner issue 8971) take this path through Base Application's batch processing event.",
    probe: "Corpus PR 1569 (open), tests/al-language/types/enum-option/OptionVarMembersCallee.al: VarHop_Same_* (a table, a codeunit, a local, a table hop, a codeunit hop, an interface callee, two hops), CalleeErrors_VarHop_Same; OptionVarMembersBatch.al: BatchEvent_Confirmed, BatchEvent_Declined (the event path). Measured and refused by the runner: a memberless value or one with another list (VarHop_Memberless_*, VarHop_Other_*, BatchQueued_*: the callee reads the value's list), the hand-on from the callee to a memberless parameter (CalleeOps_VarHop_*). Not measured: the other origins and contracts the entry refuses.",
    awaiting: Awaiting {
        issues: &[8971],
        files: &[
            "tests/al-language/types/enum-option/OptionVarMembersCallee.al",
            "tests/al-language/types/enum-option/OptionVarMembersBatch.al",
        ],
        since: "2026-11-24",
    },
}
