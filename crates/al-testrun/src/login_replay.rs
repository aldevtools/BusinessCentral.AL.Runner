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
use std::sync::{Mutex, PoisonError};

/// The inputs of the login and its subscribers this check keys on, when the test session opens the company: the fields
/// below. Two sessions with the same key may share one run of the login only when they also hold the same after it
/// ([`Check`]); a field left out of the key would let a replay hand a session the store of another input. The engine
/// (the compiled program, its apps and their code) is not a field: the check lives on the engine (`Engine::derived`).
/// Not yet fields, though the login can read them or run otherwise under them: the clock start
/// (`Scratch::clock_start`), the stability perturbation (`Scratch::perturb`), the host's company list (the test data's
/// `known_companies`) and the modes that follow the code a session runs (coverage, the impact trace, the observer);
/// runtime `install_once.rs` `Key` holds the first two. Here a missing field can only show as a difference (the check
/// replays nothing); the replay adds them before it serves a session (al-runner issue 8211).
///
/// The maintainer's two conditions for the replay (al-runner issue 8211): every field holds its input by content or by a
/// SHA-256 digest ([`LinesDigest`]), never a 64-bit hash; and the test codeunit stays a field until a repeat across test
/// codeunits is measured on a real app. Tests `every_key_field_is_content_or_a_sha256_digest` and
/// `two_test_codeunits_never_share_a_key`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Key {
    /// Every committed row before the login: the install baseline, what install code and an earlier codeunit left.
    database: LinesDigest,
    /// The rows the session holds now, written and not committed included, when some are not committed before the login:
    /// the login reads them, so two sessions whose uncommitted rows differ have other keys. `None` when every row is
    /// committed (the rows are then `database`).
    uncommitted: Option<LinesDigest>,
    /// Who offers the rows of the tables no write reached (the test data, `MemStore::backing_identity`).
    backing: BackingKey,
    /// The session state before the login (`Session::state_lines`): language, WorkDate, number sequences, isolated
    /// storage, SingleInstance globals, manual bindings, `Random`, filter group 1, the clock offset, the Guid key and
    /// draws, row versions, AutoIncrement counters, the last error.
    session: LinesDigest,
    /// `CompanyName()` and the company every record reads in.
    company: String,
    /// The run seed: the session clock steps by a key derived from it (`al_host::seed::clock_key`), so the same login
    /// moves the clock by another amount under another seed (measured: two requests of one serve daemon, seeds 1 and 2;
    /// alrunner-cli tests/serve.rs `the_login_replay_check_finds_no_difference_across_entries_and_requests`).
    seed: i32,
    /// `UserId()`.
    user: String,
    /// The test codeunit the session opens for: `login::decide` reads it, and a replay may not cross codeunits until a
    /// repeat across test codeunits is measured on a real app (the maintainer's second condition, al-runner issue 8211).
    test_codeunit: Option<usize>,
    /// The resource texts the pass reads in place of the apps' files (a serve entry's `resources`,
    /// `Engine::resource_overrides`): `NavApp.GetResource` in a subscriber reads them.
    resources: LinesDigest,
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
        let uncommitted = store.has_uncommitted().then(|| digest(&current_rows(store)));
        let backing = BackingKey::of(store.backing_identity());
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
            session: digest(&[digest(&state.lines).hex(), digest(&state.opaque).hex()]),
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

/// Who offers the rows of the tables no write reached, as a field of [`Key`].
#[derive(Debug, Clone, PartialEq, Eq)]
enum BackingKey {
    /// `MemStore::backing_identity`: two backings with the same identity offer the same rows (`Backing::identity`).
    Known((u64, u64)),
    /// The backing does not say who it is (`Backing::identity` `None`: not known), so it may offer other rows than
    /// another such backing: a number no other session gets, so that the key matches no other key.
    Unknown(u64),
}

impl BackingKey {
    fn of(identity: Option<(u64, u64)>) -> BackingKey {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        identity.map_or_else(
            || BackingKey::Unknown(NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)),
            BackingKey::Known,
        )
    }
}

impl Key {
    /// The fields in which this key differs from `other`, by name, in the order they are declared.
    /// Every field is named, so a field added to [`Key`] does not compile until it is listed here.
    pub(crate) fn differs_in(&self, other: &Key) -> Vec<&'static str> {
        let Key {
            database,
            uncommitted,
            backing,
            session,
            company,
            seed,
            user,
            test_codeunit,
            resources,
            client,
            flags,
            license,
        } = self;
        [
            (*database != other.database, "database"),
            (*uncommitted != other.uncommitted, "uncommitted"),
            (*backing != other.backing, "backing"),
            (*session != other.session, "session"),
            (*company != other.company, "company"),
            (*seed != other.seed, "seed"),
            (*user != other.user, "user"),
            (*test_codeunit != other.test_codeunit, "test_codeunit"),
            (*resources != other.resources, "resources"),
            (*client != other.client, "client"),
            (*flags != other.flags, "flags"),
            (*license != other.license, "license"),
        ]
        .into_iter()
        .filter_map(|(differs, name)| differs.then_some(name))
        .collect()
    }
}

