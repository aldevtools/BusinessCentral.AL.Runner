//! The .NET sidecar host (ADR-014): the only code that starts a process for `DotNet` calls.
//! Core crates see the `DotNetHost` trait; this implements it for front ends that may touch the
//! OS. One sidecar process per VM session, started at the first call that reaches it.
//!
//! Protocol (internal to this file and `tools/dotnet-sidecar`): one request, one reply, in order,
//! over stdin and stdout. A message is a 4-byte little-endian length and that many bytes of UTF-8
//! JSON. Values are tagged objects (`{"t":"s","v":"text"}`), see `tools/dotnet-sidecar/Marshal.cs`.

use al_host::dotnet::{
    DotNetArg, DotNetFailure, DotNetHandle, DotNetHost, DotNetReply, DotNetSession, DotNetSessionConfig, DotNetTarget,
    DotNetType, DotNetUnavailable, DotNetValue, StreamEffects,
};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

/// The protocol version the sidecar must speak.
pub const PROTOCOL: u64 = 3;

/// Where the sidecar and .NET are found and how a call is bounded.
#[derive(Debug, Clone)]
pub struct SidecarConfig {
    /// The `dotnet` executable; default: `DOTNET_ROOT`, `PATH`, the usual install folders.
    pub dotnet: Option<PathBuf>,
    /// `--dotnet-sidecar PATH` / `ALRT_DOTNET_SIDECAR`: the built `AlDotNetSidecar.dll`.
    pub sidecar: Option<PathBuf>,
    /// The repository root, for `tools/dotnet-sidecar` (the source the cache is built from).
    pub source_root: Option<PathBuf>,
    /// Where a built sidecar is cached; default `~/.cache/al-runtime-rs/dotnet-sidecar`.
    pub cache_dir: Option<PathBuf>,
    /// `--dotnet-assembly-dir DIR` (repeatable): searched for assemblies the runtime does not
    /// ship, after the platform package folders (ADR-014 section 3, step 3).
    pub assembly_dirs: Vec<PathBuf>,
    /// `--dotnet-allow-roll-forward`: run on a newer major than 8, with a warning.
    pub allow_roll_forward: bool,
    /// Where each process gets its working folders; default: the system temporary folder.
    pub work_root: Option<PathBuf>,
    /// Deadline of one call; on expiry the process is killed (ADR-014 section 10).
    pub timeout: Duration,
}

impl Default for SidecarConfig {
    fn default() -> Self {
        SidecarConfig {
            dotnet: None,
            sidecar: None,
            source_root: None,
            cache_dir: None,
            assembly_dirs: Vec::new(),
            allow_roll_forward: false,
            work_root: None,
            timeout: Duration::from_secs(60),
        }
    }
}

/// What `availability` found out, once.
#[derive(Debug, Clone)]
struct Resolved {
    dotnet: PathBuf,
    sidecar: PathBuf,
}

/// What the sidecars of one host cost, so start-up is reported apart from the calls
/// (`--timings`, the perf budget). `start_*` is the time from spawning the process to its
/// answer to `hello` (and the assembly probe), measured per process.
#[derive(Debug, Clone, Copy, Default)]
pub struct SidecarStats {
    /// Processes that started and answered `hello`: one per session that made a call, and the
    /// standbys (see [`SidecarHost::prewarm`]).
    pub started: usize,
    pub start_total: Duration,
    pub start_max: Duration,
    /// Finding `dotnet` and the sidecar, once per host (it runs `dotnet --list-runtimes`, and
    /// builds the sidecar the first time).
    pub find_total: Duration,
    /// Sessions that were given a standby process instead of starting their own.
    pub from_standby: usize,
    /// How long sessions waited for their process: the start-up share of their wall time. A
    /// standby that was ready makes it zero, however long the start took.
    pub waited_total: Duration,
    /// Requests after the start, and their total time.
    pub calls: usize,
    pub call_total: Duration,
}

/// How many standby sidecars a run that has the Microsoft apps keeps ready (`prewarm`). The
/// corpus has two test codeunits that call .NET; two is enough for the sessions that start
/// together, and a taken one is replaced while the next codeunit runs.
pub const STANDBY_SIDECARS: usize = 2;

thread_local! {
    /// Start-up time of the sidecars that threads started on this thread since the last
    /// [`take_start_time`]: the test runner reads it after each test codeunit.
    static START_ON_THIS_THREAD: std::cell::Cell<Duration> = const { std::cell::Cell::new(Duration::ZERO) };
}

/// The time sidecar processes started on this thread took since the last call, and resets it.
/// A test codeunit runs on one thread, so this is the start-up share of its wall time.
pub fn take_start_time() -> Duration {
    START_ON_THIS_THREAD.with(|c| c.replace(Duration::ZERO))
}

/// What the host, its sessions and its background starts share.
struct Shared {
    cfg: SidecarConfig,
    resolved: OnceLock<Result<Resolved, String>>,
    stats: Mutex<SidecarStats>,
    pool: Pool,
}

impl Shared {
    fn resolve(&self) -> &Result<Resolved, String> {
        self.resolved.get_or_init(|| {
            let began = std::time::Instant::now();
            let r = resolve(&self.cfg);
            lock(&self.stats).find_total = began.elapsed();
            r
        })
    }

    /// `resolve`, charging the time the caller waited (it may be waiting for another thread's
    /// `dotnet --list-runtimes`) to its codeunit as sidecar start-up.
    fn resolve_waiting(&self) -> &Result<Resolved, String> {
        let began = std::time::Instant::now();
        let r = self.resolve();
        self.waited(began.elapsed());
        r
    }

    fn waited(&self, w: Duration) {
        lock(&self.stats).waited_total += w;
        START_ON_THIS_THREAD.with(|c| c.set(c.get() + w));
    }
}

