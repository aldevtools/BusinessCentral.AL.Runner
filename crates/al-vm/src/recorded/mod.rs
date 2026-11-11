//! Recorded runs (ADR-073 decisions 1 to 6): one real run of a piece of AL code, kept with what it read and what it left,
//! and replayed in a later session whose inputs are all equal. The one owner of recording, the session image and replay;
//! al-testrun's login (`install.rs`) uses it and owns only its key.
//!
//! AL code the VM runs is a function of what it reads. Its inputs here are:
//! - the session's image before the run ([`SessionImage`], `image.rs`: every field of `Session` has a class), and no
//!   SingleInstance codeunit exists yet;
//! - the rows: the caller's key (the baseline the session started from with no table changed since, and who offers the
//!   rows of the untouched tables; `install.rs` `memo_key`);
//! - the engine, which is immutable while sessions run.
//!
//! Its result is the image after it, the SingleInstance codeunits it left (`globals.rs`), the rows it changed (as
//! committed row changes on top of the rows it started from), and the assumption marks it made. A replay checks that the
//! session's image now equals the recorded image before, then puts the result in place. A run that used a page, a report,
//! .NET or a file (the impact trace says so), that ended in an error or a refusal (the caller does not finish such a
//! run), or that left state no image covers, is not recorded ([`NotRecorded`]).
//!
//! Thread-local state AL code can change and a later statement reads: the assumption marks (recorded), the turkic code
//! and filter language (set from the session language on restore) and the store's operation count (the SQL counters
//! keep their counts relative to it, `sqlcount.rs` `SqlImage`). The others are scoped to one call or are tooling.
//!
//! A session that notes the code it runs (coverage, the impact trace, value capture, the debugger) never records and
//! never takes a replay (decision 6): its image check fails on `coverage`, `capture` or `debug`.

mod globals;
mod image;

pub use image::SessionImage;

use crate::Session;

/// Why a run is not recorded, or why a session takes no replay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotRecorded {
    /// The session notes the code it runs (decision 6).
    NotesCode,
    /// A SingleInstance codeunit existed before the run.
    SingleInstanceBefore,
    /// A must-be-untouched field did not hold what a new session holds.
    Untouched(&'static str),
    /// A value has no plain image.
    Value(&'static str),
    /// The run used something whose result is not a function of the session's state (decision 5).
    Used(&'static str),
    /// The rows the run changed have no plain image, or do not apply on the session's rows.
    Rows,
}

impl std::fmt::Display for NotRecorded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotRecorded::NotesCode => write!(f, "the session notes the code it runs"),
            NotRecorded::SingleInstanceBefore => write!(f, "a SingleInstance codeunit existed before"),
            NotRecorded::Untouched(what) => write!(f, "{what}"),
            NotRecorded::Value(what) => write!(f, "{what} has no image"),
            NotRecorded::Used(what) => write!(f, "the run used {what}"),
            NotRecorded::Rows => write!(f, "the rows the run changed have no image"),
        }
    }
}

/// A run being recorded ([`Session::start_recording`]).
pub struct Recording {
    before: SessionImage,
    store: al_data::memory::MemStore,
    counter: u64,
    hits_before: al_builtins::assumptions::HitBits,
}

/// One real run with its inputs and its result (see the module comment). Plain data: it can go to another thread.
#[derive(Clone)]
pub struct RecordedRun {
    before: SessionImage,
    after: SessionImage,
    singles: Vec<globals::SingleImage>,
    rows: Option<crate::baseline::EncodedChanges>,
    hits: al_builtins::assumptions::HitBits,
    touches: crate::impact::Touches,
}

// The result goes to other test threads (ADR-073 decision 1, kept on the engine behind a lock).
const _: fn() = || {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<RecordedRun>();
};

impl RecordedRun {
    /// What the impact trace saw of the run: the tables it read and wrote, the events, the routines it entered.
    pub fn touches(&self) -> &crate::impact::Touches {
        &self.touches
    }

    /// The assumption marks the run made.
    pub fn hits(&self) -> al_builtins::assumptions::HitBits {
        self.hits
    }

    /// Whether the run changed rows.
    pub fn wrote_rows(&self) -> bool {
        self.rows.is_some()
    }

    /// The WorkDate the run left (`None`: today).
    pub fn work_date_after(&self) -> Option<i32> {
        self.after.work_date()
    }
}

