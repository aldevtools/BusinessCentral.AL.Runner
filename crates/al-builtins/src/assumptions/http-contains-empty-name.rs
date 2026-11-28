HttpContainsEmptyName => {
    id: "http-contains-empty-name",
    assumed: "HttpHeaders.Contains('') raises InvalidHeader:Argument, with the text 'Invalid HTTP header. Please, make sure the format of the header is correct.', on the minors the probe measured (al-version http_probe_1424_measured) and only on a collection no Add of a text value, TryAddWithoutValidation, Remove, Clear, SetCookie, second WriteFrom(Text), WriteFrom(InStream) or WriteFrom(SecretText) changed and that no HttpContent.Clear, content copy or client rebuild made: request, response or client headers that hold nothing; an HttpHeaders variable nothing filled; request headers that hold only X-Secret added from a SecretText of 'body'; content headers that hold only 'Content-Type: text/plain; charset=utf-8' (a fresh content, the request's own content after one WriteFrom(Text)); and the fresh content of a response never sent. Refused: every other collection, a standalone or response content after a text write, the headers and content of a response a handler filled, the headers of a secret content, and every other minor.",
    tests: "No Microsoft test. The writer of runner PR 7870 ran the probe on local hosts of three minors (al-version http_probe_1424_measured), and its corpus CI passed the three Contains cells on its three CI legs, one per major. The probe measures a content after one WriteFrom(Text) only as request content, and a response only as a response variable never sent.",
    probe: "Corpus PR 1424, merged, not yet pinned: tests/al-language/http/HttpHeaders.al:531 HttpHeaders_ContainsEmpty_ReceiversError, :558 HttpHeaders_ContainsEmpty_SecretError, :579 HttpHeaders_ContainsEmpty_RequestAndResponseContentError. Still to measure (runner PR 7870 body): a collection after Add then Remove, client headers after Clear, content headers after the Content-Type is removed and added again, a standalone content after one WriteFrom(Text), a response content after a write, the content and headers of a response a test handler filled, a content after any ReadAs (all refused).",
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
