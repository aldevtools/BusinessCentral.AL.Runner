//! What an `IsolatedStorage` read answers where the stored value alone does not decide it: a missing key read as a statement,
//! and a read inside a page background task, which runs as another session (al-runner issue 7576).
//!
//! A worker sees a key by who wrote it and whether that write is behind a commit
//! (session-system/isolated-storage/IsolatedStorageInBackgroundTask.al, codeunits 75108-75114):
//! - a key this test wrote and did not commit: not there (Contains false, Get false; :287-298);
//! - a key this codeunit wrote that a commit or the end of an earlier test put behind it, with the same value since: Contains
//!   answers true (:383-397), Get waits on the test's lock and fails with NDBCS:LockTimeout (:478-488 after an earlier test's
//!   commit, :573-590 after an earlier test's write, :313-331 a write an earlier test also made; session-system/CnIsolatedStorageLeads.al:445-522);
//! - any other missing key: not there; a Get as a statement raises DB:RecordNotFound (:214-240, :399-414, :501-512, :645-664;
//!   a key a committed Delete removed: :513-526).
//!
//! A key nobody in the codeunit wrote that is stored anyway (left by the install code), a key whose committed value the test
//! changed again, and a key the test deleted while a committed copy exists, are not pinned and refuse.
//!
//! It also holds the key forms the corpus pins only in part: a key of more than 200 characters, a key that matches a stored
//! key only without trailing spaces, and the missing-key error of a Guid key or a SecretText target.

use al_builtins::native::unsupported;
use al_builtins::value::Value;

use crate::Session;
use al_builtins::native::Signal;

/// How a page background task sees one isolated storage key.
pub(crate) enum WorkerSees {
    /// As the stored value says: the key is there or not.
    Stored,
    /// Not there, although the test session holds it.
    Missing,
    /// There, behind the test session's lock: Contains answers true, Get fails.
    Locked,
}

impl Session {
    /// How a page background task sees `slot`, which the session holds; the caller is in a worker.
    pub(crate) fn worker_sees(&self, slot: &(String, String)) -> Result<WorkerSees, Signal> {
        let stored = self.isolated_storage.get(slot);
        let committed = self.isolated_committed.get(slot);
        let written_in_test = self.isolated_set_in_test.contains(slot);
        if stored.is_none() {
            // A key a committed Delete removed is not there (:513-526). A key the test deleted after a commit stored it is
            // not pinned (the worker may still see the committed copy).
            if committed.is_some() {
                return unsupported(
                    "IsolatedStorage read in a page background task of a key the caller deleted while a committed copy exists (not pinned)",
                );
            }
            return Ok(WorkerSees::Stored);
        }
        if !written_in_test && !self.isolated_written_in_codeunit.contains(slot) {
            return unsupported(
                "IsolatedStorage read in a page background task of a key the test codeunit did not write (not pinned)",
            );
        }
        match committed {
            None if written_in_test => Ok(WorkerSees::Missing),
            Some(c) if Some(c) == stored => Ok(WorkerSees::Locked),
            _ => unsupported(
                "IsolatedStorage read in a page background task of a committed key the caller changed again (not pinned)",
            ),
        }
    }

    /// The error of a Get or Delete of a missing key as a statement, in the main session and in a page background task:
    /// DB:RecordNotFound with the app, the scope, the company of the Company scope and the null user id
    /// (session-system/P107IsoStorage.al:41-44 and errors/errorinfo/EnTSecret.al:127-128, :215-218 for Module;
    /// session-system/isolated-storage/IsolatedStorageInBackgroundTask.al:226-240 for Company). The user scopes are not pinned.
    pub(crate) fn isolated_storage_missing(
        &self,
        method: &str,
        scope: Option<&Value>,
        key: &str,
    ) -> Result<Value, Signal> {
        if key.ends_with(' ') {
            return unsupported(format!(
                "IsolatedStorage.{method} of a missing key with trailing spaces as a statement (the Key of the error text is not pinned)"
            ));
        }
        let (scope_name, company) = match scope {
            None | Some(Value::Option(0, _)) => ("Module", ""),
            Some(Value::Option(1, _)) => ("Company", &*self.cx.company),
            _ => {
                return unsupported(format!(
                    "IsolatedStorage.{method} of a missing key as a statement in a user scope (error text not pinned)"
                ));
            }
        };
        let app = self
            .frames
            .last()
            .map(|o| self.engine.program.objects[*o].app as usize)
            .and_then(|a| self.engine.program.apps.get(a))
            .and_then(|a| al_builtins::ops::parse_guid(&a.id));
        let Some(app) = app else {
            return unsupported(format!(
                "IsolatedStorage.{method} of a missing key as a statement outside an app with an id (error text not pinned)"
            ));
        };
        Err(crate::transactions::al_error_code(
            &format!(
                "The Isolated Storage does not exist. Identification fields and values: App Id='{}',Scope='{scope_name}',Company Name='{company}',User Id='{{00000000-0000-0000-0000-000000000000}}',Key='{key}'",
                al_builtins::format::format_guid(&app)
            ),
            "DB:RecordNotFound",
        ))
    }

