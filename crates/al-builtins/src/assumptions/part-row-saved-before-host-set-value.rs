PartRowSavedBeforeHostSetValue => {
    id: "part-row-saved-before-host-set-value",
    assumed: "A SetValue on a control of a host page that is not a Document page, while a part holds a typed row that is not saved (a new row or an edited one), saves that row first (Insert or Modify with its triggers), as on a Document host (testpart/QiHostSaveParts.al:500 QI_PartNewRow_ThenHostSetValue, :554 QI_PartEditedRow_ThenHostSetValue); then the host field validates.",
    tests: "Microsoft tests that type a line into a subform of a host other than a Document page, then set a host field; and the corpus cases that do the same on a Card host.",
    probe: "The QiHostSaveParts.al probe (QI_PartNewRow_ThenHostSetValue, QI_PartEditedRow_ThenHostSetValue) on a Card host.",
}
