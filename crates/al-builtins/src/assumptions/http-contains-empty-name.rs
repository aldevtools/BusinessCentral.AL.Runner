HttpContainsEmptyName => {
    id: "http-contains-empty-name",
    assumed: "HttpHeaders.Contains('') raises InvalidHeader:Argument, as Add with an empty name does (network/lt-ay-http/AyHeaderAddResult.al:159).",
    tests: "Code that asks a header collection for an empty name.",
    probe: "Contains('') on request, content, response and client headers inside a TryFunction: GetLastErrorCode and GetLastErrorText.",
}
