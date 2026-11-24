XmlportStoredAgainAssignTriggers => {
    id: "xmlport-stored-again-assign-triggers",
    assumed: "XmlPort import of a table element right below the root text element, AutoSave without AutoUpdate, AutoReplace or UseTemporary, a real table, that is stored before its linked child rows and stored again after them (XmlPortTriggerFlow.al:452, pinned without assign triggers), whose children are field attributes, text attributes, table elements and field elements of its own without nodes below them, where an OnAfterAssignField or OnAfterAssignVariable of one of them assigns fields of the row: the second save stores the record as it stands. Run only when the first save stored the row and no trigger of a field attribute or field element asked for Skip, Break or BreakUnbound, and when the record and the stored row hold the same value in every field the document does not carry, so every candidate (the record as it stands, or the stored row with the carried fields of the record) gives the same row; anything else refuses.",
    tests: "No Microsoft test. An outside report (al-runner issue 8877): an install routine that imports an AI test suite with the AI Test Toolkit's XmlPort 149031.",
    probe: "Corpus PR 1550, tests/al-language/xmlports/shapes-properties/XpAutoUpdateAssign.al Suite_Fresh (attribute triggers, child and grandchild rows), Again_FieldElementTrigger (a field element trigger), Suite_SameAppSameHash_Skips (a skipped first save is not stored again), Stale_ChildTriggerSetsParentVariable and StaleDb_ChildOnInsertChangesStoredParent (the record and the stored row differ).",
    awaiting: Awaiting {
        issues: &[8877],
        files: &["tests/al-language/xmlports/shapes-properties/XpAutoUpdateAssign.al"],
        since: "2026-11-23",
    },
}
