//! The artifact cache (ADR-013 decision 3): downloaded entries of Microsoft's artifacts, outside the
//! repository.
//!
//! ```text
//! <root>/artifacts/<build>/platform.tail        last bytes of the artifact: central directory + end record
//!                         /platform.tail.json   {build, url, total, length, crc}
//!                         /apps/<file>          one entry exactly as Microsoft ships it
//!                         /apps/<file>.json     {build, entry, url, size, crc}  (what was downloaded)
//! ```
//!
//! A cached entry is trusted only when its sidecar exists and the file still has the recorded size
//! and CRC-32. Writes go to a temporary name and are renamed, so two processes filling the same
//! cache never see a half-written file.

use crate::build::BuildId;
use std::path::{Path, PathBuf};

/// Where the cache lives, first match wins: `--cache-dir`, `ALRUNNER_CACHE` (ADR-016),
/// `AL_RUNTIME_CACHE` (the name ADR-013 first drafted), `AL_RUNNER_CACHE_ROOT` plus `v3`
/// (AL Runner v2's variable; v3 keeps its data in a subfolder, ADR-016), and `~/.cache/al-runtime`.
pub fn cache_root(flag: Option<&Path>, env: &dyn Fn(&str) -> Option<String>) -> PathBuf {
    chosen_cache_root(flag, env)
        .or_else(|| default_cache_root(env))
        .unwrap_or_else(|| PathBuf::from(".cache").join("al-runtime"))
}

/// The cache root the user chose: `--cache-dir` or one of the cache variables of [`cache_root`]; `None` when
/// neither is given.
pub fn chosen_cache_root(flag: Option<&Path>, env: &dyn Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let var = |k: &str| env(k).filter(|v| !v.trim().is_empty());
    if let Some(f) = flag {
        return Some(f.to_path_buf());
    }
    if let Some(v) = var("ALRUNNER_CACHE").or_else(|| var("AL_RUNTIME_CACHE")) {
        return Some(PathBuf::from(v));
    }
    var("AL_RUNNER_CACHE_ROOT").map(|v| PathBuf::from(v).join("v3"))
}

/// The default cache root of [`cache_root`]: `$XDG_CACHE_HOME/al-runtime`, else `.cache/al-runtime` under `HOME`
/// or, on Windows, which sets no `HOME`, `USERPROFILE`. `None` when none of the three is set.
pub fn default_cache_root(env: &dyn Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let var = |k: &str| env(k).filter(|v| !v.trim().is_empty());
    let base = var("XDG_CACHE_HOME").map(PathBuf::from).or_else(|| {
        var("HOME")
            .or_else(|| var("USERPROFILE"))
            .map(|h| PathBuf::from(h).join(".cache"))
    })?;
    Some(base.join("al-runtime"))
}

/// `cache_root` over the process environment.
pub fn cache_root_from_env(flag: Option<&Path>) -> PathBuf {
    cache_root(flag, &|k| std::env::var(k).ok())
}

/// BcContainerHelper's artifact folder, `~/.bcartifacts.cache/sandbox`, when it exists. The
/// artifacts there are extracted: `<build>/platform/Applications/**` holds the same files as the
/// platform archive the feed serves.
pub fn bcartifacts_sandbox(env: &dyn Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let home = env("HOME")
        .or_else(|| env("USERPROFILE"))
        .filter(|v| !v.trim().is_empty())?;
    let dir = PathBuf::from(home).join(".bcartifacts.cache").join("sandbox");
    dir.is_dir().then_some(dir)
}

/// [`bcartifacts_sandbox`] over the process environment.
pub fn bcartifacts_sandbox_from_env() -> Option<PathBuf> {
    bcartifacts_sandbox(&|k| std::env::var(k).ok())
}

/// The builds of a folder of extracted artifacts (`<root>/<build>/platform`).
pub fn local_builds(root: &Path) -> Vec<BuildId> {
    local_builds_for(root, "w1")
}