/// The default probe folder (ADR-014 section 3, step 2): the service-tier folder of the platform
/// artifact in `sandbox` (`~/.bcartifacts.cache/sandbox`) for the BC `major` in use.
/// `build` (the chosen build, when known) is tried first; else the newest build of that major.
/// The folder is `<build>/platform/ServiceTier/PFiles64/Microsoft Dynamics NAV/<NNN>/Service`,
/// where `<NNN>` is derived from what is on disk (270, 280, 290), never fixed. `None` when the
/// artifact is not extracted locally. Only the top level is searched, as BC does.
pub fn default_probe_dir(sandbox: &Path, build: Option<&str>, major: u32) -> Option<PathBuf> {
    let service_of = |build_dir: &Path| -> Option<PathBuf> {
        let nav = build_dir.join("platform/ServiceTier/PFiles64/Microsoft Dynamics NAV");
        let mut found: Vec<(u32, PathBuf)> = std::fs::read_dir(nav)
            .ok()?
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let n: u32 = e.file_name().to_string_lossy().parse().ok()?;
                let svc = e.path().join("Service");
                svc.is_dir().then_some((n, svc))
            })
            .collect();
        found.sort();
        found.pop().map(|(_, p)| p)
    };
    if let Some(dir) = build
        .filter(|b| !b.contains(['/', '\\']))
        .and_then(|b| service_of(&sandbox.join(b)))
    {
        return Some(dir);
    }
    let mut builds: Vec<(Vec<u64>, PathBuf)> = std::fs::read_dir(sandbox)
        .ok()?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let parts: Vec<u64> = name.split('.').map(|p| p.parse().ok()).collect::<Option<_>>()?;
            (parts.first().copied() == Some(u64::from(major))).then_some((parts, e.path()))
        })
        .collect();
    builds.sort();
    builds.iter().rev().find_map(|(_, p)| service_of(p))
}

