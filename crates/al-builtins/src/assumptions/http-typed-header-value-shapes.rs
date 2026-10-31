HttpTypedHeaderValueShapes => {
    id: "http-typed-header-value-shapes",
    assumed: "A value of a typed header (other than Cache-Control) in a shape no probe measured follows the rule of the shapes measured for that header (network/lt-ay-http/AyHeaderAddResult.al, the lt-http-header-rules/P87 files and lt-se-header-values/SeHeaderValues.al measured a few valid and invalid values of each, Y2_* also two-digit years, numeric offsets, case and spacing); a request-only name the probes did not add to response headers is stored as written there, and one the probes did not add to the client's default headers checks there as on request headers (SeHeaderValues.al Y2_RequestOnlyNames_* measured all ten request-only names).",
    tests: "Every Microsoft or NP Retail test that builds a request with headers (Authorization, Accept, Content-Type, custom X- headers).",
    probe: "More value shapes per typed header on request, response, content and client headers (lists with parameters, quoted strings, two-digit years other than 49 and 50, dates without a weekday and with an offset other than -0500 and +0000). Split from http-header-rules (2026-10-25).",
}
