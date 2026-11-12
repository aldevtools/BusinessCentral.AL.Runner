CardNewSkipsEmptyPartCurrentRecord => {
    id: "card-new-skips-empty-part-current-record",
    assumed: "A directly linked empty ListPart does not run OnAfterGetCurrRecord when its editable Card host runs New().",
    tests: "Local neutral probe HostNew.al.",
    probe: "Add the Card New and empty ListPart trigger trace to the pinned corpus.",
}
