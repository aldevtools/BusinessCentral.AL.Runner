//! The login key and the check that a replay of the raised login gives each test session what running the login gives
//! it (al-runner issue 8211). The maintainer's condition for a replay: the replayed store is the same as running the
//! login for each session, including what the subscribers write. The replay itself is `login_start.rs`; this module
//! keys it and measures it, and replays nothing.
//!
//! The key ([`Key::of`]) holds the inputs the login reads, each by content or by a SHA-256 digest; its doc lists them,
//! and `login_start/outside.rs` names what it does with every input outside `Session`. A session that holds a value
//! with no comparable form has no key. What no key holds keeps a run from the starts instead (`login_start::run_gate`:
//! the code a run notes, a stability perturbation or a set clock start, single instances moved in, the real clock, a
//! real files directory).
//!
//! The check, under `RunOptions::login_replay_check` (`ALRUNNER_LOGIN_REPLAY_CHECK=1`, read by the front ends):
//! `runtime/run.rs` `open_company` takes the key of each test session before its login and, when the login was raised
//! (`install::CompanyOpenWrites::raised`), what the session holds after it ([`LoginState`]: committed rows, al-vm
//! `Session::state_lines`, assumption marks, the open's writes). [`Check`] (on the engine) compares each with the first
//! session of its key, and across test codeunits; a new key's session is also frozen, thawn into a twin and compared
//! (the round trip), and a later session is compared with a thaw of its key's start (replay vs run). Outcomes do not
//! change. `login_replay/difference.rs` owns [`differences`] and their text: where two states differ, never a value.
//! The keys line ([`Check::keys_line`]) counts each new key by the key fields in which it differs from its nearest
//! earlier key, and the parts of them (`login_replay/key_parts.rs`: per table and per state line name, by name only).
//! The front ends print [`report`] from `Phases::login_replay`. Tests: `runtime/login_replay_tests.rs`,
//! `runtime/login_start_tests.rs`, alrunner-cli `tests/serve.rs`.

mod difference;
mod key_parts;
mod rows_digest;

use al_vm::{Engine, Session};
pub(crate) use difference::differences;
use difference::{KINDS, StoredRow, stored_rows};
use std::sync::{Mutex, PoisonError};

/// Why a session has no key ([`Key::of`]): it then runs its login.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NoKey {
    /// The session holds a value with no comparable form (al-vm `NotComparable`), at this path (ids and indexes only).
    NotComparable(String),
    /// A row holds a value with no byte form, of this kind (`rows_digest::NoByteForm`).
    NoByteForm(&'static str),
}

impl From<rows_digest::NoByteForm> for NoKey {
    fn from(no: rows_digest::NoByteForm) -> NoKey {
        NoKey::NoByteForm(no.0)
    }
}

impl NoKey {
    /// The reason as the starts line names it.
    pub(crate) fn reason(&self) -> String {
        match self {
            NoKey::NotComparable(path) => format!("no comparable form: {path}"),
            NoKey::NoByteForm(kind) => format!("no byte form: {kind}"),
        }
    }
}

/// The inputs the login and its subscribers read when the test session opens the company, as far as the code shows them:
/// a session state the key does not hold is held in place by the thaw's rule (al-vm `session_fields.rs`, `login_image/fields.rs`), and a
/// static a request sets is a field here (`login_start/outside.rs`, which also names what the key does with each field
/// of the run's `Scratch` and each engine field a run sets). Two sessions with the same key may share one run of the
/// login only when they also hold the same after it ([`Check`]); a field left out of the key would let a replay hand a
/// session the store of another input. The engine (the compiled program, its apps and their code) is not a field: the
/// check lives on the engine (`Engine::derived`). What a run sets on it is, where a kept engine of a warm server may
/// take another value in a later request (`emit_version`, `refused_reads`).
///
/// The maintainer's two conditions for the replay (al-runner issue 8211): every field holds its input by content or by a
/// SHA-256 digest ([`LinesDigest`]), never a 64-bit hash; and the test codeunit stays a field until a repeat across test
/// codeunits is measured on a real app. Tests `every_key_field_is_content_or_a_sha256_digest` and
/// `two_test_codeunits_never_share_a_key`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Key {
    /// Every committed row before the login: the install baseline, what install code and an earlier codeunit left.
    database: LinesDigest,
    /// The rows the session holds now, written and not committed included, when some are not committed before the
    /// login: the login reads them, so two sessions whose uncommitted rows differ have other keys. `None` when every
    /// row is committed (the rows are then `database`). A write since the last Commit that leaves the SQL counters with
    /// no comparable form gives no key at all ([`Key::of`]).
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
    /// The parts of `database` and `session`, each with a digest of its own (`key_parts.rs`): which tables and which state
    /// lines a new key differs in, for the keys line. Never compared (`NotCompared`), so they change no key equality.
    parts: key_parts::NotCompared<key_parts::KeyParts>,
    /// The install code that could not run (`runtime::install_once`): the open runs under the watch of its tables, whose
    /// hit marks an assumption or refuses the codeunit (`runtime/blocked.rs` `watching_skipped_installs`). By content.
    skipped: Vec<al_vm::SkippedInstall>,
    /// The test isolation the codeunit runs under and whether BC's TestRunnerHub runs it (`Scratch::hub`): a request
    /// sets both (`testIsolation`), and they set the session's isolation fields before the login.
    isolation: (crate::isolation::TestIsolation, bool),
    /// The settings of the process a serve request sets and the system host answers from ([`RunSettings`]).
    run: RunSettings,
    /// The emit version of the run's build, which the missing-object error names (`Engine::emit_version`): a warm
    /// server sets it per request, one per build (`set_run_options`), also on a kept engine. By its reader
    /// ([`EmitKey`]).
    emit_version: EmitKey,
    /// The tables a read of is refused on the engine: those the install baseline should have seeded, with why
    /// (`Engine::unseeded`), and those whose demo rows the runtime does not have (`Engine::demo_tables`). A run sets
    /// both after its install baseline, also on a kept engine; a login that reads one is refused there.
    refused_reads: LinesDigest,
}

