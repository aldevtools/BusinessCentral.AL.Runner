//! The check that a replay of the raised login would give each test session what running the login gives it (al-runner
//! issue 8211). The maintainer's condition for a replay: the replayed store is the same as running the login for each
//! session, including what the subscribers write. This module measures that and replays nothing.
//!
//! With `RunOptions::login_replay_check` (`ALRUNNER_LOGIN_REPLAY_CHECK=1`, read by the front ends), `runtime/run.rs`
//! `open_company` takes the [`Key`] of each test session before it opens the company and, when the login was raised
//! (`install::CompanyOpenWrites::raised`), what the session holds after it ([`LoginState`]). The first session of a key
//! keeps its state on the engine ([`Check`]); every later session of the key is compared with it
//! ([`first_difference`]). Outcomes do not change. [`report`] is the line the front ends print.

use al_vm::{Engine, Session};
use std::collections::BTreeSet;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Mutex, PoisonError};

/// Everything the login and its subscribers can read when the test session opens the company. Two sessions with the same
/// key may share one run of the login only when they also hold the same after it ([`Check`]); a field left out of the
/// key would let a replay hand a session the store of another input. The engine (the compiled program, its apps and
/// their code) is not a field: the check lives on the engine (`Engine::derived`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Key {
    /// Every committed row before the login: the install baseline, what install code and an earlier codeunit left.
    database: u64,
    /// Rows written and not committed before the login: the login reads them.
    uncommitted: bool,
    /// Who offers the rows of the tables no write reached (the test data, `MemStore::backing_identity`).
    backing: Option<(u64, u64)>,
    /// The session state before the login (`Session::state_lines`): language, WorkDate, number sequences, isolated
    /// storage, SingleInstance globals, manual bindings, `Random`, filter group 1, the clock offset, the Guid key and
    /// draws, row versions, AutoIncrement counters, the last error.
    session: u64,
    /// `CompanyName()` and the company every record reads in.
    company: String,
    /// The run seed: the session clock steps by a key derived from it (`al_host::seed::clock_key`), so the same login
    /// moves the clock by another amount under another seed (measured: two requests of one serve daemon, seeds 1 and 2;
    /// alrunner-cli tests/serve.rs `the_login_replay_check_finds_no_difference_across_entries_and_requests`).
    seed: i32,
    /// `UserId()`.
    user: String,
    /// The test codeunit the session opens for: `login::decide` reads it, and a replay may not cross codeunits until a
    /// probe shows no login code can tell them apart.
    test_codeunit: Option<usize>,
    /// The resource texts the pass reads in place of the apps' files (a serve entry's `resources`,
    /// `Engine::resource_overrides`): `NavApp.GetResource` in a subscriber reads them.
    resources: u64,
    /// The session has a client (BC's login code runs only then), runs without one (`--no-client`), and runs under
    /// RequiredTestIsolation = Disabled: these choose what the login does (`install::open_test_session_company`).
    client: (bool, bool, bool),
    /// The run options every test session takes as they are (`runtime::session_flags::SessionFlags`: `--permissions off`,
    /// `--ignore-permission-mock`, `--dev-endpoint-ui`, ...): they change what the login code may do (a permission error,
    /// an unhandled UI call), and a serve request sets them while the engine and its check outlive it. The whole value,
    /// so a flag added later is a key field without a change here.
    flags: crate::runtime::session_flags::SessionFlags,
    /// The license the session runs under (ADR-060, `Engine::license`, chosen per request): the permission checks of the
    /// login code read it. Compared by content.
    license: Option<std::sync::Arc<al_host::license::License>>,
}

