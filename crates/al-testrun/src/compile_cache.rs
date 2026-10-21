//! The compile cache of the dependency apps (ADR-022): which key a run has, where the entry file
//! lives, how it is read, built and written. The format is `al-cache`; this is the OS side.

use al_cache::{AppKey, Entry, Key};
use al_sema::provided::{DepLayer, DepSource, LayerSource, Stack};
use al_sema::workspace::AppSource;
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

/// How a run uses the compile cache.
#[derive(Debug, Clone)]
pub struct CompileCache {
    /// The folder of the entry files: `<cache root>/compile/`.
    pub dir: PathBuf,
    /// A miss is an error (CI: the measured run must be a hit).
    pub require: bool,
    /// `--rebuild-compile-cache`: no entry is read, every one this run needs is built and written anew (the
    /// layers, the source index and the install baseline). Other entries in the folder stay.
    pub rebuild: bool,
    /// The country of the run (`w1` or empty: the default). A country other than `w1` is part of every layer's key, so
    /// layers made for one country are never read for another, whatever the source hashes say (ADR-020 amendment).
    pub country: String,
}

impl CompileCache {
    /// The folder under the cache root the download cache already uses.
    pub fn under(root: &Path) -> PathBuf {
        root.join("compile")
    }
}

/// What the cache did for a run (`--timings`, the job summary).
#[derive(Debug, Clone, Default)]
pub struct CacheReport {
    /// The entry file of the top layer.
    pub file: PathBuf,
    /// Every layer of the dependency set, lowest (the Microsoft apps) first.
    pub layers: Vec<LayerReport>,
    /// Every layer was found and used as it was.
    pub hit: bool,
    /// At least one layer was built by this run (a miss, or a file that was refused).
    pub built: bool,
    /// Making the key and reading and opening the file.
    pub open: Duration,
    /// Building and writing the entry (zero on a hit).
    pub build: Duration,
    pub bytes: usize,
    /// Dependency objects lowered again because the workspace changes what they embed.
    pub relowered: usize,
    /// The dependency objects in the entry.
    pub objects: usize,
    /// Why a file was not used, or why the cache was not used at all.
    pub notes: Vec<String>,
    /// Dependency files that did not parse when the entry was built: (path, why).
    pub parse_problems: Vec<(String, String)>,
    /// With lazily read dependency apps: where the scan of their sources came from, `read` (the index
    /// in the cache folder) or `built` (scanned now, index saved).
    pub source_index: Option<&'static str>,
}

/// What the cache did for one layer.
#[derive(Debug, Clone, Default)]
pub struct LayerReport {
    pub file: PathBuf,
    /// The names of the layer's apps.
    pub apps: Vec<String>,
    pub hit: bool,
    pub built: bool,
    pub bytes: usize,
    /// Building and writing the layer (zero on a hit).
    pub build: Duration,
}

/// The layers of a dependency set, opened, lowest first.
pub struct Layers {
    entries: Vec<Entry>,
}

impl Layers {
    /// The layers as one source of lowered dependency objects. An error says they do not fit.
    pub fn stack(&self) -> Result<Stack<'_>, String> {
        Stack::new(self.entries.iter().map(|e| e as &dyn LayerSource).collect())
    }

    /// The number of dependency objects in all layers.
    pub fn len(&self) -> usize {
        self.stack().map_or(0, |s| s.len())
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Reads every block of every layer (CI, `--build-compile-cache`).
    pub fn verify(&self) -> Result<(), al_cache::CacheError> {
        self.entries.iter().try_for_each(Entry::verify)
    }

    /// The dependency files that did not parse when the layers were built.
    pub fn parse_problems(&self) -> Vec<(String, String)> {
        self.entries
            .iter()
            .flat_map(|e| e.parse_problems().iter().cloned())
            .collect()
    }
}

