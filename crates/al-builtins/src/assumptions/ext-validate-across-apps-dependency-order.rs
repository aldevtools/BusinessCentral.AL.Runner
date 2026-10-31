ExtValidateAcrossAppsDependencyOrder => {
    id: "ext-validate-across-apps-dependency-order",
    assumed: "OnBeforeValidate and OnAfterValidate triggers that table extensions of different apps add to one field run app by app, an app before the apps that depend on it (directly or through others), both before and after the field's own OnValidate.",
    tests: "No Microsoft test: al-runner issue 6486, an ISV eBill app and its base app both extend Sales Header.\"Bill-to Customer No.\" (140 tests refused). Measured on BC by corpus writer Y20 (Y20CrossAppValidateOrder.al, SaaS sandbox, newest major).",
    probe: "Y20CrossAppValidateOrder.al on branch al-runtime-rs-staging-y20 with fixtures xapp-a to xapp-d: the log of every trigger for one Validate.",
    awaiting: Awaiting {
        issues: &[6486],
        files: &["tests/al-language/tableextension/lt-bw-cross-app/Y20CrossAppValidateOrder.al"],
        since: "2026-10-31",
    },
}
