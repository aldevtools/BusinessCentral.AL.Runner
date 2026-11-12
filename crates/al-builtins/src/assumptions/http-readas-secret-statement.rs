HttpReadasSecretStatement => {
    id: "http-readas-secret-statement",
    assumed: "HttpContent.ReadAs(var SecretText) as a statement gives the body as the secret, as the result-used form does: of a content written from a SecretText or a Text, a new or an empty content, and the content of a request read in place or through a copy. On the content of a request that has none, the statement raises NotInitialized (Microsoft.Dynamics.Nav.Runtime.NavHttpContent variable not initialized.) and the result-used form answers false; both leave the secret as it was. A statement on a content written from or read as a stream stays refused.",
    tests: "Measured by the probe on a local BC service tier of the middle supported major (observe form). al-runner issue 7862 (an outside report: a test mock that reads the request body into a SecretText).",
    probe: "http/N4ReadAsSecretStatement.al in the OnPrem app (Unwrap of the read secret): the statement and result-used forms on written, new and empty contents, on a request's content in place and through a copy, and on a request without content, in a TryFunction and under asserterror. Corpus branch al-runtime-rs-staging-natives4.",
    awaiting: Awaiting {
        issues: &[7862],
        files: &["tests/al-language-onprem/http/N4ReadAsSecretStatement.al"],
        since: "2026-11-12",
    },
}
