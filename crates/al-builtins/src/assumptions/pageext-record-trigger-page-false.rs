PageExtRecordTriggerPageFalse => {
    id: "pageext-record-trigger-page-false",
    assumed: "When page extensions declare OnInsertRecord or OnModifyRecord and answer true, the page's own false answer still vetoes the insert or the modify, as it does on a page without extensions (handlers/TestPageRecordTriggers.al:76).",
    tests: "No Microsoft test: the rule generalizes the corpus writer's pins of page extension triggers (an extension's false answer) to the page's own false answer next to extensions.",
    probe: "UeTriggerTests.al with the base page's OnInsertRecord and OnModifyRecord answering false and both extensions true: the stored row and the trigger log.",
}