/// The emit version as a field of [`Key`]: by the reader the engine holds (`EmitVersion::same_reader`), since the
/// number is read only when a missing-object error first needs it. Like [`BackingKey`] it names who answers, not a
/// hash: a clone of one reader answers the same number, and a reader made apart is another key even for one build.
#[derive(Debug, Clone)]
pub(crate) struct EmitKey(al_vm::emit_version::EmitVersion);

impl PartialEq for EmitKey {
    fn eq(&self, other: &EmitKey) -> bool {
        self.0.same_reader(&other.0)
    }
}

impl Eq for EmitKey {}

/// The lines of [`Key::refused_reads`]: each unseeded table with why, then each table of demo rows, in table order.
fn refused_reads(engine: &Engine) -> LinesDigest {
    let mut unseeded: Vec<_> = engine.unseeded.iter().collect();
    unseeded.sort_by_key(|(t, _)| **t);
    let mut demo: Vec<_> = engine.demo_tables.iter().collect();
    demo.sort_unstable();
    let lines: Vec<String> = (unseeded.into_iter().map(|(t, why)| format!("unseeded {t} {why}")))
        .chain(demo.into_iter().map(|t| format!("demo {t}")))
        .collect();
    digest(&lines)
}

/// The process settings a serve request sets and the login can read through the system host (`host.rs`): the server
/// base address (`GetUrl`), the companies of the database, the company name and whether the publish order is known. By
/// content: two requests of one daemon with other settings never share a key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunSettings {
    server_url: (String, bool),
    companies: al_host::Companies,
    company_name: Option<String>,
    publish_order_known: bool,
}

impl RunSettings {
    /// The settings now.
    fn now() -> RunSettings {
        RunSettings {
            server_url: crate::host::server_url(),
            companies: crate::host::companies(),
            company_name: crate::host::company_name(),
            publish_order_known: crate::host::publish_order_known(),
        }
    }
}

impl Key {
    /// The key of `session` before it opens the company; `isolation`: the codeunit's test isolation and whether the hub
    /// runs it; `flags`: the run options the session took; `skipped`: the install code that could not run, whose tables
    /// the open is watched on. [`NoKey`] when the session holds a value with no comparable form (al-vm `NotComparable`:
    /// the key would hold it by its path only, so two sessions with other values there would share a key) or a row holds
    /// a value with no byte form (`rows_digest.rs`). Such a session runs its login.
    pub(crate) fn of(
        session: &mut Session,
        isolation: (crate::isolation::TestIsolation, bool),
        flags: crate::runtime::session_flags::SessionFlags,
        skipped: &[al_vm::SkippedInstall],
    ) -> Result<Key, NoKey> {
        let state = session.state_lines();
        if let Some(opaque) = state.opaque.first() {
            return Err(NoKey::NotComparable(opaque.path.clone()));
        }
        let engine = session.engine.clone();
        let store = session.store_mut();
        let tables = key_parts::store_table_digests(&engine, store)?;
        let database = key_parts::database_digest(&tables);
        let uncommitted = match store.has_uncommitted() {
            true => Some(rows_digest::current_digest(store)?),
            false => None,
        };
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
        Ok(Key {
            database,
            uncommitted,
            backing,
            session: session_digest(&state),
            company: session.access_company().to_string(),
            seed: session.run_seed(),
            user: session.user_id(),
            test_codeunit: session.test_object,
            resources,
            client: (
                session.has_interactive_client(),
                session.is_clientless_session(),
                isolation.0 == crate::isolation::TestIsolation::Disabled,
            ),
            flags,
            license: session.engine.license.clone(),
            parts: key_parts::NotCompared(key_parts::KeyParts::of(tables, &state)),
            skipped: skipped.to_vec(),
            isolation,
            run: RunSettings::now(),
            emit_version: EmitKey(engine.emit_version.clone()),
            refused_reads: refused_reads(&engine),
        })
    }
}

/// [`Key::session`] of the state lines `state`: the lines, then the paths and reasons of the values with no comparable
/// form.
fn session_digest(state: &al_vm::StateLines) -> LinesDigest {
    let opaque: Vec<String> = state.opaque.iter().map(al_vm::NotComparable::line).collect();
    digest(&[digest(&state.lines).hex(), digest(&opaque).hex()])
}

#[cfg(test)]
impl Key {
    /// Whether the session state of this key is `state` ([`Key::session`]).
    pub(crate) fn holds_session(&self, state: &al_vm::StateLines) -> bool {
        self.session == session_digest(state)
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
            // A detail of `database` and `session` (see `differing_parts`), not a field of its own.
            parts: _,
            skipped,
            isolation,
            run,
            emit_version,
            refused_reads,
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
            (*skipped != other.skipped, "skipped"),
            (*isolation != other.isolation, "isolation"),
            (*run != other.run, "run"),
            (*emit_version != other.emit_version, "emit_version"),
            (*refused_reads != other.refused_reads, "refused_reads"),
        ]
        .into_iter()
        .filter_map(|(differs, name)| differs.then_some(name))
        .collect()
    }

    /// Per field of [`Key::differs_in`] that has parts (`database`, `session`), the parts that differ, each with its field:
    /// by name only, never a value, a company by its number in `companies` (`key_parts.rs`).
    pub(crate) fn differing_parts(
        &self,
        other: &Key,
        companies: &mut key_parts::Companies,
    ) -> Vec<(&'static str, String)> {
        (self.differs_in(other).into_iter())
            .flat_map(|field| {
                let parts = match field {
                    "database" => self.parts.0.database_differs(&other.parts.0, companies),
                    "session" => self.parts.0.session_differs(&other.parts.0),
                    _ => Vec::new(),
                };
                parts.into_iter().map(move |part| (field, part))
            })
            .collect()
    }
}

