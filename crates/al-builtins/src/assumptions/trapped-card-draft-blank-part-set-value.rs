TrappedCardDraftBlankPartSetValue => {
    id: "trapped-card-draft-blank-part-set-value",
    assumed: "SetValue of a non-key field after New on a trapped Card with a part showing rows of the blank key edits the new header without changing those part rows.",
    tests: "AL Runner issue 7879, measured on all three supported BC majors. The issue records the exact builds.",
    probe: "Corpus pull request 1343.",
}
