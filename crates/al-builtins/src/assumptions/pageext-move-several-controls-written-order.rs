PageExtMoveSeveralControlsWrittenOrder => {
    id: "pageext-move-several-controls-written-order",
    assumed: "A page extension move of several layout controls at once (movefirst, movelast, moveafter, movebefore with a list) places them in the order they are written, which Page Control Field reports as their Sequence; the controls read and write their fields as before.",
    tests: "No Microsoft test: al-runner issue 6483, an ISV test app (288 tests refused). Measured on BC by corpus writer Y20 (Y20PageExtMoves.al, SaaS sandbox, newest major: movefirst(Details; ACtl, BCtl) and moveafter(KCtl; ECtl, DCtl) give the Sequence KCtl 0, ECtl 1, DCtl 2, ACtl 3, BCtl 4, CCtl 5).",
    probe: "Y20PageExtMoves.al on branch al-runtime-rs-staging-y20: the field values after the moves and the Page Control Field Sequence of each control.",
    awaiting: Awaiting {
        issues: &[6483],
        files: &["tests/al-language/pageext/lt-y20-moves/Y20PageExtMoves.al"],
        since: "2026-10-31",
    },
}
