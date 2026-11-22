HttpContainsEmptyName => {
    id: "http-contains-empty-name",
    assumed: "HttpHeaders.Contains('') raises InvalidHeader:Argument, with the text 'Invalid HTTP header. Please, make sure the format of the header is correct.', on the minors the probe measured (al-version http_probe_1424_measured) and only on a collection no Add of a text value, TryAddWithoutValidation, Remove, Clear, SetCookie, second WriteFrom(Text), WriteFrom(InStream) or WriteFrom(SecretText) changed and that no HttpContent.Clear, content copy or client rebuild made: request, response or client headers that hold nothing; an HttpHeaders variable nothing filled; request headers that hold only X-Secret added from a SecretText of 'body'; content headers that hold only 'Content-Type: text/plain; charset=utf-8' (a fresh content, a content after one WriteFrom(Text)); and the fresh content of a response. Refused: every other collection, the headers of a secret content, and every other minor.",
    tests: "No Microsoft test. The writer of runner PR 7870 ran the probe on local hosts of three minors (al-version http_probe_1424_measured), and its corpus CI passed the three Contains cells on its three CI legs, one per major. The probe measures a content after one WriteFrom(Text) only as request content.",
    probe: "Open corpus PR 1424: tests/al-language/http/HttpHeaders.al:518 HttpHeaders_ContainsEmpty_ReceiversError, :545 HttpHeaders_ContainsEmpty_SecretError, :566 HttpHeaders_ContainsEmpty_RequestAndResponseContentError. Still to measure (runner PR 7870 body): a collection after Add then Remove, client headers after Clear, content headers after the Content-Type is removed and added again, a standalone content after one WriteFrom(Text), a response content a test handler wrote.",
    awaiting: Awaiting {
        issues: &[],
        files: &[
            "tests/al-language/http/HttpHeaders.al::HttpHeaders_ContainsEmpty_ReceiversError",
            "tests/al-language/http/HttpHeaders.al::HttpHeaders_ContainsEmpty_SecretError",
            "tests/al-language/http/HttpHeaders.al::HttpHeaders_ContainsEmpty_RequestAndResponseContentError",
        ],
        since: "2026-11-16",
    },
}
