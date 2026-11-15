HttpReadasSecretStatement => {
    id: "http-readas-secret-statement",
    assumed: "HttpContent.ReadAs(var SecretText) as a statement gives the body as the secret after the one call sequence corpus http/CzHttpNoNetwork.al:25-57 pins only as not empty (`<secret empty=No>`, :80): a new content, Clear(), WriteFrom(SecretText), GetHeaders, copies, IsSecretContent, one read (http::SecretReadShape::ClearedSecretCopyAsked). The body itself is assumed. Every other statement read runs only in the sequences al-language-onprem/http/N4ReadAsSecretStatement.al measures, or keeps the refusal.",
    tests: "No Microsoft test. The sequence is the one System Application's Http Content codeunit makes (Create(SecretText) then GetHttpContent).",
    probe: "None written yet: a probe in the OnPrem app that unwraps the secret read after the Http Content codeunit's Create(SecretText) and GetHttpContent. The statement and result-used forms after WriteFrom(SecretText) alone are pinned by al-language-onprem/http/N4ReadAsSecretStatement.al:113-157.",
}
