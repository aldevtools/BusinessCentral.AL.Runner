BooleanToTextAsFormat => {
    id: "boolean-to-text-as-format",
    assumed: "A Boolean given where a Text is expected (Record.Get(Boolean) on a Text key) converts as Format writes it: Yes or No. The Code case is pinned (types/lt-ax-text-format-dates/AxEvaluateText.al:348 GetTrue_CodeKey_Yes: YES).",
    tests: "Corpus baseline cases 74885 AssertEquals_TextControl_NumbersAndText and 70974 P189_FldRefMismatch_OnTextField (refusing the Text case loses them). Item Tracking Code Get(Serial) in the SCM kitting tests; about 90 tests of the Tests-SCM-Assembly bucket.",
    probe: "Get(true) and Get(false) on a table with a Text[10] key holding the rows 'Yes', 'YES', 'No' and '1'. Asked in corpus #649 (the Code key half is pinned).",
}
