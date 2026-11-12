CardNewSkipsEmptyPartCurrentRecord => {
    id: "card-new-skips-empty-part-current-record",
    assumed: "A directly linked empty ListPart does not run OnAfterGetCurrRecord when its editable Card host runs New().",
    tests: "AL Runner issue 7888; corpus pull request 1350.",
    probe: "Corpus pull request 1350.",
}
