//! The demo database backup of a build (ADR-020): `BusinessCentral-<COUNTRY>.bak` inside the
//! country artifact (`<feed>/<build>/w1`). The entry is about 520 MB stored and 900 MB inflated,
//! so it is streamed to a file in the artifact cache instead of held in memory, and checked
//! against the size and CRC-32 of the central directory before it is put in place.
//!
//! The central directory is read by range first (a few hundred kilobytes), exactly as for the
//! platform artifact.
//!
//! The demo database backup of a build: streamed from the country artifact into the artifact cache
//! (ADR-020). Entry points: `ensure_demo_backup`, `cached_backup`, `DemoBackup`.

use crate::artifact::Cdn;
use crate::build::BuildId;
use crate::cache::Store;
use crate::remote::{RemoteError, RemoteZip};
use std::io::Write;
use std::path::{Path, PathBuf};

/// The file name of the backup in a country artifact: `BusinessCentral-W1.bak`.
pub fn backup_name(country: &str) -> String {
    format!("BusinessCentral-{}.bak", country.to_ascii_uppercase())
}

/// Where the backup of `build` and `country` lies in the artifact cache.
pub fn cached_backup(cache_root: &Path, build: &BuildId, country: &str) -> PathBuf {
    Store::new(cache_root)
        .build(build)
        .path()
        .join("demo")
        .join(country.to_ascii_lowercase())
        .join(backup_name(country))
}

/// What [`ensure_demo_backup`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DemoBackup {
    pub path: PathBuf,
    /// True when this call downloaded it, false when the cache already held it.
    pub downloaded: bool,
}

