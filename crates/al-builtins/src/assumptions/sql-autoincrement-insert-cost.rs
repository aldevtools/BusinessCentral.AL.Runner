SqlAutoincrementInsertCost => {
    id: "sql-autoincrement-insert-cost",
    assumed: "An Insert(true) that runs no AL code, into a table with an AutoIncrement field, with nothing buffered, is sent at once: one statement that reads one row, plus the one statement of the session's first write to the table when its Global Triggers setup enables nothing; it leaves nothing buffered. Only the sum of the reporter's loop is pinned: five FindFirst that find nothing, each followed by such an Insert, 11/5 (session/lt-bl-sql-counters/SkSqlBudget.al:600 TW_FindFirstOrInsert_Counts, with FindFirst 1/0 from :786 and the first write from SZ_FirstInsert_*).",
    tests: "No Microsoft test: an outside reporter's partner-app SQL budget test (SkSqlBudget.al TW_FindFirstOrInsert_Counts).",
    probe: "One Insert(true) and one Insert(false) into a committed table with an AutoIncrement field between two counter reads, as the first write of the session and as a later one; the same with two and five Inserts in a row.",
}