/// A hash of the running executable: it changes with every change of the compiler or of the
/// builtins catalog. It cannot miss a crate, which a hash of source files can.
pub fn runtime_build() -> [u8; 32] {
    static HASH: OnceLock<[u8; 32]> = OnceLock::new();
    *HASH.get_or_init(|| {
        let bytes = std::env::current_exe().and_then(std::fs::read);
        match bytes {
            Ok(b) => hash_build_bytes(&b),
            // Without the executable the key cannot say which compiler made an entry: a hash of
            // the time would only ever miss. Use a value no file can match.
            Err(e) => Sha256::digest(format!("no executable: {e}").as_bytes()).into(),
        }
    })
}

/// The chunk size of [`hash_build_bytes`].
pub const BUILD_HASH_CHUNK: usize = 1 << 20;

/// A hash of the bytes of an executable: SHA-256 over the length and the SHA-256 of each chunk, in
/// order. The chunks are hashed in parallel (one SHA-256 over the whole executable runs on a single core while the
/// others wait). It only has to change with every byte, the length and the
/// chunk order; it is a cache key, not a published digest.
pub fn hash_build_bytes(bytes: &[u8]) -> [u8; 32] {
    let per_chunk: Vec<[u8; 32]> = bytes
        .par_chunks(BUILD_HASH_CHUNK)
        .map(|c| Sha256::digest(c).into())
        .collect();
    let mut h = Sha256::new();
    h.update((bytes.len() as u64).to_le_bytes());
    for c in &per_chunk {
        h.update(c);
    }
    h.finalize().into()
}

/// A hash of the source of an app: every path and every byte.
pub fn source_hash(app: &AppSource) -> [u8; 32] {
    let per_file: Vec<[u8; 32]> = app
        .files
        .par_iter()
        .map(|(path, text)| {
            let mut h = Sha256::new();
            h.update((path.len() as u64).to_le_bytes());
            h.update(path.as_bytes());
            h.update((text.len() as u64).to_le_bytes());
            h.update(text.as_bytes());
            h.finalize().into()
        })
        .collect();
    let mut h = Sha256::new();
    h.update((per_file.len() as u64).to_le_bytes());
    for f in &per_file {
        h.update(f);
    }
    h.finalize().into()
}

/// The target as the key holds it. Compiling the dependency apps reads the target only through the
/// builtins catalog, which depends on the runtime version (`Catalog::for_target`); the BC version and
/// build do not change a compile (the build is in the apps' own versions and source hashes). Keeping
/// them out lets projects on one BC build whose app.json names another BC minor share a layer.
fn target_text(t: &al_version::Target) -> String {
    // Destructured without `..`: a field added to the target must be decided here to compile.
    // `app_runtimes` lists apps under test only; the cache holds dependency apps, which it never gates.
    let al_version::Target {
        bc: _,
        runtime,
        app_runtimes: _,
    } = t;
    let al_version::Runtime {
        major: rt_major,
        minor: rt_minor,
    } = runtime;
    format!("runtime {rt_major}.{rt_minor}")
}

/// The key of the entry for the dependency `apps` (program order), their manifests, the
/// preprocessor symbols and the target.
pub fn key_for(
    apps: &[&AppSource],
    manifests: &[crate::runtime::Manifest],
    target: Option<&al_version::Target>,
) -> Key {
    key_with(apps, manifests, target, &[])
}

/// [`key_for`] where `identities[i]`, when set, stands for the source of app `i`: the hash of the zip
/// directory of a package read lazily, which needs no read of the files (and is the hash the app has
/// whether or not its files were read since). An app without one is hashed over every path and byte.
pub fn key_with(
    apps: &[&AppSource],
    manifests: &[crate::runtime::Manifest],
    target: Option<&al_version::Target>,
    identities: &[Option<[u8; 32]>],
) -> Key {
    key_layer(apps, manifests, target, identities, [0; 32])
}

