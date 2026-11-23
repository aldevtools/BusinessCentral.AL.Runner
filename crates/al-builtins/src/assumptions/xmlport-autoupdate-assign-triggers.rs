XmlportAutoupdateAssignTriggers => {
    id: "xmlport-autoupdate-assign-triggers",
    assumed: "XmlPort import (Format = Xml) of a table element right below the root text element, AutoSave and AutoUpdate without AutoReplace, a real table, no table element anywhere below it, into a row that exists, where an OnAfterAssignField of a field element, an OnAfterAssignVariable of a text element, or its OnAfterInitRecord assigns fields of the row: the save reads the stored row and puts back the fields the document carries as the record holds them, before OnBeforeModifyRecord, so what the triggers assign to a field the document does not carry is dropped and OnBeforeModifyRecord sees the stored row with the carried fields. An OnBeforeModifyRecord that changes the row refuses; so does an assign trigger on an attribute.",
    tests: "No Microsoft test. An outside report (al-runner issue 8877) names this refusal for the AI Test Toolkit's XmlPort 149031.",
    probe: "Corpus PR 1550, tests/al-language/xmlports/shapes-properties/XpAutoUpdateAssign.al KeyTrig_Existing_NameCarried, KeyTrig_Existing_NameAbsent (an OnAfterAssignField of the key field element), FieldTrig_Existing (of a carried field element), TextTrig_Existing (an OnAfterAssignVariable of a text element), InitTrig_Existing_NameCarried, InitTrig_Existing_NameAbsent (an OnAfterInitRecord); measured identical on the three supported majors.",
    awaiting: Awaiting {
        issues: &[8877],
        files: &["tests/al-language/xmlports/shapes-properties/XpAutoUpdateAssign.al"],
        since: "2026-11-23",
    },
}