/// What a test session holds after the raised login: every committed row (system fields included), the session state
/// (`Session::state_lines`), the assumption marks the login made (a replayed flagged pass stays flagged) and what the
/// company open noted for the tests of the codeunit (`install::CompanyOpenWrites`: a later test is watched or refused by
/// it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoginState {
    rows: Vec<StoredRow>,
    state: al_vm::StateLines,
    hits: al_builtins::assumptions::HitBits,
    open: String,
}

impl LoginState {
    /// The committed rows, field by field.
    #[cfg(test)]
    pub(crate) fn rows(&self) -> &[StoredRow] {
        &self.rows
    }

    /// The session state lines.
    #[cfg(test)]
    pub(crate) fn state_lines(&self) -> &al_vm::StateLines {
        &self.state
    }

    /// What the thawn `twin` holds now, with the marks `hits` and the open writes `open` that came with its image: those
    /// are not session state (al-testrun keeps them next to an image).
    fn of_twin(twin: &mut Session, (hits, open): (al_builtins::assumptions::HitBits, String)) -> LoginState {
        LoginState {
            rows: stored_rows(&twin.store_mut().export_committed().unwrap_or_default()),
            state: twin.state_lines(),
            hits,
            open,
        }
    }

    /// What `session` holds now, with the marks `hits` the company open made and what it noted (`wrote`).
    pub(crate) fn of(
        session: &mut Session,
        wrote: &crate::install::CompanyOpenWrites,
        hits: al_builtins::assumptions::HitBits,
    ) -> LoginState {
        LoginState {
            rows: stored_rows(&session.store_mut().export_committed().unwrap_or_default()),
            state: session.state_lines(),
            hits,
            open: format!("{wrote:?}"),
        }
    }
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
    image: ImageTally,
    /// Each later session of a key with a start against a thaw of the start (`Check::replay_vs_run`).
    replay_vs_run: Differences,
}

/// Comparisons of thawn twins with real sessions: how many, how many differ, by kind of [`KINDS`], and the first.
#[derive(Default)]
struct Differences {
    checked: u64,
    differ: u64,
    differ_by: [u64; KINDS.len()],
    first: Option<String>,
}

impl Differences {
    fn note(&mut self, differs: &[(usize, String)]) {
        self.checked += 1;
        if let Some((_, what)) = differs.first() {
            self.differ += 1;
            self.first.get_or_insert(what.clone());
        }
        for (k, _) in differs {
            self.differ_by[*k] += 1;
        }
    }

    /// `C checked, D differ`, with the kinds and the first difference when one differs.
    fn line(&self) -> String {
        let first = self.first.as_ref().map(|first| {
            let by: Vec<String> = (KINDS.iter().zip(&self.differ_by))
                .filter(|(_, n)| **n > 0)
                .map(|(kind, n)| format!("{kind} {n}"))
                .collect();
            format!(" ({}; first: {first})", by.join(", "))
        });
        format!(
            "{} checked, {} differ{}",
            self.checked,
            self.differ,
            first.unwrap_or_default()
        )
    }
}

/// The round trips of the session image (al-vm `login_image`): per new key, whether its session was imaged, why not, its
/// size, and whether the thawn twin holds what the session holds.
#[derive(Default)]
struct ImageTally {
    keys: u64,
    /// Per reason (`NotImaged::reason`), the keys whose session was not imaged.
    not_imaged: std::collections::BTreeMap<String, u64>,
    /// Per reason (`NotThawed::reason`), the images the twin did not take: no round trip is compared for them.
    not_thawed: std::collections::BTreeMap<String, u64>,
    sizes: Vec<al_vm::ImageSize>,
    /// The round trips.
    differences: Differences,
}

impl ImageTally {
    /// The report line (see [`Check::image_line`]).
    fn line(&self) -> String {
        let not: u64 = self.not_imaged.values().sum();
        let reasons = (!self.not_imaged.is_empty()).then(|| {
            let by: Vec<String> = (self.not_imaged.iter()).map(|(why, n)| format!("{n}: {why}")).collect();
            format!(" ({})", by.join(", "))
        });
        let refused = (!self.not_thawed.is_empty()).then(|| {
            let n: u64 = self.not_thawed.values().sum();
            let by: Vec<String> = (self.not_thawed.iter()).map(|(why, n)| format!("{n}: {why}")).collect();
            format!(", {n} not thawed ({})", by.join(", "))
        });
        let trips = self.differences.line().replacen(" checked", "", 1);
        format!(
            "login image: {} keys, {} imaged, {not} not imaged{}; round trips {trips}{}; per key: {}",
            self.keys,
            self.sizes.len(),
            reasons.unwrap_or_default(),
            refused.unwrap_or_default(),
            self.per_key()
        )
    }

    /// The medians of the images' sizes, and the largest image.
    fn per_key(&self) -> String {
        if self.sizes.is_empty() {
            return "none".into();
        }
        let median = |f: &dyn Fn(&al_vm::ImageSize) -> u64| {
            let mut v: Vec<u64> = self.sizes.iter().map(f).collect();
            v.sort_unstable();
            v[v.len() / 2]
        };
        let mb = |b: u64| format!("{:.1} MB", b as f64 / 1_048_576.0);
        let largest = self.sizes.iter().map(|s| s.kept_bytes).max().unwrap_or(0);
        // Per kind of node with any in an image: the median count, names only.
        let kinds: Vec<String> = (al_vm::NODE_KINDS.iter().enumerate())
            .filter(|(k, _)| self.sizes.iter().any(|s| s.kinds[*k].count > 0))
            .map(|(k, name)| format!("{name} {}", median(&|s| s.kinds[k].count as u64)))
            .collect();
        format!(
            "rows {} (delta), state {}, kept {} ({}), {} nodes, {} instances, {} temporary tables, {} temporary rows; \
             largest key {}",
            mb(median(&|s| s.rows_bytes)),
            mb(median(&|s| s.state_bytes)),
            mb(median(&|s| s.kept_bytes)),
            kinds.join(", "),
            median(&|s| s.nodes as u64),
            median(&|s| s.instances as u64),
            median(&|s| s.temp_tables as u64),
            median(&|s| s.temp_rows as u64),
            mb(largest)
        )
    }
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
    new_keys_by: std::collections::BTreeMap<String, NewKeys>,
    /// The companies `new_keys_by` names, by number.
    companies: key_parts::Companies,
}