impl Key {
    /// The key of `session` before it opens the company; `disabled`: RequiredTestIsolation = Disabled; `flags`: the run options
    /// the session took.
    pub(crate) fn of(session: &mut Session, disabled: bool, flags: crate::runtime::session_flags::SessionFlags) -> Key {
        let store = session.store_mut();
        let database = digest(&committed_rows(store));
        let uncommitted = store.has_uncommitted();
        let backing = store.backing_identity();
        let resources = {
            let overrides = session
                .engine
                .resource_overrides
                .read()
                .unwrap_or_else(PoisonError::into_inner);
            let mut texts: Vec<_> = overrides.iter().collect();
            texts.sort_by_key(|(app, _)| **app);
            digest(
                &texts
                    .iter()
                    .map(|(app, files)| format!("{app} {files:?}"))
                    .collect::<Vec<_>>(),
            )
        };
        let state = session.state_lines();
        Key {
            database,
            uncommitted,
            backing,
            session: digest(&[digest(&state.lines).to_string(), digest(&state.opaque).to_string()]),
            company: session.access_company().to_string(),
            seed: session.run_seed(),
            user: session.user_id(),
            test_codeunit: session.test_object,
            resources,
            client: (
                session.has_interactive_client(),
                session.is_clientless_session(),
                disabled,
            ),
            flags,
            license: session.engine.license.clone(),
        }
    }
}

/// What a test session holds after the raised login: every committed row (system fields included), the session state
/// (`Session::state_lines`) and the assumption marks the login made (a replayed flagged pass stays flagged).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoginState {
    rows: Vec<String>,
    state: al_vm::StateLines,
    hits: al_builtins::assumptions::HitBits,
}

impl LoginState {
    /// The committed rows, one line each.
    #[cfg(test)]
    pub(crate) fn rows(&self) -> &[String] {
        &self.rows
    }

    /// The session state lines.
    #[cfg(test)]
    pub(crate) fn state_lines(&self) -> &al_vm::StateLines {
        &self.state
    }

    /// What `session` holds now, with the marks `hits` the company open made.
    pub(crate) fn of(session: &mut Session, hits: al_builtins::assumptions::HitBits) -> LoginState {
        LoginState {
            rows: committed_rows(session.store_mut()),
            state: session.state_lines(),
            hits,
        }
    }
}

/// The kinds of difference between two sessions' states, in the order [`differences`] looks at them: a value with no
/// comparable form (which no replay could give back), the committed rows, the session state lines, the assumption marks.
const KINDS: [&str; 4] = ["not comparable", "rows", "session state", "marks"];

/// What differs between `a` and `b`, the states of two sessions of one key: per kind of [`KINDS`] that differs, its index
/// and its first difference. One kind does not hide another.
pub(crate) fn differences(a: &LoginState, b: &LoginState) -> Vec<(usize, String)> {
    let lines = |x: &[String], y: &[String], what: &str| {
        let n = x
            .iter()
            .zip(y)
            .position(|(p, q)| p != q)
            .unwrap_or(x.len().min(y.len()));
        (x.len() != y.len() || n < x.len()).then(|| {
            format!(
                "{what} {}: {} | {}",
                n + 1,
                x.get(n).map_or("(none)", String::as_str),
                y.get(n).map_or("(none)", String::as_str)
            )
        })
    };
    let opaque: BTreeSet<&String> = a.state.opaque.iter().chain(&b.state.opaque).collect();
    let opaque = (!opaque.is_empty()).then(|| {
        let what: Vec<&str> = opaque.into_iter().map(String::as_str).collect();
        format!("not comparable: {}", what.join("; "))
    });
    [
        opaque,
        lines(&a.rows, &b.rows, "row"),
        lines(&a.state.lines, &b.state.lines, "session state"),
        (a.hits != b.hits).then(|| format!("assumption marks: {:?} | {:?}", a.hits, b.hits)),
    ]
    .into_iter()
    .enumerate()
    .filter_map(|(k, d)| Some((k, d?)))
    .collect()
}

/// The first of [`differences`]: `None` when the two states are the same.
#[cfg(test)]
pub(crate) fn first_difference(a: &LoginState, b: &LoginState) -> Option<String> {
    differences(a, b).into_iter().next().map(|(_, what)| what)
}

