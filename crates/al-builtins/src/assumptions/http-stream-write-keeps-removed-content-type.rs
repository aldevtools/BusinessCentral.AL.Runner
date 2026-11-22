HttpStreamWriteKeepsRemovedContentType => {
    id: "http-stream-write-keeps-removed-content-type",
    assumed: "HttpContent.WriteFrom(InStream) leaves the Content-Type as it is, instead of setting 'text/plain; charset=utf-8', on the minors the probe measured (al-version http_probe_1424_measured) and only when: the headers hold nothing, or after a WriteFrom exactly 'Content-Type: application/json'; the content is initialized and not secret, with no WriteFrom(InStream) and no ReadAs since the last WriteFrom or Clear, its body not handed out as a stream, and no secret header; the stream is an InStream of a Blob field at its start, never read, assigned or handed to a content. This covers copies of a content and the content of a response. A Content-Type of exactly 'text/plain; charset=utf-8' and a secret content keep the pinned default (http/headers/P87ContentStream.al:44, xml/xml/P97ContentSecret.al:167). Every other state refuses, and so does every other minor.",
    tests: "No Microsoft test. The writer of runner PR 7870 ran the probe on local hosts of three minors (al-version http_probe_1424_measured), and its corpus CI passed the stream cells on its three CI legs, one per major (two of the three legs are red on an unrelated test).",
    probe: "Open corpus PR 1424: tests/al-language/http/content/HttpContent.al:717 HttpContent_WriteStream_FreshAndChangedTypesPreserved, :733 _ResetAndCopyPreserveType, :748 _CopiesPreserveIndependentTypes, :766 _ResponseContentPreservesType, :782 _RewriteAndClearLeavesTypeAbsent, :975 HttpContent_WriteStream_ChangedTypesAcrossStreamStates, :988 HttpResponseContent_WriteStream_ChangedTypesAcrossStreamStates (source state 0: an empty Temp Blob InStream at position 0). The partly read InStream (Remove, 3 characters read first) is measured by :975 and not yet admitted (refused). Still to measure (runner PR 7870 body): a ReadAs before the stream write, an InStream not on a Blob field, a second WriteFrom(InStream), a Content-Type of 'application/json; charset=utf-8'.",
    awaiting: Awaiting {
        issues: &[],
        files: &[
            "tests/al-language/http/content/HttpContent.al::HttpContent_WriteStream_FreshAndChangedTypesPreserved",
            "tests/al-language/http/content/HttpContent.al::HttpContent_WriteStream_ResetAndCopyPreserveType",
            "tests/al-language/http/content/HttpContent.al::HttpContent_WriteStream_CopiesPreserveIndependentTypes",
            "tests/al-language/http/content/HttpContent.al::HttpContent_WriteStream_ResponseContentPreservesType",
            "tests/al-language/http/content/HttpContent.al::HttpContent_WriteStream_RewriteAndClearLeavesTypeAbsent",
            "tests/al-language/http/content/HttpContent.al::HttpContent_WriteStream_ChangedTypesAcrossStreamStates",
            "tests/al-language/http/content/HttpContent.al::HttpResponseContent_WriteStream_ChangedTypesAcrossStreamStates",
        ],
        since: "2026-11-16",
    },
}
