TrappedCardDraftBlankPartSetValue => {
    id: "trapped-card-draft-blank-part-set-value",
    assumed: "SetValue of a non-key field after New on a trapped Card with a part showing rows of the blank key edits the new header without changing those part rows.",
    tests: "AL Runner issue 7879, measured on BC 27.5.46862.56996, 28.5.54151.57550 and 29.0.54011.57566.",
    probe: "Corpus pull request 1343.",
}
