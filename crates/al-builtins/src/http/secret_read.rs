//! Which `HttpContent.ReadAs(var SecretText)` calls have evidence: as a statement, and on the content of a request that has
//! none also with the result used (`content::read_as_secret`).
//!
//! A content carries the calls made on it so far as one [`SecretReadShape`]. Each shape is one call sequence of the probe
//! al-language-onprem/http/N4ReadAsSecretStatement.al (cited below as N4, measured under assumption
//! `http-readas-secret-statement`) or of corpus http/CzHttpNoNetwork.al (Cz). A call, a copy or a request that those files do
//! not have at that point gives `Other`, and `Other` never leaves: the read keeps the refusal it has on main.

/// The calls made on an `HttpContent` so far, as far as the evidence follows them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretReadShape {
    /// A content variable no method was called on (N4:117 `Fresh`). `Clear(C)` and `ClearAll` give a new variable
    /// (http/client/UfHttpClear.al:83-104 reads it, lists its headers and writes it as a new one; Cz:49 through the ClearAll
    /// of Http Content Impl. Create).
    New,
    /// `New`, then `WriteFrom(SecretText)` of a secret that is not empty (N4:120).
    SecretWritten,
    /// `SecretWritten`, then one statement read (N4:123).
    SecretReadOnce,
    /// `New`, then `WriteFrom(Text)`, of a text or of '' (N4:121-122).
    TextWritten,
    /// The content a request without one, never assigned or passed by value, got with `Content(C)` from a `SecretWritten` C
    /// (N4:167-168).
    RequestSecret,
    /// `RequestSecret`, then one statement read in place (N4:171).
    RequestSecretStatement,
    /// `RequestSecretStatement`, then one result-used read in place (N4:172).
    RequestSecretRead,
    /// A copy of a `RequestSecretRead` content (N4:175).
    RequestSecretCopy,
    /// As `RequestSecret`, from a `TextWritten` C (N4:169-170).
    RequestText,
    /// The content of a request that has none and was never assigned or passed by value (N4:193, :216).
    RequestNone,
    /// `RequestNone`, then one statement read in place (N4:194).
    RequestNoneStatement,
    /// `RequestNoneStatement`, then one result-used read in place (N4:195).
    RequestNoneRead,
    /// A copy of a `RequestNoneRead` content (N4:196).
    RequestNoneCopy,
    /// `RequestNoneCopy`, then one statement read (N4:197).
    RequestNoneCopyStatement,
    /// `New`, then `Clear()` (Cz:49 and :57 through Http Content Impl. SetContent).
    Cleared,
    /// `Cleared`, then `WriteFrom(SecretText)` (the same SetContent).
    ClearedSecret,
    /// `ClearedSecret`, then one `GetHeaders` (the same SetContent, through SetHeader).
    ClearedSecretHeaders,
    /// A copy of a `ClearedSecretHeaders` content, or of such a copy (Cz:49: GetHttpContent returns it and DescribeContent, :25,
    /// takes it by value).
    ClearedSecretCopy,
    /// `ClearedSecretCopy`, then one `IsSecretContent` (Cz:33).
    ClearedSecretCopyAsked,
    /// Any other content: a response's, or one after a call, a copy or a request use that the evidence does not have there.
    Other,
}

/// A content method, as the shapes tell them apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Call {
    WriteText,
    /// `WriteFrom(SecretText)` of a secret that is not empty (N4:120, :167; Cz:67). An empty one is `Other`.
    WriteSecret,
    Clear,
    GetHeaders,
    IsSecretContent,
    /// `ReadAs(var SecretText)` as a statement.
    SecretStatement,
    /// `ReadAs(var SecretText)` with its result used.
    SecretUsed,
    Other,
}

impl SecretReadShape {
    /// The shape after `call`. Every pair the evidence does not have is `Other`.
    pub(super) fn after(self, call: Call) -> Self {
        use SecretReadShape as S;
        match (self, call) {
            (S::New, Call::WriteSecret) => S::SecretWritten,
            (S::New, Call::WriteText) => S::TextWritten,
            (S::New, Call::Clear) => S::Cleared,
            (S::SecretWritten, Call::SecretStatement) => S::SecretReadOnce,
            (S::RequestSecret, Call::SecretStatement) => S::RequestSecretStatement,
            (S::RequestSecretStatement, Call::SecretUsed) => S::RequestSecretRead,
            (S::RequestNone, Call::SecretStatement) => S::RequestNoneStatement,
            (S::RequestNoneStatement, Call::SecretUsed) => S::RequestNoneRead,
            (S::RequestNoneCopy, Call::SecretStatement) => S::RequestNoneCopyStatement,
            (S::Cleared, Call::WriteSecret) => S::ClearedSecret,
            (S::ClearedSecret, Call::GetHeaders) => S::ClearedSecretHeaders,
            (S::ClearedSecretCopy, Call::IsSecretContent) => S::ClearedSecretCopyAsked,
            _ => S::Other,
        }
    }

    /// The shape of a copy of this content (an assignment, a by-value argument, a return value). The evidence never calls
    /// the content a copy was taken of again, so the caller makes that one `Other`.
    pub(super) fn of_copy(self) -> Self {
        use SecretReadShape as S;
        match self {
            S::RequestSecretRead => S::RequestSecretCopy,
            S::RequestNoneRead => S::RequestNoneCopy,
            S::ClearedSecretHeaders | S::ClearedSecretCopy => S::ClearedSecretCopy,
            _ => S::Other,
        }
    }

    /// The shape of the content a request gets with `Content(C)` from a C of this shape, when the request had no content and
    /// was never assigned or passed by value.
    pub(super) fn set_on_request(self) -> Self {
        use SecretReadShape as S;
        match self {
            S::SecretWritten => S::RequestSecret,
            S::TextWritten => S::RequestText,
            _ => S::Other,
        }
    }

    /// A statement read gives the body: N4:123 `secret`, :127 `secret_again`, :124 `plain`, :126 `empty`, :125 `fresh`,
    /// :171 `secret_inplace_statement`, :173 `plain_inplace_statement`, :176 `secret_copy_statement`; Cz:34 (pinned as not
    /// empty by :80, the body is the assumption's).
    pub(super) fn statement_gives_body(self) -> bool {
        use SecretReadShape as S;
        matches!(
            self,
            S::SecretWritten
                | S::SecretReadOnce
                | S::TextWritten
                | S::New
                | S::RequestSecret
                | S::RequestText
                | S::RequestSecretCopy
                | S::ClearedSecretCopyAsked
        )
    }

    /// A read of the content of a request that has none is measured: the statement raises NotInitialized (N4:194 and :218
    /// `inplace_statement`, :197 `copy_statement`), the result-used form answers false (N4:195 `inplace_used`, :198
    /// `copy_used`).
    pub(super) fn without_content_measured(self, result_used: bool) -> bool {
        use SecretReadShape as S;
        matches!(
            (self, result_used),
            (S::RequestNone | S::RequestNoneCopy, false)
                | (S::RequestNoneStatement | S::RequestNoneCopyStatement, true)
        )
    }
}
