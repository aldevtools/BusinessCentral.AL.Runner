NextOnlyCardAssistEditRefresh => {
    id: "next-only-card-assist-edit-refresh",
    assumed: "On a Card with OnNextRecord and OnAfterGetCurrRecord, AssistEdit that stores and reloads the current edited row before CurrPage.Update(false) shows that record and refreshes its linked part without raising OnAfterGetCurrRecord on the Card.",
    tests: "AL Runner issue 7881, measured on all three supported BC majors. The issue records the exact builds.",
    probe: "Corpus pull request 1345.",
}
