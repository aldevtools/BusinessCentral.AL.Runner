TernaryLabelBranch => {
    id: "ternary-label-branch",
    assumed: "A conditional expression with a Label branch against a Code[N] or Text[N] branch: the true branch decides the result type. A Code[N] true branch makes the Label false branch a Code (upper case; StringLengthExceeded past N, with the label's own text in the message); a Text[N] true branch limits the Label to N; a Label true branch is a Text that takes the false branch as it is. Pinned for Code[20] and Text[10] against Labels of 7, 10, 15, 20 and 30 characters, each way (types/TestTernaryContracts.al TI_Ternary_*); assumed alike for every other length N.",
    tests: "No Microsoft test and no user report: the reported shape (a Code[20] true branch and a Locked Label false branch) is pinned; other lengths take the same lowering.",
    probe: "The TI_Ternary_* tests of TestTernaryContracts.al with Code[10], Code[50] and Text[30] branches, and an unbounded Text branch against a Label either way.",
}