/// [`key_with`] for a layer above another: `below` is the digest of the key of the layer below.
pub fn key_layer(
    apps: &[&AppSource],
    manifests: &[crate::runtime::Manifest],
    target: Option<&al_version::Target>,
    identities: &[Option<[u8; 32]>],
    below: [u8; 32],
) -> Key {
    let apps = apps
        .iter()
        .zip(manifests)
        .enumerate()
        .map(|(i, (a, m))| AppKey {
            id: a.id.clone(),
            name: a.name.clone(),
            dependencies: a.dependencies.clone(),
            publisher: m.publisher.clone(),
            version: m.version.clone(),
            symbols: m
                .symbols
                .iter()
                .cloned()
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect(),
            source: identities.get(i).copied().flatten().unwrap_or_else(|| source_hash(a)),
        })
        .collect();
    Key::layered(
        runtime_build(),
        // `AL_UNPINNED_BASE_ACCESS` changes what the lowering produces (ADR-023), so it is an input.
        format!(
            "{}{}",
            target.map(target_text).unwrap_or_default(),
            std::env::var("AL_UNPINNED_BASE_ACCESS").map_or(String::new(), |v| format!(" base-access {v}")),
        )
        // Positions in the dependency IR change what the entry holds (ADR-041).
        + if al_sema::lower::dependency_positions() { "" } else { " no-dep-positions" },
        apps,
        below,
    )
}

/// The preprocessor symbols each dependency app is compiled with: those of its own app.json, none
/// for the project's. (A dependency is compiled by its publisher, not by the project.)
pub fn app_symbols(manifests: &[crate::runtime::Manifest]) -> Vec<Option<std::collections::BTreeSet<String>>> {
    manifests
        .iter()
        .map(|m| Some(m.symbols.iter().cloned().collect()))
        .collect()
}

/// The publisher whose apps make the first layer (the platform and Microsoft's apps).
pub const MICROSOFT: &str = "Microsoft";

/// The Microsoft apps every Business Central app stands on. They and what they need in the set
/// are the base layer, which projects on one BC build share whatever test libraries or other
/// Microsoft apps they add.
const CORE: [&str; 4] = [
    "System",
    "System Application",
    "Business Foundation",
    "Base Application",
];

/// The indexes of the apps in the base layer: the [`CORE`] apps of Microsoft in the set and, by
/// their `dependencies` (app ids), every app of the set they need. Empty when the set has none
/// of the core apps (a small fixture, or a project that does not stand on Business Central).
fn core_set(apps: &[&AppSource], manifests: &[crate::runtime::Manifest]) -> Vec<bool> {
    let by_id: std::collections::HashMap<&str, usize> =
        apps.iter().enumerate().map(|(i, a)| (a.id.as_str(), i)).collect();
    let mut core = vec![false; apps.len()];
    let mut queue: Vec<usize> = (0..apps.len())
        .filter(|&i| {
            manifests.get(i).is_some_and(|m| m.publisher == MICROSOFT)
                && CORE.iter().any(|c| c.eq_ignore_ascii_case(&apps[i].name))
        })
        .collect();
    for &i in &queue {
        core[i] = true;
    }
    while let Some(i) = queue.pop() {
        for d in &apps[i].dependencies {
            if let Some(&j) = by_id.get(d.as_str()) {
                if !core[j] {
                    core[j] = true;
                    queue.push(j);
                }
            }
        }
    }
    core
}

