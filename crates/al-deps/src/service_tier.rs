//! The BC service-tier assemblies a DotNet declaration needs (ADR-014 section 3, step 2), fetched by range from the
//! platform artifact into the artifact cache when no extracted artifact is on disk (al-runner issue 6924).
//!
//! ```text
//! <root>/artifacts/<build>/service/<Name>.dll   one entry of `ServiceTier/.../Service/` (or its `Add-ins/`), as shipped
//! ```
//!
//! Only what is asked for is fetched: the named assembly and the service-tier assemblies it references, followed
//! through their `AssemblyRef` tables ([`crate::assembly_refs`]). A name the service folder does not hold (the .NET
//! standard library) is skipped. The files come from Microsoft's artifact feed to the user's machine, like the apps;
//! nothing is committed (hard rule 7). A file is written under a temporary name and renamed, and a file already in
//! the folder with the size the archive records is not fetched again.
use crate::artifact::Cdn;
use crate::build::BuildId;
use crate::cache::Store;
use crate::remote::RemoteZip;
use crate::zip::Entry;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The folder the service-tier assemblies of `build` are fetched into; a probe folder of the sidecar.
pub fn service_dir(cache_root: &Path, build: &BuildId) -> PathBuf {
    Store::new(cache_root).build(build).path().join("service")
}

/// What [`ServiceTier::ensure`] wrote.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Fetched {
    /// The file names fetched by this call.
    pub files: Vec<String>,
    /// Their inflated size in bytes.
    pub bytes: u64,
    /// The names of a set ([`ServiceTier::ensure_set`]) the service folder does not hold.
    pub missing: Vec<String>,
}

/// Fetches service-tier assemblies of one build on demand. Safe to share between threads.
pub struct ServiceTier<C: Cdn> {
    cdn: C,
    cache_root: PathBuf,
    build: BuildId,
    download: bool,
    state: Mutex<State<C::Src>>,
}

struct State<S> {
    /// The service folder's entries by lower-case simple name; `None` until the archive is first needed.
    index: Option<Result<Index<S>, String>>,
    /// Lower-case names already followed.
    done: HashSet<String>,
}

struct Index<S> {
    zip: RemoteZip<S>,
    by_name: BTreeMap<String, usize>,
}

/// `ServiceTier/PFiles64/Microsoft Dynamics NAV/<NNN>/Service/` plus the rest of the entry, when `name` is in it.
fn in_service(name: &str) -> Option<&str> {
    let rest = name.strip_prefix("ServiceTier/")?;
    let at = rest.find("/Service/")?;
    Some(&rest[at + "/Service/".len()..])
}

/// The lower-case simple name an entry serves, when it is an assembly of the service folder's top level or of its
/// `Add-ins` folder (any depth); with whether it is in the top level.
fn served_name(entry: &str) -> Option<(String, bool)> {
    let rest = in_service(entry)?;
    let lower = rest.to_ascii_lowercase();
    let top = !lower.contains('/');
    if !top && !lower.starts_with("add-ins/") {
        return None;
    }
    let file = lower.rsplit('/').next()?;
    let simple = file.strip_suffix(".dll").or_else(|| file.strip_suffix(".exe"))?;
    Some((simple.to_string(), top))
}

fn file_name(entry: &str) -> &str {
    entry.rsplit('/').next().unwrap_or(entry)
}

impl<C: Cdn> ServiceTier<C> {
    pub fn new(cdn: C, cache_root: &Path, build: BuildId, download: bool) -> Self {
        ServiceTier {
            cdn,
            cache_root: cache_root.to_path_buf(),
            build,
            download,
            state: Mutex::new(State {
                index: None,
                done: HashSet::new(),
            }),
        }
    }

    /// The folder this fetches into.
    pub fn dir(&self) -> PathBuf {
        service_dir(&self.cache_root, &self.build)
    }

