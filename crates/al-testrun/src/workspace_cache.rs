//! The workspace layer of the compile cache (ADR-022 amendment 2026-11-15, Proposed; prototype): the compiled workspace
//! apps, stored on top of the dependency layers so that a run whose workspace did not change (a restarted `serve`
//! daemon, an `al-runner test` rerun) reads its compile instead of parsing, checking and lowering it again.
//!
//! This module owns the key ([`WorkspaceKey`], decision 2), the list of the environment switches the compile reads
//! ([`COMPILE_ENV`]), what the entry holds besides the blocks ([`WorkspaceExtra`], decision 3), the report, the canonical
//! dump that compares a read compile with a fresh one ("Equality requirement"), and the writes that run off the critical
//! path (decision 6). The read and the store over a compile are `runtime/workspace_layer.rs`; the format is
//! `al_cache::build_workspace`; the files are `compile_cache.rs` and `cache_gc.rs`.

use crate::runtime::{Compiled, Manifest};
use al_sema::workspace::AppSource;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::time::Duration;

/// Turns the workspace layer off, the dependency layers stay (decision 7). Set, not empty and not `0`.
pub const SWITCH: &str = "ALRUNNER_NO_WORKSPACE_CACHE";

/// How a run uses the workspace layer (`RunOptions::workspace_cache`; `None` there: off).
#[derive(Debug, Clone, Default)]
pub struct WorkspaceCacheOptions {
    /// The options the compile depends on, as text: `RunInputs::compile_text` of the command line (decision 2.6).
    pub compile_options: String,
    /// Per app as the caller numbers them: what its app.json says that the manifest does not (`platform`, `application`,
    /// its `preprocessorSymbols`), as text. Missing: empty.
    pub app_json: Vec<String>,
    /// Write the entry before the compile returns instead of after the run (tests, and the measurement of the encode
    /// time).
    pub write_now: bool,
    /// Say on stderr why an entry was refused and removed (`--verbose`; decision 10).
    pub verbose: bool,
}

/// What the workspace layer did for a compile.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WorkspaceState {
    /// Not used: switched off, or a compile the layer does not cover (the note says which).
    #[default]
    Off,
    /// No entry was found (or one was refused) and none is written for this compile.
    Miss,
    /// No entry was found (or one was refused): compiled, and the entry is written.
    Built,
    /// The compile was read from the entry.
    Read,
}

impl WorkspaceState {
    /// The name in `--timings` and in serve's `workspaceCache`.
    pub fn as_str(self) -> &'static str {
        match self {
            WorkspaceState::Off => "off",
            WorkspaceState::Miss => "miss",
            WorkspaceState::Built => "built",
            WorkspaceState::Read => "read",
        }
    }
}

/// What the workspace layer did for a compile (`Phases::workspace_cache`).
#[derive(Debug, Clone, Default)]
pub struct WorkspaceReport {
    pub state: WorkspaceState,
    /// The entry file.
    pub file: Option<PathBuf>,
    /// Its size (read or written).
    pub bytes: usize,
    /// Making the key, reading and checking the entry and making the compile from it.
    pub open: Duration,
    /// Taking the parts of a fresh compile (on the run's thread) and encoding them (on the writer's thread, when
    /// `write_now`).
    pub store: Duration,
    /// Why an entry was not used or not written.
    pub notes: Vec<String>,
}

/// How the compile uses an environment variable it reads (decision 2.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvUse {
    /// It changes what the compile produces: its value is in the key.
    Keyed,
    /// The workspace layer is not used when it is set.
    Off,
    /// It does not change what the compile produces; the reason.
    NotAnOutput(&'static str),
}