/// [`local_builds`] that also have the folder of `country` (`<root>/<build>/<country>`), unless the
/// country is `w1`: the platform artifact serves that one.
pub fn local_builds_for(root: &Path, country: &str) -> Vec<BuildId> {
    let Ok(rd) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let w1 = crate::artifact::is_w1(country);
    let mut out: Vec<BuildId> = rd
        .filter_map(|e| e.ok())
        .filter(|e| e.path().join("platform").is_dir())
        .filter(|e| w1 || e.path().join(country.to_ascii_lowercase()).is_dir())
        .filter_map(|e| BuildId::parse(&e.file_name().to_string_lossy()).ok())
        .collect();
    out.sort();
    out
}

/// The files of an extracted platform artifact that can hold a Microsoft app: everything under
/// `Applications` and `ModernDev` (the platform's `System.app`), as (entry name as the archive
/// would spell it, path). The rest of the artifact (service tier, web client) is not walked.
pub fn local_platform_files(root: &Path, build: &BuildId) -> Vec<(String, PathBuf)> {
    let platform = root.join(build.as_str()).join("platform");
    let mut out = Vec::new();
    for top in ["Applications", "ModernDev"] {
        let mut stack = vec![platform.join(top)];
        while let Some(d) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&d) else { continue };
            for e in rd.filter_map(|e| e.ok()) {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if let Ok(rel) = p.strip_prefix(&platform) {
                    out.push((rel.to_string_lossy().replace('\\', "/"), p));
                }
            }
        }
    }
    out.sort();
    out
}

/// The files of an extracted country artifact that hold that country's Microsoft apps:
/// `<root>/<build>/<country>/Applications.<COUNTRY>/*`, as (entry name as the archive would spell it,
/// path). Empty when the folder is not there.
pub fn local_country_files(root: &Path, build: &BuildId, country: &str) -> Vec<(String, PathBuf)> {
    let folder = crate::artifact::applications_folder(country);
    let dir = root
        .join(build.as_str())
        .join(country.to_ascii_lowercase())
        .join(&folder);
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut out: Vec<(String, PathBuf)> = rd
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file())
        .filter_map(|p| Some((format!("{folder}/{}", p.file_name()?.to_string_lossy()), p)))
        .collect();
    out.sort();
    out
}

pub struct Store {
    artifacts: PathBuf,
}

impl Store {
    pub fn new(root: &Path) -> Store {
        Store {
            artifacts: root.join("artifacts"),
        }
    }

    pub fn dir(&self) -> &Path {
        &self.artifacts
    }

    /// Builds that have a saved central directory. Directories that are not named like a build, and
    /// builds whose directory has nothing saved yet, are not listed.
    pub fn list_builds(&self) -> Vec<BuildId> {
        let Ok(rd) = std::fs::read_dir(&self.artifacts) else {
            return Vec::new();
        };
        let mut out: Vec<BuildId> = rd
            .filter_map(|e| e.ok())
            .filter(|e| e.path().join("platform.tail").is_file())
            .filter_map(|e| BuildId::parse(&e.file_name().to_string_lossy()).ok())
            .collect();
        out.sort();
        out
    }

    pub fn build(&self, b: &BuildId) -> BuildDir {
        BuildDir {
            dir: self.artifacts.join(b.as_str()),
            build: b.clone(),
        }
    }

    /// Where the Microsoft apps of a country other than `w1` are cached: a folder of their own
    /// (`<build>/country-<country>/`), because they have the file names of the platform's apps
    /// (`Base Application.Source.zip`) and other contents. It keeps the country artifact's central
    /// directory under the same file names a build folder uses for the platform's.
    pub fn country_build(&self, b: &BuildId, country: &str) -> BuildDir {
        BuildDir {
            dir: self
                .artifacts
                .join(b.as_str())
                .join(format!("country-{}", country.to_ascii_lowercase())),
            build: b.clone(),
        }
    }
}