    /// Makes `assembly` (a simple name) and the service-tier assemblies it references present in [`Self::dir`]. A name
    /// the service folder does not hold is no error: the runtime serves it, or the sidecar says it is missing.
    pub fn ensure(&self, assembly: &str) -> Result<Fetched, String> {
        let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let dir = self.dir();
        let mut out = Fetched::default();
        let mut todo = vec![assembly.to_ascii_lowercase()];
        while let Some(name) = todo.pop() {
            if !st.done.insert(name.clone()) {
                continue;
            }
            let bytes = match self.local(&dir, &name, &mut st) {
                Some(b) => b,
                None => match self.fetch(&dir, &name, &mut st) {
                    Ok(Some((file, b))) => {
                        out.bytes += b.len() as u64;
                        out.files.push(file);
                        b
                    }
                    Ok(None) => continue,
                    Err(e) => {
                        st.done.remove(&name);
                        return Err(e);
                    }
                },
            };
            let refs = crate::assembly_refs::assembly_refs(&bytes).unwrap_or_default();
            todo.extend(refs.into_iter().map(|r| r.to_ascii_lowercase()));
        }
        Ok(out)
    }

    /// Makes exactly the files `names` (simple names) present in [`Self::dir`], following no reference: a named set of
    /// the report render process ([`crate::render_set`]). A file already there with the size the archive records is not
    /// fetched again. A name the service folder does not hold is listed in [`Fetched::missing`].
    pub fn ensure_set(&self, names: &[&str]) -> Result<Fetched, String> {
        let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let dir = self.dir();
        let mut out = Fetched::default();
        for name in names {
            let lower = name.to_ascii_lowercase();
            if self.present(&dir, &lower, &mut st)? {
                continue;
            }
            match self.fetch(&dir, &lower, &mut st)? {
                Some((file, b)) => {
                    out.bytes += b.len() as u64;
                    out.files.push(file);
                }
                None => out.missing.push((*name).to_string()),
            }
        }
        Ok(out)
    }

    /// Whether `name`'s file is in `dir` with the size the archive records. Opens the archive's index (from the cache)
    /// only when a file is there, to compare the size; a name the index lacks is not present.
    fn present(&self, dir: &Path, name: &str, st: &mut State<C::Src>) -> Result<bool, String> {
        let Some(len) = std::fs::read_dir(dir).ok().and_then(|rd| {
            rd.flatten().find_map(|e| {
                let f = e.file_name().to_string_lossy().to_ascii_lowercase();
                (f == format!("{name}.dll") || f == format!("{name}.exe"))
                    .then(|| e.metadata().ok().map(|m| m.len()))?
            })
        }) else {
            return Ok(false);
        };
        let index = st
            .index
            .get_or_insert_with(|| self.open())
            .as_ref()
            .map_err(Clone::clone)?;
        Ok(index
            .by_name
            .get(name)
            .is_some_and(|&i| index.zip.entries[i].size as u64 == len))
    }

