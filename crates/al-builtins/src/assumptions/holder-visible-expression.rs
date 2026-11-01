HolderVisibleExpression => {
    id: "holder-visible-expression",
    assumed: "A cuegroup's Visible bound to an expression (not a variable, an array element, a Rec field or expression, not Rec.IsEmpty()) reads the value it has after the page opened, and bound to a call of the page's own procedure with no arguments reads No; a fixed or grid control's Visible bound to an expression reads its value like a group's. Measured on SaaS (newest major) by corpus writer Y65 for cuegroups; fixed and grid are not measured.",
    tests: "No Microsoft test: al-runner issue 6675, three Base Application role-center activity pages whose cuegroups take Visible from page variables.",
    probe: "Y65CueVisible.al Y65_CueVisible_NoRowYet and Y65_CueVisible_RowWithFlag on corpus branch al-runtime-rs-staging-y65 (commit eefaf01b); a fixed and a grid control with Visible bound to a variable set in OnOpenPage.",
    awaiting: Awaiting {
        issues: &[6675],
        files: &["tests/al-language/pages/lt-y65-cue-visible/Y65CueVisible.al"],
        since: "2026-11-01",
    },
}