/// The probe folders of a run: the user's `--dotnet-assembly-dir` folders first, then the default
/// service-tier folder (see [`default_probe_dir`]) unless it is already listed.
pub fn probe_dirs(user: &[PathBuf], sandbox: Option<&Path>, build: Option<&str>, major: u32) -> Vec<PathBuf> {
    let mut dirs = user.to_vec();
    if let Some(d) = sandbox.and_then(|s| default_probe_dir(s, build, major)) {
        if !dirs.contains(&d) {
            dirs.push(d);
        }
    }
    dirs
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// A started process nobody has used. One session takes it whole, so static .NET state never
/// leaks between sessions (ADR-014 section 6).
struct Standby {
    proc: Proc,
    next_id: u64,
}

#[derive(Default)]
struct PoolState {
    ready: Vec<Standby>,
    /// Standbys being started in the background.
    starting: usize,
    /// How many to keep ready or starting. A taken one is replaced.
    target: usize,
    /// The target the first session that starts a process of its own sets (`prewarm_on_first_use`).
    on_first_use: usize,
    closed: bool,
}

#[derive(Default)]
struct Pool {
    state: Mutex<PoolState>,
    changed: std::sync::Condvar,
}

impl Pool {
    /// Counts the standbys the caller must start now to reach the target.
    fn reserve(&self) -> usize {
        let mut st = lock(&self.state);
        if st.closed {
            return 0;
        }
        let need = st.target.saturating_sub(st.ready.len() + st.starting);
        st.starting += need;
        need
    }

    /// A background start ended. A failed one stops the pool: the session that needs a process
    /// starts its own and reports the real reason.
    fn deposit(&self, started: Result<Standby, String>) {
        let mut st = lock(&self.state);
        st.starting -= 1;
        match started {
            Ok(s) if !st.closed => st.ready.push(s),
            Ok(_) => {}
            Err(_) => st.target = 0,
        }
        self.changed.notify_all();
    }

    /// A ready standby, or one that is starting (waited for at most `within`), or `None`.
    fn take(&self, within: Duration) -> Option<Standby> {
        let until = std::time::Instant::now() + within;
        let mut st = lock(&self.state);
        loop {
            if let Some(s) = st.ready.pop() {
                return Some(s);
            }
            let left = until.checked_duration_since(std::time::Instant::now())?;
            if st.starting == 0 {
                return None;
            }
            st = self.changed.wait_timeout(st, left).unwrap_or_else(|e| e.into_inner()).0;
        }
    }

    fn close(&self) {
        let ready = {
            let mut st = lock(&self.state);
            st.closed = true;
            st.target = 0;
            std::mem::take(&mut st.ready)
        };
        self.changed.notify_all();
        drop(ready); // kills the processes and removes their folders
    }
}

/// Starts `n` (already reserved) standby processes, each on its own thread.
fn start_standbys(shared: &Arc<Shared>, n: usize) {
    for _ in 0..n {
        let shared = shared.clone();
        let spawned = std::thread::Builder::new().name("dotnet-standby".into()).spawn({
            let shared = shared.clone();
            move || {
                let started = (|| {
                    shared.resolve().clone()?;
                    let mut s = SidecarSession::new(shared.clone(), shared.resolve().clone());
                    s.start_process()?;
                    let proc = s.proc.take().ok_or("the process is gone")?;
                    Ok(Standby {
                        proc,
                        next_id: s.next_id,
                    })
                })();
                shared.pool.deposit(started);
            }
        });
        if spawned.is_err() {
            shared.pool.deposit(Err("no room for another thread".into()));
        }
    }
}

pub struct SidecarHost {
    shared: Arc<Shared>,
}

impl SidecarHost {
    pub fn new(cfg: SidecarConfig) -> SidecarHost {
        SidecarHost {
            shared: Arc::new(Shared {
                cfg,
                resolved: OnceLock::new(),
                stats: Mutex::new(SidecarStats::default()),
                pool: Pool::default(),
            }),
        }
    }

    /// Start-up and call time of every sidecar this host has run so far.
    pub fn stats(&self) -> SidecarStats {
        *lock(&self.shared.stats)
    }

    /// The sidecar this host runs, once it has been found (or built).
    pub fn sidecar_path(&self) -> Option<PathBuf> {
        self.shared.resolve().as_ref().ok().map(|r| r.sidecar.clone())
    }

    /// Starts up to `n` sidecar processes in the background and keeps them waiting, so the first
    /// sessions that make a `DotNet` call find one running instead of paying for the start
    /// (ADR-014 section 6, "warm standby"). Call it at the beginning of a run that is likely to
    /// reach `DotNet`, so the starts overlap the compile. A session takes a standby whole and
    /// alone; the host starts a replacement. Nothing is loaded into a standby, so it cannot be
    /// told from a fresh process. [`SidecarHost::shutdown`] ends the unused ones.
    pub fn prewarm(&self, n: usize) {
        lock(&self.shared.pool.state).target = n;
        let need = self.shared.pool.reserve();
        start_standbys(&self.shared, need);
    }

    /// Like [`prewarm`](Self::prewarm), but nothing starts until a session needs a process: that
    /// session starts its own, and `n` standbys start beside it for the sessions that follow. A run
    /// that never reaches `DotNet` starts no process at all. For a run whose compile is short
    /// (a compile cache hit), where a start ahead has little to overlap.
    pub fn prewarm_on_first_use(&self, n: usize) {
        lock(&self.shared.pool.state).on_first_use = n;
    }

    /// Ends the standby processes nobody used and stops starting new ones. Sessions that call
    /// later start their own process.
    pub fn shutdown(&self) {
        self.shared.pool.close();
    }
}

impl Drop for SidecarHost {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl DotNetHost for SidecarHost {
    fn availability(&self) -> Result<(), DotNetUnavailable> {
        match self.shared.resolve_waiting() {
            Ok(_) => Ok(()),
            Err(why) => Err(DotNetUnavailable(why.clone())),
        }
    }

    fn open_session(&self, _cfg: &DotNetSessionConfig) -> Box<dyn DotNetSession> {
        Box::new(SidecarSession::new(
            self.shared.clone(),
            self.shared.resolve_waiting().clone(),
        ))
    }
}

// ---- finding dotnet and the sidecar -------------------------------------------------------

fn usual_dotnet_dirs() -> Vec<PathBuf> {
    let mut v = vec![
        PathBuf::from("/usr/share/dotnet"),
        PathBuf::from("/usr/lib/dotnet"),
        PathBuf::from("/usr/local/share/dotnet"),
        PathBuf::from("/usr/local/lib/dotnet"),
        PathBuf::from("/opt/dotnet"),
    ];
    if let Ok(home) = std::env::var("HOME") {
        v.push(Path::new(&home).join(".dotnet"));
    }
    v
}

fn locate_dotnet() -> Option<PathBuf> {
    let exe = if cfg!(windows) { "dotnet.exe" } else { "dotnet" };
    if let Ok(root) = std::env::var("DOTNET_ROOT") {
        let p = Path::new(&root).join(exe);
        if p.is_file() {
            return Some(p);
        }
    }
    if let Ok(path) = std::env::var("PATH") {
        for d in std::env::split_paths(&path) {
            let p = d.join(exe);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    usual_dotnet_dirs()
        .into_iter()
        .map(|d| d.join(exe))
        .find(|p| p.is_file())
}

/// `Microsoft.NETCore.App` versions `dotnet --list-runtimes` reports.
fn runtimes(dotnet: &Path) -> Vec<String> {
    let Ok(out) = Command::new(dotnet).arg("--list-runtimes").output() else {
        return Vec::new();
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            (it.next() == Some("Microsoft.NETCore.App")).then(|| it.next().map(str::to_string))?
        })
        .collect()
}

fn major(v: &str) -> Option<u32> {
    v.split('.').next()?.parse().ok()
}

fn resolve(cfg: &SidecarConfig) -> Result<Resolved, String> {
    if let Some(p) = &cfg.dotnet {
        if !p.is_file() {
            return Err(format!(
                "needs the .NET 8 runtime; 'dotnet' not found ({} does not exist)",
                p.display()
            ));
        }
    }
    // An explicit path is a file, read against the runner's folder as the sidecar's path is (`find_sidecar`).
    let explicit = cfg.dotnet.clone().map(|p| std::path::absolute(&p).unwrap_or(p));
    let Some(dotnet) = explicit.or_else(locate_dotnet) else {
        return Err(
            "needs the .NET 8 runtime; 'dotnet' not found (looked in DOTNET_ROOT, PATH, /usr/share/dotnet, /usr/lib/dotnet, ~/.dotnet)"
                .into(),
        );
    };
    let versions = runtimes(&dotnet);
    let has8 = versions.iter().any(|v| major(v) == Some(8));
    if !has8 {
        let newer = versions.iter().any(|v| major(v).is_some_and(|m| m > 8));
        if !(cfg.allow_roll_forward && newer) {
            let found = if versions.is_empty() {
                "no Microsoft.NETCore.App runtime".to_string()
            } else {
                format!("found {}", versions.join(", "))
            };
            return Err(format!(
                "needs the .NET 8 runtime; {found}; the sidecar needs Microsoft.NETCore.App 8.x (--dotnet-allow-roll-forward runs it on a newer major)"
            ));
        }
        eprintln!(
            "warning: running the .NET sidecar on {} instead of 8.x (--dotnet-allow-roll-forward); BCL behaviour can differ",
            versions.join(", ")
        );
    }
    let sidecar = find_sidecar(cfg, &dotnet)?;
    Ok(Resolved { dotnet, sidecar })
}

/// The sidecar's source files, built into this binary, so a copied or installed `al-runner` can
/// find or build its helper without the repository (the cache is keyed by the same hash as the
/// repository's own build). A test keeps the list equal to the folder.
const EMBEDDED: &[(&str, &str)] = &[
    (
        "AlDotNetSidecar.csproj",
        include_str!("../../../tools/dotnet-sidecar/AlDotNetSidecar.csproj"),
    ),
    ("Dispatch.cs", include_str!("../../../tools/dotnet-sidecar/Dispatch.cs")),
    ("Marshal.cs", include_str!("../../../tools/dotnet-sidecar/Marshal.cs")),
    ("Program.cs", include_str!("../../../tools/dotnet-sidecar/Program.cs")),
    ("Resolver.cs", include_str!("../../../tools/dotnet-sidecar/Resolver.cs")),
    ("global.json", include_str!("../../../tools/dotnet-sidecar/global.json")),
];

/// FNV-1a over file names and contents, in the order given: a stable cache key.
fn hash_files<'a>(files: impl IntoIterator<Item = (&'a str, &'a [u8])>) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for (name, bytes) in files {
        for b in name.bytes().chain(bytes.iter().copied()) {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    }
    h
}

/// FNV-1a over the sidecar's source files in `dir`: a stable cache key.
fn source_hash(dir: &Path) -> Option<u64> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension().is_some_and(|x| x == "cs" || x == "csproj")
                || p.file_name().is_some_and(|n| n == "global.json")
        })
        .collect();
    files.sort();
    let mut read = Vec::new();
    for f in files {
        read.push((f.file_name()?.to_string_lossy().into_owned(), std::fs::read(&f).ok()?));
    }
    Some(hash_files(read.iter().map(|(n, b)| (n.as_str(), b.as_slice()))))
}

/// The hash of the sources built into this binary.
fn embedded_hash() -> u64 {
    hash_files(EMBEDDED.iter().map(|(n, c)| (*n, c.as_bytes())))
}

