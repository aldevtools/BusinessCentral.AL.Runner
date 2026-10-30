//! The BC license that ships with the artifacts of a build (ADR-060 decision 2): `Cronus.bclicense` in the platform
//! artifact (`ServiceTier/.../Service/`) and in the country artifact. It is the default license of a run.
//!
//! The file is read into memory for one run and never written anywhere: no license is cached, shipped or committed
//! (ADR-060 decision 3). Only the platform artifact's central directory, which the dependency loading already keeps in the
//! artifact cache, is reused to find the entry.
use crate::artifact::{Cdn, Fetch};
use crate::build::BuildId;
use crate::cache::Store;
use crate::remote::RemoteZip;
use std::path::{Path, PathBuf};

/// A license file read from the artifacts, and where it came from (for the run report).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactLicense {
    pub bytes: Vec<u8>,
    /// A path of an extracted artifact, or `<artifact url>#<entry>`.
    pub from: String,
}

fn is_license(name: &str) -> bool {
    name.rsplit('/')
        .next()
        .is_some_and(|n| n.to_ascii_lowercase().ends_with(".bclicense"))
}

/// The license files of an extracted artifact of `build` under `root` (BcContainerHelper's layout,
/// `<root>/<build>/<country>/` and `<root>/<build>/platform/ServiceTier/...`), best first.
fn local_files(root: &Path, build: &BuildId, country: &str) -> Vec<PathBuf> {
    let base = root.join(build.as_str());
    let mut out = Vec::new();
    let mut countries = vec![country.to_ascii_lowercase()];
    if !countries.contains(&"w1".to_string()) {
        countries.push("w1".into());
    }
    for c in countries.iter().filter(|c| !c.is_empty()) {
        let mut found: Vec<PathBuf> = std::fs::read_dir(base.join(c))
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file() && is_license(&p.to_string_lossy()))
            .collect();
        found.sort();
        out.extend(found);
    }
    let mut stack = vec![base.join("platform").join("ServiceTier")];
    let mut found = Vec::new();
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if is_license(&p.to_string_lossy()) {
                found.push(p);
            }
        }
    }
    found.sort();
    out.extend(found);
    out
}

/// The license of the artifacts of `build`: from an extracted artifact on disk if `fetch.local_artifacts` holds one,
/// else the entry of the platform artifact, read by range (the central directory from the artifact cache, or from the
/// feed when downloading is allowed). The error says why there is none; the run then has no license (ADR-060 decision
/// 2, "No license available").
pub fn artifact_license<C: Cdn>(cdn: &C, fetch: &Fetch, build: &BuildId) -> Result<ArtifactLicense, String> {
    if let Some(root) = &fetch.local_artifacts
        && let Some(p) = local_files(root, build, &fetch.country).into_iter().next()
    {
        let bytes = std::fs::read(&p).map_err(|e| format!("{}: {e}", p.display()))?;
        return Ok(ArtifactLicense {
            bytes,
            from: p.display().to_string(),
        });
    }
    let (url, src) = cdn.platform(build);
    let bd = Store::new(&fetch.cache_root).build(build);
    let zip = match bd.load_tail() {
        Some((total, tail)) => RemoteZip::from_tail(src, total, tail).map_err(|e| format!("{url}: {e}"))?,
        None if !fetch.download => {
            return Err(format!(
                "the platform artifact of build {build} is not in the artifact cache and downloading is off"
            ));
        }
        None => RemoteZip::open(src).map_err(|e| format!("{url}: {e}"))?,
    };
    let mut entries: Vec<_> = zip.entries.iter().filter(|e| is_license(&e.name)).collect();
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    let entry = entries
        .first()
        .ok_or_else(|| format!("the platform artifact of build {build} holds no .bclicense file"))?;
    let bytes = zip
        .read(entry)
        .map_err(|e| format!("{url}: entry `{}`: {e}", entry.name))?;
    Ok(ArtifactLicense {
        bytes,
        from: format!("{url}#{}", entry.name),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::BuildQuery;
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

    fn build() -> BuildId {
        BuildId::parse("27.5.46862.48827").unwrap()
    }

    fn fetch(cache: &Path, download: bool, local: Option<PathBuf>) -> Fetch {
        Fetch {
            cache_root: cache.to_path_buf(),
            download,
            query: BuildQuery::new(None, 27, None),
            local_artifacts: local,
            country: String::new(),
        }
    }

    #[test]
    fn the_license_entry_of_the_platform_artifact_is_read_and_nothing_is_written() {
        let dir = tempfile::tempdir().unwrap();
        let zip = Builder::new()
            .add("Applications/Base/Base Application.Source.zip", b"app", true)
            .add(
                "ServiceTier/PFiles64/Microsoft Dynamics NAV/270/Service/Test.bclicense",
                b"synthetic",
                true,
            )
            .finish();
        let path = dir.path().join("platform.zip");
        std::fs::write(&path, zip).unwrap();
        let cache = dir.path().join("cache");
        let got = artifact_license(&Platform(path.clone()), &fetch(&cache, true, None), &build()).unwrap();
        assert_eq!(got.bytes, b"synthetic");
        assert!(
            got.from
                .ends_with("#ServiceTier/PFiles64/Microsoft Dynamics NAV/270/Service/Test.bclicense")
        );
        assert!(!cache.exists(), "nothing is written to the cache");
        let off = artifact_license(&Platform(path), &fetch(&cache, false, None), &build()).unwrap_err();
        assert!(off.contains("downloading is off"), "{off}");
    }

    #[test]
    fn an_artifact_without_a_license_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("platform.zip");
        std::fs::write(&path, Builder::new().add("a.txt", b"x", false).finish()).unwrap();
        let e = artifact_license(&Platform(path), &fetch(dir.path(), true, None), &build()).unwrap_err();
        assert!(e.contains("holds no .bclicense"), "{e}");
    }

    #[test]
    fn an_extracted_artifact_on_disk_comes_first() {
        let dir = tempfile::tempdir().unwrap();
        let w1 = dir.path().join("27.5.46862.48827").join("w1");
        std::fs::create_dir_all(&w1).unwrap();
        std::fs::write(w1.join("Test.bclicense"), b"local").unwrap();
        let got = artifact_license(
            &Platform(dir.path().join("missing.zip")),
            &fetch(dir.path(), false, Some(dir.path().to_path_buf())),
            &build(),
        )
        .unwrap();
        assert_eq!(got.bytes, b"local");
        assert!(got.from.ends_with("Test.bclicense"));
    }
}