    /// The lock timeout of a worker's Get of a key behind the test session's lock (see the module comment).
    pub(crate) fn isolated_storage_locked() -> Signal {
        crate::transactions::al_error_code(
            "We can't save your changes right now, because a record in table 'Isolated Storage' is being updated in a transaction done by another session.\\\\You'll have to wait until the other transaction has completed, which may take a while. Please try again later.",
            "NDBCS:LockTimeout",
        )
    }
}

/// A key of more than 200 characters (or 200 UTF-16 units): `Set` and `Contains` of an ASCII key without trailing spaces in the
/// default scope raise "Key must not be longer than 200." with code AL (errors/errorinfo/EnTSecret.al:250-283, af207 to af223);
/// every other method, scope and key form is not pinned and refuses. `None` for a key of at most 200.
pub(crate) fn isolated_long_key(method: &str, key: &str, scope_given: bool) -> Option<Result<Value, Signal>> {
    if key.chars().count() <= 200 && key.encode_utf16().count() <= 200 {
        return None;
    }
    let pinned = matches!(method, "Set" | "Contains") && !scope_given && key.is_ascii() && !key.ends_with(' ');
    Some(if pinned {
        crate::sessionops::error_code("Key must not be longer than 200.".into(), "AL")
    } else {
        unsupported(format!(
            "IsolatedStorage.{method} of a key of more than 200 characters in this form (pinned for Set and Contains of an ASCII key without trailing spaces in the default scope only)"
        ))
    })
}

/// The missing-key error of a Get or Delete as a statement is pinned for a Text key and a Text target only
/// (errors/errorinfo/EnTSecret.al:127-128, :215-218; session-system/isolated-storage/IsolatedStorageInBackgroundTask.al:226-240).
pub(crate) fn refuse_missing_key_form(method: &str, key: &Value, secret_target: bool) -> Result<(), Signal> {
    if matches!(key, Value::Guid(_)) {
        return unsupported(format!(
            "IsolatedStorage.{method} of a missing Guid key as a statement (the Key of the error text is not pinned)"
        ));
    }
    if secret_target {
        return unsupported(format!(
            "IsolatedStorage.{method} of a missing key into a SecretText as a statement (error text not pinned)"
        ));
    }
    Ok(())
}

impl Session {
    /// The slot of `key` in `scope`. A stored key that differs from `key` only in trailing spaces is found by Contains of
    /// a key without trailing spaces that is not empty (Contains('EnT trail') finds the key Set as 'EnT trail ':
    /// errors/errorinfo/EnTSecret.al:182-183, :199), outside a page background task. Every other method and form that
    /// would depend on that match is not pinned and refuses. Letter case counts (:172-174, :193).
    pub(crate) fn trailing_twin(&self, method: &str, scope: String, key: &str) -> Result<(String, String), Signal> {
        let slot = (scope, key.to_string());
        if self.isolated_storage.contains_key(&slot) {
            return Ok(slot);
        }
        let trimmed = key.trim_end_matches(' ');
        let twin = self
            .isolated_storage
            .keys()
            .find(|(s, k)| *s == slot.0 && k != key && k.trim_end_matches(' ') == trimmed);
        let Some(twin) = twin else {
            return Ok(slot);
        };
        if method == "Contains" && trimmed == key && !key.is_empty() && self.pages.bg_worker.is_none() {
            return Ok(twin.clone());
        }
        unsupported(format!(
            "IsolatedStorage.{method} of a key that matches a stored key only without their trailing spaces (pinned for Contains of a key without trailing spaces only)"
        ))
    }
}
