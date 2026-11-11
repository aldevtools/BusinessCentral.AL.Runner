//! Recorded runs (ADR-073 decisions 1 to 6): one real run of a piece of AL code, kept with what it read and what it left,
//! and replayed in a later session whose inputs are all equal. The one owner of recording, the session image and replay;
//! al-testrun's login (`install.rs`) uses it and owns only its key.
//!
//! AL code the VM runs is a function of what it reads. Its inputs here are:
//! - the session's image before the run ([`SessionImage`], `image.rs`: every field of `Session` has a class), and the
//!   images of its SingleInstance codeunits (an earlier step of the same start can leave some, `globals.rs`);
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
//! never takes a replay (decision 6): `notes_code_run`, and its image check fails on `coverage`, `capture` or `debug`.

mod globals;
mod image;

pub use image::SessionImage;

use crate::Session;

/// Whether an engine's sessions record and replay runs (`Engine::recorded_runs`, ADR-073 decisions 8 and the state check).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RecordedRunsMode {
    /// Record and replay.
    #[default]
    On,
    /// The off switch: every session runs the code for real, as before ADR-073.
    Off,
    /// The state check: a session that could take a replay runs the code for real, records it, and the caller compares
    /// the result with the kept run's ([`RecordedRun::result_difference`]).
    Check,
}

/// Why a run is not recorded, or why a session takes no replay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotRecorded {
    /// The session notes the code it runs (decision 6).
    NotesCode,
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
    singles_before: Vec<globals::SingleImage>,
    store: al_data::memory::MemStore,
    counter: u64,
    hits_before: al_builtins::assumptions::HitBits,
}

/// One real run with its inputs and its result (see the module comment). Plain data: it can go to another thread.
#[derive(Clone)]
pub struct RecordedRun {
    before: SessionImage,
    /// The SingleInstance codeunits before the run (an earlier step of the same start can leave some): part of the key.
    singles_before: Vec<globals::SingleImage>,
    after: SessionImage,
    singles: Vec<globals::SingleImage>,
    rows: Option<crate::baseline::EncodedChanges>,
    hits: al_builtins::assumptions::HitBits,
    touches: crate::impact::Touches,
    /// The run read neither the clock nor a platform table (whose identity rows hold the login instant): its images
    /// leave the host's instant out, so sessions whose clocks start at other instants take it.
    clock_free: bool,
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

    /// The globals the run left in SingleInstance codeunit `object`, by slot (a global that holds its initial empty
    /// collection is left out): the plain values, `None` for a record. `None` when the run left no instance of it.
    pub fn single_globals(&self, object: usize) -> Option<Vec<(usize, Option<al_builtins::value::Value>)>> {
        Some(globals::plain_globals(
            self.singles.iter().find(|s| s.object() == object)?,
        ))
    }

    /// The first part of the result in which `other`, a run of the same code from an equal image, differs from this run:
    /// the image after, the SingleInstance codeunits, the rows or the assumption marks. `None`: the same result. The
    /// state check (ADR-073, Conformance) compares a real run with the kept run a replay would have given.
    pub fn result_difference(&self, other: &RecordedRun) -> Option<String> {
        if let Some(d) = first_difference(&self.after, &other.after) {
            return Some(format!("session image after the run: {d}"));
        }
        if let Some(d) = first_difference(&self.singles, &other.singles) {
            return Some(format!("SingleInstance codeunits: {d}"));
        }
        if let Some(d) = first_difference(&self.rows, &other.rows) {
            return Some(format!("rows: {d}"));
        }
        (self.hits != other.hits).then(|| "assumption marks".to_string())
    }
}

/// Where the debug forms of `a` and `b` first differ: the top-level field and the first line that differs; `None` when
/// they are equal. Diagnosis only (`--explain`, `--timings`, the state check).
fn first_difference<T: std::fmt::Debug>(a: &T, b: &T) -> Option<String> {
    let (a, b) = (format!("{a:#?}"), format!("{b:#?}"));
    if a == b {
        return None;
    }
    let (la, lb): (Vec<&str>, Vec<&str>) = (a.lines().collect(), b.lines().collect());
    let at = (0..la.len().max(lb.len())).find(|&i| la.get(i) != lb.get(i))?;
    let field = (0..=at.min(la.len().saturating_sub(1)))
        .rev()
        .map(|i| la[i])
        .find(|l| l.starts_with("    ") && !l.starts_with("     "))
        .map_or("", |l| l.trim().split(':').next().unwrap_or(""));
    let line = |l: Option<&&str>| l.map_or("(none)".to_string(), |l| l.trim().chars().take(100).collect());
    Some(format!("{field}: {} against {}", line(la.get(at)), line(lb.get(at))))
}

impl Session {
    /// Starts recording the code the caller runs next, or says why it cannot be recorded. The caller runs the code, then
    /// calls [`Session::finish_recording`] when the code ended normally; a run that ended otherwise is simply not
    /// finished (call [`Session::abandon_recording`] to switch the trace off).
    pub fn start_recording(&mut self) -> Result<Recording, NotRecorded> {
        if self.notes_code_run() {
            return Err(NotRecorded::NotesCode);
        }
        let before = image::image_of(self)?;
        let singles_before = globals::singles_image(&self.singletons)?;
        let hits_before = al_builtins::assumptions::take_hit_bits();
        self.enable_impact_trace();
        Ok(Recording {
            before,
            singles_before,
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
        let clock_free = !touches.clock && touches.tables.iter().all(|&t| self.table_stamp(t).is_some());
        if clock_free {
            before.forget_clock(touches.clock_day);
            after.forget_clock(touches.clock_day);
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
            singles_before: r.singles_before,
            after,
            singles,
            rows,
            hits,
            touches,
            clock_free,
        })
    }

    /// Whether this session may take `run`'s result now: it notes no code, and its image and its SingleInstance
    /// codeunits equal those `run` started from. The caller has checked the rows (its key).
    pub fn replay_applies(&self, run: &RecordedRun) -> bool {
        if self.notes_code_run() {
            return false;
        }
        let Ok(mut now) = image::image_of(self) else {
            return false;
        };
        if !run.touches.test_object {
            now.forget_test_object();
        }
        if run.clock_free {
            now.forget_clock(run.touches.clock_day);
        }
        now == run.before && globals::singles_image(&self.singletons).is_ok_and(|s| s == run.singles_before)
    }

    /// Why this session may not take `run`'s result now, for diagnosis: the first field of the image that differs, or
    /// another reason. `None`: it may ([`Session::replay_applies`]).
    pub fn replay_miss(&self, run: &RecordedRun) -> Option<String> {
        if self.notes_code_run() {
            return Some(NotRecorded::NotesCode.to_string());
        }
        let singles = match globals::singles_image(&self.singletons) {
            Ok(singles) => singles,
            Err(why) => return Some(why.to_string()),
        };
        if let Some(d) = first_difference(&run.singles_before, &singles) {
            return Some(format!("the SingleInstance codeunits differ: {d}"));
        }
        let mut now = match image::image_of(self) {
            Ok(now) => now,
            Err(why) => return Some(why.to_string()),
        };
        if !run.touches.test_object {
            now.forget_test_object();
        }
        if run.clock_free {
            now.forget_clock(run.touches.clock_day);
        }
        first_difference(&run.before, &now).map(|d| format!("the session image differs in {d}"))
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
        self.singletons.clear();
        globals::restore_singles(self, &run.singles);
        image::restore(self, &run.after);
        al_builtins::assumptions::add_hit_bits(run.hits);
        Ok(())
    }
}
