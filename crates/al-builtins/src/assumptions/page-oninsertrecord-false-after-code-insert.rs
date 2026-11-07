PageOnInsertRecordFalseAfterCodeInsert => {
    id: "page-oninsertrecord-false-after-code-insert",
    assumed: "When the OnInsertRecord of a page on a temporary source inserts the row itself and answers false, the page goes on with that row as a stored one: the next edit of the row runs OnModifyRecord instead of OnInsertRecord again. (Pinned and not assumed on a database table, in a card and a list: pages/triggers/OnInsertRecordFalse.al Card_OpenNew_KeyThenField_OK, Card_OpenNew_KeyThenField_Close, Card_OpenNew_KeyOnly_Close, List_New_KeyThenField.)",
    tests: "Item Tracking Lines (InsertRecord inserts into the page's temporary source, ValidateAndInsert answers false); Tests-Manufacturing PostProductionJournalUsingLotItemTracking and the item tracking tests that set a lot and a quantity.",
    probe: "The same on a page with SourceTableTemporary = true: a page with OnInsertRecord { Rec.Insert(); exit(false) } and OnModifyRecord { Rec.Modify(); exit(false) }: OpenNew, set the key, set a second field, OK; read the row and count the trigger calls. Asked in corpus #641.",
}
