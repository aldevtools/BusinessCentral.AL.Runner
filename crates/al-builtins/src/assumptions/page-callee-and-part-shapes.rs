PageCalleeAndPartShapes => {
    id: "page-callee-and-part-shapes",
    assumed: "Permission shapes measured on two Windows BC majors by corpus writer Y90 (branch al-runtime-rs-staging-y90, not yet in the pinned corpus): a codeunit call from a page trigger under a session with wildcard Execute opens the page when an assigned set grants the callee by name (Set('D365 BASIC') + Assign), and also under an exact set with `system * = X` and the callee by name; an unguarded part under an exact set shows only when the session executes the part page and its source table.",
    tests: "Runner issue 6867 (the reporter's CI passes the shape on the newest major: Suite A, 4 tests; Suite C, 21 tests). Corpus: Y90PermOpenPage.al Y90_Open_D365BasicPlusAppSet, Y90_Open_CodeunitStarSystemStarSet; Y90PermParts.al Y90_Part_ExactA..C.",
    probe: "Y90PermOpenPage.al and Y90PermParts.al on branch al-runtime-rs-staging-y90 (3f54e878): measured on two Windows majors; the Linux run of the newest major matched. Wanted from Y93: whether a wildcard Execute grant reaches the objects of other apps (the callee granted by the wildcard alone still refuses).",
    awaiting: Awaiting {
        issues: &[6867],
        files: &[
            "tests/al-language/permissions/lt-y90-perm-shapes/Y90PermOpenPage.al",
            "tests/al-language/permissions/lt-y90-perm-shapes/Y90PermParts.al",
        ],
        since: "2026-11-03",
    },
}