/// Every environment variable the compile reads (the compile crates and the compile path of al-testrun), with how the
/// workspace layer treats it. A test (`env_scan_tests.rs`) scans al-testrun, al-version, al-deps and the compile crates
/// and fails on a variable named neither here nor in its list of the run phase's and the tools' variables.
pub const COMPILE_ENV: &[(&str, EnvUse)] = &[
    ("AL_UNPINNED_BASE_ACCESS", EnvUse::Keyed),
    ("AL_NO_DEP_POSITIONS", EnvUse::Keyed),
    ("AL_NO_BASELINE", EnvUse::Keyed),
    ("AL_FORCE_BASELINE", EnvUse::Keyed),
    ("AL_EAGER_DEPENDENCIES", EnvUse::Off),
    // A self-check of the lowering, and the report file of the demand: a read lowers nothing, so it would skip them.
    ("AL_CHECK_HEADS", EnvUse::Off),
    ("AL_CHECK_LINK", EnvUse::Off),
    ("AL_DUMP_REACH", EnvUse::Off),
    (
        "AL_CHECK_THREADS",
        EnvUse::NotAnOutput("a thread count; the output does not depend on it"),
    ),
    (
        "AL_LOWER_THREADS",
        EnvUse::NotAnOutput("a thread count; the output does not depend on it"),
    ),
    (
        "ALRUNNER_INCR_TIMINGS",
        EnvUse::NotAnOutput("timing lines of the incremental compile on stderr"),
    ),
    (
        "ALRUNNER_EAGER_DECODE",
        EnvUse::NotAnOutput("decodes the cached objects at start instead of on first touch"),
    ),
    ("AL_REACH_REPORT", EnvUse::NotAnOutput("a report on stderr")),
    (
        "ALRUNNER_LAYER_BY_LAYER",
        EnvUse::NotAnOutput("how the dependency layers are built, the same bytes"),
    ),
];

/// The values of the [`EnvUse::Keyed`] switches, as the key holds them: whether each is set, and its bytes (an empty
/// value is not an unset one: `base_access.rs` reads an empty `AL_UNPINNED_BASE_ACCESS` as "pin nothing").
pub fn env_values() -> Vec<(String, Option<Vec<u8>>)> {
    env_values_with(&|name| std::env::var_os(name))
}

/// [`env_values`] with the environment read through `get`.
pub fn env_values_with(get: &dyn Fn(&str) -> Option<std::ffi::OsString>) -> Vec<(String, Option<Vec<u8>>)> {
    COMPILE_ENV
        .iter()
        .filter(|(_, u)| *u == EnvUse::Keyed)
        .map(|(name, _)| (name.to_string(), get(name).map(|v| v.into_encoded_bytes())))
        .collect()
}

/// Whether a switch of [`EnvUse::Off`] is set, or [`SWITCH`].
pub fn switched_off() -> Option<String> {
    if crate::switches::on(SWITCH) {
        return Some(format!("{SWITCH} is set"));
    }
    COMPILE_ENV
        .iter()
        .filter(|(_, u)| *u == EnvUse::Off)
        .find(|(name, _)| std::env::var_os(name).is_some())
        .map(|(name, _)| format!("{name} is set"))
}

/// The format integer of the workspace layer as the key holds it (decision 2.1).
pub const KEY_FORMAT: u32 = al_cache::WORKSPACE_FORMAT;

/// One app of the workspace layer in the key (decision 2.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceAppKey {
    pub id: String,
    pub name: String,
    pub dependencies: Vec<String>,
    /// What the app.json says that the manifest does not (`WorkspaceCacheOptions::app_json`).
    pub app_json: String,
    /// `baseline_key::manifest_digest`: every manifest field but the resources (issue 5635), destructured there.
    pub manifest: [u8; 32],
    /// The paths of the files, in order.
    pub paths: [u8; 32],
    /// `compile_cache::source_hash` (every path and decoded text), or the identity of a lazily read package.
    pub source: [u8; 32],
    /// Review LOW 2: a workspace app or a dependency app that stands on one (both are in this layer).
    pub kind: al_sema::AppKind,
}

/// The key of a workspace layer entry (decision 2). Every field is hashed by [`WorkspaceKey::parts`], which destructures
/// it without `..`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceKey {
    /// 2.1: the format of the layer.
    pub format: u32,
    /// 2.1: `compile_cache::runtime_build`.
    pub runtime_build: [u8; 32],
    /// 2.2: the digest of the top dependency layer's key; zeros without one.
    pub below: [u8; 32],
    /// 2.3: the apps of the layer, in program order.
    pub apps: Vec<WorkspaceAppKey>,
    /// 2.4: the run target (BC version and runtime), as text.
    pub target: String,
    /// 2.5: the project's preprocessor symbols, sorted.
    pub symbols: Vec<String>,
    /// 2.6: `RunInputs::compile_text`.
    pub compile_options: String,
    /// 2.7: the roots of the compile ([`roots_text`]).
    pub roots: String,
    /// 2.8: the values of the keyed environment switches ([`env_values`]).
    pub env: Vec<(String, Option<Vec<u8>>)>,
    /// Not in the ADR text (prototype choice): what decides the compile's path in al-testrun besides the above (the
    /// lean selection, the weighing of the install baseline, `--require-compile-cache`, the suites, the program order).
    pub path: String,
    /// Not in the ADR text (review MED 3): the ids of the apps the platform package grants its internals to
    /// (`internals_grant::granted_app_ids`, sorted), read from every manifest, a dependency's too.
    pub granted: Vec<String>,
}

