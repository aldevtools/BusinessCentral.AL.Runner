//! The `linked` fields of the session field table (`session_fields.rs`) that are not the object graph or the rows: the
//! session's sub-states field by field, the SQL counters, WorkDate and the language, and the baseline origin. Each is
//! taken by value and written back through the session's own setter where one exists, so what it keeps in step follows.

use super::NotImaged;
use crate::Session;
use al_builtins::media::{Id, MediaObject};
use std::collections::{HashMap, HashSet};

/// The linked fields by value.
pub(crate) struct SessionParts {
    /// `SqlCounters::left_inactive`.
    sql: crate::sqlcount::SqlLeft,
    /// `Session::baseline_origin()`: the thaw gives it the thawn store's counter.
    baseline_origin: Option<u64>,
    /// GlobalLanguage and WorkDate.
    carried: crate::CarriedState,
    tx: TxPart,
    /// `UiState::sent_notifications`, `UiState::clientless_login_skipped`.
    sent_notifications: Vec<[u8; 16]>,
    clientless_login_skipped: bool,
    events: EventsPart,
    media: MediaPart,
}

/// What a Commit leaves in `TxState` for a later test to read (state_facts.rs `transaction`); every other field holds a
/// new session's value (`install_left::tx_left_as_new`). `commit_counter` equals the store's change counter, a fact the
/// thaw writes again for the thawn store.
struct TxPart {
    phase: crate::transactions::TxPhase,
    transaction_type: i32,
    type_origin: crate::transaction_type::TypeOrigin,
    lock_timeout: bool,
    lock_timeout_duration: i64,
    committed_row_version: i64,
    rolled_back: bool,
    last_commit_wrote: bool,
    /// Read against the counts a record's row version source took (`version_entry.rs`), some taken in the login.
    write_counts: crate::state::WriteCounts,
}

/// The event state besides the manual bindings (the arena holds those) and what holds a new session's value
/// (`clients::events_left_as_new`).
struct EventsPart {
    caption_context_revision: u64,
    global_setups: HashMap<usize, crate::globaltriggers::GlobalSetup>,
    calls: HashMap<(usize, usize), i64>,
    active: HashMap<usize, i64>,
    language: i32,
}

/// The session's media, the four maps copied out of their shared cells, and what a rollback brings back.
struct MediaPart {
    objects: HashMap<Id, MediaObject>,
    sets: HashMap<Id, Vec<Id>>,
    removed: HashSet<Id>,
    outside_tenant: HashSet<Id>,
    removed_by_writes: Vec<(Id, MediaObject)>,
}

impl SessionParts {
    /// The linked fields of `s`, or the first that holds what the image does not take.
    pub(crate) fn take(s: &Session) -> Result<SessionParts, NotImaged> {
        let sql = s.sql.left_inactive().ok_or(NotImaged::Field("sql"))?;
        if let Some(field) = crate::install_left::tx_not_as_new(&s.tx) {
            return Err(NotImaged::Field(field));
        }
        if s.tx.commit_counter != s.store.change_counter() {
            return Err(NotImaged::Field("tx.commit_counter"));
        }
        if !crate::install_left::clients::ui_left_as_new(&s.ui_state) {
            return Err(NotImaged::Field("ui_state"));
        }
        if let Some(field) = crate::install_left::clients::events_not_as_new(&s.events) {
            return Err(NotImaged::Field(field));
        }
        // What a rollback brings back: after the open's Commit nothing is in a transaction.
        if !s.media_removed.0.is_empty() {
            return Err(NotImaged::Field("media_removed"));
        }
        Ok(SessionParts {
            sql,
            baseline_origin: s.baseline_origin(),
            carried: crate::CarriedState {
                language: s.global_language(),
                work_date: s.work_date,
            },
            tx: TxPart::take(&s.tx),
            sent_notifications: s.ui_state.sent_notifications.clone(),
            clientless_login_skipped: s.ui_state.clientless_login_skipped,
            events: EventsPart::take(&s.events),
            media: MediaPart::take(s),
        })
    }

