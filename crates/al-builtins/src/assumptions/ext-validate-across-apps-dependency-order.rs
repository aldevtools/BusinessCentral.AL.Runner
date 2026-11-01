ExtValidateAcrossAppsDependencyOrder => {
    id: "ext-validate-across-apps-dependency-order",
    assumed: "OnBeforeValidate and OnAfterValidate triggers that table extensions of different apps add to one field run app by app, an app before the apps that depend on it (directly or through others), both before and after the field's own OnValidate.",
    tests: "No Microsoft test: an outside report, an ISV eBill app and its base app both extend Sales Header.\"Bill-to Customer No.\" (140 tests refused). Measured on BC by corpus writer Y20 (Y20CrossAppValidateOrder.al, SaaS sandbox, newest major).",
    probe: "Pinned by tableextension/lt-bw-cross-app/Y20CrossAppValidateOrder.al (corpus pin 92031e20), but this runtime refuses its 2 tests (more than one table extension declares OnBeforeValidate or OnAfterValidate for the field in a shape the runtime does not order). Retire once they run.",
}
