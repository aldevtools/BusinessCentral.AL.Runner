TreeRefillByValidateRereads => {
    id: "tree-refill-by-validate-rereads",
    assumed: "A filter field's OnValidate that empties and refills the temporary source of a collapsed tree Worksheet: the client reads every row of the tree as the flat Worksheet reads its view (RmTempRefill.al), keeps the row it stood on when its key came back, else Rec's row, keeps the rows it had expanded by key, expands the parent of a kept row below the top level, and shows the rows the tree's cut shows.",
    tests: "No Microsoft test: al-runner issue 6550, the reporter's run on real BC 28.5 and 29.0 (refilled from A and B to B the walk is `B,`; from D to A and B under the same keys the top-level rows are `A,B,`).",
    probe: "Corpus writer Y34, RmTempRefill.al Y34_Tree_* on branch al-runtime-rs-staging-y34 (SaaS BC 29, two runs): fewer rows, same keys, a held key kept, expanded keys carried, a kept key now a child, a refill to no rows.",
    awaiting: Awaiting {
        issues: &[6550],
        files: &["tests/al-language/pages/lt-outside-worksheet/RmTempRefill.al::Y34_Tree_Fewer_AB_To_B"],
        since: "2026-10-31",
    },
}