    /// The bytes of `name` when its file is already in `dir` and, if the archive is open, has the recorded size.
    fn local(&self, dir: &Path, name: &str, st: &mut State<C::Src>) -> Option<Vec<u8>> {
        let path = std::fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).find(|p| {
            p.file_name()
                .map(|f| f.to_string_lossy().to_ascii_lowercase())
                .is_some_and(|f| f == format!("{name}.dll") || f == format!("{name}.exe"))
        })?;
        let bytes = std::fs::read(&path).ok()?;
        if let Some(Ok(index)) = &st.index
            && let Some(&i) = index.by_name.get(name)
            && index.zip.entries[i].size != bytes.len()
        {
            return None;
        }
        Some(bytes)
    }

    /// Reads `name`'s entry from the platform artifact and writes it into `dir`; `None` when the service folder has no
    /// such assembly.
    fn fetch(&self, dir: &Path, name: &str, st: &mut State<C::Src>) -> Result<Option<(String, Vec<u8>)>, String> {
        let index = st
            .index
            .get_or_insert_with(|| self.open())
            .as_ref()
            .map_err(Clone::clone)?;
        let Some(&i) = index.by_name.get(name) else {
            return Ok(None);
        };
        let entry: &Entry = &index.zip.entries[i];
        let (url, _) = self.cdn.platform(&self.build);
        let bytes = index
            .zip
            .read(entry)
            .map_err(|e| format!("{url}: entry `{}`: {e}", entry.name))?;
        let file = file_name(&entry.name).to_string();
        write_atomic(&dir.join(&file), &bytes)?;
        Ok(Some((file, bytes)))
    }

    /// The platform artifact's central directory (from the artifact cache, else from the feed when downloading is on),
    /// indexed by the names the service folder serves; a top-level file wins over an `Add-ins` one.
    fn open(&self) -> Result<Index<C::Src>, String> {
        let (url, src) = self.cdn.platform(&self.build);
        let bd = Store::new(&self.cache_root).build(&self.build);
        let zip = match bd.load_tail() {
            Some((total, tail)) => RemoteZip::from_tail(src, total, tail).map_err(|e| format!("{url}: {e}"))?,
            None if !self.download => {
                return Err(format!(
                    "the platform artifact of build {} is not in the artifact cache and downloading is off",
                    self.build
                ));
            }
            None => {
                let zip = RemoteZip::open(src).map_err(|e| format!("{url}: {e}"))?;
                let (total, tail) = zip.tail();
                // Kept like the dependency loading keeps it, so the next run needs no request for it.
                let _ = bd.save_tail(&url, total, tail);
                zip
            }
        };
        let mut by_name: BTreeMap<String, (bool, usize)> = BTreeMap::new();
        for (i, e) in zip.entries.iter().enumerate() {
            if let Some((name, top)) = served_name(&e.name) {
                let keep = match by_name.get(&name) {
                    Some(&(had_top, j)) => (top && !had_top) || (top == had_top && e.name < zip.entries[j].name),
                    None => true,
                };
                if keep {
                    by_name.insert(name, (top, i));
                }
            }
        }
        Ok(Index {
            zip,
            by_name: by_name.into_iter().map(|(k, (_, i))| (k, i)).collect(),
        })
    }
}

pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("no parent directory")?;
    std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    let tmp = path.with_extension(format!("tmp{}", std::process::id()));
    std::fs::write(&tmp, bytes).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assembly_refs::testutil::assembly;
    use crate::remote::{FileSource, RemoteError};
    use crate::zip::testutil::Builder;

    struct Platform(PathBuf);

    impl Cdn for Platform {
        type Src = FileSource;
        fn base_url(&self) -> String {
            "https://cdn.test".into()
        }
        fn index(&self) -> Result<Vec<u8>, RemoteError> {
            panic!("the index is not needed")
        }
        fn platform(&self, build: &BuildId) -> (String, FileSource) {
            (format!("https://cdn.test/{build}/platform"), FileSource(self.0.clone()))
        }
        fn country(&self, _: &BuildId, _: &str) -> (String, FileSource) {
            panic!("the country artifact is not needed")
        }
    }

    const SVC: &str = "ServiceTier/PFiles64/Microsoft Dynamics NAV/270/Service";

    fn build() -> BuildId {
        BuildId::parse("27.5.46862.48827").unwrap()
    }

    /// A platform artifact whose service folder holds a small reference graph; nothing here is Microsoft's.
    fn platform(dir: &Path) -> PathBuf {
        let zip = Builder::new()
            .add("Applications/Base/Base Application.Source.zip", b"app", true)
            .add(
                &format!("{SVC}/Microsoft.Dynamics.Nav.OpenXml.dll"),
                &assembly(&[
                    "System.Runtime",
                    "DocumentFormat.OpenXml",
                    "Microsoft.Dynamics.Nav.Types.Report",
                ]),
                true,
            )
            .add(
                &format!("{SVC}/DocumentFormat.OpenXml.dll"),
                &assembly(&["System.Runtime", "DocumentFormat.OpenXml.Framework"]),
                true,
            )
            .add(
                &format!("{SVC}/DocumentFormat.OpenXml.Framework.dll"),
                &assembly(&["System.Runtime"]),
                false,
            )
            .add(
                &format!("{SVC}/Microsoft.Dynamics.Nav.Types.Report.dll"),
                &assembly(&[]),
                true,
            )
            .add(&format!("{SVC}/Microsoft.Dynamics.Nav.Ncl.dll"), &assembly(&[]), true)
            .add(&format!("{SVC}/Add-ins/QR/QrProvider.dll"), &assembly(&[]), true)
            .add(
                &format!("{SVC}/Add-ins/Other/Microsoft.Dynamics.Nav.Ncl.dll"),
                b"shadowed",
                true,
            )
            .finish();
        let path = dir.join("platform.zip");
        std::fs::write(&path, zip).unwrap();
        path
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    }

    #[test]
    fn a_declared_assembly_and_the_service_assemblies_it_references_are_fetched_and_nothing_else() {
        let t = tempfile::tempdir().unwrap();
        let cache = t.path().join("cache");
        let tier = ServiceTier::new(Platform(platform(t.path())), &cache, build(), true);
        let got = tier.ensure("Microsoft.Dynamics.Nav.OpenXml").unwrap();
        assert_eq!(tier.dir(), cache.join("artifacts/27.5.46862.48827/service"));
        assert_eq!(
            names(&tier.dir()),
            vec![
                "DocumentFormat.OpenXml.Framework.dll",
                "DocumentFormat.OpenXml.dll",
                "Microsoft.Dynamics.Nav.OpenXml.dll",
                "Microsoft.Dynamics.Nav.Types.Report.dll",
            ]
        );
        assert_eq!(got.files.len(), 4);
        assert!(got.bytes > 0);
        // The central directory is kept, so a later run reads no tail from the feed.
        assert!(Store::new(&cache).build(&build()).load_tail().is_some());
        // Asked again, nothing is fetched.
        assert_eq!(
            tier.ensure("Microsoft.Dynamics.Nav.OpenXml").unwrap(),
            Fetched::default()
        );
        // A new process finds the files on disk and fetches nothing either, even offline.
        let again = ServiceTier::new(Platform(t.path().join("gone.zip")), &cache, build(), false);
        assert_eq!(
            again.ensure("Microsoft.Dynamics.Nav.OpenXml").unwrap(),
            Fetched::default()
        );
    }

    #[test]
    fn a_named_set_fetches_exactly_its_files_and_follows_no_reference() {
        let t = tempfile::tempdir().unwrap();
        let tier = ServiceTier::new(Platform(platform(t.path())), &t.path().join("c"), build(), true);
        let got = tier
            .ensure_set(&["Microsoft.Dynamics.Nav.OpenXml", "QrProvider", "Not.In.The.Folder"])
            .unwrap();
        assert_eq!(
            names(&tier.dir()),
            vec!["Microsoft.Dynamics.Nav.OpenXml.dll", "QrProvider.dll"]
        );
        assert_eq!(got.files.len(), 2);
        assert_eq!(got.missing, vec!["Not.In.The.Folder".to_string()]);
        // Asked again, nothing is fetched; the missing name is still reported.
        let again = tier
            .ensure_set(&["Microsoft.Dynamics.Nav.OpenXml", "Not.In.The.Folder"])
            .unwrap();
        assert!(again.files.is_empty());
        assert_eq!(again.missing, vec!["Not.In.The.Folder".to_string()]);
    }

    #[test]
    fn add_ins_are_served_flat_and_a_top_level_file_wins_over_an_add_in() {
        let t = tempfile::tempdir().unwrap();
        let tier = ServiceTier::new(Platform(platform(t.path())), &t.path().join("c"), build(), true);
        tier.ensure("QrProvider").unwrap();
        tier.ensure("Microsoft.Dynamics.Nav.Ncl").unwrap();
        assert_eq!(
            names(&tier.dir()),
            vec!["Microsoft.Dynamics.Nav.Ncl.dll", "QrProvider.dll"]
        );
        let ncl = std::fs::read(tier.dir().join("Microsoft.Dynamics.Nav.Ncl.dll")).unwrap();
        assert_ne!(ncl, b"shadowed");
    }

    #[test]
    fn a_name_outside_the_service_folder_is_skipped_and_offline_without_a_cached_tail_says_so() {
        let t = tempfile::tempdir().unwrap();
        let tier = ServiceTier::new(Platform(platform(t.path())), &t.path().join("c"), build(), true);
        assert_eq!(tier.ensure("System.Text.Json").unwrap(), Fetched::default());
        assert!(!tier.dir().exists());
        let off = ServiceTier::new(Platform(platform(t.path())), &t.path().join("d"), build(), false);
        let e = off.ensure("Microsoft.Dynamics.Nav.OpenXml").unwrap_err();
        assert!(e.contains("downloading is off"), "{e}");
    }
}