/// The backup of `build` in the cache, downloaded first when it is not there. `progress` gets
/// (stored bytes read, stored bytes in all) while the download runs. A failure leaves no file
/// behind that a later run could take for a complete backup.
pub fn ensure_demo_backup<C: Cdn>(
    cdn: &C,
    cache_root: &Path,
    build: &BuildId,
    country: &str,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<DemoBackup, String> {
    let dest = cached_backup(cache_root, build, country);
    if dest.is_file() {
        return Ok(DemoBackup {
            path: dest,
            downloaded: false,
        });
    }
    let (url, src) = cdn.country(build, country);
    let zip = RemoteZip::open(src).map_err(|e| format!("{url}: {e}"))?;
    let wanted = backup_name(country);
    let entry = zip
        .entries
        .iter()
        .find(|e| {
            e.name
                .rsplit('/')
                .next()
                .is_some_and(|n| n.eq_ignore_ascii_case(&wanted))
        })
        .ok_or_else(|| format!("{url} holds no {wanted}"))?;
    let dir = dest.parent().ok_or("no cache folder")?;
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let part = dest.with_extension(format!("bak.part{}", std::process::id()));
    let total = entry.compressed_size() as u64;
    let result = (|| -> Result<(), String> {
        let mut file = std::fs::File::create(&part).map_err(|e| format!("{}: {e}", part.display()))?;
        let io = |e: std::io::Error| RemoteError::Io(format!("{}: {e}", part.display()));
        zip.stream_entry(entry, &mut |bytes| file.write_all(bytes).map_err(io), &mut |read| {
            progress(read, total)
        })
        .map_err(|e| format!("{url}: {e}"))?;
        file.flush().map_err(|e| format!("{}: {e}", part.display()))?;
        std::fs::rename(&part, &dest).map_err(|e| format!("{}: {e}", dest.display()))
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    result?;
    Ok(DemoBackup {
        path: dest,
        downloaded: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::FileSource;
    use crate::zip::testutil::Builder;
    use std::cell::Cell;

    struct Country {
        zip: PathBuf,
        calls: Cell<u32>,
    }

    impl Cdn for Country {
        type Src = FileSource;
        fn base_url(&self) -> String {
            "https://cdn.test".into()
        }
        fn index(&self) -> Result<Vec<u8>, RemoteError> {
            panic!("the index is not needed for a build that is already chosen")
        }
        fn platform(&self, _: &BuildId) -> (String, FileSource) {
            panic!("the platform artifact is not needed")
        }
        fn country(&self, build: &BuildId, country: &str) -> (String, FileSource) {
            self.calls.set(self.calls.get() + 1);
            (
                format!("https://cdn.test/{build}/{country}"),
                FileSource(self.zip.clone()),
            )
        }
    }

    fn build() -> BuildId {
        BuildId::parse("27.5.46862.48827").unwrap()
    }

    /// A pretend backup: compressible, longer than one read piece of the streaming code.
    fn backup_bytes() -> Vec<u8> {
        (0..3_000_000u32).map(|i| (i % 251) as u8).collect()
    }

    fn country_zip(dir: &Path, deflate: bool) -> Country {
        let zip = Builder::new()
            .add("Cronus.bclicense", b"license", true)
            .add("BusinessCentral-W1.bak", &backup_bytes(), deflate)
            .finish();
        let path = dir.join("w1.zip");
        std::fs::write(&path, zip).unwrap();
        Country {
            zip: path,
            calls: Cell::new(0),
        }
    }

    #[test]
    fn the_backup_is_streamed_into_the_cache_once() {
        for deflate in [true, false] {
            let dir = tempfile::tempdir().unwrap();
            let cdn = country_zip(dir.path(), deflate);
            let cache = dir.path().join("cache");
            let mut last = (0, 0);
            let got = ensure_demo_backup(&cdn, &cache, &build(), "w1", &mut |a, b| last = (a, b)).unwrap();
            assert!(got.downloaded);
            assert_eq!(got.path, cached_backup(&cache, &build(), "w1"));
            assert!(
                got.path.ends_with("27.5.46862.48827/demo/w1/BusinessCentral-W1.bak"),
                "{:?}",
                got.path
            );
            assert_eq!(std::fs::read(&got.path).unwrap(), backup_bytes());
            assert_eq!(last.0, last.1, "progress ends at the stored size");
            assert!(last.1 > 0);
            // The second call finds it and asks nobody.
            let again = ensure_demo_backup(&cdn, &cache, &build(), "w1", &mut |_, _| {}).unwrap();
            assert!(!again.downloaded);
            assert_eq!(cdn.calls.get(), 1);
            // Nothing else is left in the folder.
            let names: Vec<_> = std::fs::read_dir(got.path.parent().unwrap())
                .unwrap()
                .map(|e| e.unwrap().file_name())
                .collect();
            assert_eq!(names.len(), 1, "{names:?}");
        }
    }

    #[test]
    fn a_country_artifact_without_a_backup_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let zip = Builder::new().add("Cronus.bclicense", b"license", true).finish();
        let path = dir.path().join("w1.zip");
        std::fs::write(&path, zip).unwrap();
        let cdn = Country {
            zip: path,
            calls: Cell::new(0),
        };
        let e = ensure_demo_backup(&cdn, &dir.path().join("cache"), &build(), "w1", &mut |_, _| {}).unwrap_err();
        assert!(e.contains("BusinessCentral-W1.bak"), "{e}");
    }

    #[test]
    fn a_damaged_download_leaves_no_backup_behind() {
        let dir = tempfile::tempdir().unwrap();
        let bytes = backup_bytes();
        let zip = Builder::new()
            .add_with_crc("BusinessCentral-W1.bak", &bytes, crc32fast::hash(&bytes) ^ 1)
            .finish();
        let path = dir.path().join("w1.zip");
        std::fs::write(&path, zip).unwrap();
        let cdn = Country {
            zip: path,
            calls: Cell::new(0),
        };
        let cache = dir.path().join("cache");
        let e = ensure_demo_backup(&cdn, &cache, &build(), "w1", &mut |_, _| {}).unwrap_err();
        assert!(e.to_lowercase().contains("crc"), "{e}");
        let folder = cached_backup(&cache, &build(), "w1");
        assert!(!folder.exists());
        assert_eq!(std::fs::read_dir(folder.parent().unwrap()).unwrap().count(), 0);
    }

    #[test]
    fn a_network_failure_is_an_error_naming_the_url() {
        let dir = tempfile::tempdir().unwrap();
        let cdn = Country {
            zip: dir.path().join("missing.zip"),
            calls: Cell::new(0),
        };
        let e = ensure_demo_backup(&cdn, &dir.path().join("cache"), &build(), "w1", &mut |_, _| {}).unwrap_err();
        assert!(e.contains("https://cdn.test/27.5.46862.48827/w1"), "{e}");
    }
}
