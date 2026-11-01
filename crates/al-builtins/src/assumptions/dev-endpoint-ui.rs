DevEndpointUi => {
    id: "dev-endpoint-ui",
    assumed: "Opt-in, `al-runner test --dev-endpoint-ui`: a UI call no handler takes gets the answer Business Central's dev-endpoint test runner gives it (measured per call shape on real BC), instead of the \"Unhandled UI\" error of the TestRunner codeunit mode the corpus runs under (handlers/TestUnhandledUiAndDialogDefaults.al:68-107). A pass under this flag would end in that error under Microsoft's Test Runner app: the test relied on a dialog it never handled.",
    tests: "Microsoft tests that raise UI with no handler declared and were written for the dev-endpoint runner. Without the flag they fail with Unhandled UI, as on real BC under a TestRunner codeunit.",
    probe: "None that removes it: the flag is the user's choice of runner mode. The answers per shape come from probes run through the dev endpoint.",
}
