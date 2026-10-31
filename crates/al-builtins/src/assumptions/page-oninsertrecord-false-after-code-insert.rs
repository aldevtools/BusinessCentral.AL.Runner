PageOnInsertRecordFalseAfterCodeInsert => {
    id: "page-oninsertrecord-false-after-code-insert",
    assumed: "When a page's OnInsertRecord inserts the row itself and answers false, the page goes on with that row as a stored one: the next edit of the row runs OnModifyRecord instead of OnInsertRecord again.",
    tests: "Item Tracking Lines (InsertRecord inserts into the page's temporary source, ValidateAndInsert answers false); Tests-Manufacturing PostProductionJournalUsingLotItemTracking and the item tracking tests that set a lot and a quantity.",
    probe: "A page with OnInsertRecord { Rec.Insert(); exit(false) } and OnModifyRecord { Rec.Modify(); exit(false) }: OpenNew, set the key, set a second field, OK; read the row and count the trigger calls. Asked in corpus #641.",
}