/// What a test session holds after the raised login: every committed row (system fields included), the session state
/// (`Session::state_lines`), the assumption marks the login made (a replayed flagged pass stays flagged) and what the
/// company open noted for the tests of the codeunit (`install::CompanyOpenWrites`: a later test is watched or refused by
/// it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoginState {
    rows: Vec<String>,
    state: al_vm::StateLines,
    hits: al_builtins::assumptions::HitBits,
    open: String,
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

    /// What `session` holds now, with the marks `hits` the company open made and what it noted (`wrote`).
    pub(crate) fn of(
        session: &mut Session,
        wrote: &crate::install::CompanyOpenWrites,
        hits: al_builtins::assumptions::HitBits,
    ) -> LoginState {
        LoginState {
            rows: committed_rows(session.store_mut()),
            state: session.state_lines(),
            hits,
            open: format!("{wrote:?}"),
        }
    }
}

/// The kinds of difference between two sessions' states, in the order [`differences`] looks at them: a value with no
/// comparable form (which no replay could give back), the committed rows, the session state lines, the assumption marks,
/// what the company open noted for the tests.
const KINDS: [&str; 5] = ["not comparable", "rows", "session state", "marks", "open writes"];

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
        (a.open != b.open).then(|| format!("open writes: {} | {}", a.open, b.open)),
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
    /// Per set of key fields (joined by `+`), the new keys after the first that differ from their nearest earlier key in
    /// exactly those fields ([`Key::differs_in`]).
    new_keys_by: std::collections::BTreeMap<String, u64>,
}

impl Tally {
    /// Compares `state` with the first state of `key`, or keeps it as that. `true`: the key was new.
    fn note(&mut self, key: Key, state: LoginState) -> bool {
        let Some((_, first)) = self.firsts.iter().find(|(k, _)| *k == key) else {
            let nearest = self.firsts.iter().map(|(k, _)| key.differs_in(k)).min_by_key(Vec::len);
            if let Some(fields) = nearest {
                *self.new_keys_by.entry(fields.join("+")).or_default() += 1;
            }
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

    /// The keys line: `None` before any session was noted.
    /// Why the keys differ: each new key after the first, by the fields in which it differs from its nearest earlier key.
    pub(crate) fn keys_line(&self) -> Option<String> {
        let s = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let keys = s.exact.firsts.len();
        if keys == 0 {
            return None;
        }
        let by: Vec<String> = s
            .exact
            .new_keys_by
            .iter()
            .map(|(fields, n)| format!("{fields} {n}"))
            .collect();
        Some(format!(
            "login replay keys: {keys} keys, {} new after the first; each differs from its nearest earlier key in: {}",
            keys - 1,
            if by.is_empty() { "-".to_string() } else { by.join(", ") }
        ))
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

/// The report of `engine`'s check, the check line then the keys line: `None` when no session raised the login under the
/// check.
pub(crate) fn report(engine: &Engine) -> Option<String> {
    let check = check(engine);
    let line = check.line()?;
    // The keys line comes after the check line, which keeps its place as the first line.
    Some(match check.keys_line() {
        Some(keys) => format!("{line}\n{keys}"),
        None => line,
    })
}

/// Every committed row of `store`, one line each ([`rows`]).
fn committed_rows(store: &al_data::memory::MemStore) -> Vec<String> {
    rows(&store.export_committed().unwrap_or_default())
}

/// Every row `store` holds now, the uncommitted changes included, one line each as [`committed_rows`] writes them.
fn current_rows(store: &al_data::memory::MemStore) -> Vec<String> {
    rows(&store.export_current())
}

/// The rows of `tables`, one line each: company, table object and the row with its system fields.
fn rows(tables: &[al_data::memory::ExportedTable]) -> Vec<String> {
    let mut out = Vec::new();
    for t in tables {
        for (_, row) in &t.rows {
            out.push(format!("{}|{}|{row:?}", t.company, t.object));
        }
    }
    out
}

/// A SHA-256 of lines, for a key field: two inputs that differ have other digests (no 64-bit hash collides them).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct LinesDigest([u8; 32]);

impl LinesDigest {
    /// The digest as 64 lower-case hex digits.
    fn hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }
}

/// Written as its hex digits, so a key reads short in a failing test.
impl std::fmt::Debug for LinesDigest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.hex())
    }
}