    /// Writes the linked fields into `s`, after its store was thawn (the counters are of the thawn store).
    pub(crate) fn give(&self, s: &mut Session) {
        s.restore_carried(self.carried);
        self.tx.give(s);
        s.sql.restore_left(&self.sql);
        s.baseline_origin = self.baseline_origin.map(|origin| (origin, s.store.change_counter()));
        s.ui_state.sent_notifications = self.sent_notifications.clone();
        s.ui_state.clientless_login_skipped = self.clientless_login_skipped;
        self.events.give(s);
        self.media.give(s);
    }
}

impl TxPart {
    fn take(tx: &crate::state::TxState) -> TxPart {
        TxPart {
            phase: tx.phase,
            transaction_type: tx.transaction_type,
            type_origin: tx.type_origin,
            lock_timeout: tx.lock_timeout,
            lock_timeout_duration: tx.lock_timeout_duration,
            committed_row_version: tx.committed_row_version,
            rolled_back: tx.rolled_back,
            last_commit_wrote: tx.last_commit_wrote,
            write_counts: tx.write_counts.clone(),
        }
    }

    fn give(&self, s: &mut Session) {
        let TxPart {
            phase,
            transaction_type,
            type_origin,
            lock_timeout,
            lock_timeout_duration,
            committed_row_version,
            rolled_back,
            last_commit_wrote,
            ref write_counts,
        } = *self;
        let tx = &mut s.tx;
        tx.phase = phase;
        tx.transaction_type = transaction_type;
        tx.type_origin = type_origin;
        tx.lock_timeout = lock_timeout;
        tx.lock_timeout_duration = lock_timeout_duration;
        tx.committed_row_version = committed_row_version;
        tx.rolled_back = rolled_back;
        tx.last_commit_wrote = last_commit_wrote;
        tx.write_counts = write_counts.clone();
        tx.commit_counter = s.store.change_counter();
    }
}

impl EventsPart {
    fn take(e: &crate::state::EventState) -> EventsPart {
        EventsPart {
            caption_context_revision: e.caption_context_revision,
            global_setups: e.global_setups.clone(),
            calls: e.event_stats.calls.borrow().clone(),
            active: e.event_stats.active.borrow().clone(),
            language: e.event_stats.language.get(),
        }
    }

    /// Into the session's own cells, which its providers share (lib.rs `Providers`).
    fn give(&self, s: &mut Session) {
        let e = &mut s.events;
        e.caption_context_revision = self.caption_context_revision;
        e.global_setups.clone_from(&self.global_setups);
        e.event_stats.calls.borrow_mut().clone_from(&self.calls);
        e.event_stats.active.borrow_mut().clone_from(&self.active);
        e.event_stats.language.set(self.language);
    }
}

impl MediaPart {
    /// The store's maps through one destructure with no `..`: a map added to `MediaStore` fails this compile.
    fn take(s: &Session) -> MediaPart {
        let al_builtins::media::MediaStore {
            objects,
            sets,
            removed,
            outside_tenant,
        } = &s.media;
        MediaPart {
            objects: objects.borrow().clone(),
            sets: sets.borrow().clone(),
            removed: removed.borrow().clone(),
            outside_tenant: outside_tenant.borrow().clone(),
            removed_by_writes: s.media_removed.1.clone(),
        }
    }

    /// Into the session's own cells, which its providers share (lib.rs `Providers`).
    fn give(&self, s: &mut Session) {
        let al_builtins::media::MediaStore {
            objects,
            sets,
            removed,
            outside_tenant,
        } = &s.media;
        objects.borrow_mut().clone_from(&self.objects);
        sets.borrow_mut().clone_from(&self.sets);
        removed.borrow_mut().clone_from(&self.removed);
        outside_tenant.borrow_mut().clone_from(&self.outside_tenant);
        s.media_removed = (Vec::new(), self.removed_by_writes.clone());
    }
}