/// The order the dependency apps are compiled in, the same for any order they came in: a
/// topological order (every app after the apps it depends on) that takes the base layer's apps
/// first, then the other Microsoft apps, then the rest, each group by app name and id as soon as
/// its dependencies are placed. Two projects on one BC build therefore start with the same base
/// layer, and apps they have in common come in the same relative order. With a cycle or a repeated
/// app id the order stays as it came. `apps` and `manifests` are the dependency apps only.
/// Returns the new order as indexes of the old.
pub fn dependency_order(apps: &[&AppSource], manifests: &[crate::runtime::Manifest]) -> Vec<usize> {
    let mut by_id: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for (i, a) in apps.iter().enumerate() {
        if by_id.insert(a.id.as_str(), i).is_some() {
            return (0..apps.len()).collect();
        }
    }
    let core = core_set(apps, manifests);
    let class = |i: usize| -> u8 {
        if core[i] {
            0
        } else if manifests.get(i).is_some_and(|m| m.publisher == MICROSOFT) {
            1
        } else {
            2
        }
    };
    let needs: Vec<Vec<usize>> = apps
        .iter()
        .enumerate()
        .map(|(i, a)| {
            a.dependencies
                .iter()
                .filter_map(|d| by_id.get(d.as_str()).copied())
                .filter(|&d| d != i)
                .collect()
        })
        .collect();
    let mut placed = vec![false; apps.len()];
    let mut order = Vec::with_capacity(apps.len());
    while order.len() < apps.len() {
        let next = (0..apps.len())
            .filter(|&i| !placed[i] && needs[i].iter().all(|&d| placed[d]))
            .min_by_key(|&i| (class(i), apps[i].name.to_lowercase(), apps[i].id.as_str()));
        let Some(i) = next else {
            return (0..apps.len()).collect();
        };
        placed[i] = true;
        order.push(i);
    }
    order
}

/// The layers of the dependency `apps` (in [`dependency_order`]): the leading base apps are the
/// first layer (without any, the leading Microsoft apps), every other app is a layer of its own
/// (a change in it rebuilds it and the layers above, never the layers below). As ranges of `apps`.
pub fn layer_ranges(apps: &[&AppSource], manifests: &[crate::runtime::Manifest]) -> Vec<std::ops::Range<usize>> {
    let core = core_set(apps, manifests);
    let base = if core.iter().any(|c| *c) {
        core.iter().take_while(|c| **c).count()
    } else {
        (0..apps.len())
            .take_while(|&i| manifests.get(i).is_some_and(|m| m.publisher == MICROSOFT))
            .count()
    };
    let mut out = Vec::new();
    if base > 0 {
        out.push(0..base);
    }
    out.extend((base..apps.len()).map(|i| i..i + 1));
    out
}

/// What the lowest layer's key starts from: nothing for `w1` (so the keys of w1 runs are the same as before the
/// country flag), else a digest of the country code.
pub fn country_seed(country: &str) -> [u8; 32] {
    if country.is_empty() || country.eq_ignore_ascii_case("w1") {
        [0; 32]
    } else {
        Sha256::digest(format!("country {}", country.to_ascii_lowercase()).as_bytes()).into()
    }
}

/// The key of each layer, lowest first: a layer's key holds its own apps and the key of the layer
/// below, so a change in any app changes its layer and every layer above it.
pub fn layer_keys(
    apps: &[&AppSource],
    manifests: &[crate::runtime::Manifest],
    target: Option<&al_version::Target>,
    identities: &[Option<[u8; 32]>],
    country: &str,
) -> Vec<(std::ops::Range<usize>, Key)> {
    let mut below = country_seed(country);
    layer_ranges(apps, manifests)
        .into_iter()
        .map(|r| {
            let ids: Vec<Option<[u8; 32]>> = r.clone().map(|i| identities.get(i).copied().flatten()).collect();
            let key = key_layer(&apps[r.clone()], &manifests[r.clone()], target, &ids, below);
            below = key.digest();
            (r, key)
        })
        .collect()
}

/// Reads the entry for `key` from `dir`. `Ok(None)`: there is no file. A file that cannot be used
/// (damaged, truncated, made for other inputs) is removed and reported in `Err`; the caller builds
/// a new one. Nothing in a refused file is used.
pub fn read_entry(dir: &Path, key: &Key) -> Result<Option<(Entry, PathBuf)>, String> {
    let path = dir.join(key.file_name());
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    match Entry::open(bytes, key) {
        Ok(e) => Ok(Some((e, path))),
        Err(e) => {
            let _ = std::fs::remove_file(&path);
            Err(format!("{} ignored and removed: {e}", path.display()))
        }
    }
}