/// The parts the keys line shows per key field of an entry, the most counted first; the rest it counts as `and N more`.
const PARTS_SHOWN: usize = 5;

/// The new keys of one set of key fields: how many, and per key field and part ([`Key::differing_parts`]) how many of
/// them differ in it.
#[derive(Default)]
struct NewKeys {
    n: u64,
    parts: std::collections::BTreeMap<(&'static str, String), u64>,
}

impl NewKeys {
    /// `N`, then per key field the parts with their counts, the most counted first, at most [`PARTS_SHOWN`] of them.
    fn text(&self) -> String {
        let mut by_field: std::collections::BTreeMap<&str, Vec<(&String, u64)>> = Default::default();
        for ((field, part), n) in &self.parts {
            by_field.entry(field).or_default().push((part, *n));
        }
        if by_field.is_empty() {
            return self.n.to_string();
        }
        let fields: Vec<String> = (by_field.into_iter())
            .map(|(field, mut parts)| {
                parts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
                let mut shown: Vec<String> = (parts.iter().take(PARTS_SHOWN))
                    .map(|(part, n)| format!("{part} {n}"))
                    .collect();
                if parts.len() > PARTS_SHOWN {
                    shown.push(format!("and {} more", parts.len() - PARTS_SHOWN));
                }
                format!("{field}: {}", shown.join(", "))
            })
            .collect();
        format!("{} ({})", self.n, fields.join("; "))
    }
}

impl Tally {
    /// Compares `state` with the first state of `key`, or keeps it as that. `true`: the key was new.
    fn note(&mut self, key: Key, state: LoginState) -> bool {
        let Some((_, first)) = self.firsts.iter().find(|(k, _)| *k == key) else {
            let nearest = (self.firsts.iter()).min_by_key(|(k, _)| key.differs_in(k).len());
            if let Some((k, _)) = nearest {
                let entry = self.new_keys_by.entry(key.differs_in(k).join("+")).or_default();
                entry.n += 1;
                for part in key.differing_parts(k, &mut self.companies) {
                    *entry.parts.entry(part).or_default() += 1;
                }
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
    /// Notes the state of a session of `key` after the raised login. `true`: the key was new.
    pub(crate) fn note(&self, key: Key, state: LoginState) -> bool {
        let mut s = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let across = Key {
            test_codeunit: None,
            ..key.clone()
        };
        let new = s.exact.note(key, state.clone());
        if new {
            s.across_codeunits.note(across, state);
        }
        new
    }

    /// A new key's session after the raised login: frozen ([`Session::login_image`]), thawn into a twin that starts from
    /// the store `base` (the session before its login), and the twin's state compared with `state` ([`differences`]).
    /// Returns the image, for the key's start (`login_start.rs`). Outcomes do not change: the twin is dropped and
    /// `session` is only read. A twin that does not take the image (`NotThawed`) is counted, and no round trip is
    /// compared for it.
    pub(crate) fn round_trip(
        &self,
        session: &Session,
        base: &al_vm::LoginBase,
        state: &LoginState,
        twin: impl FnOnce() -> Session + Send,
    ) -> Result<al_vm::LoginImage, al_vm::NotImaged> {
        let image = session.login_image(base);
        let thawn = image
            .as_ref()
            .ok()
            .map(|image| thaw_twin(image, base, (state.hits, state.open.clone()), twin));
        let mut s = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let t = &mut s.image;
        t.keys += 1;
        match (&image, thawn) {
            (Err(why), _) => *t.not_imaged.entry(why.reason()).or_default() += 1,
            (Ok(image), Some(thawn)) => {
                t.sizes.push(image.size());
                match thawn {
                    Ok(twin) => t.differences.note(&differences(state, &twin)),
                    Err(Uncompared::NotThawed(why)) => *t.not_thawed.entry(why.reason()).or_default() += 1,
                    Err(why) => t.differences.note(&[(0, format!("not comparable: {why}"))]),
                }
            }
            (Ok(_), None) => unreachable!("an image is thawn"),
        }
        image
    }

    /// A later session of a key after its real login (`state`; `base`: the session before it) against a thaw of the key's
    /// start into a twin that starts from the same store: what a replay would have given it ([`differences`]).
    pub(crate) fn replay_vs_run(
        &self,
        start: &crate::login_start::Start,
        base: &al_vm::LoginBase,
        state: &LoginState,
        twin: impl FnOnce() -> Session + Send,
    ) {
        let thawn = thaw_twin(&start.image, base, (start.hits, format!("{:?}", start.wrote)), twin);
        let mut s = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        s.replay_vs_run.note(&match thawn {
            Ok(twin) => differences(state, &twin),
            Err(why) => vec![(0, format!("not comparable: {why}"))],
        });
    }
    /// `C checked, D differ` of the later sessions compared with a thaw of their key's start.
    fn replay_vs_run_line(&self) -> String {
        self.inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .replay_vs_run
            .line()
    }

    /// The image line: `None` before a round trip was tried.
    fn image_line(&self) -> Option<String> {
        let s = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        (s.image.keys > 0).then(|| s.image.line())
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
            .map(|(fields, keys)| format!("{fields} {}", keys.text()))
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

#[cfg(test)]
thread_local! {
    /// Run on the twin after the thaw ([`Check::round_trip`]): a test plants a difference the check must report.
    pub(crate) static TWIN_PLANT: std::cell::Cell<Option<fn(&mut Session)>> = const { std::cell::Cell::new(None) };
    /// Run on the twin before the thaw ([`Check::round_trip`]): a test makes it a session the thaw must refuse.
    pub(crate) static TWIN_TARGET: std::cell::Cell<Option<fn(&mut Session)>> = const { std::cell::Cell::new(None) };
    /// [`serve_twin`] is on, and how many sessions it replaced.
    pub(crate) static SERVE_TWIN: std::cell::Cell<(bool, u32)> = const { std::cell::Cell::new((false, 0)) };
}

/// Under [`SERVE_TWIN`] (tests only), each session after its raised login is replaced by a twin made by `twin` and thawn
/// from the session's image on this thread, so the codeunit's tests run on the twin: the evidence, at the level of test
/// results, that a thawn session serves the tests as the session that ran its login does (issue 8211).
#[cfg(test)]
pub(crate) fn serve_twin(session: &mut Session, base: &al_vm::LoginBase, twin: impl FnOnce() -> Session) {
    let (on, served) = SERVE_TWIN.with(std::cell::Cell::get);
    if !on {
        return;
    }
    let image = (session.login_image(base)).unwrap_or_else(|why| panic!("no image: {}", why.reason()));
    let mut twin = twin();
    twin.start_from(&base.portable().expect("the store before the login has a rows form"));
    twin.thaw_login_image(&image).expect("a new session takes the image");
    *session = twin;
    SERVE_TWIN.with(|s| s.set((true, served + 1)));
}

/// Why a thawn twin was not compared ([`thaw_twin`]).
enum Uncompared {
    /// The store before the login has no rows form.
    NoBase,
    /// The twin did not take the image (`Session::thaw_login_image`).
    NotThawed(al_vm::NotThawed),
    /// The thaw's thread did not end.
    NoEnd,
}

impl std::fmt::Display for Uncompared {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Uncompared::NoBase => {
                f.write_str("the store before the login has no rows form (a value with no bytes, or company copies)")
            }
            Uncompared::NotThawed(why) => write!(f, "not thawed: {}", why.reason()),
            Uncompared::NoEnd => f.write_str("the thaw did not end"),
        }
    }
}

/// `image` thawn into `twin` (a new session brought to its login by the runner's steps, `runtime/run.rs` `Steps::twin`)
/// that starts from the store of `base`, on a thread of its own (so nothing of the thaw reaches this thread's state:
/// its store operation count, its assumption marks, its instance registry); its state, with the marks and open writes
/// `with` of the image. `Err`: why it could not be compared.
fn thaw_twin(
    image: &al_vm::LoginImage,
    base: &al_vm::LoginBase,
    with: (al_builtins::assumptions::HitBits, String),
    twin: impl FnOnce() -> Session + Send,
) -> Result<LoginState, Uncompared> {
    #[cfg(test)]
    let (target, plant) = (
        TWIN_TARGET.with(std::cell::Cell::get),
        TWIN_PLANT.with(std::cell::Cell::get),
    );
    let base = base.portable().ok_or(Uncompared::NoBase)?;
    std::thread::scope(|scope| {
        // A test thread's stack: the drop of a deep graph recurses on it.
        std::thread::Builder::new()
            .stack_size(crate::runtime::TEST_STACK)
            .spawn_scoped(scope, || -> Result<LoginState, Uncompared> {
                // Declared first, so it drops last: the twin's instances that hold each other are freed.
                let _run_end = al_builtins::instance_cycles::RunEnd::new();
                let mut twin = twin();
                twin.start_from(&base);
                #[cfg(test)]
                if let Some(target) = target {
                    target(&mut twin);
                }
                twin.thaw_login_image(image).map_err(Uncompared::NotThawed)?;
                #[cfg(test)]
                if let Some(plant) = plant {
                    plant(&mut twin);
                }
                Ok(LoginState::of_twin(&mut twin, with))
            })
            .ok()
            .and_then(|thread| thread.join().ok())
            .unwrap_or(Err(Uncompared::NoEnd))
    })
}

/// Under the check, after the real raised login of a session of `key` (`base`: the session before it; `gate`: why its
/// run takes no starts): the session is compared with the first of its key; a new key's session is frozen, thawn into a
/// twin and compared (the round trip) and its image kept as the key's start, as the replay would keep it; a later
/// session is compared with a thaw of the key's start (`Check::replay_vs_run`) where the replay would have taken it. No
/// session takes a start. `twin`: a new session brought to its login by the runner's steps, as the replay's target is
/// (`runtime/run.rs` `Steps::twin`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn checked(
    engine: &Engine,
    session: &mut Session,
    key: Key,
    base: &al_vm::LoginBase,
    wrote: &crate::install::CompanyOpenWrites,
    hits: al_builtins::assumptions::HitBits,
    gate: Option<crate::login_start::NotTaken>,
    twin: impl FnOnce() -> Session + Send,
) {
    let state = LoginState::of(session, wrote, hits);
    let check = check(engine);
    let starts = crate::login_start::starts(engine);
    if check.note(key.clone(), state.clone()) {
        let image = check.round_trip(session, base, &state, twin);
        match gate {
            Some(why) => starts.not_taken(why),
            None => crate::login_start::keep(
                engine,
                key,
                base,
                image.map(|image| crate::login_start::Start {
                    image,
                    wrote: wrote.clone(),
                    hits,
                }),
            ),
        }
        return;
    }
    if let Some(why) = gate {
        starts.not_taken(why);
        return;
    }
    let Some(start) = starts.take(&key) else { return };
    match crate::login_start::target_gate(base.target_why(), base.shape(), &start) {
        Some(why) => starts.not_taken(why),
        None => check.replay_vs_run(&start, base, &state, twin),
    }
}

/// The check of `engine`.
pub(crate) fn check(engine: &Engine) -> std::sync::Arc<Check> {
    engine.derived(Check::default)
}

/// The report of `engine`'s check: the check line, the image line when a round trip was tried, then the keys line, then
/// the starts line (`login_start::Starts::line`) with the check's replay vs run. Without a check line (the replay alone,
/// or no session raised the login under the check): the starts line alone. `None` when there is neither.
pub(crate) fn report(engine: &Engine) -> Option<String> {
    let check = check(engine);
    let Some(line) = check.line() else {
        return crate::login_start::starts(engine).line();
    };
    // The keys line comes after them, so the check line and the image line keep their places; then the starts.
    let mut keys = check.keys_line().map_or(String::new(), |k| format!("\n{k}"));
    if let Some(starts) = crate::login_start::starts(engine).line() {
        let replay_vs_run = check.replay_vs_run_line();
        keys.push_str(&format!("\n{starts}; replay vs run: {replay_vs_run}"));
    }
    Some(match check.image_line() {
        Some(image) => format!("{line}\n{image}{keys}"),
        None => format!("{line}{keys}"),
    })
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
    use al_builtins::value::Value;

    /// A state whose committed rows are one Text field each, of table object 1.
    fn state(rows: &[&str], lines: &[&str]) -> LoginState {
        let rows: Vec<_> = (rows.iter())
            .map(|r| (1, vec![al_builtins::value::Value::Text((*r).into())]))
            .collect();
        state_of(&rows, lines)
    }

    /// A state whose committed rows are `rows` (table object, values) of company C.
    fn state_of(rows: &[(u32, Vec<al_builtins::value::Value>)], lines: &[&str]) -> LoginState {
        let tables: Vec<al_data::memory::ExportedTable> = (rows.iter())
            .map(|(object, row)| al_data::memory::ExportedTable {
                company: "C".into(),
                object: *object,
                pk_storage: None,
                rows: vec![(Vec::new(), row.clone())],
            })
            .collect();
        LoginState {
            rows: stored_rows(&tables),
            state: al_vm::StateLines {
                lines: lines.iter().map(|s| s.to_string()).collect(),
                opaque: Vec::new(),
            },
            hits: Default::default(),
            open: String::new(),
        }
    }

    /// A value with no comparable form at `path`, for `why`.
    fn not_comparable(path: &str, why: &str) -> al_vm::NotComparable {
        al_vm::NotComparable {
            path: path.into(),
            why: why.into(),
        }
    }

    /// A state with no rows and no state lines.
    pub(super) fn empty_state() -> LoginState {
        state_of(&[], &[])
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
            parts: Default::default(),
            skipped: Vec::new(),
            isolation: Default::default(),
            run: RunSettings {
                server_url: ("http://localhost:8080/BC".into(), true),
                companies: Default::default(),
                company_name: None,
                publish_order_known: false,
            },
            emit_version: EmitKey(Default::default()),
            refused_reads: digest(&[]),
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
            parts,
            skipped,
            isolation,
            run,
            emit_version,
            refused_reads,
        } = &k;
        let digests: [&LinesDigest; 4] = [database, session, resources, refused_reads];
        let _: &Option<LinesDigest> = uncommitted;
        let _: &BackingKey = backing;
        let _: (&String, &i32, &String, &Option<usize>) = (company, seed, user, test_codeunit);
        let _: &(bool, bool, bool) = client;
        let _: &crate::runtime::session_flags::SessionFlags = flags;
        let _: &Option<std::sync::Arc<al_host::license::License>> = license;
        // SHA-256 digests per table and per state line name (key_parts.rs scanned below too).
        let _: &key_parts::NotCompared<key_parts::KeyParts> = parts;
        // The skipped installs by content: their tables, the tables left out on an assumption, and why.
        let _: &Vec<al_vm::SkippedInstall> = skipped;
        let _: &(crate::isolation::TestIsolation, bool) = isolation;
        // Who answers the emit version, as `backing` says who offers the rows: the reader itself, never a number of it.
        let _: &EmitKey = emit_version;
        // The process settings a request sets, each by content.
        let RunSettings {
            server_url,
            companies,
            company_name,
            publish_order_known,
        } = run;
        let _: (&(String, bool), &al_host::Companies, &Option<String>, &bool) =
            (server_url, companies, company_name, publish_order_known);
        for d in digests {
            assert_eq!(std::mem::size_of_val(&d.0), 32);
        }
        let code = include_str!("login_replay.rs").split("#[cfg(test)]").next().unwrap();
        let parts = include_str!("login_replay/key_parts.rs");
        for hasher in ["DefaultHasher", "std::hash::", "FxHash", "ahash", "-> u64"] {
            assert!(!code.contains(hasher) && !parts.contains(hasher), "{hasher}");
        }
    }

    // Condition 2 of the maintainer's acceptance on al-runner issue 8211: the test codeunit stays in the key. Two sessions
    // that differ only in their test codeunit have other keys, so neither is compared with (nor, later, replayed from) the
    // other's first state; only the evidence tally across test codeunits sees them together.
    // The server base address a request sets, and the codeunit's isolation and hub, are inputs of
    // the login (`GetUrl`, the isolation fields `enter_test_codeunit` sets): another value is another key.
    #[test]
    fn another_server_url_or_isolation_is_another_key() {
        let mut url = key(1);
        url.run.server_url = ("https://other.example/BC280/".into(), false);
        assert_ne!(key(1), url);
        assert_eq!(url.differs_in(&key(1)), vec!["run"]);
        let mut hub = key(1);
        hub.isolation.1 = true;
        assert_eq!(hub.differs_in(&key(1)), vec!["isolation"]);
        let mut function = key(1);
        function.isolation.0 = crate::isolation::TestIsolation::Function;
        assert_ne!(key(1), function);
    }

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
        let a = state_of(&[(1990, vec![Value::Integer(7)])], &["session x"]);
        assert_eq!(first_difference(&a, &a.clone()), None);
    }