pub struct BuildDir {
    dir: PathBuf,
    build: BuildId,
}

/// What the sidecar of a cached entry records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recorded {
    pub entry: String,
    pub size: u64,
    pub crc: u32,
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("no parent directory")?;
    std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    let tmp = path.with_extension(format!("tmp{}", std::process::id()));
    std::fs::write(&tmp, bytes).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

/// A file name for an archive entry: its last path segment, made safe. `Applications/X/Y.Source.zip`
/// is stored as `Y.Source.zip`.
fn file_name_of(entry: &str) -> String {
    let base = entry.rsplit('/').next().unwrap_or(entry);
    let safe: String = base
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .collect();
    if safe.is_empty() || safe == "." || safe == ".." {
        "entry".to_string()
    } else {
        safe
    }
}

impl BuildDir {
    pub fn path(&self) -> &Path {
        &self.dir
    }

    /// The directory of cached entries; a search directory for the dependency pool.
    pub fn apps_dir(&self) -> PathBuf {
        self.dir.join("apps")
    }

    fn tail_path(&self) -> PathBuf {
        self.dir.join("platform.tail")
    }

    pub fn save_tail(&self, url: &str, total: u64, tail: &[u8]) -> Result<(), String> {
        let meta = serde_json::json!({
            "build": self.build.as_str(), "url": url, "total": total,
            "length": tail.len(), "crc": crc32fast::hash(tail),
        });
        write_atomic(&self.tail_path(), tail)?;
        write_atomic(&self.dir.join("platform.tail.json"), meta.to_string().as_bytes())
    }

    /// The saved central directory, or `None` when absent or damaged (it is fetched again then).
    pub fn load_tail(&self) -> Option<(u64, Vec<u8>)> {
        let meta: serde_json::Value =
            serde_json::from_slice(&std::fs::read(self.dir.join("platform.tail.json")).ok()?).ok()?;
        let tail = std::fs::read(self.tail_path()).ok()?;
        let ok =
            meta["length"].as_u64()? == tail.len() as u64 && meta["crc"].as_u64()? == crc32fast::hash(&tail) as u64;
        ok.then(|| (meta["total"].as_u64().unwrap_or(0), tail))
    }

    /// Saves one entry's bytes (already inflated and CRC-checked by the caller) and records them.
    pub fn put_entry(&self, entry: &str, url: &str, bytes: &[u8]) -> Result<PathBuf, String> {
        self.put_entry_in("apps", entry, url, bytes)
    }

    /// [`BuildDir::put_entry`] into another folder of the build: a file that is not an app (the RapidStart package)
    /// stays out of the folder the dependency pool searches.
    pub fn put_entry_in(&self, folder: &str, entry: &str, url: &str, bytes: &[u8]) -> Result<PathBuf, String> {
        let name = file_name_of(entry);
        let path = self.dir.join(folder).join(&name);
        let crc = crc32fast::hash(bytes);
        let meta = serde_json::json!({
            "build": self.build.as_str(), "entry": entry, "url": url, "size": bytes.len(), "crc": crc,
        });
        // The file first, then its sidecar: a crash in between leaves a file nobody trusts.
        write_atomic(&path, bytes)?;
        write_atomic(&sidecar(&path), meta.to_string().as_bytes())?;
        Ok(path)
    }

    /// The cached copy of one archive entry, when it is there, intact and recorded as that entry.
    pub fn cached(&self, entry: &str) -> Option<PathBuf> {
        self.cached_in("apps", entry)
    }

    /// [`BuildDir::cached`] in another folder of the build (`packages` for the RapidStart package).
    pub fn cached_in(&self, folder: &str, entry: &str) -> Option<PathBuf> {
        let path = self.dir.join(folder).join(file_name_of(entry));
        let r = self.check(&path).ok()?;
        (r.entry == entry).then_some(path)
    }