use al_cache::digest::{hex, put, sha};

impl WorkspaceKey {
    /// The names of the fields, for the test that each has a miss test.
    pub const FIELDS: [&'static str; 11] = [
        "format",
        "runtime_build",
        "below",
        "apps",
        "target",
        "symbols",
        "compile_options",
        "roots",
        "env",
        "path",
        "granted",
    ];

    /// The key's fields as named bytes; with `texts` false, the apps' source hashes are left out (`run::warm_key`, the
    /// kept compile of serve, takes a changed text in as an edit).
    pub fn parts(&self, texts: bool) -> Vec<(&'static str, Vec<u8>)> {
        let WorkspaceKey {
            format,
            runtime_build,
            below,
            apps,
            target,
            symbols,
            compile_options,
            roots,
            env,
            path,
            granted,
        } = self;
        let mut a = Sha256::new();
        put(&mut a, &(apps.len() as u64).to_le_bytes());
        for app in apps {
            let WorkspaceAppKey {
                id,
                name,
                dependencies,
                app_json,
                manifest,
                paths,
                source,
                kind,
            } = app;
            put(&mut a, id.as_bytes());
            put(&mut a, name.as_bytes());
            put(&mut a, format!("{dependencies:?}").as_bytes());
            put(&mut a, app_json.as_bytes());
            put(&mut a, manifest);
            put(&mut a, paths);
            put(&mut a, if texts { source } else { &[] });
            put(&mut a, format!("{kind:?}").as_bytes());
        }
        vec![
            ("format", format.to_le_bytes().to_vec()),
            ("runtime_build", runtime_build.to_vec()),
            ("below", below.to_vec()),
            ("apps", a.finalize().to_vec()),
            ("target", target.as_bytes().to_vec()),
            ("symbols", format!("{symbols:?}").into_bytes()),
            ("compile_options", compile_options.as_bytes().to_vec()),
            ("roots", roots.as_bytes().to_vec()),
            ("env", format!("{env:?}").into_bytes()),
            ("path", path.as_bytes().to_vec()),
            ("granted", format!("{granted:?}").into_bytes()),
        ]
    }

    /// The digest of the key, over every field.
    pub fn digest(&self) -> [u8; 32] {
        Self::digest_of(&self.parts(true))
    }

    /// The digest without the texts of the apps' files (see [`WorkspaceKey::parts`]).
    pub fn digest_without_texts(&self) -> [u8; 32] {
        Self::digest_of(&self.parts(false))
    }

