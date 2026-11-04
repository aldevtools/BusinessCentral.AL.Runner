HolderVisibleExpression => {
    id: "holder-visible-expression",
    assumed: "A fixed or grid control's Visible bound to an expression reads its value after the page opened, as a cuegroup's does (pinned for cuegroups by pages/cue-visible/Y65CueVisible.al).",
    tests: "No Microsoft test: an outside report, three Base Application role-center activity pages whose cuegroups take Visible from page variables.",
    probe: "A fixed and a grid control with Visible bound to a variable set in OnOpenPage, to a `not` of a variable and to a Rec field.",
}