/// The check on one engine: per key, the first state and the sessions compared with it; and, for evidence about the
/// test codeunit field, the first state of each key compared across test codeunits (the key without the codeunit).
#[derive(Default)]
pub(crate) struct Check {
    inner: Mutex<CheckState>,
}

#[derive(Default)]
struct CheckState {
    exact: Tally,
    across_codeunits: Tally,
}

/// The first state of each key, and the sessions compared with it.
#[derive(Default)]
struct Tally {
    firsts: Vec<(Key, LoginState)>,
    compared: u64,
    differ: u64,
    /// Per kind of [`KINDS`], the compared sessions that differ in it.
    differ_by: [u64; KINDS.len()],
    first_difference: Option<String>,
}

impl Tally {
    /// Compares `state` with the first state of `key`, or keeps it as that. `true`: the key was new.
    fn note(&mut self, key: Key, state: LoginState) -> bool {
        let Some((_, first)) = self.firsts.iter().find(|(k, _)| *k == key) else {
            self.firsts.push((key, state));
            return true;
        };
        let differs = differences(first, &state);
        self.compared += 1;
        if let Some((_, what)) = differs.first() {
            self.differ += 1;
            self.first_difference.get_or_insert(what.clone());
        }
        for (k, _) in &differs {
            self.differ_by[*k] += 1;
        }
        false
    }

    fn counts(&self) -> String {
        let by: Vec<String> = (KINDS.iter().zip(&self.differ_by))
            .map(|(kind, n)| format!("{kind} {n}"))
            .collect();
        format!(
            "{} compared, {} differ{}",
            self.compared,
            self.differ,
            self.first_difference
                .as_ref()
                .map_or(String::new(), |w| format!(" ({}; first: {w})", by.join(", ")))
        )
    }
}

impl Check {
    /// Notes the state of a session of `key` after the raised login.
    pub(crate) fn note(&self, key: Key, state: LoginState) {
        let mut s = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let across = Key {
            test_codeunit: None,
            ..key.clone()
        };
        if s.exact.note(key, state.clone()) {
            s.across_codeunits.note(across, state);
        }
    }

    /// The first state of each key, in the order the keys came.
    #[cfg(test)]
    pub(crate) fn firsts(&self) -> Vec<(Key, LoginState)> {
        self.inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .exact
            .firsts
            .clone()
    }

    /// The report line: `None` before any session was noted.
    pub(crate) fn line(&self) -> Option<String> {
        let s = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        if s.exact.firsts.is_empty() {
            return None;
        }
        let sessions = s.exact.compared + s.exact.firsts.len() as u64;
        Some(format!(
            "login replay check: {sessions} sessions over {} keys (a key seen before: {} sessions, {}%), {}; across test \
             codeunits: {}",
            s.exact.firsts.len(),
            s.exact.compared,
            s.exact.compared * 100 / sessions,
            s.exact.counts(),
            s.across_codeunits.counts()
        ))
    }
}

/// The check of `engine`.
pub(crate) fn check(engine: &Engine) -> std::sync::Arc<Check> {
    engine.derived(Check::default)
}

/// The report line of `engine`'s check: `None` when no session raised the login under the check.
pub(crate) fn report(engine: &Engine) -> Option<String> {
    check(engine).line()
}

/// Every committed row of `store`, one line each: company, table object and the row with its system fields.
fn committed_rows(store: &al_data::memory::MemStore) -> Vec<String> {
    let tables = store.export_committed().unwrap_or_default();
    let mut out = Vec::new();
    for t in &tables {
        for (_, row) in &t.rows {
            out.push(format!("{}|{}|{row:?}", t.company, t.object));
        }
    }
    out
}

