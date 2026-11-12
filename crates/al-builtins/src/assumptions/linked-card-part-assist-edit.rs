LinkedCardPartAssistEdit => {
    id: "linked-card-part-assist-edit",
    assumed: "On an empty CardPart linked from a Document to the part's single Code primary key, AssistEdit on that key receives the linked value in Rec. If the trigger inserts the matching row and returns it in Rec, a later AssistEdit and CurrPage.Update can operate on that row.",
    tests: "Local neutral LinkedPartAssist.al probe.",
    probe: "Add the measured linked CardPart primary-key AssistEdit and subsequent refresh to the corpus.",
}