fn default_cache_dir() -> Option<PathBuf> {
    if let Ok(x) = std::env::var("XDG_CACHE_HOME") {
        return Some(Path::new(&x).join("al-runtime-rs/dotnet-sidecar"));
    }
    let home = std::env::var("HOME").ok()?;
    Some(Path::new(&home).join(".cache/al-runtime-rs/dotnet-sidecar"))
}

const NOT_BUILT: &str = "sidecar not built; run 'dotnet publish tools/dotnet-sidecar -c Release -o <dir> --disable-build-servers' and pass --dotnet-sidecar <dir>/AlDotNetSidecar.dll";

/// ADR-014 section 10, lookup order: the flag or `ALRT_DOTNET_SIDECAR`; `dotnet-sidecar/` next to
/// this binary; a cache keyed by the source hash, built there once when the .NET SDK is present.
fn find_sidecar(cfg: &SidecarConfig, dotnet: &Path) -> Result<PathBuf, String> {
    let from_env = std::env::var("ALRT_DOTNET_SIDECAR").ok().map(PathBuf::from);
    if let Some(p) = cfg.sidecar.clone().or(from_env) {
        // The process starts in a working folder of its own (`current_dir` below): a relative path is read against the
        // folder the runner was started in.
        let p = std::path::absolute(&p).unwrap_or(p);
        return if p.is_file() {
            Ok(p)
        } else {
            Err(format!("sidecar not built; {} does not exist", p.display()))
        };
    }
    if let Some(p) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|d| d.join("dotnet-sidecar/AlDotNetSidecar.dll")))
        .filter(|p| p.is_file())
    {
        return Ok(p);
    }
    // The repository's sources when this binary was built inside one (they may be newer than the
    // binary while developing); otherwise the copy built into the binary.
    let src = cfg
        .source_root
        .as_ref()
        .map(|r| r.join("tools/dotnet-sidecar"))
        .filter(|s| s.join("AlDotNetSidecar.csproj").is_file());
    let hash = match &src {
        Some(dir) => source_hash(dir),
        None => Some(embedded_hash()),
    };
    let (Some(hash), Some(cache)) = (hash, cfg.cache_dir.clone().or_else(default_cache_dir)) else {
        return Err(NOT_BUILT.into());
    };
    let dir = cache.join(format!("{hash:016x}"));
    let dll = dir.join("AlDotNetSidecar.dll");
    if dll.is_file() {
        return Ok(dll);
    }
    // One build at a time in this process; another process may build too, see the rename below.
    static BUILD: Mutex<()> = Mutex::new(());
    let _building = BUILD.lock().unwrap_or_else(|e| e.into_inner());
    if dll.is_file() {
        return Ok(dll);
    }
    let sdks = Command::new(dotnet)
        .arg("--list-sdks")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    if !sdks.lines().any(|l| l.trim_start().starts_with("8.")) {
        return Err(format!("{NOT_BUILT} (no .NET 8 SDK found to build it)"));
    }
    eprintln!("building the .NET sidecar into {} (once)", dir.display());
    let tmp = cache.join(format!(".build-{hash:016x}-{}", std::process::id()));
    let src = match src {
        Some(dir) => dir,
        None => {
            // Write the built-in sources next to the build, then build from them.
            let dir = cache.join(format!(".src-{hash:016x}-{}", std::process::id()));
            std::fs::create_dir_all(&dir).map_err(|e| format!("{NOT_BUILT} (cannot write {}: {e})", dir.display()))?;
            for (name, content) in EMBEDDED {
                std::fs::write(dir.join(name), content)
                    .map_err(|e| format!("{NOT_BUILT} (cannot write {}: {e})", dir.join(name).display()))?;
            }
            dir
        }
    };
    let built_from_embedded = src
        .file_name()
        .is_some_and(|n| n.to_string_lossy().starts_with(".src-"));
    let out = Command::new(dotnet)
        .arg("publish")
        .arg(&src)
        .args(["-c", "Release", "-o"])
        .arg(&tmp)
        // No build servers: a VBCSCompiler left running for minutes inherits our open file
        // descriptors (pipes, locks) and keeps whoever waits on them waiting.
        .args(["--disable-build-servers", "-p:UseSharedCompilation=false"])
        .env("DOTNET_NOLOGO", "1")
        .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
        .output()
        .map_err(|e| format!("{NOT_BUILT} (dotnet publish failed to start: {e})"));
    if built_from_embedded {
        let _ = std::fs::remove_dir_all(&src);
    }
    let out = out?;
    if !out.status.success() || !tmp.join("AlDotNetSidecar.dll").is_file() {
        let _ = std::fs::remove_dir_all(&tmp);
        let tail: String = String::from_utf8_lossy(&out.stdout)
            .lines()
            .rev()
            .take(3)
            .collect::<Vec<_>>()
            .join(" | ");
        return Err(format!("{NOT_BUILT} (dotnet publish failed: {tail})"));
    }
    // Another process may have finished first; its copy is as good.
    if std::fs::rename(&tmp, &dir).is_err() {
        let _ = std::fs::remove_dir_all(&tmp);
    }
    if dll.is_file() {
        Ok(dll)
    } else {
        Err(format!("{NOT_BUILT} (could not move the build into {})", dir.display()))
    }
}

// ---- one sidecar process per VM session ---------------------------------------------------

impl Drop for Proc {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.workdir);
    }
}

struct Proc {
    child: Child,
    stdin: ChildStdin,
    replies: Receiver<Option<Vec<u8>>>,
    last_stderr: Arc<Mutex<String>>,
    /// Drains stderr into `last_stderr`; ends when the process does.
    stderr_thread: Option<std::thread::JoinHandle<()>>,
    /// Holds `cwd` (the working directory, empty), `home` and `tmp`; removed with the session.
    workdir: PathBuf,
}

struct SidecarSession {
    shared: Arc<Shared>,
    resolved: Result<Resolved, String>,
    /// Unique in the process, whichever host starts the process: its working folder is named by it.
    number: u64,
    proc: Option<Proc>,
    /// Why every further call is refused: the process died, timed out or never started.
    dead: Option<String>,
    next_id: u64,
    /// Ids of `release` requests sent without waiting. The sidecar answers in order, so their replies come first and are
    /// dropped by the next request that waits for its own.
    unacked: std::collections::VecDeque<u64>,
}

fn read_frame(r: &mut impl Read) -> Option<Vec<u8>> {
    let mut head = [0u8; 4];
    r.read_exact(&mut head).ok()?;
    let mut body = vec![0u8; u32::from_le_bytes(head) as usize];
    r.read_exact(&mut body).ok()?;
    Some(body)
}

