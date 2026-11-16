//! What the report render process needs on disk (ADR-040 amendment 2026-11-13b): two named sets of the BC service
//! folder, fetched by range from the platform artifact ([`crate::service_tier::ServiceTier::ensure_set`]), and on Linux
//! a few files of nuget.org packages, each checked against a SHA-256 kept here.
//!
//! The sets are lists of files, not reference walks: the round-0 spike recorded which assemblies a merge and a
//! conversion load on builds 27.5, 28.1, 28.5 and 29.1. Nothing of Microsoft's or of the packages is committed; the
//! files come to the user's machine like the apps do.
use crate::remote::{RangeSource, RemoteZip};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// A named set of service-folder files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderSet {
    /// The Word merger and what a merge loads.
    Merge,
    /// The document converter (PDF and Html) and what a conversion loads. Aspose.PDF and Aspose.Cells are in it: every
    /// conversion loads them (`LicenseHandler.InitLicense`).
    Convert,
}

impl RenderSet {
    /// The set's name in messages and on the wire of `render_probe`.
    pub fn name(self) -> &'static str {
        match self {
            RenderSet::Merge => "merge",
            RenderSet::Convert => "conversion",
        }
    }
}

/// The simple names (no `.dll`) of a set's files on this platform. The Windows-only entries are the artifact's own
/// `System.Drawing.Common` (picture merges) and the two native libraries of the converter; on Linux those come from
/// [`linux_pieces`] and the distribution.
pub fn set_files(set: RenderSet, windows: bool) -> Vec<&'static str> {
    let mut v: Vec<&'static str> = match set {
        RenderSet::Merge => vec![
            "Microsoft.Dynamics.Nav.OpenXml",
            "DocumentFormat.OpenXml",
            "DocumentFormat.OpenXml.Framework",
            "System.IO.Packaging",
            "Microsoft.Dynamics.Nav.Types.Report",
        ],
        RenderSet::Convert => vec![
            "Microsoft.BusinessCentral.DocumentProcessor",
            "Aspose.Words",
            "Aspose.PDF",
            "Aspose.Cells",
            "Aspose.Words.Shaping.HarfBuzz",
            "SkiaSharp",
            "System.IO.Packaging",
        ],
    };
    if windows {
        match set {
            RenderSet::Merge => v.extend(["System.Drawing.Common", "Microsoft.Win32.SystemEvents"]),
            RenderSet::Convert => v.extend(["libSkiaSharp", "harfbuzz"]),
        }
    }
    v
}

/// One file of a nuget.org package, pinned by the SHA-256 of the file itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NugetPiece {
    /// The package id in lower case, as the flat container names it.
    pub package: &'static str,
    pub version: &'static str,
    /// The entry inside the `.nupkg`.
    pub entry: &'static str,
    /// SHA-256 of the entry's inflated bytes, lower-case hex. Taken from the packages whose `.nupkg` hashes the spike
    /// recorded (System.Drawing.Common 6.0.0 `ffd11a01…`, Microsoft.Win32.SystemEvents 6.0.0 `37d11565…`,
    /// SkiaSharp.NativeAssets.Linux 3.119.0 `cac1d718…` and 4.152.1 `995f53ed…`).
    pub sha256: &'static str,
}

impl NugetPiece {
    /// The file name the piece is written under.
    pub fn file_name(&self) -> &'static str {
        self.entry.rsplit('/').next().unwrap_or(self.entry)
    }

    /// Where nuget.org serves the package.
    pub fn url(&self) -> String {
        format!(
            "https://api.nuget.org/v3-flatcontainer/{p}/{v}/{p}.{v}.nupkg",
            p = self.package,
            v = self.version
        )
    }
}

/// `System.Drawing.Common` 6.0.0 for Unix, which the merger loads in place of the artifact's Windows-only build for a
/// picture control (with libgdiplus and `System.Drawing.EnableUnixSupport`).
pub const DRAWING_COMMON: NugetPiece = NugetPiece {
    package: "system.drawing.common",
    version: "6.0.0",
    entry: "runtimes/unix/lib/net6.0/System.Drawing.Common.dll",
    sha256: "89ea595ccb1fd1384480c1e30bd487c92af5d06d3b06f295323eff601acd9019",
};

/// What `System.Drawing.Common` 6.0.0 references.
pub const SYSTEM_EVENTS: NugetPiece = NugetPiece {
    package: "microsoft.win32.systemevents",
    version: "6.0.0",
    entry: "lib/net6.0/Microsoft.Win32.SystemEvents.dll",
    sha256: "ca495bf6c86b7f092d624928101985a941532d149dcd8acfea170f8a53be8850",
};

