ExtLocalProcNamedLikeBase => {
    id: "ext-local-proc-named-like-base",
    assumed: "A report extension of another app's report may declare a local procedure with the name and parameter types of a public or protected procedure of its base. A bare call in the extension's code runs the extension's local procedure; the base's own calls and calls on a report variable from outside run the base's procedure. Pinned for a page extension (pageext/lt-ext-local-same-name/ExtLocalSameNameTests.al); assumed alike for a report extension.",
    tests: "No Microsoft test and no user report for a report extension: the reported page extension shape is pinned (ExtLocalSameNameTests.al), and the lowering treats both extension kinds alike.",
    probe: "The page extension test of ExtLocalSameNameTests.al repeated with a report extension of another app's report: bare calls from the extension, the base's own calls, and a call on a report variable from a test codeunit.",
}
