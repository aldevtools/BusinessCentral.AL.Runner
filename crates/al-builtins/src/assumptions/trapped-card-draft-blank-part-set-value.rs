TrappedCardDraftBlankPartSetValue => {
    id: "trapped-card-draft-blank-part-set-value",
    assumed: "On a trapped Card after New(), whose single linked ListPart shows rows of the blank key: SetValue of a non-key field whose validation changes no other field of Rec and leaves the key blank, a read of a host field and Close store the new header under the key its table's OnInsert gives, without changing those part rows. The Card and part have no page extension or subscriber, the Card has no page or control triggers and no DelayedInsert, and the link is one field on the whole primary key. Other calls after the SetValue, and a header stored under the blank key, keep the refusal.",
    tests: "AL Runner issue 7879 (reporter's repro SetValueOnNewCardWithBlankLinkedRows, measured on all three supported BC majors; the issue records the exact builds).",
    probe: "Corpus pull request 1343.",
    awaiting: Awaiting {
        issues: &[7879],
        files: &["tests/al-language/pages/parts/TestTrappedCardBlankPart.al"],
        since: "2026-11-12",
    },
}
