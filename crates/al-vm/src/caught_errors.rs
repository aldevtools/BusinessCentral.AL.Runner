//! The count of errors AL code caught in a session. Code that clears or restores the last error (ClearLastError, a caught
//! error with the same text as the one before) leaves the count as it was, so "the code caught an error" is read from the
//! count, never from a change of the last error. Its reader: the close of the pinned Document after its ModalPageHandler
//! raised (pages/document_handler_error.rs `close_document_after_handler_error`), whose pinned handler catches nothing
//! (pages/close-errors/CemDocumentTests.al:925-975 `CemDocModal`).

use al_builtins::native::{AlError, ExceptionKind};

use crate::Session;

impl Session {
    /// AL code caught error `e`: asserterror, a TryFunction call whose result is used, `Codeunit.Run` with its result used,
    /// an isolated event subscriber, or a builtin with its result used. `e` becomes the last error and the count goes up.
    pub(crate) fn catch_error(&mut self, e: AlError) {
        self.last_error = e;
        self.caught_errors += 1;
    }

    /// `Ctx::set_last_error`: ClearLastError writes the cleared error (native/error_fns.rs `clear_last_error`); every other
    /// builtin that calls it caught `e` because its result is used.
    pub(crate) fn set_last_error_of_builtin(&mut self, e: AlError) {
        if e.exception == ExceptionKind::Cleared {
            self.last_error = e;
        } else {
            self.catch_error(e);
        }
    }

    /// How many errors AL code caught in this session so far.
    pub(crate) fn caught_errors(&self) -> u64 {
        self.caught_errors
    }
}
