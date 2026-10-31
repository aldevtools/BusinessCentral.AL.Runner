TernaryLongerTextFalseBranchFits => {
    id: "ternary-longer-text-false-branch-fits",
    assumed: "A conditional expression whose true branch is a Text[n] and whose false branch is a longer Text or Code, outside an assignment to a target of exactly Text[n] (pinned: types/lt-qx-ternary-sized/QxTernarySized.al:142-155), yields the false branch's value unchanged when it is no longer than n. A longer value is refused when the branch is taken.",
    tests: "Every test of Microsoft's Legacy test lane (for example Tests-SCM-Warehouse codeunit 137015): their company is built by the DemoTool, which inserts Acc. Schedule Lines whose OnInsert runs `Description := Description = '' ? GLAccCategory.Description : Description` (Base App AccScheduleLine.Table.al:113, a Text[80] true branch and a Text[100] false branch into a Text[100] field), and they pass in Microsoft's CI.",
    probe: "`T100 := Cond ? T80 : T100` and `T50 := Cond ? T80 : T100` with a false-branch value of 10, 80 and 90 characters: the value read back or the error (text and code); the same with a Code[100] false branch.",
}
