ApplicationAreaAssignIsCall => {
    id: "application-area-assign-is-call",
    assumed: "`ApplicationArea := <text>` is the call `ApplicationArea(<text>)`: it sets the application area of the session, and `ApplicationArea()` then reads the assigned text. The call form is pinned (pages/application-area/TestPageApplicationAreaControlRemoval.al:87-93); the assignment form is not.",
    tests: "No Microsoft test. A user's report: BC 29 compiles and runs `ApplicationArea := '#All,#Basic'` and `ApplicationArea := ApplicationArea() + ',#Basic'`, and `ApplicationArea()` then returns `#All,#Basic`, as the call form does (al-runner issue 7597).",
    probe: "pages/application-area/ApplicationAreaAssignForm.al: the assignment of a literal, the append form, the call form, an assignment of blank text, and the assignment inside a subscriber of System Initialization OnAfterLogin, each observed with ApplicationArea() afterwards.",
    awaiting: Awaiting {
        issues: &[7597],
        files: &["tests/al-language/pages/application-area/ApplicationAreaAssignForm.al"],
        since: "2026-11-10",
    },
}