    fn digest_of(parts: &[(&'static str, Vec<u8>)]) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(b"al-runtime workspace layer key");
        for (name, value) in parts {
            put(&mut h, name.as_bytes());
            put(&mut h, value);
        }
        h.finalize().into()
    }

    /// Each field's digest by name, as the entry stores it to compare field by field (decision 10).
    pub fn field_digests(&self) -> Vec<(String, [u8; 32])> {
        self.parts(true)
            .into_iter()
            .map(|(name, value)| (name.to_string(), sha(&value)))
            .collect()
    }

    /// What groups the entries of one project (decision 8): the dependency layer below and the ids of the apps.
    pub fn identity(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        put(&mut h, &self.below);
        for a in &self.apps {
            put(&mut h, a.id.to_ascii_lowercase().as_bytes());
        }
        h.finalize().into()
    }

    /// `<tag>-ws-<identity 16 hex>-<key 32 hex>.alcw` (decision 8).
    pub fn file_name(&self) -> String {
        format!(
            "{}-ws-{}-{}.{}",
            al_cache::build_tag(&self.runtime_build),
            &hex(&self.identity())[..16],
            &hex(&self.digest())[..32],
            crate::compile_cache::WORKSPACE_EXTENSION
        )
    }
}

/// An app of the layer in the key. `identity`: that of a lazily read package, which stands for its source.
pub fn app_key(
    app: &AppSource,
    kind: al_sema::AppKind,
    manifest: &Manifest,
    app_json: &str,
    identity: Option<[u8; 32]>,
) -> WorkspaceAppKey {
    let AppSource {
        id,
        name,
        dependencies,
        files,
    } = app;
    let mut p = Sha256::new();
    put(&mut p, &(files.len() as u64).to_le_bytes());
    for (path, _) in files {
        put(&mut p, path.as_bytes());
    }
    WorkspaceAppKey {
        id: id.clone(),
        name: name.clone(),
        dependencies: dependencies.clone(),
        app_json: app_json.to_string(),
        manifest: crate::runtime::baseline_key::manifest_digest(manifest),
        paths: p.finalize().into(),
        source: identity.unwrap_or_else(|| crate::compile_cache::source_hash(app)),
        kind,
    }
}

/// The roots of a compile as the key holds them (decision 2.7): every field but `workers`, destructured without `..`.
pub fn roots_text(r: &al_sema::check::Roots) -> String {
    let al_sema::check::Roots {
        baseline,
        company_data,
        company_methods,
        only_tests,
        reach_only,
        lean,
        no_login_root,
        eager,
        // The thread count: the output does not depend on it (the equality test runs with two counts).
        workers: _,
        provided,
        provided_queries,
        only_objects,
        skipped_before,
        tdd,
        tdd_id_ranges,
        object_hashes,
        app_contexts,
        keep_demand,
        test_runners,
    } = r;
    format!(
        "baseline {baseline} company_data {company_data:?} company_methods {company_methods:?} only_tests {only_tests:?} \
         reach_only {reach_only} lean {lean} no_login_root {no_login_root} eager {eager} provided {provided:?} \
         provided_queries {provided_queries:?} only_objects {only_objects:?} skipped_before {skipped_before:?} tdd {tdd} \
         tdd_id_ranges {tdd_id_ranges:?} object_hashes {object_hashes} app_contexts {app_contexts:?} \
         keep_demand {keep_demand} test_runners {test_runners:?}"
    )
}

/// The digest of the dependency apps as the key of serve's kept compile holds them (`run::warm_key`): each app's id,
/// file count and identity (a lazily read package) or file names and lengths, and the summary lines.
pub fn dependency_digest(apps: &[AppSource], lazy: &[Option<al_sema::LazySource>], summary: &[String]) -> [u8; 32] {
    let mut h = Sha256::new();
    for (i, app) in apps.iter().enumerate() {
        put(&mut h, app.id.as_bytes());
        put(&mut h, &(app.files.len() as u64).to_le_bytes());
        match lazy.get(i).and_then(Option::as_ref) {
            Some(l) => put(&mut h, &l.identity),
            None => {
                for (path, text) in &app.files {
                    put(&mut h, path.as_bytes());
                    put(&mut h, &(text.len() as u64).to_le_bytes());
                }
            }
        }
    }
    put(&mut h, format!("{summary:?}").as_bytes());
    h.finalize().into()
}

/// The key of serve's kept compile (`run::warm_key`, ADR-028): the [`WorkspaceKey`] of the workspace apps without the
/// texts of their files, over `dependencies` ([`dependency_digest`]) in place of the layer below, without the roots and
/// the path (the kept compile is made without a selection). One owner of the field list for both keys.
pub fn warm_key(
    apps: &[(&AppSource, &Manifest, &str)],
    dependencies: [u8; 32],
    symbols: &std::collections::BTreeSet<String>,
    target: &str,
    compile_options: &str,
) -> String {
    let key = WorkspaceKey {
        format: KEY_FORMAT,
        runtime_build: crate::compile_cache::runtime_build(),
        below: dependencies,
        apps: apps
            .iter()
            // The texts are left out of the digest (the source is not hashed); the paths are not.
            .map(|(a, m, json)| app_key(a, al_sema::AppKind::Workspace, m, json, Some([0; 32])))
            .collect(),
        target: target.to_string(),
        symbols: symbols.iter().cloned().collect(),
        compile_options: compile_options.to_string(),
        roots: String::new(),
        env: env_values(),
        path: String::new(),
        // The grants come from the dependency packages, whose identities `dependencies` holds.
        granted: Vec::new(),
    };
    hex(&key.digest_without_texts()[..8])
}

/// What a compile decided beside its program, which a read compile takes as the fresh one decided it.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Decided {
    pub baseline: bool,
    pub baseline_note: Option<String>,
    /// The compile of the weighing is the compile: the selected tests are free of data, compiled without the dependency
    /// layers (`Compiled::compile_cache_skipped`).
    pub compile_cache_skipped: bool,
}

/// What the entry holds besides the blocks (decision 3, `WorkspaceExtra`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceExtra {
    /// [`WorkspaceKey::field_digests`] of the key the entry was made for.
    pub key: Vec<(String, [u8; 32])>,
    /// How many objects the dependency layers below hold.
    pub base_len: u64,
    pub lowered: al_sema::lower::stored::StoredLowered,
    pub workspace: al_sema::StoredWorkspace,
    pub decided: Decided,
}