    #[test]
    fn a_row_a_session_line_an_assumption_mark_or_a_value_with_no_comparable_form_is_a_difference() {
        let a = state_of(&[(1990, vec![Value::Integer(7)])], &["session x"]);
        let row = state_of(&[(1990, vec![Value::Integer(8)])], &["session x"]);
        assert_eq!(
            first_difference(&a, &row).as_deref(),
            Some("row 1 (table object 1990): field 1")
        );
        let more = state_of(
            &[(1990, vec![Value::Integer(7)]), (1990, vec![Value::Integer(9)])],
            &["session x"],
        );
        assert_eq!(
            first_difference(&a, &more).as_deref(),
            Some("row 2 (table object 1990): added")
        );
        let session = state_of(&[(1990, vec![Value::Integer(7)])], &["session y"]);
        assert_eq!(first_difference(&a, &session).as_deref(), Some("session state 1"));
        // A `field.` line is named by its field (a name, never its value), and when more lines
        // differ the text says how many.
        let row = [(1990, vec![Value::Integer(7)])];
        let fields = state_of(&row, &["session x", "field.a 1", "field.b 2", "field.c 3"]);
        let other = state_of(&row, &["session x", "field.a 1", "field.b 9", "field.c 8"]);
        assert_eq!(
            first_difference(&fields, &other).as_deref(),
            Some("session state 3 (field.b), 2 lines differ")
        );
        let mut marked = a.clone();
        marked.hits.0[0] = 1;
        assert_eq!(first_difference(&a, &marked).as_deref(), Some("assumption marks"));
        let mut opaque = a.clone();
        opaque
            .state
            .opaque
            .push(not_comparable("codeunit 5 global 0", "DotNet"));
        assert_eq!(
            first_difference(&opaque, &opaque.clone()).as_deref(),
            Some("not comparable: codeunit 5 global 0")
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
                 1 differ (not comparable 0, rows 1, session state 0, marks 0, open writes 0; first: row 1 (table object 1): field 1)"
            )
        );
        check.note(key(1), b);
        assert_eq!(
            check.line().as_deref(),
            Some(
                "login replay check: 4 sessions over 2 keys (a key seen before: 2 sessions, 50%), 2 compared, 1 differ (not comparable 0, rows 1, session state 0, marks 0, open writes 0; first: row 1 (table object 1): field 1); across test \
                 codeunits: 1 compared, 1 differ (not comparable 0, rows 1, session state 0, marks 0, open writes 0; first: row 1 (table object 1): field 1)"
            )
        );
    }

