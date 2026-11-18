//! The session language (`GlobalLanguage`) and its one writer, `Session::set_language` (al-runner issue 7659). Besides
//! the field, the language lives in three copies the builtins read without the session: `EventStats::language` (the
//! Report Layout List's Description follows it), the thread-local Turkish `i` rule of Code conversions
//! (`al_builtins::ops::set_turkic_code`, CaseConv.Codeunit.al:280-310) and the language a new field filter records
//! (`al_builtins::filter::set_filter_language`). The value can only be made here, so every change keeps the copies in step.

use crate::Session;

/// The session's `GlobalLanguage`; read with [`SessionLanguage::get`], changed only by [`Session::set_language`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SessionLanguage(i32);

impl SessionLanguage {
    /// A placeholder until the new session's [`Session::set_language`] runs.
    pub(crate) fn unset() -> SessionLanguage {
        SessionLanguage(0)
    }

    pub(crate) fn get(self) -> i32 {
        self.0
    }
}

impl Session {
    /// Sets the session language to `language` and every copy of it.
    pub fn set_language(&mut self, language: i32) {
        self.global_language = SessionLanguage(language);
        self.events.event_stats.language.set(language);
        al_builtins::ops::set_turkic_code(al_builtins::native::is_turkic_language(language));
        al_builtins::filter::set_filter_language(language);
    }
}
