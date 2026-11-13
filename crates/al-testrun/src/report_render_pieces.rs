//! The pieces of the report render process on disk (ADR-040 amendment 2026-11-13b): the two named service-folder sets,
//! fetched by range when no extracted artifact serves the build, and on Linux the nuget.org pieces pinned by hash
//! (`al_deps::render_set`). Builds the [`PrepareFn`] a [`crate::report_render::RenderHost`] calls on first use.

use crate::report_render::{PrepareFn, RenderPieces};
use al_deps::artifact::Cdn;
use al_deps::remote::{HttpSource, http_agent};
use al_deps::render_set::{NugetPiece, RenderSet, ensure_piece, linux_pieces, set_files};
use al_deps::service_tier::ServiceTier;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Where the pieces come from.
pub struct PieceSources<C: Cdn> {
    /// The probe folders of the run, in order (`crate::dotnet::probe_dirs`: the user's folders, an extracted artifact, the
    /// fetch folder).
    pub dirs: Vec<PathBuf>,
    /// The range fetch of the build's platform artifact; `None` when an extracted artifact serves the build or no build is
    /// known.
    pub tier: Option<Arc<ServiceTier<C>>>,
    /// Where nuget pieces are cached (the app cache root).
    pub cache_root: PathBuf,
    /// `false` with `--no-download`: only what is in the cache is used.
    pub download: bool,
    /// Whether the Linux pieces are needed (they are on Linux only).
    pub linux: bool,
}

/// The [`PrepareFn`] of `src`.
pub fn prepare<C: Cdn + Send + Sync + 'static>(src: PieceSources<C>) -> Box<PrepareFn>
where
    C::Src: Send,
{
    Box::new(move || pieces(&src, &|p: &NugetPiece| HttpSource::new(http_agent(), p.url())))
}

/// The pieces of `src`; `nuget` opens a package (tests give a file).
pub(crate) fn pieces<C: Cdn, S: al_deps::remote::RangeSource>(
    src: &PieceSources<C>,
    nuget: &dyn Fn(&NugetPiece) -> S,
) -> RenderPieces {
    let mut out = RenderPieces {
        dirs: src.dirs.clone(),
        ..RenderPieces::default()
    };
    let windows = !src.linux && cfg!(windows);
    if let Some(tier) = &src.tier {
        for set in [RenderSet::Merge, RenderSet::Convert] {
            match tier.ensure_set(&set_files(set, windows)) {
                Ok(got) => {
                    if !got.files.is_empty() {
                        eprintln!(
                            "fetched {} BC service-tier files ({:.1} MB) for report rendering into {}",
                            got.files.len(),
                            got.bytes as f64 / 1_000_000.0,
                            tier.dir().display()
                        );
                    }
                    for m in got.missing {
                        out.notes.push(format!("the platform artifact has no {m}.dll"));
                    }
                }
                Err(e) => out.notes.push(format!("the {set:?} set was not fetched: {e}")),
            }
        }
    }
    if !src.linux {
        return out;
    }
    let skia = find(&out.dirs, "SkiaSharp.dll")
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| al_deps::assembly_refs::assembly_version(&b));
    for set in [RenderSet::Merge, RenderSet::Convert] {
        for piece in linux_pieces(set, skia) {
            match ensure_piece(&src.cache_root, &piece, src.download, || nuget(&piece)) {
                Ok(path) if set == RenderSet::Merge => out.drawing.push(path),
                Ok(path) => out.skia = Some(path),
                Err(e) => out.notes.push(e),
            }
        }
    }
    if let Some(v) = skia
        && out.skia.is_none()
        && al_deps::render_set::skia_for(v).is_none()
    {
        out.notes.push(format!(
            "no Linux Skia native is pinned for SkiaSharp {}.{}.{}.{}",
            v[0], v[1], v[2], v[3]
        ));
    }
    out
}

/// The first `name` in the top level of `dirs`.
fn find(dirs: &[PathBuf], name: &str) -> Option<PathBuf> {
    dirs.iter().map(|d| d.join(name)).find(|p| Path::new(p).is_file())
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_deps::remote::FileSource;

    struct NoFeed;
    impl Cdn for NoFeed {
        type Src = FileSource;
        fn base_url(&self) -> String {
            "https://cdn.test".into()
        }
        fn index(&self) -> Result<Vec<u8>, al_deps::remote::RemoteError> {
            panic!("not needed")
        }
        fn platform(&self, _: &al_deps::build::BuildId) -> (String, FileSource) {
            panic!("not needed")
        }
        fn country(&self, _: &al_deps::build::BuildId, _: &str) -> (String, FileSource) {
            panic!("not needed")
        }
    }

    #[test]
    fn offline_on_linux_without_cached_pieces_names_each_missing_piece_and_fetches_nothing() {
        let t = tempfile::tempdir().unwrap();
        let src = PieceSources::<NoFeed> {
            dirs: vec![t.path().join("svc")],
            tier: None,
            cache_root: t.path().join("c"),
            download: false,
            linux: true,
        };
        let p = pieces(&src, &|_: &NugetPiece| -> FileSource {
            panic!("nothing is opened offline")
        });
        assert_eq!(p.dirs, vec![t.path().join("svc")]);
        assert!(p.drawing.is_empty() && p.skia.is_none());
        // No SkiaSharp.dll in the folders: no Skia native is asked for; the drawing pair is named.
        assert_eq!(
            p.notes,
            vec![
                "System.Drawing.Common.dll 6.0.0 is not in the cache and downloading is off".to_string(),
                "Microsoft.Win32.SystemEvents.dll 6.0.0 is not in the cache and downloading is off".to_string(),
            ]
        );
    }

    #[test]
    fn off_linux_only_the_folders_are_passed() {
        let src = PieceSources::<NoFeed> {
            dirs: vec![PathBuf::from("/svc")],
            tier: None,
            cache_root: PathBuf::from("/c"),
            download: false,
            linux: false,
        };
        let p = pieces(&src, &|_: &NugetPiece| -> FileSource { panic!("not opened") });
        assert_eq!(
            p,
            RenderPieces {
                dirs: vec![PathBuf::from("/svc")],
                ..RenderPieces::default()
            }
        );
    }
}
