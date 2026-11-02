UserAuthEmailRfcForms => {
    id: "user-auth-email-rfc-forms",
    assumed: "User.\"Authentication Email\" on Insert and Modify: a quoted local part, the RFC 5322 signs and braces in an unquoted one, a trailing dot, a comment before the '@', a word and a space before the address, a dotless, IPv4-literal, '_' or leading '-' domain, non-ASCII letters, and the invalid forms (two '@', empty local part or domain, a leading dot, a doubled dot on the newest major), as corpus writer Y66 measured them on real BC of three majors (not on corpus master yet).",
    tests: "No Microsoft test: al-runner issue 6662, a User whose \"Authentication Email\" is a formatted Guid '{...}@EXAMPLE.COM'.",
    probe: "Y66AuthEmailLocalPart.al:25-125: the reporter's shape, local parts, domains and the version-dependent forms, Insert under asserterror.",
    awaiting: Awaiting {
        issues: &[6662],
        files: &["tests/al-language-onprem/access-control/lt-y66-auth-email-local-part/Y66AuthEmailLocalPart.al"],
        since: "2026-11-02",
    },
}