    // The report never prints a value (the rows are the company's data, issue 8211): a row, a session state line, a value
    // with no comparable form and the open writes that hold a planted value are named by where they differ only, in the
    // check's line and in each difference the image line takes its first from.
    #[test]
    fn the_difference_text_names_where_and_never_a_value() {
        let secret = |s: &str| Value::Text(format!("SECRET-VALUE-{s}").as_str().into());
        let a = state_of(
            &[(15, vec![Value::Integer(1), secret("123")])],
            &["session SECRET-VALUE-123"],
        );
        let mut b = state_of(
            &[(15, vec![Value::Integer(1), secret("124")])],
            &["session SECRET-VALUE-124"],
        );
        (b.state.opaque).push(not_comparable("codeunit 5 global 1", "Text 'SECRET-VALUE-123'"));
        b.open = "SECRET-VALUE-123".into();
        let check = Check::default();
        check.note(key(1), a.clone());
        check.note(key(1), b.clone());
        let line = check.line().unwrap();
        assert!(!line.contains("SECRET"), "{line}");
        for (_, what) in differences(&a, &b) {
            assert!(!what.contains("SECRET"), "{what}");
        }
        let missing = state_of(&[], &["session SECRET-VALUE-123"]);
        for (_, what) in differences(&a, &missing).into_iter().chain(differences(&missing, &a)) {
            assert!(!what.contains("SECRET"), "{what}");
        }
    }

