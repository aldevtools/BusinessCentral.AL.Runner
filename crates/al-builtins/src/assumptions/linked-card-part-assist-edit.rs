LinkedCardPartAssistEdit => {
    id: "linked-card-part-assist-edit",
    assumed: "On an empty CardPart linked from a Document to the part's single Code primary key, AssistEdit on that key receives the linked value in Rec. If the trigger inserts the matching row and returns it in Rec, a later AssistEdit and CurrPage.Update can operate on that row.",
    tests: "AL Runner issue 7883; corpus pull request 1347.",
    probe: "Corpus pull request 1347.",
}
