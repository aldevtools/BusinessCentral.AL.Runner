NextOnlyCardAssistEditRefresh => {
    id: "next-only-card-assist-edit-refresh",
    assumed: "On a Card with OnNextRecord and OnAfterGetCurrRecord, AssistEdit that stores and reloads the current edited row before CurrPage.Update(false) shows that record and refreshes its linked part without raising OnAfterGetCurrRecord on the Card.",
    tests: "AL Runner issue 7881, measured on BC 27.5.46862.56996, 28.5.54151.57550 and 29.0.54011.57566.",
    probe: "Corpus pull request 1345.",
}