/// Writes `bytes` as the entry for `key`: to a temporary file in the same folder, then renamed, so
/// a reader sees the whole old file or the whole new one.
pub fn write_entry(dir: &Path, key: &Key, bytes: &[u8]) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(key.file_name());
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let tmp = dir.join(format!(
        ".{}.tmp-{}-{}",
        key.file_name(),
        std::process::id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let written = std::fs::write(&tmp, bytes).and_then(|()| std::fs::rename(&tmp, &path));
    if written.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    written?;
    Ok(path)
}

/// Builds the entry for `apps` (dependency apps in program order) and encodes it.
pub fn build_entry(
    apps: Vec<AppSource>,
    manifests: &[crate::runtime::Manifest],
    target: Option<&al_version::Target>,
    key: &Key,
) -> Vec<u8> {
    let set = al_sema::lower::lower_for_cache_with(apps, &app_symbols(manifests), &Default::default(), target);
    al_cache::build(&set, key)
}

/// How long an entry that nothing used stays in the folder.
pub const KEEP_DAYS: u64 = 30;

/// Marks the entry as used now, so that [`prune`] keeps what runs still use. Best effort.
pub fn touch(path: &Path) {
    let _ = std::fs::File::options()
        .write(true)
        .open(path)
        .and_then(|f| f.set_modified(std::time::SystemTime::now()));
}

/// Removes the entries in `dir`, except `keep`, that nothing used for `KEEP_DAYS` days: an app
/// that changes with every run would otherwise leave 13 MB behind each time. Best effort.
pub fn prune(dir: &Path, keep: &[PathBuf]) -> Vec<PathBuf> {
    let limit = std::time::Duration::from_secs(KEEP_DAYS * 24 * 3600);
    let mut removed = Vec::new();
    let indexes = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .is_some_and(|x| x == "alci" || x == crate::baseline_cache::EXTENSION)
        });
    for path in list(dir).into_iter().map(|(p, _)| p).chain(indexes) {
        if keep.contains(&path) {
            continue;
        }
        let old = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age > limit);
        if old && std::fs::remove_file(&path).is_ok() {
            removed.push(path);
        }
    }
    removed
}

/// Removes every entry of the compile cache in `dir`: the layers (`alcc`), the source indexes (`alci`), the install
/// baselines (`albr`) and the temporary files of a write that did not finish. Returns how many files went and
/// how many bytes they held. Other files in the folder stay.
pub fn clear(dir: &Path) -> (usize, u64) {
    let (mut files, mut bytes) = (0, 0);
    for e in std::fs::read_dir(dir).into_iter().flatten().filter_map(|e| e.ok()) {
        let path = e.path();
        let ours = path.extension().is_some_and(|x| {
            x == "alcc"
                || x == "alci"
                || x == crate::baseline_cache::EXTENSION
                || x.to_string_lossy().starts_with("tmp")
        }) || path
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with('.') && n.to_string_lossy().contains(".tmp-"));
        if ours {
            let len = e.metadata().map(|m| m.len()).unwrap_or(0);
            if std::fs::remove_file(&path).is_ok() {
                files += 1;
                bytes += len;
            }
        }
    }
    (files, bytes)
}

/// The entry files in `dir` and their sizes.
pub fn list(dir: &Path) -> Vec<(PathBuf, u64)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<(PathBuf, u64)> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "alcc"))
        .map(|e| (e.path(), e.metadata().map(|m| m.len()).unwrap_or(0)))
        .collect();
    out.sort();
    out
}

/// The layers for the dependency `apps` (in [`dependency_order`]): the files in the cache folder,
/// or built now and saved. A file that cannot be used is reported in the notes and replaced. With
/// `cc.require` a missing or unusable file is an error.
pub fn open_or_build(
    cc: &CompileCache,
    apps: &[AppSource],
    manifests: &[crate::runtime::Manifest],
    target: Option<&al_version::Target>,
) -> Result<(Layers, CacheReport), String> {
    let refs: Vec<&AppSource> = apps.iter().collect();
    let order = dependency_order(&refs, manifests);
    let mut apps: Vec<AppSource> = order.iter().map(|&i| apps[i].clone()).collect();
    let manifests: Vec<crate::runtime::Manifest> = order
        .iter()
        .map(|&i| manifests.get(i).cloned().unwrap_or_default())
        .collect();
    open_or_build_with(cc, &mut apps, &manifests, target, &[], &mut |_| {})
}