impl SidecarSession {
    fn new(shared: Arc<Shared>, resolved: Result<Resolved, String>) -> SidecarSession {
        static PROCESSES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        SidecarSession {
            shared,
            resolved,
            number: PROCESSES.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1,
            proc: None,
            dead: None,
            next_id: 1,
            unacked: std::collections::VecDeque::new(),
        }
    }

    /// Gets this session its process: a standby when one is ready or starting, else a new one.
    /// The time this takes is what the session's codeunit waits for (`take_start_time`).
    fn start(&mut self) -> Result<(), String> {
        let began = std::time::Instant::now();
        // A standby that is starting has a head start on a process begun now; wait for it, but
        // not past the deadline a start of our own would get.
        let standby = self
            .shared
            .pool
            .take(self.shared.cfg.timeout.max(Duration::from_secs(30)));
        let result = match standby {
            Some(sb) => {
                self.proc = Some(sb.proc);
                self.next_id = sb.next_id;
                lock(&self.shared.stats).from_standby += 1;
                // Keep the pool at its target for the sessions that follow.
                let need = self.shared.pool.reserve();
                start_standbys(&self.shared, need);
                Ok(())
            }
            None => {
                // The first process of a run kept for first use: the standbys start now, beside it.
                let first = {
                    let mut st = lock(&self.shared.pool.state);
                    let n = std::mem::take(&mut st.on_first_use);
                    if n > 0 && st.target == 0 && !st.closed {
                        st.target = n;
                    }
                    n > 0
                };
                if first {
                    let need = self.shared.pool.reserve();
                    start_standbys(&self.shared, need);
                }
                self.start_process()
            }
        };
        self.shared.waited(began.elapsed());
        result
    }

    /// Spawns the process and checks its `hello`.
    fn start_process(&mut self) -> Result<(), String> {
        let began = std::time::Instant::now();
        let r = self.resolved.clone()?;
        let workdir = self
            .shared
            .cfg
            .work_root
            .clone()
            .unwrap_or_else(std::env::temp_dir)
            .join(format!("al-runtime-dotnet-{}-{}", std::process::id(), self.number));
        let (cwd, home, tmp) = (workdir.join("cwd"), workdir.join("home"), workdir.join("tmp"));
        for d in [&cwd, &home, &tmp] {
            std::fs::create_dir_all(d).map_err(|e| format!("cannot create {}: {e}", d.display()))?;
        }
        // A cleared environment, UTC, an empty working directory (ADR-014 section 9).
        let mut cmd = Command::new(&r.dotnet);
        cmd.arg(&r.sidecar)
            .env_clear()
            .env("HOME", &home)
            .env("TMPDIR", &tmp)
            .env("TZ", "UTC")
            .env("DOTNET_NOLOGO", "1")
            .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
            .env("DOTNET_TieredPGO", "0")
            .env("DOTNET_TC_QuickJitForLoops", "1")
            .env("DOTNET_gcServer", "0")
            .current_dir(&cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(root) = r.dotnet.parent() {
            cmd.env("DOTNET_ROOT", root);
        }
        if self.shared.cfg.allow_roll_forward {
            cmd.env("DOTNET_ROLL_FORWARD", "Major");
        }
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("cannot start {}: {e}", r.dotnet.display()))?;
        let stdin = child.stdin.take().expect("piped");
        let mut stdout = child.stdout.take().expect("piped");
        let stderr = child.stderr.take().expect("piped");
        let last_stderr = Arc::new(Mutex::new(String::new()));
        let sink = last_stderr.clone();
        let stderr_thread = std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                if !line.trim().is_empty() {
                    *sink.lock().unwrap() = line;
                }
            }
        });
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            while let Some(frame) = read_frame(&mut stdout) {
                if tx.send(Some(frame)).is_err() {
                    return;
                }
            }
            let _ = tx.send(None);
        });
        self.proc = Some(Proc {
            child,
            stdin,
            replies: rx,
            last_stderr,
            stderr_thread: Some(stderr_thread),
            workdir,
        });
        // The first message checks the protocol version and the runtime.
        // Starting a process is not a call: it gets at least 30 s however short the call deadline.
        let start_deadline = self.shared.cfg.timeout.max(Duration::from_secs(30));
        let hello = self
            .raw_request_within(json!({"op":"hello","protocol":PROTOCOL}), start_deadline)
            .map_err(|f| match f {
                DotNetFailure::Refused(s) => s,
                DotNetFailure::Exception(e) => e.to_string(),
            })?;
        let v = &hello["value"];
        if v["protocol"].as_u64() != Some(PROTOCOL) || v["compatible"] != json!(true) {
            return Err(format!(
                "protocol mismatch: the sidecar speaks version {}, the host version {PROTOCOL}",
                v["protocol"]
            ));
        }
        if v["major"].as_u64() != Some(8) && !self.shared.cfg.allow_roll_forward {
            return Err(format!(
                "the sidecar runs on {}; it needs Microsoft.NETCore.App 8.x",
                v["runtime"].as_str().unwrap_or("an unknown runtime")
            ));
        }
        // Where assemblies other than the standard library come from (ADR-014 section 3).
        if !self.shared.cfg.assembly_dirs.is_empty() {
            let dirs: Vec<Value> = self
                .shared
                .cfg
                .assembly_dirs
                .iter()
                .map(|d| json!({"path": std::path::absolute(d).unwrap_or_else(|_| d.clone()).to_string_lossy(), "recursive": false}))
                .collect();
            self.raw_request(json!({"op":"probe","dirs":dirs}))
                .map_err(|f| f.to_string())?;
        }
        let took = began.elapsed();
        let mut st = lock(&self.shared.stats);
        st.started += 1;
        st.start_total += took;
        st.start_max = st.start_max.max(took);
        Ok(())
    }

    /// Sends one request and returns the whole reply object, or the refusal or exception.
    fn raw_request(&mut self, req: Value) -> Result<Value, DotNetFailure> {
        let timeout = self.shared.cfg.timeout;
        self.raw_request_within(req, timeout)
    }

    fn raw_request_within(&mut self, mut req: Value, timeout: Duration) -> Result<Value, DotNetFailure> {
        let id = self.next_id;
        self.next_id += 1;
        req["id"] = json!(id);
        let body = req.to_string().into_bytes();
        let p = self.proc.as_mut().expect("started");
        let mut frame = (body.len() as u32).to_le_bytes().to_vec();
        frame.extend_from_slice(&body);
        if p.stdin.write_all(&frame).and_then(|_| p.stdin.flush()).is_err() {
            return Err(self.died("the pipe to the sidecar is closed"));
        }
        let p = self.proc.as_mut().expect("started");
        let mut answer = p.replies.recv_timeout(timeout);
        // The replies to releases sent without waiting come first, in order: drop them.
        while let Ok(Some(bytes)) = &answer {
            let Some(front) = self.unacked.front().copied() else {
                break;
            };
            let is_front = serde_json::from_slice::<Value>(bytes).is_ok_and(|v| v["id"] == json!(front));
            if !is_front {
                break;
            }
            self.unacked.pop_front();
            let p = self.proc.as_mut().expect("started");
            answer = p.replies.recv_timeout(timeout);
        }
        match answer {
            Ok(Some(bytes)) => {
                let v: Value = serde_json::from_slice(&bytes)
                    .map_err(|e| DotNetFailure::Refused(format!("the sidecar wrote invalid JSON: {e}")))?;
                if v["id"] != json!(id) {
                    return Err(DotNetFailure::Refused(format!(
                        "the sidecar answered out of order (expected {id}, got {})",
                        v["id"]
                    )));
                }
                if v["ok"] == json!(true) {
                    return Ok(v);
                }
                if v["kind"] == "exception" {
                    return Err(DotNetFailure::Exception(al_host::dotnet::DotNetException {
                        clr_type: v["type"].as_str().unwrap_or("").to_string(),
                        message: v["message"].as_str().unwrap_or("").to_string(),
                        inner: v["inner"]
                            .as_array()
                            .map(|a| {
                                a.iter()
                                    .map(|e| {
                                        (
                                            e[0].as_str().unwrap_or("").to_string(),
                                            e[1].as_str().unwrap_or("").to_string(),
                                        )
                                    })
                                    .collect()
                            })
                            .unwrap_or_default(),
                    }));
                }
                Err(DotNetFailure::Refused(
                    v["reason"].as_str().unwrap_or("refused by the sidecar").to_string(),
                ))
            }
            Ok(None) | Err(RecvTimeoutError::Disconnected) => Err(self.died("the sidecar process ended")),
            Err(RecvTimeoutError::Timeout) => {
                let why = format!(
                    "the call timed out after {} s; the sidecar was killed",
                    timeout.as_secs()
                );
                self.kill();
                self.dead = Some(why.clone());
                Err(DotNetFailure::Refused(why))
            }
        }
    }

    fn kill(&mut self) {
        if let Some(mut p) = self.proc.take() {
            let _ = p.child.kill();
            let _ = p.child.wait();
            let _ = std::fs::remove_dir_all(&p.workdir);
        }
    }

    /// The process is gone: say how, once, and refuse everything after.
    fn died(&mut self, what: &str) -> DotNetFailure {
        let mut detail = String::new();
        if let Some(mut p) = self.proc.take() {
            let status = p.child.wait().ok().map(|s| s.to_string()).unwrap_or_default();
            // The process is gone, so its stderr is at its end: let the reader take the last line.
            if let Some(t) = p.stderr_thread.take() {
                let _ = t.join();
            }
            let last = p.last_stderr.lock().unwrap().clone();
            detail = format!(" ({status}; last stderr: {last})");
            let _ = std::fs::remove_dir_all(&p.workdir);
        }
        let why = format!("{what}{detail}");
        self.dead = Some(why.clone());
        DotNetFailure::Refused(why)
    }

    fn request(&mut self, req: Value) -> Result<Value, DotNetFailure> {
        if let Some(why) = &self.dead {
            return Err(DotNetFailure::Refused(why.clone()));
        }
        if self.proc.is_none() {
            if let Err(why) = self.start() {
                self.kill();
                self.dead = Some(why.clone());
                return Err(DotNetFailure::Refused(why));
            }
        }
        let began = std::time::Instant::now();
        let reply = self.raw_request(req);
        let mut st = lock(&self.shared.stats);
        st.calls += 1;
        st.call_total += began.elapsed();
        reply
    }
}

