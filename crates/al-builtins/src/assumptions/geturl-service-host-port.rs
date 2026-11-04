GetUrlServiceHostPort => {
    id: "geturl-service-host-port",
    assumed: "GetUrl(ClientType::SOAP, OData, ODataV4, Api) without an object uses the scheme, host name and instance of the server base address (--server-url, ADR-042), with port 7047 for SOAP and 7048 for OData, ODataV4 and Api. Measured on real BC (three majors) by corpus branch al-runtime-rs-staging-p6924c; the entry goes when that file is in the pinned corpus.",
    tests: "Company-Initialize and Microsoft tests that build web service addresses (al-runner issue 6924).",
    probe: "session-system/session/GetUrlShape.al GU6924_ServiceForms_ShareWebHost and GU6924_ServiceForms_WithObject_ShareWebHost on corpus branch al-runtime-rs-staging-p6924c.",
    awaiting: Awaiting {
        issues: &[6924],
        files: &[
            "tests/al-language/session-system/session/GetUrlShape.al::GU6924_ServiceForms_ShareWebHost",
            "tests/al-language/session-system/session/GetUrlShape.al::GU6924_ServiceForms_WithObject_ShareWebHost",
        ],
        since: "2026-11-04",
    },
}
