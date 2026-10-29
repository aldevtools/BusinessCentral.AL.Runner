//! `StartSession` under Disabled test isolation (al-runner issue 5609). BC starts a background session there; what the
//! probes measure is that it starts, gets a positive id, ends within the starting test's wait (`Session.IsSessionActive`
//! then answers false) and that what it wrote is committed and seen by the starting test and the later ones
//! (codeunit/lt-qu-commit-isolation/RqIsolationModes.al:101-128 with RQ_D03/RQ_D04 at :585-614, BC 28 and later;
//! tests/al-language-isolation-probe/src/SdIsolStartSession.Codeunit.al:22-30, :38-63, runners 61304 and 61305).
//!
//! The runtime runs the session's codeunit to its end inside the call, on this `Session` with the state of a new session
//! swapped in: no SingleInstance codeunits, no manual subscriptions, no handler functions, no filters of the global
//! filter group, no last error, a transaction of its own and its own id. Whatever no probe measures is refused: a start
//! while the starting session holds an uncommitted write or a lock (whether the new session waits, and what it sees), a
//! start into another company, a temporary record or one with filters or marks, an error in the new session, a
//! session started from it, and its reads of session state a new session sets up for itself (WorkDate, GlobalLanguage,
//! GuiAllowed, CurrentClientType). Its UI calls are refused as UI outside a test method.

use super::*;
use al_builtins::builtin_id::ids;

/// What the starting session keeps aside while the new one runs.
struct Parked {
    singletons: std::collections::HashMap<usize, Rc<RefCell<Instance>>>,
    bindings: Vec<dispatch::Binding>,
    global_group1: std::collections::HashMap<u32, String>,
    last_error: AlError,
    session_id_override: Option<i32>,
    ui_scope: Option<ui::UiScope>,
    tx: TxState,
    tx_model: TxModel,
    in_test_codeunit: bool,
    test_object: Option<usize>,
    hub_disabled: bool,
    isolation_from_hub: bool,
    error_scopes: errorcollect::Scopes,
}

impl Session {
    /// Runs codeunit `codeunit` in a new session to its end and commits what it wrote; the new session's id (see the
    /// module documentation for what is refused).
    pub(crate) fn start_background_session(
        &mut self,
        codeunit: i32,
        company: Option<String>,
        record: Option<Value>,
    ) -> Result<i32, Signal> {
        let refuse = |why: &str| {
            Err(Signal::Unsupported(format!(
                "StartSession under Disabled test isolation: {why} (not pinned)"
            )))
        };
        if self.background_session || self.pages.bg_worker.is_some() {
            return refuse("a session started from a background session or task");
        }
        if company.as_deref().is_some_and(|c| c != &*self.cx.company) {
            return refuse("a session in another company");
        }
        if self.in_write_transaction()? || !self.tx.locked_tables.is_empty() {
            return refuse("a start while the starting session holds an uncommitted write or a lock");
        }
        let record = match record {
            None => None,
            Some(Value::Record(rc)) => {
                let r = rc.borrow();
                if r.temporary || !r.filters.is_empty() || !r.marks.is_empty() || r.marked_only {
                    return refuse("a temporary record, or one with filters or marks, handed to the session");
                }
                drop(r);
                Some(self.record_by_value(&rc))
            }
            Some(other) => {
                return refuse(&format!("a {} handed to the session", crate::value_kind(&other)));
            }
        };
        self.sessions_started += 1;
        let id = self.session_id().saturating_add(self.sessions_started);
        let tx = TxState {
            committed_row_version: self.tx.committed_row_version,
            ..TxState::fresh()
        };
        let parked = Parked {
            singletons: std::mem::take(&mut self.singletons),
            bindings: std::mem::take(&mut self.events.bindings),
            global_group1: std::mem::take(&mut self.global_group1),
            last_error: std::mem::take(&mut self.last_error),
            session_id_override: self.session_id_override.replace(id),
            ui_scope: self.ui_state.scope.take(),
            tx: std::mem::replace(&mut self.tx, tx),
            tx_model: std::mem::take(&mut self.tx_model),
            in_test_codeunit: std::mem::replace(&mut self.in_test_codeunit, false),
            test_object: self.test_object.take(),
            hub_disabled: std::mem::replace(&mut self.hub_disabled, false),
            isolation_from_hub: std::mem::replace(&mut self.isolation_from_hub, false),
            error_scopes: std::mem::take(&mut self.error_scopes),
        };
        self.background_session = true;
        let mut args = vec![Value::Integer(codeunit)];
        args.extend(record);
        // The guarded form: the run is a transaction of its own and commits when it succeeds.
        let ran = self.codeunit_run(ids::CODEUNIT_RUN_3, None, &args, true);
        let failed = !matches!(ran, Ok(Value::Boolean(true)));
        if failed && self.store.has_uncommitted() {
            self.rollback_transaction();
        }
        self.background_session = false;
        let committed_row_version = self.tx.committed_row_version;
        self.singletons = parked.singletons;
        self.events.bindings = parked.bindings;
        self.global_group1 = parked.global_group1;
        self.last_error = parked.last_error;
        self.session_id_override = parked.session_id_override;
        self.ui_state.scope = parked.ui_scope;
        // Rows the new session committed move the minimum active row version only while the isolation transaction has
        // not written: not pinned.
        if self.iso_min_row_version.is_some() && committed_row_version != parked.tx.committed_row_version {
            self.iso_min_unpinned = true;
        }
        self.tx = parked.tx;
        // The rows the new session committed are durable for the starting session too.
        self.tx.committed_row_version = committed_row_version;
        self.tx_model = parked.tx_model;
        self.in_test_codeunit = parked.in_test_codeunit;
        self.test_object = parked.test_object;
        self.hub_disabled = parked.hub_disabled;
        self.isolation_from_hub = parked.isolation_from_hub;
        self.error_scopes = parked.error_scopes;
        match ran {
            Ok(Value::Boolean(true)) => Ok(id),
            Ok(_) | Err(Signal::Error(_)) => refuse("an error in the new session"),
            Err(other) => Err(other),
        }
    }

    /// A read of session state a new session sets up for itself, in a session `StartSession` started: refused, as no
    /// probe measures what it holds there.
    pub(crate) fn refuse_in_background_session(&self, what: &str) -> Result<(), Signal> {
        if self.background_session {
            return Err(Signal::Unsupported(format!(
                "{what} in a session StartSession started (what a new session holds is not pinned)"
            )));
        }
        Ok(())
    }
}
