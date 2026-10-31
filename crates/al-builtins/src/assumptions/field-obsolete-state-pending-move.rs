FieldObsoleteStatePendingMove => {
    id: "field-obsolete-state-pending-move",
    assumed: "The Field row of a field with ObsoleteState = PendingMove exists and its ObsoleteState is ordinal 3 (Format prints `3`; the members of Field.ObsoleteState are No,Pending,Removed), so a filter `<> Removed` keeps it; ObsoleteReason is kept. Measured on Windows BC of the newest major (corpus branch winprobe-wc6, tests/wc6-probe/ContosoProbe.al, cu 61320 CS_FieldRowPendingMove).",
    tests: "The Contoso company build of Microsoft's IntegrationTest and Uncategorized lanes for apps that pull in Power BI Report embeddings: Dimension Management.FillNormalFieldBuffer scans the Field table with `ObsoleteState <> Removed` (Default Dimension.OnValidate), and PowerBI Reports Setup.\"Subs. Billing Report Name\" is PendingMove; the build runs in Microsoft's CI.",
    probe: "A corpus test of the measured case on all three majors: Get of the Field row, Format of ObsoleteState with formats 0 and 2, a filter `<> Removed` and `= Pending`.",
}
