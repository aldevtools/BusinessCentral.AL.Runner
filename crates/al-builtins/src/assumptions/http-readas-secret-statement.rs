HttpReadasSecretStatement => {
    id: "http-readas-secret-statement",
    assumed: "HttpContent.ReadAs(var SecretText) as a statement gives the body as the secret, as the result-used form does, on exactly these contents: a content variable no method was called on (Clear of the variable makes a new one); a content variable written once from a Text or a SecretText (later ReadAs(SecretText) calls kept); the content a request got with Content(C) from such a written C, read in place; a copy of that request content when C was written from a SecretText; and the shape corpus http/CzHttpNoNetwork.al:25-57 pins as not empty (a new content, Clear, WriteFrom(SecretText), then GetHeaders, IsSecretContent and copies), whose body is assumed. On the content of a request that has none, in place or copied and with no other call on it, the statement raises NotInitialized (Microsoft.Dynamics.Nav.Runtime.NavHttpContent variable not initialized.) and the result-used form answers false; both leave the secret as it was. Every other content (a response's, after ReadAs(Text), ReadAs(InStream), a Clear after a write, GetHeaders or IsSecretContent on a plain content, WriteFrom(InStream), a second WriteFrom, a copy of any other content) keeps the refusal.",
    tests: "Measured by the probe on local BC service tiers of three supported majors (observe form), with the same result on each. al-runner issue 7862 (an outside report: a test mock that reads the request body into a SecretText).",
    probe: "http/N4ReadAsSecretStatement.al in the OnPrem app (Unwrap of the read secret): the statement and result-used forms on written, new and empty contents, on a request's content in place and through a copy, and on a request without content, in a TryFunction and under asserterror. Corpus branch al-runtime-rs-staging-natives4.",
    awaiting: Awaiting {
        issues: &[7862],
        files: &["tests/al-language-onprem/http/N4ReadAsSecretStatement.al"],
        since: "2026-11-12",
    },
}
