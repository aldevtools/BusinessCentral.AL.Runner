HostSaveRecordRefreshesLinkedParts => {
    id: "host-save-record-refreshes-linked-parts",
    assumed: "A SetValue on a control of a host page that is not a Document page (a Card), or a Document page with a part hidden by a Visible expression, whose OnValidate calls CurrPage.SaveRecord() (and no CurrPage.Update), makes the host's parts that have a SubPageLink read their rows again from their first rows, as after CurrPage.Update on the host (testpart/TestPartTriggerOrderAndRefresh.al HostCurrPageUpdate_ReReadsThePart), without the host reading its own row again. On a Document host testpart/QiHostSaveParts.al pins it: every part reads again where it stands, the unlinked ones too.",
    tests: "Corpus baseline case 60412 NewHostRowSavedByItsOwnTrigger_IsInsertedOnce (a Card host); Tests-ERM ERM Purchase Subform (134394) and Document Totals Pages: Purchase documents have parts hidden by a Visible expression; they pass in Microsoft's CI.",
    probe: "The QiHostSaveParts.al probe (testpart/QiHostSaveParts.al QI_HostSaveRecord_*) on a Card host with a linked ListPart and an unlinked one: which part triggers run, in which order, with which Rec and xRec.",
}
