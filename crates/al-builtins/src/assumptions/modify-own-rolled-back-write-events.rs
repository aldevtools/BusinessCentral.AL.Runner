ModifyOwnRolledBackWriteEvents => {
    id: "modify-own-rolled-back-write-events",
    assumed: "A Modify of a buffer whose own last Modify a rollback undid, the buffer unchanged since, answers true and writes nothing, and runs the OnBeforeModifyEvent and OnAfterModifyEvent subscribers once each with Rec and xRec both the buffer and the RunTrigger the call gave. Observed on all three majors of the matrix with Modify(false) (the default); the OnModify trigger of Modify(true), database triggers and a subscriber that changes Rec are refused.",
    tests: "No corpus test on the pinned master yet. The probe is corpus pull request 1279 (record/events/Y7464ModifyAfterRolledBackModify.al), observed on all three majors of the matrix and pinned on that branch; record/StaleBuffer.al pins the same Modify on a table without events.",
    probe: "record/events/Y7464ModifyAfterRolledBackModify.al; the Modify(true) cells (ModifyTrue_*) give the OnModify trigger.",
    awaiting: Awaiting {
        issues: &[7464],
        files: &["tests/al-language/record/events/Y7464ModifyAfterRolledBackModify.al"],
        since: "2026-11-10",
    },
}