/// [`open_or_build`] with the identities of [`key_with`], for apps that are already in
/// [`dependency_order`]. `prepare` is called with the apps just before a layer is built, which
/// needs their text: a caller that left it unread reads it then. A hit never calls it.
pub fn open_or_build_with(
    cc: &CompileCache,
    apps: &mut [AppSource],
    manifests: &[crate::runtime::Manifest],
    target: Option<&al_version::Target>,
    identities: &[Option<[u8; 32]>],
    prepare: &mut dyn FnMut(&mut [AppSource]),
) -> Result<(Layers, CacheReport), String> {
    let opened = std::time::Instant::now();
    let planned = {
        let dep_refs: Vec<&AppSource> = apps.iter().collect();
        layer_keys(&dep_refs, manifests, target, identities, &cc.country)
    };
    let mut report = CacheReport::default();
    let mut entries: Vec<Option<Entry>> = Vec::new();
    for (range, key) in &planned {
        let file = cc.dir.join(key.file_name());
        // `--rebuild-compile-cache`: nothing is read; the layers are built and written again.
        let found = match if cc.rebuild { Ok(None) } else { read_entry(&cc.dir, key) } {
            Ok(Some((e, _))) => Some(e),
            Ok(None) => None,
            Err(note) => {
                eprintln!("compile cache: {note}");
                report.notes.push(note);
                None
            }
        };
        report.layers.push(LayerReport {
            file,
            apps: apps[range.clone()].iter().map(|a| a.name.clone()).collect(),
            hit: found.is_some(),
            bytes: found.as_ref().map_or(0, Entry::size),
            ..Default::default()
        });
        entries.push(found);
    }
    report.open = opened.elapsed();
    report.hit = entries.iter().all(Option::is_some);
    if !report.hit {
        if cc.require {
            let missing: Vec<String> = report
                .layers
                .iter()
                .filter(|l| !l.hit)
                .map(|l| l.file.display().to_string())
                .collect();
            return Err(format!(
                "--require-compile-cache: there is no usable entry {}{}",
                missing.join(", "),
                report.notes.iter().map(|n| format!(" ({n})")).collect::<String>()
            ));
        }
        eprintln!(
            "compile cache: building {} of {} layers of the {} dependency apps once ({})",
            report.layers.iter().filter(|l| !l.hit).count(),
            report.layers.len(),
            apps.len(),
            report
                .layers
                .iter()
                .filter(|l| !l.hit)
                .map(|l| l.apps.join(" + "))
                .collect::<Vec<_>>()
                .join("; ")
        );
        prepare(apps);
        for (li, (range, key)) in planned.iter().enumerate() {
            if entries[li].is_some() {
                continue;
            }
            let started = std::time::Instant::now();
            let layer = {
                let below: Vec<&dyn LayerSource> = entries[..li]
                    .iter()
                    .map(|e| e.as_ref().expect("the layers below are built") as &dyn LayerSource)
                    .collect();
                let stack = Stack::new(below).map_err(|e| format!("the layers below do not fit: {e}"))?;
                lower_one_layer(apps, manifests, range, target, &stack)?
            };
            // The layers above the first compress with its dictionary: training one takes most of
            // the time of a small layer, and the blocks are the same kind of data.
            let bytes = match entries.first().and_then(Option::as_ref).filter(|_| li > 0) {
                Some(first) => al_cache::build_layer_with_dictionary(&layer, key, first.dictionary()),
                None => al_cache::build_layer(&layer, key),
            };
            if let Err(e) = write_entry(&cc.dir, key, &bytes) {
                let note = format!("the entry was not saved: {e}");
                eprintln!("compile cache: {note}");
                report.notes.push(note);
            }
            let l = &mut report.layers[li];
            l.bytes = bytes.len();
            l.built = true;
            entries[li] =
                Some(Entry::open(bytes, key).map_err(|e| format!("a fresh compile cache entry does not open: {e}"))?);
            report.layers[li].build = started.elapsed();
        }
        report.built = true;
        report.build = report.layers.iter().map(|l| l.build).sum();
    }
    let keep: Vec<PathBuf> = report.layers.iter().map(|l| l.file.clone()).collect();
    for f in &keep {
        if report.layers.iter().any(|l| &l.file == f && l.hit) {
            touch(f);
        }
    }
    if report.built {
        prune(&cc.dir, &keep);
    }
    report.bytes = report.layers.iter().map(|l| l.bytes).sum();
    report.file = keep.last().cloned().unwrap_or_default();
    let layers = Layers {
        entries: entries.into_iter().map(|e| e.expect("every layer is open")).collect(),
    };
    if let Err(why) = layers.stack() {
        // Layers made for each other by their keys always fit; this is a defect. Drop the files so
        // that the next run builds them again.
        for f in &keep {
            let _ = std::fs::remove_file(f);
        }
        return Err(format!(
            "the compile cache layers do not fit each other ({why}); the files are removed"
        ));
    }
    Ok((layers, report))
}

