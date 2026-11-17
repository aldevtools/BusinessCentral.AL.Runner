HttpReadasSecretStatement => {
    id: "http-readas-secret-statement",
    assumed: "HttpContent.ReadAs(var SecretText) as a statement gives the body as the secret after the one call sequence corpus http/CzHttpNoNetwork.al:25-57 pins only as not empty (`<secret empty=No>`, :80): a new content, Clear(), WriteFrom(SecretText), GetHeaders, copies, IsSecretContent, one read (http::SecretReadShape::ClearedSecretCopyAsked). The body itself is assumed. Every other statement read runs only in the sequences al-language-onprem/http/N4ReadAsSecretStatement.al measures, or keeps the refusal.",
    tests: "No Microsoft test. The sequence is the one System Application's Http Content codeunit makes (Create(SecretText) then GetHttpContent).",
    probe: "Open corpus PR 1418 (tests/al-language-onprem/http/LtHttpReadAsSecretCleared.al): unwraps the secret read after a cleared copy, the sequence of CzHttpNoNetwork.al:25-57. The statement and result-used forms after WriteFrom(SecretText) alone are pinned by al-language-onprem/http/N4ReadAsSecretStatement.al:113-157.",
    awaiting: Awaiting {
        issues: &[],
        files: &["tests/al-language-onprem/http/LtHttpReadAsSecretCleared.al"],
        since: "2026-11-15",
    },
}