impl Drop for SidecarSession {
    fn drop(&mut self) {
        self.kill();
    }
}

fn type_json(t: &DotNetType) -> Value {
    json!({"assembly": t.assembly, "version": t.version, "culture": t.culture, "token": t.public_key_token, "name": t.name})
}

fn target_json(t: &DotNetTarget) -> Value {
    match t {
        DotNetTarget::Static(ty) => json!({"static": type_json(ty)}),
        DotNetTarget::Instance(h) => json!({"handle": h}),
        DotNetTarget::Value(v) => json!({"value": encode(v)}),
    }
}

fn args_json(args: &[DotNetArg]) -> Value {
    Value::Array(
        args.iter()
            .map(|a| json!({"v": encode(&a.value), "ref": a.by_ref}))
            .collect(),
    )
}

fn reply_of(v: Value) -> Result<DotNetReply, DotNetFailure> {
    let bad = |e: String| DotNetFailure::Refused(format!("unreadable reply from the sidecar: {e}"));
    let value = decode(&v["value"]).map_err(bad)?;
    let mut by_ref = Vec::new();
    if let Some(a) = v["byref"].as_array() {
        for e in a {
            by_ref.push((e[0].as_u64().unwrap_or(0) as usize, decode(&e[1]).map_err(bad)?));
        }
    }
    let s = &v["streams"];
    let mut fx = StreamEffects::default();
    for e in s["consumed"].as_array().into_iter().flatten() {
        fx.consumed
            .push((e[0].as_u64().unwrap_or(0) as usize, e[1].as_u64().unwrap_or(0) as usize));
    }
    for e in s["bridges"].as_array().into_iter().flatten() {
        fx.bridges
            .push((e[0].as_u64().unwrap_or(0) as usize, e[1].as_u64().unwrap_or(0)));
    }
    for e in s["written"].as_array().into_iter().flatten() {
        fx.written
            .push((e[0].as_u64().unwrap_or(0), unbase64(e[1].as_str().unwrap_or(""))));
    }
    Ok(DotNetReply {
        value,
        by_ref,
        streams: fx,
    })
}

