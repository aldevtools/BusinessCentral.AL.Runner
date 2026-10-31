EvaluateRecordIdCompositeKey => {
    id: "evaluate-recordid-composite-key",
    assumed: "Evaluate into a RecordId reads a Code key value that has blanks around it as the trimmed value (the length check counts the trimmed text).",
    tests: "Tests-Rapid Start (ERM RS Package Base Operations: the Config. Package Record IDs hold ' GUAAAAAAAB'); about 39 tests.",
    probe: "Evaluate(Rid, 'T: ' + ' ABC') and Evaluate(Rid, 'T: ABC ') into a RecordId of a table with a Code[10] key, and a value of 10 characters with a leading blank: the key value and whether it is accepted.",
}