/// A digest of `lines`, for a key field (compared within one process only).
fn digest(lines: &[String]) -> u64 {
    let mut h = DefaultHasher::new();
    lines.hash(&mut h);
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(rows: &[&str], lines: &[&str]) -> LoginState {
        LoginState {
            rows: rows.iter().map(|s| s.to_string()).collect(),
            state: al_vm::StateLines {
                lines: lines.iter().map(|s| s.to_string()).collect(),
                opaque: Vec::new(),
            },
            hits: Default::default(),
        }
    }

    fn key(test_codeunit: usize) -> Key {
        Key {
            database: 1,
            uncommitted: false,
            backing: None,
            session: 2,
            company: "C".into(),
            seed: 0,
            user: "U".into(),
            test_codeunit: Some(test_codeunit),
            resources: 3,
            client: (true, false, false),
            flags: Default::default(),
            license: None,
        }
    }

    #[test]
    fn equal_states_have_no_difference() {
        let a = state(&["C|1990|[Integer(7)]"], &["session x"]);
        assert_eq!(first_difference(&a, &a.clone()), None);
    }

    #[test]
    fn a_row_a_session_line_an_assumption_mark_or_a_value_with_no_comparable_form_is_a_difference() {
        let a = state(&["C|1990|[Integer(7)]"], &["session x"]);
        let row = state(&["C|1990|[Integer(8)]"], &["session x"]);
        assert_eq!(
            first_difference(&a, &row).as_deref(),
            Some("row 1: C|1990|[Integer(7)] | C|1990|[Integer(8)]")
        );
        let more = state(&["C|1990|[Integer(7)]", "C|1990|[Integer(9)]"], &["session x"]);
        assert_eq!(
            first_difference(&a, &more).as_deref(),
            Some("row 2: (none) | C|1990|[Integer(9)]")
        );
        let session = state(&["C|1990|[Integer(7)]"], &["session y"]);
        assert_eq!(
            first_difference(&a, &session).as_deref(),
            Some("session state 1: session x | session y")
        );
        let mut marked = a.clone();
        marked.hits.0[0] = 1;
        assert!(first_difference(&a, &marked).is_some_and(|w| w.starts_with("assumption marks")));
        let mut opaque = a.clone();
        opaque.state.opaque.push("codeunit 5 global 0: DotNet".into());
        assert_eq!(
            first_difference(&opaque, &opaque.clone()).as_deref(),
            Some("not comparable: codeunit 5 global 0: DotNet")
        );
    }

    #[test]
    fn a_session_is_compared_with_the_first_of_its_key_only() {
        let check = Check::default();
        let a = state(&["r"], &["s"]);
        let b = state(&["other"], &["s"]);
        check.note(key(1), a.clone());
        check.note(key(1), a.clone());
        check.note(key(2), b.clone());
        assert_eq!(
            check.line().as_deref(),
            Some(
                "login replay check: 3 sessions over 2 keys (a key seen before: 1 sessions, 33%), 1 compared, 0 differ; across test codeunits: 1 compared, \
                 1 differ (not comparable 0, rows 1, session state 0, marks 0; first: row 1: r | other)"
            )
        );
        check.note(key(1), b);
        assert_eq!(
            check.line().as_deref(),
            Some(
                "login replay check: 4 sessions over 2 keys (a key seen before: 2 sessions, 50%), 2 compared, 1 differ (not comparable 0, rows 1, session state 0, marks 0; first: row 1: r | other); across test \
                 codeunits: 1 compared, 1 differ (not comparable 0, rows 1, session state 0, marks 0; first: row 1: r | other)"
            )
        );
    }

    #[test]
    fn no_line_before_a_session_was_noted() {
        assert_eq!(Check::default().line(), None);
    }

    // A value with no comparable form no longer hides the rows that differ too: each kind is counted.
    #[test]
    fn every_kind_that_differs_is_counted() {
        let check = Check::default();
        let a = state(&["r"], &["s"]);
        let mut b = state(&["other"], &["s"]);
        b.state.opaque.push("codeunit 5 global 1: Codeunit instance".into());
        check.note(key(1), a);
        check.note(key(1), b);
        let line = check.line().unwrap();
        assert!(
            line.contains(
                "1 compared, 1 differ (not comparable 1, rows 1, session state 0, marks 0; first: not comparable: \
                 codeunit 5 global 1: Codeunit instance)"
            ),
            "{line}"
        );
    }
}
