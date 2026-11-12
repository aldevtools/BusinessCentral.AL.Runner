HttpStreamWriteKeepsRemovedContentType => {
    id: "http-stream-write-keeps-removed-content-type",
    assumed: "WriteFrom(InStream) on content after HttpHeaders.Remove('Content-Type') leaves it absent; a text WriteFrom restores text/plain (lt-http-header-rules/P87ContentType.al:WriteAfterRemovedType).",
    tests: "Code that removes the Content-Type and then writes a stream.",
    probe: "Write content, remove 'Content-Type', then WriteFrom(InStream) and Contains('Content-Type'); repeat on response content.",
}