/// A write of an entry that waits for the end of the run (decision 6).
type Job = Box<dyn FnOnce() + Send>;

/// The writes this process deferred, in the order the compiles made them.
static DEFERRED: std::sync::Mutex<Vec<Job>> = std::sync::Mutex::new(Vec::new());
/// The writer threads [`start_writes`] started and nothing joined yet.
static WRITERS: std::sync::Mutex<Vec<std::thread::JoinHandle<()>>> = std::sync::Mutex::new(Vec::new());

/// Defers `write` to the end of the run (decision 6): the encode and the write wait until the results are printed
/// (`al-runner test`, [`wait_for_writes`]) or the response is sent (serve, [`start_writes`]), so they do not share the
/// cores with the tests.
pub(crate) fn defer_write(write: impl FnOnce() + Send + 'static) {
    DEFERRED.lock().unwrap_or_else(|e| e.into_inner()).push(Box::new(write));
}

/// Starts the deferred writes on a thread of their own, in order (serve, after the response of a request is sent), and
/// joins the writer threads that finished (review LOW 6: the list does not grow with the requests).
pub fn start_writes() {
    let jobs: Vec<Job> = std::mem::take(&mut *DEFERRED.lock().unwrap_or_else(|e| e.into_inner()));
    let mut writers = WRITERS.lock().unwrap_or_else(|e| e.into_inner());
    let (done, running): (Vec<_>, Vec<_>) = std::mem::take(&mut *writers).into_iter().partition(|h| h.is_finished());
    for h in done {
        let _ = h.join();
    }
    *writers = running;
    if jobs.is_empty() {
        return;
    }
    let handle = std::thread::Builder::new()
        .name("workspace-cache-write".into())
        .spawn(move || jobs.into_iter().for_each(|job| job()));
    if let Ok(h) = handle {
        writers.push(h);
    }
}

/// Runs the deferred writes and waits for every write of this process (`al-runner test` before it exits, after the
/// results are printed; serve when it shuts down). A write that did not finish leaves no file (temporary file, rename).
pub fn wait_for_writes() {
    let jobs: Vec<Job> = std::mem::take(&mut *DEFERRED.lock().unwrap_or_else(|e| e.into_inner()));
    jobs.into_iter().for_each(|job| job());
    let writers: Vec<_> = std::mem::take(&mut *WRITERS.lock().unwrap_or_else(|e| e.into_inner()));
    for h in writers {
        let _ = h.join();
    }
}

