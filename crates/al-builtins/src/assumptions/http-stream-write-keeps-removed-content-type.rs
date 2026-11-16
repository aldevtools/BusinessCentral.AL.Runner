HttpStreamWriteKeepsRemovedContentType => {
    id: "http-stream-write-keeps-removed-content-type",
    assumed: "On BC 27.5, 28.5 and 29.1, WriteFrom(InStream) preserves an absent Content-Type or application/json re-added after a text write, on plain content with no other headers or previously used content stream, including copies and response content, using a fresh Blob stream. A text write restoring the default is separately pinned by http/headers/P87ContentType.al:137.",
    tests: "Local BC 27.5, 28.5 and 29.1 runs of the neutral AL reproduction in the probe PR.",
    probe: "Open aldevtools/BusinessCentral.AL.Language.Tests#1424, tests/al-language/http/content/HttpContent.al:717-793; the BC 29.0 CI result is pending.",
    awaiting: Awaiting {
        issues: &[],
        files: &[
            "tests/al-language/http/content/HttpContent.al::HttpContent_WriteStream_FreshAndChangedTypesPreserved",
            "tests/al-language/http/content/HttpContent.al::HttpContent_WriteStream_ResetAndCopyPreserveType",
            "tests/al-language/http/content/HttpContent.al::HttpContent_WriteStream_CopiesPreserveIndependentTypes",
            "tests/al-language/http/content/HttpContent.al::HttpContent_WriteStream_ResponseContentPreservesType",
            "tests/al-language/http/content/HttpContent.al::HttpContent_WriteStream_RewriteAndClearLeavesTypeAbsent",
        ],
        since: "2026-11-16",
    },
}
