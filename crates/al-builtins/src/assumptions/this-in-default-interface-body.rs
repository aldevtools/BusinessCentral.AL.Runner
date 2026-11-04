ThisInDefaultInterfaceBody => {
    id: "this-in-default-interface-body",
    assumed: "In the default body of an interface procedure (runtime 18.0), `this.Member(..)` calls that member of the implementing codeunit the body runs on, as a bare `Member(..)` does (codeunits/CwDefaultInterfaceMethods.al:30-33 pins the bare call).",
    tests: "Microsoft PEPPOL Tests: PEPPOL30 Management Tests and PEPPOL BIS BillingTests, every test that writes a tax subtotal (interface PEPPOL Tax Info Provider, default GetTaxExemptionReason calls this.GetTaxExemptionReason; 52 tests).",
    probe: "R6924dThisInDefaultBody.al on corpus branch al-runtime-rs-staging-r6924d: this.Req() from a default body for a codeunit that declares Req and one that inherits a default that calls it; this.Overload(..) choosing the overload by arity.",
    awaiting: Awaiting {
        issues: &[6924],
        files: &["tests/al-language/codeunits/interfaces/R6924dThisInDefaultBody.al"],
        since: "2026-11-04",
    },
}
