HiddenPartOutsideChainSkipsSync => {
    id: "hidden-part-outside-chain-skips-sync",
    assumed: "A part a Visible expression hides, with no Provider and no part linked to it, on a host page that has a Provider chain elsewhere, runs none of its triggers (OnAfterGetCurrRecord) on a sync that is neither an open nor a move, for example a key typed on a new host row. Parts inside a Provider chain are pinned and follow.",
    tests: "No Microsoft test: al-runner issue 6924 (the reporter's Data Check factbox on Sales Credit Memo; 34 CopyPurchSalesDoc tests passed on real BC and on edge 4e20bf926).",
    probe: "HiddenPartOutsideChain.al (corpus pull request 1288): a hidden FactBox part outside every chain, OpenNew with a key typed, GoToKey, Previous, Next.",
    awaiting: Awaiting {
        issues: &[6924],
        files: &["tests/al-language/testpage/parts/HiddenPartOutsideChain.al"],
        since: "2026-11-10",
    },
}
