ListOpenFullWindowRowsBefore => {
    id: "list-open-full-window-rows-before",
    assumed: "The open read of a list with OnFindRecord, OnNextRecord and OnAfterGetRecord but no OnAfterGetCurrRecord, whose first window of 20 rows is full, steps back with OnNextRecord(-1) and reads each row until it answers 0 (pinned with 9 rows above, for a page of its own and a ListPart: pages/lt-ai-list-order/AiListOrderTests.al:1090, :1127, misc/lt-probes-needed-6/PnF06FindListMoves.al:1947, :1968). Assumed: a step back that reaches 20 rows above stops there and asks no more (as the descending find list's step back does, PnF06FindListMoves.al RK_DescKind_FindNext_21Rows).",
    tests: "ERM Base Calendar (BaseCalendarEntriesShowCorrespondingRecords): Base Calendar Entries Subform keeps its position on the Date table in a global that OnAfterGetRecord sets.",
    probe: "The list of PnF06FindListMoves.al SF_DayList_OpensOnDayTen_Next_Previous (rows on every day 1 to 40), opened on day 30 (more than a window above): the reads at open, then Previous, First and GoToKey.",
}
