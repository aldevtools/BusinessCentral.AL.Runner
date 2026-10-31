DatabaseSidNumbers => {
    id: "database-sid-numbers",
    assumed: "Database.SID answers S-1-5-21-a-b-c-rid with a fixed domain part and a stable hash of the name as the rid.",
    tests: "Library - Document Approvals.GetUser; 214 Tests-Dimension tests.",
    probe: "Nothing in AL can compare the numbers with BC's; the shape is pinned (PXDatabaseSid.al).",
}