    /// A key whose parts are made of the rows `rows` ((company, table object, values)) and the state lines `lines`.
    fn key_with(table_rows: &[(&str, u32, Vec<Value>)], lines: &[&str]) -> Key {
        let tables: Vec<al_data::memory::ExportedTable> = (table_rows.iter())
            .map(|(company, object, row)| table(company, *object, vec![row.clone()]))
            .collect();
        key_of_tables(&tables, lines)
    }

    /// An exported table of `company` and table object `object` with the rows `rows`.
    fn table(company: &str, object: u32, rows: Vec<Vec<Value>>) -> al_data::memory::ExportedTable {
        al_data::memory::ExportedTable {
            company: company.to_string(),
            object,
            pk_storage: None,
            rows: rows.into_iter().map(|row| (Vec::new(), row)).collect(),
        }
    }

    /// A key whose `database` and parts are made of the exported tables `tables` and the state lines `lines`.
    fn key_of_tables(tables: &[al_data::memory::ExportedTable], lines: &[&str]) -> Key {
        let state = al_vm::StateLines {
            lines: lines.iter().map(|s| s.to_string()).collect(),
            opaque: Vec::new(),
        };
        let digests = key_parts::table_digests(tables);
        Key {
            database: key_parts::database_digest(&digests),
            session: digest(&state.lines),
            parts: key_parts::NotCompared(key_parts::KeyParts::of(digests, &state)),
            ..key(1)
        }
    }

