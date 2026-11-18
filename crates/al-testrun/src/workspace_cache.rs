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
    /// Write the entry before the compile returns instead of on a background thread (tests, and the measurement of the
    /// encode time).
    pub write_now: bool,
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

/// Every environment variable the compile crates read (al-sema, al-syntax, al-ir, al-link, al-builtins, al-cache) and the
/// compile path of al-testrun reads, with how the workspace layer treats it. A test greps those sources and fails on a
/// read this list does not name.
pub const COMPILE_ENV: &[(&str, EnvUse)] = &[
    ("AL_UNPINNED_BASE_ACCESS", EnvUse::Keyed),
    ("AL_NO_DEP_POSITIONS", EnvUse::Keyed),
    ("AL_NO_BASELINE", EnvUse::Keyed),
    ("AL_FORCE_BASELINE", EnvUse::Keyed),
    ("AL_EAGER_DEPENDENCIES", EnvUse::Off),
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
        "AL_CHECK_HEADS",
        EnvUse::NotAnOutput("a self-check that panics on a difference"),
    ),
    (
        "AL_CHECK_LINK",
        EnvUse::NotAnOutput("a self-check that fails the compile on a difference"),
    ),
    (
        "ALRUNNER_EAGER_DECODE",
        EnvUse::NotAnOutput("decodes the cached objects at start instead of on first touch"),
    ),
    (
        "AL_DUMP_REACH",
        EnvUse::NotAnOutput("writes a report file of the demand"),
    ),
    ("AL_REACH_REPORT", EnvUse::NotAnOutput("a report on stderr")),
    (
        "UPDATE_ASSUMPTIONS_DOC",
        EnvUse::NotAnOutput("a test of al-builtins that rewrites a document"),
    ),
    (
        "LAYER_BY_LAYER",
        EnvUse::NotAnOutput("ALRUNNER_LAYER_BY_LAYER: how the dependency layers are built, the same bytes"),
    ),
];

/// The values of the [`EnvUse::Keyed`] switches, as the key holds them.
pub fn env_values() -> Vec<(String, String)> {
    COMPILE_ENV
        .iter()
        .filter(|(_, u)| *u == EnvUse::Keyed)
        .map(|(name, _)| (name.to_string(), std::env::var(name).unwrap_or_default()))
        .collect()
}

/// Whether a switch of [`EnvUse::Off`] is set, or [`SWITCH`].
pub fn switched_off() -> Option<String> {
    let set = |name: &str| std::env::var(name).is_ok_and(|v| !v.is_empty() && v != "0");
    if set(SWITCH) {
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
    pub env: Vec<(String, String)>,
    /// Not in the ADR text (prototype choice): what decides the compile's path in al-testrun besides the above (the
    /// lean selection, the weighing of the install baseline, `--require-compile-cache`, the suites, the program order).
    pub path: String,
}

fn put(h: &mut Sha256, bytes: &[u8]) {
    h.update((bytes.len() as u64).to_le_bytes());
    h.update(bytes);
}

fn sha(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|b| format!("{b:02x}")).collect()
}

impl WorkspaceKey {
    /// The names of the fields, for the test that each has a miss test.
    pub const FIELDS: [&'static str; 10] = [
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
            } = app;
            put(&mut a, id.as_bytes());
            put(&mut a, name.as_bytes());
            put(&mut a, format!("{dependencies:?}").as_bytes());
            put(&mut a, app_json.as_bytes());
            put(&mut a, manifest);
            put(&mut a, paths);
            put(&mut a, if texts { source } else { &[] });
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
pub fn app_key(app: &AppSource, manifest: &Manifest, app_json: &str, identity: Option<[u8; 32]>) -> WorkspaceAppKey {
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
            .map(|(a, m, json)| app_key(a, m, json, Some([0; 32])))
            .collect(),
        target: target.to_string(),
        symbols: symbols.iter().cloned().collect(),
        compile_options: compile_options.to_string(),
        roots: String::new(),
        env: env_values(),
        path: String::new(),
    };
    hex(&key.digest_without_texts()[..8])
}

/// What a compile decided beside its program, which a read compile takes as the fresh one decided it.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Decided {
    pub baseline: bool,
    pub baseline_note: Option<String>,
    pub compiles: usize,
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

/// The writes of entries that run on a thread of their own (decision 6), for [`wait_for_writes`].
static PENDING: std::sync::Mutex<Vec<std::thread::JoinHandle<()>>> = std::sync::Mutex::new(Vec::new());

/// Runs `write` on a thread of its own, off the critical path.
pub(crate) fn spawn_write(write: impl FnOnce() + Send + 'static) {
    let handle = std::thread::Builder::new()
        .name("workspace-cache-write".into())
        .spawn(write);
    if let Ok(h) = handle {
        PENDING.lock().unwrap_or_else(|e| e.into_inner()).push(h);
    }
}

/// Waits for the writes of entries this process started (`al-runner test` before it exits, after the results are
/// printed; serve when it shuts down). A write that did not finish leaves no file (temporary file, rename).
pub fn wait_for_writes() {
    let pending: Vec<_> = std::mem::take(&mut *PENDING.lock().unwrap_or_else(|e| e.into_inner()));
    for h in pending {
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
        compiles,
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
         compiles {compiles} grow_kept {} uncovered {uncovered:?}",
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