/// The Linux `libSkiaSharp.so` for each managed SkiaSharp assembly version a measured build ships: 3.119.0.0 (27.5,
/// 28.1, 29.0; file version 3.119.0.0) and 4.152.0.0 (28.5, 29.1; file version 4.152.1.0). The render process binds
/// SkiaSharp's native imports to this file and asks SkiaSharp's own compatibility check before any conversion; a version
/// not in this table gets no native.
pub const SKIA_LINUX: &[([u16; 2], NugetPiece)] = &[
    (
        [3, 119],
        NugetPiece {
            package: "skiasharp.nativeassets.linux",
            version: "3.119.0",
            entry: "runtimes/linux-x64/native/libSkiaSharp.so",
            sha256: "09e7aec4a2c32f1a856f824297161c9b97895302c118e376629a657c0f5918e5",
        },
    ),
    (
        [4, 152],
        NugetPiece {
            package: "skiasharp.nativeassets.linux",
            version: "4.152.1",
            entry: "runtimes/linux-x64/native/libSkiaSharp.so",
            sha256: "6ac9cb9132bb0250550417e8794d64419b38bc8fe6c83893b50a45aad8ff6092",
        },
    ),
];

/// The Skia native for a managed SkiaSharp of `version` (from its `Assembly` row), when one is pinned.
pub fn skia_for(version: [u16; 4]) -> Option<NugetPiece> {
    SKIA_LINUX
        .iter()
        .find(|(v, _)| v[0] == version[0] && v[1] == version[1])
        .map(|(_, p)| *p)
}

/// The nuget pieces of a set on Linux: the drawing pair for the merge, the Skia native for the conversion (`skia` is the
/// managed SkiaSharp version of the build; `None` when it is not known yet or not pinned).
pub fn linux_pieces(set: RenderSet, skia: Option<[u16; 4]>) -> Vec<NugetPiece> {
    match set {
        RenderSet::Merge => vec![DRAWING_COMMON, SYSTEM_EVENTS],
        RenderSet::Convert => skia.and_then(skia_for).into_iter().collect(),
    }
}

