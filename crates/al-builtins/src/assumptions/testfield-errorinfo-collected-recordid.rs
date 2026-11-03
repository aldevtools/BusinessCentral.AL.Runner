TestfieldErrorinfoCollectedRecordId => {
    id: "testfield-errorinfo-collected-recordid",
    assumed: "Record.TestField(Field [, Value], ErrorInfo) with a collectible ErrorInfo, under ErrorBehavior::Collect, collects an ErrorInfo whose RecordId is the RecordId of the record the call was made on. FieldNo and TableId are pinned (types/PUTestFieldErrorInfo.al:143-166); RecordId is not.",
    tests: "Microsoft Tests-General_Journal, codeunit 134932 \"ERM Gen. Jnl. Error Handling\": JournalLineErrorsFactBoxTestFieldNotEmpty and the seven *JournalSunshine tests. Error Message Management.CollectErrors copies ErrInfo.RecordId into the Context Record ID of the Error Message, and the Journal Errors FactBox counts the errors of the current line by that field.",
    probe: "Under ErrorBehavior::Collect: TestField(Field, ErrorInfo.Create()) and TestField(Field, Value, ErrorInfo.Create()) on a record with a primary key, then GetCollectedErrors().Get(1).RecordId compared with the record's RecordId. Also on a temporary record and a record with an empty key.",
}