impl Session {
    /// Starts recording the code the caller runs next, or says why it cannot be recorded. The caller runs the code, then
    /// calls [`Session::finish_recording`] when the code ended normally; a run that ended otherwise is simply not
    /// finished (call [`Session::abandon_recording`] to switch the trace off).
    pub fn start_recording(&mut self) -> Result<Recording, NotRecorded> {
        if self.notes_code_run() {
            return Err(NotRecorded::NotesCode);
        }
        if !self.singletons.is_empty() {
            return Err(NotRecorded::SingleInstanceBefore);
        }
        let before = image::image_of(self)?;
        let hits_before = al_builtins::assumptions::take_hit_bits();
        self.enable_impact_trace();
        Ok(Recording {
            before,
            store: self.store.clone(),
            counter: self.store.change_counter(),
            hits_before,
        })
    }

    /// Ends a recording without a result: the trace is switched off and the assumption marks are kept.
    pub fn abandon_recording(&mut self, r: Recording) {
        self.disable_impact_trace();
        let hits = al_builtins::assumptions::take_hit_bits();
        al_builtins::assumptions::add_hit_bits(r.hits_before | hits);
    }

    /// The recorded run of the code run since [`Session::start_recording`], which ended normally, or why it is not one.
    /// The session itself is left as the run left it, whatever the answer.
    pub fn finish_recording(&mut self, r: Recording) -> Result<RecordedRun, NotRecorded> {
        let touches = self.take_impact_trace();
        self.disable_impact_trace();
        let hits = al_builtins::assumptions::take_hit_bits();
        al_builtins::assumptions::add_hit_bits(r.hits_before | hits);
        if !touches.pages.is_empty() {
            return Err(NotRecorded::Used("a page"));
        }
        if !touches.reports.is_empty() {
            return Err(NotRecorded::Used("a report"));
        }
        if touches.dotnet {
            return Err(NotRecorded::Used(".NET"));
        }
        if touches.files {
            return Err(NotRecorded::Used("a file"));
        }
        // Private tables of temporary records no variable holds any more are dead: freed as the next use would.
        self.temporary_tables_held();
        let mut after = image::image_of(self)?;
        if after.test_object() != r.before.test_object() {
            return Err(NotRecorded::Untouched(
                "test_object (the run changed which test codeunit runs)",
            ));
        }
        let mut before = r.before;
        if !touches.test_object {
            before.forget_test_object();
            after.forget_test_object();
        }
        let singles = globals::singles_image(&self.singletons)?;
        let rows = if self.store.change_counter() == r.counter {
            None
        } else {
            let mut same = |a: &[al_builtins::value::Value], b: &[al_builtins::value::Value]| {
                Some(image::row_bytes(a)? == image::row_bytes(b)?)
            };
            let changes = self
                .store
                .committed_row_changes_from(&r.store, &mut same)
                .ok_or(NotRecorded::Rows)?;
            Some(crate::baseline::EncodedChanges::encode(changes).ok_or(NotRecorded::Rows)?)
        };
        Ok(RecordedRun {
            before,
            after,
            singles,
            rows,
            hits,
            touches,
        })
    }

    /// Whether this session may take `run`'s result now: it notes no code, has no SingleInstance codeunit, and its image
    /// equals the image `run` started from. The caller has checked the rows (its key).
    pub fn replay_applies(&self, run: &RecordedRun) -> bool {
        if self.notes_code_run() || !self.singletons.is_empty() {
            return false;
        }
        let Ok(mut now) = image::image_of(self) else {
            return false;
        };
        if !run.touches.test_object {
            now.forget_test_object();
        }
        now == run.before
    }

    /// Puts `run`'s result in place of running its code; the caller checked [`Session::replay_applies`]. `Err` (the
    /// rows do not apply) leaves the session as it was.
    pub fn replay(&mut self, run: &RecordedRun) -> Result<(), NotRecorded> {
        if let Some(rows) = &run.rows {
            let changes = rows.decode().ok_or(NotRecorded::Rows)?;
            let store = self
                .store
                .clone()
                .with_committed_row_changes(changes)
                .ok_or(NotRecorded::Rows)?;
            self.store = store;
        }
        globals::restore_singles(self, &run.singles);
        image::restore(self, &run.after);
        al_builtins::assumptions::add_hit_bits(run.hits);
        Ok(())
    }
}
