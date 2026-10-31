UpdateRefreshOnFindRecord => {
    id: "update-refresh-onfindrecord",
    assumed: "CurrPage.Update on a page with OnFindRecord and OnAfterGetCurrRecord asks OnFindRecord for the current row with Which `=`, then reads the row as after a move: OnAfterGetRecord, OnAfterGetCurrRecord and the parts' reads.",
    tests: "Discount Setup Notifications (T130_NotifyAboutMissingSalesSetupOnInvDiscountValidation, a Sales document page) and about 87 more Tests-ERM tests.",
    probe: "A card with OnFindRecord and OnAfterGetCurrRecord that log, an action that calls CurrPage.Update: the Which string and the trigger order; the same on a list, and after an edit not yet saved.",
}