/// Compiles the layer of `apps[range]` on top of `below` (the layers under it), whose apps are
/// `apps[..range.start]`.
fn lower_one_layer(
    apps: &[AppSource],
    manifests: &[crate::runtime::Manifest],
    range: &std::ops::Range<usize>,
    target: Option<&al_version::Target>,
    below: &Stack<'_>,
) -> Result<DepLayer, String> {
    if range.start == 0 {
        let set = al_sema::lower::lower_for_cache_with(
            apps[range.clone()].to_vec(),
            &app_symbols(&manifests[range.clone()]),
            &Default::default(),
            target,
        );
        return Ok(DepLayer::whole(set));
    }
    al_sema::lower::lower_layer_with(
        apps[..range.end].to_vec(),
        range.start,
        &app_symbols(&manifests[..range.end]),
        &Default::default(),
        target,
        below,
    )
}

/// The index entry of the dependency apps: what the scan of their sources found (`al_cache::index`).
/// Its key is the key of the apps without a target: the scan depends on the sources, the preprocessor
/// symbols and the compiler, not on the BC version.
pub fn index_key(apps: &[&AppSource], manifests: &[crate::runtime::Manifest], identities: &[Option<[u8; 32]>]) -> Key {
    key_with(apps, manifests, None, identities)
}

/// Reads the index for `key` from `dir`: `None` when there is no file or it cannot be used (damaged,
/// cut, made for other inputs; such a file is removed). The run scans the sources instead.
pub fn read_index(dir: &Path, key: &Key) -> Option<Vec<Vec<al_sema::FileIndex>>> {
    let path = dir.join(al_cache::index_file_name(key));
    let bytes = std::fs::read(&path).ok()?;
    match al_cache::open_index(&bytes, key) {
        Ok(index) => {
            touch(&path);
            Some(index)
        }
        Err(e) => {
            eprintln!("compile cache: {} ignored and removed: {e}", path.display());
            let _ = std::fs::remove_file(&path);
            None
        }
    }
}

/// Writes the index for `key` (a temporary file, then a rename). Best effort: a run that cannot
/// save it is not worse off than before.
pub fn write_index(dir: &Path, key: &Key, index: &[Vec<al_sema::FileIndex>]) {
    let bytes = al_cache::build_index(key, index);
    let _ = std::fs::create_dir_all(dir);
    let path = dir.join(al_cache::index_file_name(key));
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let tmp = dir.join(format!(
        ".{}.tmp-{}-{}",
        al_cache::index_file_name(key),
        std::process::id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    if std::fs::write(&tmp, bytes)
        .and_then(|()| std::fs::rename(&tmp, &path))
        .is_err()
    {
        let _ = std::fs::remove_file(&tmp);
    } else {
        prune(dir, &[path]);
    }
}