    /// The entries that are cached and intact. Files that are damaged, or have no sidecar, are
    /// removed and named in the returned notes: the cache is ours, and a bad file would only be
    /// read as a package and fail later.
    pub fn verify(&self) -> (Vec<Recorded>, Vec<String>) {
        let mut ok = Vec::new();
        let mut notes = Vec::new();
        let Ok(rd) = std::fs::read_dir(self.apps_dir()) else {
            return (ok, notes);
        };
        let mut files: Vec<PathBuf> = rd
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x != "json") && !is_temp(p))
            .collect();
        files.sort();
        for f in files {
            match self.check(&f) {
                Ok(r) => ok.push(r),
                Err(why) => {
                    notes.push(format!("cache: removed {} ({why}); it is fetched again", f.display()));
                    let _ = std::fs::remove_file(&f);
                    let _ = std::fs::remove_file(sidecar(&f));
                }
            }
        }
        (ok, notes)
    }

    fn check(&self, file: &Path) -> Result<Recorded, String> {
        let meta: serde_json::Value = std::fs::read(sidecar(file))
            .map_err(|_| "no record of what was downloaded".to_string())
            .and_then(|b| serde_json::from_slice(&b).map_err(|e| format!("unreadable record: {e}")))?;
        let bytes = std::fs::read(file).map_err(|e| e.to_string())?;
        let size = meta["size"].as_u64().ok_or("record has no size")?;
        let crc = meta["crc"].as_u64().ok_or("record has no CRC-32")?;
        if bytes.len() as u64 != size || crc32fast::hash(&bytes) as u64 != crc {
            return Err(format!(
                "it no longer matches its recorded size {size} and CRC-32 {crc:08x}"
            ));
        }
        Ok(Recorded {
            entry: meta["entry"].as_str().unwrap_or_default().to_string(),
            size,
            crc: crc as u32,
        })
    }
}

fn is_temp(p: &Path) -> bool {
    p.extension().is_some_and(|x| x.to_string_lossy().starts_with("tmp"))
}

