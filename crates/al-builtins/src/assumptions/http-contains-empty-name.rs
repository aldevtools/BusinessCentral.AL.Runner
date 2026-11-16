HttpContainsEmptyName => {
    id: "http-contains-empty-name",
    assumed: "On BC 27.5, 28.5 and 29.1, HttpHeaders.Contains('') raises InvalidHeader:Argument on an uninitialized, request, content, response or client header collection, including a named secret header. Existing receiver and secret-content refusals remain.",
    tests: "Local BC 27.5, 28.5 and 29.1 runs of the neutral AL reproduction in the probe PR.",
    probe: "Open aldevtools/BusinessCentral.AL.Language.Tests#1424, tests/al-language/http/HttpHeaders.al:518 and :545; the BC 29.0 CI result is pending.",
    awaiting: Awaiting {
        issues: &[],
        files: &[
            "tests/al-language/http/HttpHeaders.al::HttpHeaders_ContainsEmpty_ReceiversError",
            "tests/al-language/http/HttpHeaders.al::HttpHeaders_ContainsEmpty_SecretError",
        ],
        since: "2026-11-16",
    },
}