impl DotNetSession for SidecarSession {
    fn construct(&mut self, ty: &DotNetType, args: Vec<DotNetArg>) -> Result<DotNetReply, DotNetFailure> {
        self.request(json!({"op":"construct","type":type_json(ty),"args":args_json(&args)}))
            .and_then(reply_of)
    }
    fn invoke(
        &mut self,
        target: DotNetTarget,
        member: &str,
        args: Vec<DotNetArg>,
    ) -> Result<DotNetReply, DotNetFailure> {
        self.request(json!({"op":"invoke","target":target_json(&target),"member":member,"args":args_json(&args)}))
            .and_then(reply_of)
    }
    fn get_property(&mut self, target: DotNetTarget, name: &str) -> Result<DotNetReply, DotNetFailure> {
        self.request(json!({"op":"get","target":target_json(&target),"member":name}))
            .and_then(reply_of)
    }
    fn set_property(
        &mut self,
        target: DotNetTarget,
        name: &str,
        value: DotNetValue,
    ) -> Result<DotNetReply, DotNetFailure> {
        self.request(json!({"op":"set","target":target_json(&target),"member":name,"value":encode(&value)}))
            .and_then(reply_of)
    }
    fn stream_write(&mut self, handle: DotNetHandle, bytes: &[u8]) -> Result<(), DotNetFailure> {
        self.request(json!({"op":"stream_write","handle":handle,"data":base64(bytes)}))
            .map(|_| ())
    }
    fn stream_read(&mut self, handle: DotNetHandle, max: Option<usize>) -> Result<Vec<u8>, DotNetFailure> {
        let mut req = json!({"op":"stream_read","handle":handle});
        if let Some(n) = max {
            req["count"] = json!(n);
        }
        let v = self.request(req)?;
        let bad = |e: String| DotNetFailure::Refused(format!("unreadable reply from the sidecar: {e}"));
        match decode(&v["value"]).map_err(bad)? {
            DotNetValue::InStream(b) => Ok(b),
            _ => Err(bad("expected bytes for stream_read".into())),
        }
    }
    fn release(&mut self, handles: &[DotNetHandle]) {
        if self.dead.is_some() {
            return;
        }
        let Some(p) = self.proc.as_mut() else { return };
        // Nothing waits for a release: send it and let the next request that waits drop its reply.
        let mut req = json!({"op":"release","handles":handles});
        let id = self.next_id;
        self.next_id += 1;
        req["id"] = json!(id);
        let body = req.to_string().into_bytes();
        let mut frame = (body.len() as u32).to_le_bytes().to_vec();
        frame.extend_from_slice(&body);
        if p.stdin.write_all(&frame).and_then(|_| p.stdin.flush()).is_ok() {
            self.unacked.push_back(id);
            lock(&self.shared.stats).calls += 1;
        }
    }
    fn close(mut self: Box<Self>) {
        if self.proc.is_some() && self.dead.is_none() {
            let _ = self.raw_request(json!({"op":"bye"}));
        }
        self.kill();
    }
}

// ---- tagged values ------------------------------------------------------------------------

fn guid_text(g: &[u8; 16]) -> String {
    let h: String = g.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32]
    )
}