/// The folder nuget pieces go into: `<cache>/render-extras/<package>.<version>/`.
pub fn piece_dir(cache_root: &Path, piece: &NugetPiece) -> PathBuf {
    cache_root
        .join("render-extras")
        .join(format!("{}.{}", piece.package, piece.version))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Makes `piece` present in [`piece_dir`] and returns its path. A file already there with the pinned hash is kept; else
/// the entry is read by range from `src()` (the `.nupkg`) and written only when its hash matches. `download` false
/// fetches nothing and says so.
pub fn ensure_piece<S: RangeSource>(
    cache_root: &Path,
    piece: &NugetPiece,
    download: bool,
    src: impl FnOnce() -> S,
) -> Result<PathBuf, String> {
    let path = piece_dir(cache_root, piece).join(piece.file_name());
    let cached = std::fs::read(&path).ok().map(|b| hex(&Sha256::digest(&b)));
    if cached.as_deref() == Some(piece.sha256) {
        return Ok(path);
    }
    if !download {
        return Err(match cached {
            Some(got) => format!(
                "{} {} in the cache has SHA-256 {got}, not the pinned {}, and downloading is off",
                piece.file_name(),
                piece.version,
                piece.sha256
            ),
            None => format!(
                "{} {} is not in the cache and downloading is off",
                piece.file_name(),
                piece.version
            ),
        });
    }
    let url = piece.url();
    let zip = RemoteZip::open(src()).map_err(|e| format!("{url}: {e}"))?;
    let entry = zip
        .find(piece.entry)
        .ok_or_else(|| format!("{url}: no entry `{}`", piece.entry))?;
    let bytes = zip.read(entry).map_err(|e| format!("{url}: {e}"))?;
    let got = hex(&Sha256::digest(&bytes));
    if got != piece.sha256 {
        return Err(format!(
            "{url}: `{}` has SHA-256 {got}, not the pinned {}; not used",
            piece.entry, piece.sha256
        ));
    }
    crate::service_tier::write_atomic(&path, &bytes)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::FileSource;
    use crate::zip::testutil::Builder;

    fn piece(sha: &'static str) -> NugetPiece {
        NugetPiece {
            package: "own.test",
            version: "1.0.0",
            entry: "lib/x/Own.dll",
            sha256: sha,
        }
    }

    // SHA-256 of b"own bytes".
    const OWN: &str = "e86b0c51f112e22b7fe8ad4685eb1061d81c9e3e002fa35368cf51f2f40c575f";

    fn nupkg(dir: &Path) -> PathBuf {
        let p = dir.join("own.test.1.0.0.nupkg");
        let zip = Builder::new()
            .add("lib/x/Own.dll", b"own bytes", true)
            .add("other.txt", b"x", false)
            .finish();
        std::fs::write(&p, zip).unwrap();
        p
    }

    #[test]
    fn a_piece_is_fetched_once_checked_by_its_hash_and_then_served_offline() {
        let t = tempfile::tempdir().unwrap();
        let sha = hex(&Sha256::digest(b"own bytes"));
        assert_eq!(sha, OWN);
        let src = nupkg(t.path());
        let cache = t.path().join("c");
        let path = ensure_piece(&cache, &piece(OWN), true, || FileSource(src.clone())).unwrap();
        assert_eq!(path, cache.join("render-extras/own.test.1.0.0/Own.dll"));
        assert_eq!(std::fs::read(&path).unwrap(), b"own bytes");
        // Offline, from the cache; the source is never opened.
        let again = ensure_piece(&cache, &piece(OWN), false, || -> FileSource { panic!("not opened") });
        assert_eq!(again.unwrap(), path);
    }

    #[test]
    fn a_wrong_hash_writes_nothing_and_offline_without_the_file_says_so() {
        let t = tempfile::tempdir().unwrap();
        let src = nupkg(t.path());
        let cache = t.path().join("c");
        let wrong = "00".repeat(32).leak();
        let e = ensure_piece(&cache, &piece(wrong), true, || FileSource(src.clone())).unwrap_err();
        assert!(e.contains("not the pinned"), "{e}");
        assert!(!piece_dir(&cache, &piece(wrong)).join("Own.dll").exists());
        let e = ensure_piece(&cache, &piece(OWN), false, || FileSource(src.clone())).unwrap_err();
        assert_eq!(e, "Own.dll 1.0.0 is not in the cache and downloading is off");
    }

    #[test]
    fn offline_a_cached_file_with_another_hash_is_named_as_such() {
        let t = tempfile::tempdir().unwrap();
        let cache = t.path().join("c");
        let path = piece_dir(&cache, &piece(OWN)).join("Own.dll");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"other bytes").unwrap();
        let e = ensure_piece(&cache, &piece(OWN), false, || -> FileSource { panic!("not opened") }).unwrap_err();
        assert!(e.starts_with("Own.dll 1.0.0 in the cache has SHA-256 "), "{e}");
        assert!(
            e.ends_with(&format!("not the pinned {OWN}, and downloading is off")),
            "{e}"
        );
    }

    #[test]
    fn the_sets_have_names() {
        assert_eq!(RenderSet::Merge.name(), "merge");
        assert_eq!(RenderSet::Convert.name(), "conversion");
    }

    #[test]
    fn the_skia_native_follows_the_managed_assembly_version_and_an_unpinned_one_gets_none() {
        assert_eq!(skia_for([3, 119, 0, 0]).unwrap().version, "3.119.0");
        assert_eq!(skia_for([4, 152, 0, 0]).unwrap().version, "4.152.1");
        assert_eq!(skia_for([4, 153, 0, 0]), None);
        assert_eq!(linux_pieces(RenderSet::Convert, None), vec![]);
        assert_eq!(
            linux_pieces(RenderSet::Merge, None),
            vec![DRAWING_COMMON, SYSTEM_EVENTS]
        );
        assert_eq!(
            DRAWING_COMMON.url(),
            "https://api.nuget.org/v3-flatcontainer/system.drawing.common/6.0.0/system.drawing.common.6.0.0.nupkg"
        );
    }

    #[test]
    fn the_sets_are_the_spike_lists_and_windows_adds_its_natives() {
        assert_eq!(set_files(RenderSet::Merge, false).len(), 5);
        assert!(set_files(RenderSet::Convert, false).contains(&"Aspose.PDF"));
        assert!(!set_files(RenderSet::Convert, false).contains(&"libSkiaSharp"));
        assert!(set_files(RenderSet::Convert, true).contains(&"libSkiaSharp"));
        assert!(set_files(RenderSet::Merge, true).contains(&"System.Drawing.Common"));
    }
}
