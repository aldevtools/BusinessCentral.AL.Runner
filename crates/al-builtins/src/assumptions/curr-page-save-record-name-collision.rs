CurrPageSaveRecordNameCollision => {
    id: "curr-page-save-record-name-collision",
    assumed: "CurrPage.SaveRecord calls the platform method when the current page declares an internal procedure named SaveRecord.",
    tests: "Local neutral AutoSplitSaveRecord.al probe.",
    probe: "Add the CurrPage.SaveRecord collision and explicit part page call to the pinned corpus.",
}
