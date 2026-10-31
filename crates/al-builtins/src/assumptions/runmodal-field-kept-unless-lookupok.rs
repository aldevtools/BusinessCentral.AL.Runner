RunModalFieldKeptUnlessLookupOk => {
    id: "runmodal-field-kept-unless-lookupok",
    assumed: "Page.RunModal(Id, Rec, Field) with a handler that answers anything but LookupOK leaves the field and the record as the handler left them; only LookupOK copies the page's value into the field (a LookupOK answer is refused).",
    tests: "Segment Line.RunCreateInteraction (PAGE.RunModal(PAGE::\"Create Interaction\", Rec, \"Interaction Template Code\")) in Tests-Marketing: CampaignAndInteractionsLinked and about 32 more, answered by Page-typed handlers that leave Response alone.",
    probe: "Page.RunModal(Page::P, Rec, Rec.F) with a ModalPageHandler that (a) invokes OK, (b) Cancel, (c) does nothing, (d) a Page-typed handler leaving Response; with P a card and a list in lookup mode, for LookupOK: what Rec.F holds and what the call returns. Asked in corpus #652.",
}
