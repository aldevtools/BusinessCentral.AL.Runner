SqlReadCostUnmeasuredShape => {
    id: "sql-read-cost-unmeasured-shape",
    assumed: "The SQL statements and rows read of one record method with nothing buffered (Get 1/1, FindSet 1/0 or 1/1 and each Next 0/1, Modify right after the Get of its row 1/1, FindFirst that finds nothing 1/0, FindLast that finds a row 1/1, IsEmpty 1/0 or 1/1, Count 1/1) are the same on any table, filter and FindSet row count as on the ones measured (session/lt-bl-sql-counters/SkSqlBudget.al, writer TX: a table of Code[20], Integer, Text[50], Integer filtered on its key fields, FindSet loops of 0, 1, 3 and 10 rows; SqlCounters.al: 1, 2, 5, 10, 20 rows).",
    tests: "No Microsoft test: an outside reporter's partner-app SQL budget tests (corpus writer TW pins their shapes, SkSqlBudget.al TW_*); the FindFirst of TW_FindFirstOrInsert_Counts is on a table of another shape.",
    probe: "The TX measurements (one record method between two counter reads, committed table) on tables of other shapes: one Integer key, a Code key with a secondary key, a table with a FlowField and a Blob field; filters on a non-key field; FindSet loops of 4 and 50 rows.",
}
