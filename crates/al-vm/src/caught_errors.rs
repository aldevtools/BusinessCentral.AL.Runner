//! The count of errors AL code caught in a session. Code that clears or restores the last error (ClearLastError, a caught
//! error with the same text as the one before) leaves the count as it was, so "the code caught an error" is read from the
//! count, never from a change of the last error. Its reader: the close of the pinned Document after its ModalPageHandler
//! raised (pages/document_handler_error.rs `close_document_after_handler_error`), whose pinned handler catches nothing
//! (pages/close-errors/CemDocumentTests.al:925-975 `CemDocModal`).
//!
//! An error that ends here also ends the write-back of the routine it left (`Session::error_finals`): the parameters a
//! failing routine leaves are for the AL call site the error reaches next (errors/TestTryFunctionContracts.al:85-95,
//! codeunits/TestScopeIsolationContracts.al:63-77). Where nothing took them (an Isolated event's subscriber, a trigger or
//! table event under a builtin), a later call that fails before its routine starts would write them into its own `var`
//! arguments (al-vm tests/language/error_finals_after_catch.rs).

use al_builtins::native::{AlError, ExceptionKind, Signal};

use crate::Session;

impl Session {
    /// AL code caught error `e`: asserterror, a TryFunction call whose result is used, `Codeunit.Run` with its result used,
    /// an isolated event subscriber, or a builtin with its result used. `e` becomes the last error and the count goes up.
    ///
    /// The record of a Rename that raised the SQL range error of its new key still lives where the error is caught: what it
    /// holds then is not measured (record/filters/SqlDateFloor1753.al:2468-2600 reads only the stored row), so that is refused.
    pub(crate) fn catch_error(&mut self, e: AlError) -> Result<(), Signal> {
        self.refuse_live_failed_rename()?;
        self.note_caught(e);
        Ok(())
    }

    /// A page, report or xmlport path ended a trigger's error itself and keeps `e` as the last error, without AL code catching
    /// it (the count stays). The record of a failed Rename is refused there as in [`Self::catch_error`].
    pub(crate) fn hold_error(&mut self, e: AlError) -> Result<(), Signal> {
        self.refuse_live_failed_rename()?;
        self.error_finals = None;
        self.last_error = e;
        Ok(())
    }

    fn refuse_live_failed_rename(&mut self) -> Result<(), Signal> {
        if self.tx.failed_sql_rename.take().is_some_and(|r| r.upgrade().is_some()) {
            return Err(Signal::unsupported(
                "a record whose Rename raised the SQL range error of its new key is still alive where the error is caught (what it holds is not measured)",
            ));
        }
        Ok(())
    }

    fn note_caught(&mut self, e: AlError) {
        self.error_finals = None;
        self.last_error = e;
        self.caught_errors += 1;
    }

    /// `Ctx::set_last_error`: ClearLastError writes the cleared error (native/error_fns.rs `clear_last_error`); every other
    /// builtin that calls it caught `e` because its result is used.
    pub(crate) fn set_last_error_of_builtin(&mut self, e: AlError) {
        if e.exception == ExceptionKind::Cleared {
            self.last_error = e;
        } else {
            self.note_caught(e);
        }
    }

    /// How many errors AL code caught in this session so far.
    pub(crate) fn caught_errors(&self) -> u64 {
        self.caught_errors
    }
}