/// The SHA-256 of `lines`, each line its length (8 bytes, little endian) and then its bytes, so no two lists of lines
/// give the same bytes.
fn digest(lines: &[String]) -> LinesDigest {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    for l in lines {
        h.update((l.len() as u64).to_le_bytes());
        h.update(l.as_bytes());
    }
    LinesDigest(h.finalize().into())
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
            open: String::new(),
        }
    }

    fn key(test_codeunit: usize) -> Key {
        Key {
            database: digest(&["1".into()]),
            uncommitted: None,
            backing: BackingKey::Known((0, 0)),
            session: digest(&["2".into()]),
            company: "C".into(),
            seed: 0,
            user: "U".into(),
            test_codeunit: Some(test_codeunit),
            resources: digest(&["3".into()]),
            client: (true, false, false),
            flags: Default::default(),
            license: None,
        }
    }

    // Condition 1 of the maintainer's acceptance on al-runner issue 8211: the key compares content or a strong hash, never
    // a 64-bit digest. Every field is named with its type, so a field added or changed does not compile until it is
    // classified here: a SHA-256 digest of lines, or the content itself. `backing` is not a digest: two process-unique
    // numbers (`TestData::id`, `Engine::uid`) that name who offers the rows, or, for a backing that does not say who it
    // is, a number no other session gets (`BackingKey`). The module's own code names no 64-bit hasher.
    #[test]
    fn every_key_field_is_content_or_a_sha256_digest() {
        let k = key(1);
        let Key {
            database,
            uncommitted,
            backing,
            session,
            company,
            seed,
            user,
            test_codeunit,
            resources,
            client,
            flags,
            license,
        } = &k;
        let digests: [&LinesDigest; 3] = [database, session, resources];
        let _: &Option<LinesDigest> = uncommitted;
        let _: &BackingKey = backing;
        let _: (&String, &i32, &String, &Option<usize>) = (company, seed, user, test_codeunit);
        let _: &(bool, bool, bool) = client;
        let _: &crate::runtime::session_flags::SessionFlags = flags;
        let _: &Option<std::sync::Arc<al_host::license::License>> = license;
        for d in digests {
            assert_eq!(std::mem::size_of_val(&d.0), 32);
        }
        let code = include_str!("login_replay.rs").split("#[cfg(test)]").next().unwrap();
        for hasher in ["DefaultHasher", "std::hash::", "FxHash", "ahash", "-> u64"] {
            assert!(!code.contains(hasher), "{hasher}");
        }
    }

    // Condition 2 of the maintainer's acceptance on al-runner issue 8211: the test codeunit stays in the key. Two sessions
    // that differ only in their test codeunit have other keys, so neither is compared with (nor, later, replayed from) the
    // other's first state; only the evidence tally across test codeunits sees them together.
    #[test]
    fn two_test_codeunits_never_share_a_key() {
        assert_ne!(key(1), key(2));
        let check = Check::default();
        check.note(key(1), state(&["r"], &["s"]));
        check.note(key(2), state(&["other"], &["s"]));
        let line = check.line().unwrap();
        assert!(
            line.starts_with(
                "login replay check: 2 sessions over 2 keys (a key seen before: 0 sessions, 0%), 0 compared"
            ),
            "{line}"
        );
    }

    // A key field is a SHA-256 of its lines (each line its length, then its bytes), not a 64-bit hash: two inputs that
    // differ never share a key by a collision of the digest.
    #[test]
    fn a_key_digest_is_the_sha256_of_its_lines() {
        assert_eq!(
            format!("{:?}", digest(&[])),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_ne!(digest(&["ab".into()]), digest(&["a".into(), "b".into()]));
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
                 1 differ (not comparable 0, rows 1, session state 0, marks 0, open writes 0; first: row 1: r | other)"
            )
        );
        check.note(key(1), b);
        assert_eq!(
            check.line().as_deref(),
            Some(
                "login replay check: 4 sessions over 2 keys (a key seen before: 2 sessions, 50%), 2 compared, 1 differ (not comparable 0, rows 1, session state 0, marks 0, open writes 0; first: row 1: r | other); across test \
                 codeunits: 1 compared, 1 differ (not comparable 0, rows 1, session state 0, marks 0, open writes 0; first: row 1: r | other)"
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
                "1 compared, 1 differ (not comparable 1, rows 1, session state 0, marks 0, open writes 0; first: not comparable: \
                 codeunit 5 global 1: Codeunit instance)"
            ),
            "{line}"
        );
    }

    // Why the keys of a run differ (issue 8211): each new key after the first is named by the key fields in which it
    // differs from the nearest earlier key (the one it differs from in the fewest fields), counted per set of fields.
    #[test]
    fn a_new_key_names_the_fields_it_differs_in_from_the_nearest_earlier_key() {
        let check = Check::default();
        let s = state(&["r"], &["s"]);
        check.note(key(1), s.clone());
        check.note(key(1), s.clone());
        check.note(key(2), s.clone());
        let other_rows = Key {
            database: digest(&["9".into()]),
            session: digest(&["8".into()]),
            ..key(1)
        };
        check.note(other_rows, s);
        assert_eq!(
            check.keys_line().as_deref(),
            Some(
                "login replay keys: 3 keys, 2 new after the first; each differs from its nearest earlier key in: database+session 1, test_codeunit 1"
            )
        );
        assert_eq!(key(1).differs_in(&key(1)), Vec::<&str>::new());
        assert_eq!(Check::default().keys_line(), None);
    }
}
