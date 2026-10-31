HttpRelativeRequestPath => {
    id: "http-relative-request-path",
    assumed: "HttpClient.Get/Post/Put/Delete/Patch with a path that is not an absolute uri joins it to the client's base address like new Uri(base, path), as a [HttpClientHandler] sees it: a rooted path replaces the base path, any other joins the base path's directory, and dot segments are removed. Pinned (no assumption) for a base address ending in a slash and a plain word, a rooted word, ../ and a word, or a query alone (al-language-onprem/network/lt-ay-http-handler/AyHttpHandler.al:BaseAddress_RelativePaths_1 and _2, newest major).",
    tests: "Microsoft and NP Retail code that sets SetBaseAddress and then Get('/orders').",
    probe: "SetBaseAddress('https://example.invalid/api') (no trailing slash) then Get('orders'); SetBaseAddress('https://example.invalid/api/v1/') then Get('a/b'), Get('./x'), Get('a/../b'), Get('../../x'), Get('orders?x=1#f') with a handler printing Request.Path.",
}