    // The maintainer's review of this check (issue 8211): the parts are a detail of `database` and `session`, never a key
    // field of their own. A store may export a table it holds no row of; `rows` writes no line for it, so `database`
    // does not see it, and the key is the key of the same rows without that table: no new key, no entry with an empty
    // name on the keys line. When the keys differ in other rows, the table without rows is not named among them.
    #[test]
    fn a_table_without_rows_changes_no_key_and_is_never_named() {
        let row = || vec![vec![Value::Integer(1)]];
        let rows = [table("C", 15, row())];
        let with_empty = [table("C", 15, row()), table("C", 18, Vec::new())];
        assert_eq!(key_of_tables(&rows, &[]), key_of_tables(&with_empty, &[]));
        let check = Check::default();
        check.note(key_of_tables(&rows, &[]), state(&["r"], &["s"]));
        check.note(key_of_tables(&with_empty, &[]), state(&["r"], &["s"]));
        let line = check.keys_line().unwrap();
        assert_eq!(
            line,
            "login replay keys: 1 keys, 0 new after the first; each differs from its nearest earlier key in: -"
        );
        let other = [table("C", 15, vec![vec![Value::Integer(2)]])];
        assert_eq!(
            key_of_tables(&other, &[]).differing_parts(&key_of_tables(&with_empty, &[]), &mut Default::default()),
            [("database", "company 1/table object 15".to_string())]
        );
    }

    // Issue 8211: a new key names the session state lines and the database tables it differs in from its nearest
    // earlier key, by name only. A planted value in a row, in a company name and in a state line never appears.
    #[test]
    fn a_new_key_names_the_session_lines_and_tables_it_differs_in_and_no_value() {
        let secret = |s: &str| Value::Text(format!("SECRET-VALUE-{s}").as_str().into());
        let lines = |n: &str| {
            [
                "session SECRET-VALUE-0".to_string(),
                format!("counters clock {n} SECRET-VALUE-1"),
            ]
        };
        let (a, b) = (lines("1"), lines("2"));
        let check = Check::default();
        let first = key_with(
            &[
                ("SECRET-CO", 15, vec![secret("1")]),
                ("SECRET-CO", 18, vec![secret("2")]),
            ],
            &a.iter().map(String::as_str).collect::<Vec<_>>(),
        );
        let second = key_with(
            &[
                ("SECRET-CO", 15, vec![secret("3")]),
                ("SECRET-CO", 18, vec![secret("2")]),
            ],
            &b.iter().map(String::as_str).collect::<Vec<_>>(),
        );
        check.note(first, state(&["r"], &["s"]));
        check.note(second, state(&["r"], &["s"]));
        let line = check.keys_line().unwrap();
        assert_eq!(
            line,
            "login replay keys: 2 keys, 1 new after the first; each differs from its nearest earlier key in: \
             database+session 1 (database: company 1/table object 15 1; session: counters 1)"
        );
        assert!(!line.contains("SECRET"), "{line}");
    }

    // The maintainer's review of this check (issue 8211): the keys line counts the new keys per set of key fields, and the
    // parts they differ in apart, per field, the most counted first and at most `PARTS_SHOWN` of them, so that an entry
    // stays short on a run with many keys.
    #[test]
    fn the_keys_line_counts_per_field_set_and_the_parts_apart() {
        let tables = |changed: &[u32]| -> Vec<al_data::memory::ExportedTable> {
            (1..=7)
                .map(|t| table("C", t, vec![vec![Value::Integer(changed.contains(&t).into())]]))
                .collect()
        };
        let check = Check::default();
        let s = state(&["r"], &["s"]);
        check.note(key_of_tables(&tables(&[]), &[]), s.clone());
        check.note(key_of_tables(&tables(&[1, 2]), &[]), s.clone());
        for t in 1..=7 {
            check.note(key_of_tables(&tables(&[t]), &[]), s.clone());
        }
        assert_eq!(
            check.keys_line().as_deref(),
            Some(
                "login replay keys: 9 keys, 8 new after the first; each differs from its nearest earlier key in: database 8 \
                 (database: company 1/table object 1 2, company 1/table object 2 2, company 1/table object 3 1, \
                 company 1/table object 4 1, company 1/table object 5 1, and 2 more)"
            )
        );
    }

    // The maintainer's review of this check (issue 8211): a company is numbered once over the whole check, by first sight,
    // so one number means one company in every entry; a table that is not per company is `global`; a table is a `table
    // object` as in the check line.
    #[test]
    fn the_keys_line_numbers_a_company_once_and_names_a_global_table() {
        let tables = |changed: (&str, u32)| -> Vec<al_data::memory::ExportedTable> {
            [("A", 15), ("B", 15), ("", 18)]
                .into_iter()
                .map(|(c, t)| table(c, t, vec![vec![Value::Integer(((c, t) == changed).into())]]))
                .collect()
        };
        let check = Check::default();
        let s = state(&["r"], &["s"]);
        check.note(key_of_tables(&tables(("", 0)), &[]), s.clone());
        for changed in [("B", 15), ("A", 15), ("", 18)] {
            check.note(key_of_tables(&tables(changed), &[]), s.clone());
        }
        assert_eq!(
            check.keys_line().as_deref(),
            Some(
                "login replay keys: 4 keys, 3 new after the first; each differs from its nearest earlier key in: database 3 \
                 (database: company 1/table object 15 1, company 2/table object 15 1, global/table object 18 1)"
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
        b.state
            .opaque
            .push(not_comparable("codeunit 5 global 1", "Codeunit instance"));
        check.note(key(1), a);
        check.note(key(1), b);
        let line = check.line().unwrap();
        assert!(
            line.contains(
                "1 compared, 1 differ (not comparable 1, rows 1, session state 0, marks 0, open writes 0; first: not comparable: \
                 codeunit 5 global 1)"
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
