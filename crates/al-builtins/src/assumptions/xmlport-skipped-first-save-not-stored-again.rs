XmlportSkippedFirstSaveNotStoredAgain => {
    id: "xmlport-skipped-first-save-not-stored-again",
    assumed: "XmlPort import of a real table element right below the root, AutoSave without AutoUpdate, that is stored before its linked child rows and stored again after them: when a Skip in OnBeforeInsertRecord left out the first save of a row that exists, the second save after the child rows does not run (no OnBeforeModifyRecord, no Modify, the row keeps its values). Measured with a Skip in the key attribute's OnAfterAssignField before it, triggers that assign fields, and child rows each left out by a Skip in their own OnAfterInitRecord; a request of a field trigger is dropped where pinned (P112XpXml.al:748-818). A first save that stored no row in any other way refuses.",
    tests: "No Microsoft test. Raised from an outside report (al-runner issue 8877, the AI Test Toolkit's XmlPort 149031); al-runner issue 8952.",
    probe: "Corpus PR 1550 (merged, not yet in the pinned corpus), tests/al-language/xmlports/shapes-properties/XpAutoUpdateAssign.al:1062-1076 Suite_SameAppSameHash_Skips.",
    awaiting: Awaiting {
        issues: &[8952],
        files: &["tests/al-language/xmlports/shapes-properties/XpAutoUpdateAssign.al"],
        since: "2026-11-26",
    },
}