/// The canonical dump of a compile ("Equality requirement"): the program with every body decoded, every field of
/// `Lowered` but `timings`, `work` and `cached_objects`, the workspace after `drop_trees` with its derived tables, and
/// the fields of `Compiled` but the timings and the cache reports. Destructured without `..`.
pub fn canonical_dump(c: &Compiled) -> String {
    use std::fmt::Write as _;
    let Compiled {
        ws,
        lowered,
        // The cache reports: what the caches did, which is what differs between a read and a fresh compile.
        cache_report: _,
        workspace_cache: _,
        baseline_wanted,
        baseline,
        baseline_note,
        order,
        suites,
        opts_only,
        compile_cache_skipped,
        source_index,
        // A measurement.
        compile_time: _,
        lean,
        // How many compiles the run took: a measurement of work (a read compiles nothing), like the timings.
        compiles: _,
        incr,
        // The engines a warm server keeps after a run: none after a compile.
        kept: _,
        grow_kept,
        uncovered,
    } = c;
    let mut out = ws.canonical_dump();
    out.push_str(&al_sema::lower::stored::canonical_dump(lowered));
    let _ = writeln!(
        out,
        "compiled baseline_wanted {baseline_wanted} baseline {baseline} note {baseline_note:?} order {order:?} \
         suites {suites:?} only {opts_only:?} skipped {compile_cache_skipped} source_index {source_index:?} lean {lean:?} \
         grow_kept {} uncovered {uncovered:?}",
        grow_kept.is_some()
    );
    let _ = writeln!(
        out,
        "incr {:?}",
        incr.as_ref()
            .map(|i| (roots_text(&i.roots), i.try_skip, &i.symbols, i.layers.top_digest()))
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> WorkspaceKey {
        WorkspaceKey {
            format: KEY_FORMAT,
            runtime_build: [1; 32],
            below: [2; 32],
            apps: vec![WorkspaceAppKey {
                id: "a".into(),
                name: "App".into(),
                dependencies: vec!["b".into()],
                app_json: "platform 27".into(),
                manifest: [3; 32],
                paths: [4; 32],
                source: [5; 32],
                kind: al_sema::AppKind::Workspace,
            }],
            target: "BC 27.5 runtime 16.0".into(),
            symbols: vec!["CLEAN27".into()],
            compile_options: "tdd=false".into(),
            roots: "baseline true".into(),
            env: vec![("AL_NO_BASELINE".into(), None)],
            path: "lean false".into(),
            granted: Vec::new(),
        }
    }

    /// Review MED 1: a switch that makes the compile check itself or write a report does something a read skips, so it
    /// turns the layer off.
    #[test]
    fn the_compile_self_checks_and_reports_turn_the_layer_off() {
        for name in [
            "AL_CHECK_HEADS",
            "AL_CHECK_LINK",
            "AL_DUMP_REACH",
            "AL_EAGER_DEPENDENCIES",
        ] {
            let used = COMPILE_ENV.iter().find(|(n, _)| *n == name).map(|(_, u)| *u);
            assert_eq!(used, Some(EnvUse::Off), "{name}");
        }
    }
    /// Decision 2.8 (review HIGH 1): a keyed switch that is set but empty is another key than one that is not set
    /// (`base_access.rs` reads an empty `AL_UNPINNED_BASE_ACCESS` as "pin nothing"), for every keyed switch.
    #[test]
    fn every_keyed_switch_misses_between_unset_and_set_but_empty() {
        let keyed: Vec<&str> = COMPILE_ENV
            .iter()
            .filter(|(_, u)| *u == EnvUse::Keyed)
            .map(|(n, _)| *n)
            .collect();
        assert!(keyed.contains(&"AL_UNPINNED_BASE_ACCESS"));
        let with = |env| WorkspaceKey { env, ..sample() };
        let unset = with(env_values_with(&|_| None));
        for name in keyed {
            let empty = with(env_values_with(&|n| (n == name).then(std::ffi::OsString::new)));
            assert_ne!(unset.digest(), empty.digest(), "{name}: unset and set but empty");
            let one = with(env_values_with(&|n| (n == name).then(|| "1".into())));
            assert_ne!(one.digest(), empty.digest(), "{name}: set to 1 and set but empty");
        }
    }
    /// Decision 2: every field of the key misses when it changes, and the list of changes names every field.
    #[test]
    fn every_field_of_the_key_misses_when_it_changes() {
        type Change = fn(&mut WorkspaceKey);
        let changes: Vec<(&str, Change)> = vec![
            ("format", |k| k.format += 1),
            ("runtime_build", |k| k.runtime_build[0] ^= 1),
            ("below", |k| k.below[31] ^= 1),
            ("apps", |k| k.apps[0].source[0] ^= 1),
            ("target", |k| k.target = "BC 28.0 runtime 17.0".into()),
            ("symbols", |k| k.symbols.push("CLEAN28".into())),
            ("compile_options", |k| k.compile_options = "tdd=true".into()),
            ("roots", |k| k.roots = "baseline false".into()),
            ("env", |k| k.env[0].1 = Some(Vec::new())),
            ("path", |k| k.path = "lean true".into()),
            ("granted", |k| k.granted.push("main".into())),
        ];
        let names: Vec<&str> = changes.iter().map(|(n, _)| *n).collect();
        assert_eq!(names, WorkspaceKey::FIELDS, "one change per field of the key");
        let parts: Vec<&str> = sample().parts(true).into_iter().map(|(n, _)| n).collect();
        assert_eq!(parts, WorkspaceKey::FIELDS, "the digest hashes every field");
        let base = sample();
        for (name, change) in changes {
            let mut k = sample();
            change(&mut k);
            assert_ne!(k.digest(), base.digest(), "{name}");
            assert_ne!(k.file_name(), base.file_name(), "{name}");
            let differs: Vec<String> = k
                .field_digests()
                .into_iter()
                .zip(base.field_digests())
                .filter(|(a, b)| a != b)
                .map(|(a, _)| a.0)
                .collect();
            assert_eq!(
                differs,
                vec![name.to_string()],
                "the stored fields name the one that changed"
            );
        }
    }

    /// Every field of an app of the key misses; the texts are left out of `digest_without_texts` (serve's kept compile)
    /// and nothing else is.
    #[test]
    fn every_field_of_an_app_misses_and_only_the_text_is_left_out_of_the_warm_digest() {
        type Change = fn(&mut WorkspaceAppKey);
        let changes: Vec<(&str, Change)> = vec![
            ("id", |a| a.id = "c".into()),
            ("name", |a| a.name = "Other".into()),
            ("dependencies", |a| a.dependencies.clear()),
            ("app_json", |a| a.app_json = "platform 28".into()),
            ("manifest", |a| a.manifest[0] ^= 1),
            ("paths", |a| a.paths[0] ^= 1),
            ("source", |a| a.source[0] ^= 1),
            ("kind", |a| a.kind = al_sema::AppKind::Dependency),
        ];
        let base = sample();
        for (name, change) in changes {
            let mut k = sample();
            change(&mut k.apps[0]);
            assert_ne!(k.digest(), base.digest(), "{name}");
            assert_eq!(
                k.digest_without_texts() == base.digest_without_texts(),
                name == "source",
                "{name}"
            );
        }
    }

    /// Decision 2.7: every field of `Roots` but `workers` is in the key.
    #[test]
    fn every_field_of_the_roots_but_the_thread_count_is_in_the_key() {
        use al_sema::check::Roots;
        type Change = fn(&mut Roots);
        let changes: Vec<(&str, Change)> = vec![
            ("baseline", |r| r.baseline = true),
            ("company_data", |r| r.company_data = vec![1]),
            ("company_methods", |r| r.company_methods = vec![(1, "M".into())]),
            ("only_tests", |r| r.only_tests = Some(vec![(0, 1)])),
            ("reach_only", |r| r.reach_only = true),
            ("lean", |r| r.lean = true),
            ("no_login_root", |r| r.no_login_root = true),
            ("eager", |r| r.eager = true),
            ("provided", |r| r.provided = vec![true]),
            ("provided_queries", |r| r.provided_queries = vec![None]),
            ("only_objects", |r| r.only_objects = Some(vec![1])),
            ("skipped_before", |r| r.skipped_before = Some(vec![true])),
            ("tdd", |r| r.tdd = true),
            ("tdd_id_ranges", |r| {
                r.tdd_id_ranges.insert("a".into(), vec![(1, 2)]);
            }),
            ("object_hashes", |r| r.object_hashes = true),
            ("app_contexts", |r| {
                r.app_contexts = vec![(
                    "a".into(),
                    al_sema::alcodes::AppContext {
                        target: al_sema::alcodes::AppTarget::Cloud,
                        runtime: None,
                        app_json_keys: Vec::new(),
                        bc_major: None,
                    },
                )]
            }),
            ("keep_demand", |r| r.keep_demand = true),
            ("test_runners", |r| r.test_runners = vec![130450]),
        ];
        let base = roots_text(&Roots::default());
        for (name, change) in &changes {
            let mut r = Roots::default();
            change(&mut r);
            assert_ne!(roots_text(&r), base, "{name}");
            assert!(roots_text(&r).contains(name), "the text names {name}");
        }
        let workers = Roots {
            workers: Some(1),
            ..Default::default()
        };
        assert_eq!(roots_text(&workers), base, "the thread count is not in the key");
        // `roots_text` destructures `Roots` without `..`: a field added there does not compile until it is decided, and
        // its miss test goes in the list above (the count is the fields but `workers`).
        assert_eq!(
            changes.len(),
            roots_text(&Roots::default()).matches(' ').count() / 2 + 1
        );
    }
}

/// Decision 2.8: the environment the compile reads (review MED 2).
#[cfg(test)]
mod env_scan_tests;