fn parse_guid(s: &str) -> Option<[u8; 16]> {
    let h: String = s.chars().filter(|c| *c != '-').collect();
    if h.len() != 32 {
        return None;
    }
    let mut g = [0u8; 16];
    for (i, b) in g.iter_mut().enumerate() {
        *b = u8::from_str_radix(&h[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(g)
}

pub fn encode(v: &DotNetValue) -> Value {
    match v {
        DotNetValue::Null => json!({"t":"n"}),
        DotNetValue::NullString => json!({"t":"ns"}),
        DotNetValue::Bool(b) => json!({"t":"b","v":b}),
        DotNetValue::Byte(b) => json!({"t":"u8","v":b}),
        DotNetValue::Char(c) => json!({"t":"c","v":c}),
        DotNetValue::Int32(i) => json!({"t":"i4","v":i}),
        DotNetValue::Int64(i) => json!({"t":"i8","v":i}),
        DotNetValue::Decimal(s) => json!({"t":"d","v":s}),
        DotNetValue::Str(s) => json!({"t":"s","v":s}),
        DotNetValue::DateTime(t) => json!({"t":"dt","v":t.to_string()}),
        DotNetValue::TimeSpan(t) => json!({"t":"ts","v":t.to_string()}),
        DotNetValue::Guid(g) => json!({"t":"g","v":guid_text(g)}),
        DotNetValue::Enum { ty, name, ordinal } => json!({"t":"e","type":ty,"v":name,"o":ordinal}),
        DotNetValue::Handle { handle, ty } => json!({"t":"h","v":handle,"type":ty}),
        DotNetValue::InStream(b) => json!({"t":"ins","v":base64(b)}),
        DotNetValue::OutStream => json!({"t":"outs"}),
    }
}

pub fn decode(v: &Value) -> Result<DotNetValue, String> {
    let tag = v["t"].as_str().ok_or("a value without a tag")?;
    let int = |k: &str| v[k].as_i64().ok_or(format!("tag {tag} without an integer {k}"));
    let text = || v["v"].as_str().ok_or(format!("tag {tag} without a text value"));
    let ticks = || text()?.parse::<i64>().map_err(|e| e.to_string());
    Ok(match tag {
        "n" => DotNetValue::Null,
        "ns" => DotNetValue::NullString,
        "b" => DotNetValue::Bool(v["v"].as_bool().ok_or("bool without a value")?),
        "u8" => DotNetValue::Byte(int("v")? as u8),
        "c" => DotNetValue::Char(int("v")? as u16),
        "i4" => DotNetValue::Int32(int("v")? as i32),
        "i8" => DotNetValue::Int64(int("v")?),
        "d" => DotNetValue::Decimal(text()?.to_string()),
        "s" => DotNetValue::Str(text()?.to_string()),
        "dt" => DotNetValue::DateTime(ticks()?),
        "ts" => DotNetValue::TimeSpan(ticks()?),
        "g" => DotNetValue::Guid(parse_guid(text()?).ok_or("a malformed Guid")?),
        "e" => DotNetValue::Enum {
            ty: v["type"].as_str().unwrap_or("").to_string(),
            name: text()?.to_string(),
            ordinal: int("o")?,
        },
        "h" => DotNetValue::Handle {
            handle: v["v"].as_u64().ok_or("handle without a number")?,
            ty: v["type"].as_str().unwrap_or("").to_string(),
        },
        "ins" => DotNetValue::InStream(unbase64(text()?)),
        other => return Err(format!("unknown value tag {other}")),
    })
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let n =
            (chunk[0] as u32) << 16 | (*chunk.get(1).unwrap_or(&0) as u32) << 8 | *chunk.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(B64[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

pub fn unbase64(text: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let (mut acc, mut bits) = (0u32, 0);
    for c in text.bytes().filter(|c| *c != b'=') {
        let Some(p) = B64.iter().position(|b| *b == c) else {
            continue;
        };
        acc = acc << 6 | p as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits & 0xff) as u8);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_artifact(root: &Path, build: &str, nav: &str) -> PathBuf {
        let svc = root
            .join(build)
            .join("platform/ServiceTier/PFiles64/Microsoft Dynamics NAV")
            .join(nav)
            .join("Service");
        std::fs::create_dir_all(&svc).unwrap();
        svc
    }

    #[test]
    fn probe_dirs_put_the_default_after_the_users_folders() {
        let t = tempfile::tempdir().unwrap();
        let svc = fake_artifact(t.path(), "27.5.1.1", "270");
        let mine = PathBuf::from("/my/dlls");
        assert_eq!(
            probe_dirs(std::slice::from_ref(&mine), Some(t.path()), Some("27.5.1.1"), 27),
            vec![mine.clone(), svc.clone()]
        );
        assert_eq!(
            probe_dirs(std::slice::from_ref(&svc), Some(t.path()), None, 27),
            vec![svc]
        );
        assert_eq!(probe_dirs(std::slice::from_ref(&mine), None, None, 27), vec![mine]);
    }

    #[test]
    fn default_probe_dir_derives_the_service_folder_from_the_major() {
        let t = tempfile::tempdir().unwrap();
        let s27 = fake_artifact(t.path(), "27.5.46862.48827", "270");
        let s28 = fake_artifact(t.path(), "28.1.49838.50621", "280");
        let _old = fake_artifact(t.path(), "28.0.1.1", "280");
        assert_eq!(default_probe_dir(t.path(), None, 27), Some(s27.clone()));
        // The newest build of the major wins when no build is named.
        assert_eq!(default_probe_dir(t.path(), None, 28), Some(s28));
        // A named build wins over the newest one.
        let old = t
            .path()
            .join("28.0.1.1/platform/ServiceTier/PFiles64/Microsoft Dynamics NAV/280/Service");
        assert_eq!(default_probe_dir(t.path(), Some("28.0.1.1"), 28), Some(old));
        assert_eq!(default_probe_dir(t.path(), None, 29), None);
        assert_eq!(default_probe_dir(&t.path().join("none"), None, 27), None);
        assert_eq!(default_probe_dir(t.path(), Some("../x"), 27), Some(s27));
    }

    fn sidecar_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/dotnet-sidecar")
    }

    #[test]
    fn the_built_in_sidecar_sources_are_the_folders_sources() {
        let mut on_disk: Vec<String> = std::fs::read_dir(sidecar_dir())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".cs") || n.ends_with(".csproj") || n == "global.json")
            .collect();
        on_disk.sort();
        let embedded: Vec<&str> = EMBEDDED.iter().map(|(n, _)| *n).collect();
        assert_eq!(on_disk, embedded, "add the new file to EMBEDDED");
        assert_eq!(
            source_hash(&sidecar_dir()),
            Some(embedded_hash()),
            "the binary's copy hashes like the folder, so one cache entry serves both"
        );
    }

    #[test]
    fn a_binary_outside_the_repository_finds_the_helper_in_the_cache_by_its_built_in_hash() {
        let cache = tempfile::tempdir().unwrap();
        let dir = cache.path().join(format!("{:016x}", embedded_hash()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("AlDotNetSidecar.dll"), b"x").unwrap();
        let cfg = SidecarConfig {
            source_root: None,
            cache_dir: Some(cache.path().to_path_buf()),
            ..Default::default()
        };
        // Without the repository and without a built sidecar next to the test binary.
        if std::env::var_os("ALRT_DOTNET_SIDECAR").is_none() {
            assert_eq!(
                find_sidecar(&cfg, Path::new("/nonexistent/dotnet")).unwrap(),
                dir.join("AlDotNetSidecar.dll")
            );
        }
    }

    #[test]
    fn tagged_values_round_trip() {
        for v in [
            DotNetValue::Null,
            DotNetValue::Bool(true),
            DotNetValue::Byte(200),
            DotNetValue::Char(65),
            DotNetValue::Int32(-7),
            DotNetValue::Int64(5_000_000_000),
            DotNetValue::Decimal("1.50".into()),
            DotNetValue::Str("héllo".into()),
            DotNetValue::DateTime(638_396_640_000_000_000),
            DotNetValue::TimeSpan(15_000_000),
            DotNetValue::Guid([0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 1, 2, 3, 4, 5, 6, 7, 8]),
            DotNetValue::Enum {
                ty: "System.X".into(),
                name: "A".into(),
                ordinal: 3,
            },
            DotNetValue::Handle {
                handle: 12,
                ty: "System.IO.MemoryStream".into(),
            },
        ] {
            assert_eq!(decode(&encode(&v)), Ok(v.clone()), "{v:?}");
        }
        assert_eq!(encode(&DotNetValue::OutStream)["t"], "outs");
        assert_eq!(encode(&DotNetValue::InStream(b"abc".to_vec()))["v"], "YWJj");
    }

    #[test]
    fn base64_round_trips_every_length() {
        for n in 0..20u8 {
            let data: Vec<u8> = (0..n).collect();
            assert_eq!(unbase64(&base64(&data)), data);
        }
    }

    #[test]
    fn guids_are_written_in_textual_order() {
        let g = parse_guid("01234567-89ab-cdef-0123-456789abcdef").unwrap();
        assert_eq!(g[0], 0x01);
        assert_eq!(guid_text(&g), "01234567-89ab-cdef-0123-456789abcdef");
    }

    #[test]
    fn runtime_majors_are_read_from_version_text() {
        assert_eq!(major("8.0.30"), Some(8));
        assert_eq!(major("10.0.11"), Some(10));
        assert_eq!(major("x"), None);
    }
}

#[cfg(test)]
mod relative_paths {
    use super::*;

    /// The sidecar process starts in a working folder of its own, so a relative `--dotnet-sidecar` path is made absolute
    /// against the folder the runner was started in (al-runner issue 5709).
    #[test]
    fn a_relative_sidecar_path_is_made_absolute() {
        let cfg = SidecarConfig {
            sidecar: Some(PathBuf::from("Cargo.toml")),
            ..Default::default()
        };
        let p = find_sidecar(&cfg, Path::new("dotnet")).expect("the file exists");
        assert!(p.is_absolute(), "{}", p.display());
        assert_eq!(p, std::env::current_dir().unwrap().join("Cargo.toml"));
    }
}
