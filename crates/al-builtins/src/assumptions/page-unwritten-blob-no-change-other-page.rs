PageUnwrittenBlobNoChangeOtherPage => {
    id: "page-unwritten-blob-no-change-other-page",
    assumed: "On a page whose ModifyAllowed is false, a Blob of Rec that no stream wrote since it was read (also after a CalcFields loaded it) is no change of the row, so the page saves nothing after an action or a SetValue: pinned for a List (pages/lt-outside-worksheet/RmReadOnlyBlob.al:316-:470); assumed for any other page type.",
    tests: "Tests-ERM Edit Posted Documents (codeunit 134658) PostedServiceInvoiceUpdateSetValuesOK and PostedServiceInvoiceUpdateSetValuesCancel (Posted Serv. Inv. - Update); they pass in Microsoft's CI.",
    probe: "RmReadOnlyBlob.al's table under a Card with ModifyAllowed = false (Editable true and false): a no-change action and a SetValue on another control, then Close: the log of OnModifyRecord and the table's OnModify.",
}
