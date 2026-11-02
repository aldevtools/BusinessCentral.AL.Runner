use super::*;

/// The error state AL code can read back (`GetLastErrorText`, ...).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AlError {
    pub message: String,
    pub code: String,
    pub call_stack: String,
    /// How the error was raised, which `GetLastErrorText(true)` needs.
    pub origin: ErrorOrigin,
    /// The .NET exception BC holds for the error, which `GetLastErrorObject` needs.
    pub exception: ExceptionKind,
}

/// The .NET exception type behind an error, for `GetLastErrorObject`. Only the kinds the corpus pins are named
/// (tests/al-language-onprem-dotnet/lt-dotnet-interop/GetLastErrorObject.al); an error raised any other way is
/// `Unknown` and `GetLastErrorObject` refuses it, naming the code.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ExceptionKind {
    #[default]
    Unknown,
    /// `Error('literal', args)`: NavNCLDialogException, code `Dialog` (GetLastErrorObject.al:138-230).
    Dialog,
    /// `Error(Text, ...)` with a text that is not a literal (a Label, a variable): assumed the same exception.
    DialogComputed,
    /// `Get` without a row: NavCSideRecordNotFoundException, code `DB:RecordNotFound` (GetLastErrorObject.al:232-255).
    RecordNotFound,
    /// A division by zero: NavALException with an inner exception, code `AL:DivideByZero` (GetLastErrorObject.al:257-280).
    DivideByZero,
    /// Set by `ClearLastError`: no error, as opposed to the default of a session where no error has happened yet.
    Cleared,
}

/// How the text of `Error(Text, ...)` or `FieldError(Field, Text)` is written in the code: the text `GetLastErrorText(true)`
/// answers depends on it (error-handling/lt-last-error-text-exclude/LastErrorTextExcludeLabel.al, error-handling/
/// lt-last-error-text-forms/). Lowering decides it from the syntax.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorTextForm {
    /// A string literal.
    Literal,
    /// A Label or TextConst variable of the object.
    Constant,
    /// A Text or Code variable of the routine, a concatenation with `+` or a `StrSubstNo` call.
    Computed,
    /// Any other form: the flag's text for it is not pinned.
    Other,
}

/// How an error was raised, for the text `GetLastErrorText(true)` answers (corpus
/// types/lt-evaluate-format-lasterror-setview/P86LastError.al, errors/lt-last-error-and-events/P70_02LastErrFlag.al,
/// misc/lt-probes-needed-6/PnF01LastErrorFlag.al). An origin the corpus does not pin is `Unknown` and refused.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ErrorOrigin {
    #[default]
    Unknown,
    /// The flag leaves the text as it is (a runtime error without customer content: a division by zero, an empty table).
    SameText,
    /// The text the flag answers instead (the personal data removed from it).
    Redacted(String),
}

/// The text of `Error(ErrorInfo)` for the flag (P86LastError.al:54, :88-104, :121, :145, :169).
pub const PERSONAL_DATA_REMOVED: &str = "This message had personal data removed. The original may still be in transient telemetry. Find it using the transientTelemetryId.";

/// The text of `Error('literal', args)` for the flag (P86LastError.al:189, PnF01LastErrorFlag.al:85, :103).
pub const USE_ERROR_WITH_A_TEXT_CONSTANT: &str = "Use ERROR with a text constant to improve telemetry details";

/// Raises an AL error with a pinned text and code, and the pinned text `GetLastErrorText(true)` answers for it.
pub fn raise_redacted<T>(message: impl Into<String>, code: &str, redacted: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Error(Box::new(AlError {
        message: message.into(),
        code: code.into(),
        origin: ErrorOrigin::Redacted(redacted.into()),
        ..Default::default()
    })))
}

/// Raises an AL error with a pinned text and code.
pub fn raise<T>(message: impl Into<String>, code: &str) -> Result<T, Signal> {
    // A division by zero carries no customer content: the flag leaves its text (P86LastError.al:RuntimeError_Other `div_excl`).
    let (origin, exception) = if code == "AL:DivideByZero" {
        (ErrorOrigin::SameText, ExceptionKind::DivideByZero)
    } else {
        (ErrorOrigin::Unknown, ExceptionKind::Unknown)
    };
    Err(Signal::Error(Box::new(AlError {
        message: message.into(),
        code: code.into(),
        origin,
        exception,
        ..Default::default()
    })))
}

/// Non-local control flow (ADR-002, ADR-011).
#[derive(Debug, Clone)]
pub enum Signal {
    Error(Box<AlError>),
    Exit(Option<Value>),
    Break,
    /// `continue`: go on with the next round of the innermost loop.
    Continue,
    /// Behaviour the runtime does not implement yet, met at run time (an edge
    /// case no corpus test pins). The runner reports the test as not run.
    Unsupported(String),
}