fn sidecar(file: &Path) -> PathBuf {
    let mut n = file.as_os_str().to_owned();
    n.push(".json");
    PathBuf::from(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |k| pairs.iter().find(|(n, _)| *n == k).map(|(_, v)| v.to_string())
    }

    #[test]
    fn the_cache_root_follows_flag_then_alrunner_cache_then_the_older_names_then_home() {
        let e = env(&[
            ("ALRUNNER_CACHE", "/a"),
            ("AL_RUNTIME_CACHE", "/b"),
            ("AL_RUNNER_CACHE_ROOT", "/c"),
            ("HOME", "/h"),
        ]);
        assert_eq!(cache_root(Some(Path::new("/flag")), &e), PathBuf::from("/flag"));
        assert_eq!(cache_root(None, &e), PathBuf::from("/a"));
        let e = env(&[
            ("AL_RUNTIME_CACHE", "/b"),
            ("AL_RUNNER_CACHE_ROOT", "/c"),
            ("HOME", "/h"),
        ]);
        assert_eq!(cache_root(None, &e), PathBuf::from("/b"));
        // v2's variable: v3 writes in a subfolder, never into v2's own folders.
        let e = env(&[("AL_RUNNER_CACHE_ROOT", "/c"), ("HOME", "/h")]);
        assert_eq!(cache_root(None, &e), PathBuf::from("/c/v3"));
        let e = env(&[("HOME", "/h")]);
        assert_eq!(cache_root(None, &e), PathBuf::from("/h/.cache/al-runtime"));
        let e = env(&[("XDG_CACHE_HOME", "/x"), ("HOME", "/h")]);
        assert_eq!(cache_root(None, &e), PathBuf::from("/x/al-runtime"));
        // An empty variable is unset.
        let e = env(&[("ALRUNNER_CACHE", " "), ("HOME", "/h")]);
        assert_eq!(cache_root(None, &e), PathBuf::from("/h/.cache/al-runtime"));
    }

    fn b(s: &str) -> BuildId {
        BuildId::parse(s).unwrap()
    }

    #[test]
    fn only_builds_with_a_saved_directory_are_listed_and_the_layout_is_stable() {
        let root = tempfile::tempdir().unwrap();
        let store = Store::new(root.path());
        assert!(store.list_builds().is_empty());
        let bd = store.build(&b("27.5.46862.56249"));
        bd.save_tail("https://x/27.5.46862.56249/platform", 100, b"tail-bytes")
            .unwrap();
        std::fs::create_dir_all(store.dir().join("not-a-build")).unwrap();
        std::fs::create_dir_all(store.dir().join("28.1.1.1")).unwrap(); // nothing saved in it
        assert_eq!(store.list_builds(), [b("27.5.46862.56249")]);
        assert!(root.path().join("artifacts/27.5.46862.56249/platform.tail").is_file());
        assert!(
            root.path()
                .join("artifacts/27.5.46862.56249/platform.tail.json")
                .is_file()
        );
        assert_eq!(bd.load_tail(), Some((100, b"tail-bytes".to_vec())));
    }

    #[test]
    fn a_damaged_tail_is_not_trusted() {
        let root = tempfile::tempdir().unwrap();
        let bd = Store::new(root.path()).build(&b("27.5.1.1"));
        bd.save_tail("u", 10, b"abcdef").unwrap();
        std::fs::write(bd.path().join("platform.tail"), b"abcdeX").unwrap();
        assert_eq!(bd.load_tail(), None);
    }

    #[test]
    fn an_entry_is_stored_under_its_last_segment_with_a_record_of_size_and_crc() {
        let root = tempfile::tempdir().unwrap();
        let bd = Store::new(root.path()).build(&b("27.5.1.1"));
        let p = bd
            .put_entry("Applications/Lib/Any.Source.zip", "https://x/platform", b"zipbytes")
            .unwrap();
        assert_eq!(p, bd.apps_dir().join("Any.Source.zip"));
        let record: serde_json::Value =
            serde_json::from_slice(&std::fs::read(bd.apps_dir().join("Any.Source.zip.json")).unwrap()).unwrap();
        assert_eq!(record["build"], "27.5.1.1");
        assert_eq!(record["entry"], "Applications/Lib/Any.Source.zip");
        assert_eq!(record["size"], 8);
        assert_eq!(record["crc"], crc32fast::hash(b"zipbytes"));
        let (ok, notes) = bd.verify();
        assert_eq!(ok.len(), 1);
        assert_eq!(ok[0].entry, "Applications/Lib/Any.Source.zip");
        assert!(notes.is_empty());
    }

    #[test]
    fn a_changed_or_unrecorded_file_is_removed_and_reported() {
        let root = tempfile::tempdir().unwrap();
        let bd = Store::new(root.path()).build(&b("27.5.1.1"));
        let p = bd.put_entry("A/Good.Source.zip", "u", b"good").unwrap();
        let bad = bd.put_entry("A/Bad.Source.zip", "u", b"original").unwrap();
        std::fs::write(&bad, b"tampered").unwrap();
        std::fs::write(bd.apps_dir().join("Orphan.Source.zip"), b"no record").unwrap();
        let (ok, notes) = bd.verify();
        assert_eq!(ok.len(), 1);
        assert!(p.exists() && !bad.exists());
        assert!(!bd.apps_dir().join("Orphan.Source.zip").exists());
        assert_eq!(notes.len(), 2, "{notes:?}");
        assert!(
            notes
                .iter()
                .any(|n| n.contains("Bad.Source.zip") && n.contains("CRC-32"))
        );
    }

    #[test]
    fn entry_names_cannot_escape_the_cache_directory() {
        assert_eq!(file_name_of("a/b/../c.zip"), "c.zip");
        assert_eq!(file_name_of("x/.."), "entry");
        assert_eq!(file_name_of("dir/we:ird*name.zip"), "we_ird_name.zip");
        assert_eq!(file_name_of(""), "entry");
    }
}
