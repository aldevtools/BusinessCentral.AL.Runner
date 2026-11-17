//! The pieces of the report render process on disk (ADR-040 amendment 2026-11-13b): the two named service-folder sets,
//! fetched by range when no extracted artifact serves the build, and on Linux the nuget.org pieces pinned by hash
//! (`al_deps::render_set`). Builds the [`PrepareFn`] a [`crate::report_render::RenderHost`] calls on first use.

use crate::report_render::{PrepareFn, RenderPieces};
use al_deps::artifact::Cdn;
use al_deps::remote::{HttpSource, http_agent};
use al_deps::render_set::{NugetPiece, RenderSet, ensure_piece, linux_pieces, set_files};
use al_deps::service_tier::ServiceTier;
use al_host::render::RenderNeed;
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
    /// The needs the run can have. Only their sets and pieces are fetched: a run with PDF off (the default) fetches no
    /// conversion set and no Skia native.
    pub needs: Vec<RenderNeed>,
}

/// The set a need loads.
pub fn set_of(need: RenderNeed) -> RenderSet {
    match need {
        RenderNeed::Merge | RenderNeed::MergePictures => RenderSet::Merge,
        RenderNeed::Pdf | RenderNeed::Html => RenderSet::Convert,
    }
}

impl<C: Cdn> PieceSources<C> {
    /// The sets of [`PieceSources::needs`], merge first.
    fn sets(&self) -> Vec<RenderSet> {
        [RenderSet::Merge, RenderSet::Convert]
            .into_iter()
            .filter(|s| self.needs.iter().any(|n| set_of(*n) == *s))
            .collect()
    }
}

/// The [`PrepareFn`] of `src`.
pub fn prepare<C: Cdn + Send + Sync + 'static>(src: PieceSources<C>) -> Box<PrepareFn>
where
    C::Src: Send,
{
    Box::new(move || pieces(&src, &|p: &NugetPiece| HttpSource::new(http_agent(), p.url())))
}

/// The needs a run prepares pieces for: the merge, and the Html conversion of a Word layout. The conversion set (about 65 MB
/// per build) is fetched by default (owner decision of 2026-11-15, ADR-040 amendment 2026-11-15); PDF stays behind its switch.
pub const RUN_NEEDS: [RenderNeed; 2] = [RenderNeed::Merge, RenderNeed::Html];

/// The renderer of a run of BC `build`: the render process of `cfg`, started on the first save that asks it, with the sets
/// of [`RUN_NEEDS`] fetched by range from the build's platform artifact into the cache when no probe folder of `cfg` holds
/// them (`download: false`: only what the cache holds).
pub fn run_renderer(
    cfg: crate::dotnet::SidecarConfig,
    build: Option<al_deps::build::BuildId>,
    cache_root: &Path,
    download: bool,
) -> Arc<dyn al_host::render::ReportRenderer> {
    let src = run_sources(cfg.assembly_dirs.clone(), build, cache_root, download);
    // The render process runs on the .NET major of the build whose sets it fetches (crate::dotnet_runtime).
    let cfg = match &src.tier {
        Some(t) => crate::dotnet::SidecarConfig {
            service_runtime: Some(crate::dotnet_runtime::ServiceRuntime::of_tier(t.clone())),
            ..cfg
        },
        None => cfg,
    };
    Arc::new(crate::report_render::RenderHost::new(cfg, prepare(src)))
}

/// `--fetch-render-pieces`: fetches now, into the cache, what the renderer of a run of BC `build` ([`run_renderer`])
/// fetches on its first save. `Ok` holds the notes of what the build does not have, or (downloading off) the cache
/// lacks; `Err` when a fetch failed with downloading on ([`RenderPieces::fetch_failed`]).
pub fn fetch_run_pieces(
    dirs: Vec<PathBuf>,
    build: al_deps::build::BuildId,
    cache_root: &Path,
    download: bool,
) -> Result<Vec<String>, String> {
    let src = run_sources(dirs, Some(build), cache_root, download);
    fetch_outcome(pieces(&src, &|p: &NugetPiece| HttpSource::new(http_agent(), p.url())))
}

/// The outcome of `--fetch-render-pieces` from the pieces it prepared.
fn fetch_outcome(p: RenderPieces) -> Result<Vec<String>, String> {
    if p.fetch_failed {
        return Err(p.notes.join("; "));
    }
    Ok(p.notes)
}

/// The piece sources of a run of BC `build`: the probe folders `dirs`, then the build's fetch folder, and the sets of
/// [`RUN_NEEDS`].
fn run_sources(
    mut dirs: Vec<PathBuf>,
    build: Option<al_deps::build::BuildId>,
    cache_root: &Path,
    download: bool,
) -> PieceSources<al_deps::artifact::HttpCdn> {
    let tier = build.map(|b| {
        Arc::new(ServiceTier::new(
            al_deps::artifact::HttpCdn::from_env(),
            cache_root,
            b,
            download,
        ))
    });
    if let Some(t) = &tier
        && !dirs.contains(&t.dir())
    {
        dirs.push(t.dir());
    }
    PieceSources {
        dirs,
        tier,
        cache_root: cache_root.to_path_buf(),
        download,
        linux: cfg!(target_os = "linux"),
        needs: RUN_NEEDS.to_vec(),
    }
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
    let sets = src.sets();
    if let Some(tier) = &src.tier {
        for &set in &sets {
            match tier.ensure_set(&set_files(set, cfg!(windows))) {
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
                Err(e) => {
                    out.fetch_failed |= src.download;
                    out.notes.push(format!("the {} set was not fetched: {e}", set.name()));
                }
            }
        }
    }
    if !src.linux {
        return out;
    }
    let skia = find(&out.dirs, "SkiaSharp.dll")
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| al_deps::assembly_refs::assembly_version(&b));
    for &set in &sets {
        for piece in linux_pieces(set, skia) {
            match ensure_piece(&src.cache_root, &piece, src.download, || nuget(&piece)) {
                Ok(path) if set == RenderSet::Merge => out.drawing.push(path),
                Ok(path) => out.skia = Some(path),
                Err(e) => {
                    out.fetch_failed |= src.download;
                    out.notes.push(e);
                }
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
            needs: RenderNeed::ALL.to_vec(),
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
    fn a_run_without_a_merge_need_asks_for_no_merge_piece() {
        let t = tempfile::tempdir().unwrap();
        let src = PieceSources::<NoFeed> {
            dirs: vec![t.path().join("svc")],
            tier: None,
            cache_root: t.path().join("c"),
            download: false,
            linux: true,
            needs: vec![RenderNeed::Pdf],
        };
        let p = pieces(&src, &|_: &NugetPiece| -> FileSource { panic!("not opened") });
        assert!(p.notes.is_empty(), "{}", p.notes.join("; "));
        assert_eq!(src.sets(), vec![RenderSet::Convert]);
        let merge_only = PieceSources::<NoFeed> {
            needs: vec![RenderNeed::Merge],
            ..src
        };
        assert_eq!(merge_only.sets(), vec![RenderSet::Merge]);
    }

    #[test]
    fn off_linux_only_the_folders_are_passed() {
        let src = PieceSources::<NoFeed> {
            dirs: vec![PathBuf::from("/svc")],
            tier: None,
            cache_root: PathBuf::from("/c"),
            download: false,
            linux: false,
            needs: RenderNeed::ALL.to_vec(),
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

    #[test]
    fn the_sources_of_a_run_add_the_fetch_folder_of_the_build_and_ask_for_the_run_needs() {
        let t = tempfile::tempdir().unwrap();
        let build = al_deps::build::BuildId::parse("27.5.46862.58057").unwrap();
        let src = run_sources(vec![t.path().join("own")], Some(build.clone()), t.path(), false);
        let svc = al_deps::service_tier::service_dir(t.path(), &build);
        assert_eq!(src.dirs, vec![t.path().join("own"), svc]);
        assert!(src.tier.is_some() && !src.download);
        assert_eq!(src.needs, RUN_NEEDS.to_vec());
    }

    #[test]
    fn fetching_the_pieces_of_a_run_offline_on_an_empty_cache_names_each_set_it_could_not_have() {
        let t = tempfile::tempdir().unwrap();
        let build = al_deps::build::BuildId::parse("27.5.46862.58057").unwrap();
        let notes = fetch_run_pieces(Vec::new(), build, t.path(), false).expect("offline, a missing set is a note");
        assert!(notes[0].starts_with("the merge set was not fetched"), "{notes:?}");
        assert!(notes[1].starts_with("the conversion set was not fetched"), "{notes:?}");
    }

    /// A feed whose platform artifact cannot be read.
    struct BrokenFeed(PathBuf);
    impl Cdn for BrokenFeed {
        type Src = FileSource;
        fn base_url(&self) -> String {
            "https://cdn.test".into()
        }
        fn index(&self) -> Result<Vec<u8>, al_deps::remote::RemoteError> {
            panic!("not needed")
        }
        fn platform(&self, _: &al_deps::build::BuildId) -> (String, FileSource) {
            ("https://cdn.test/platform".into(), FileSource(self.0.clone()))
        }
        fn country(&self, _: &al_deps::build::BuildId, _: &str) -> (String, FileSource) {
            panic!("not needed")
        }
    }

    #[test]
    fn a_failed_platform_read_with_downloading_on_fails_the_fetch() {
        let t = tempfile::tempdir().unwrap();
        let build = al_deps::build::BuildId::parse("27.5.46862.58057").unwrap();
        let tier = ServiceTier::new(BrokenFeed(t.path().join("no-such-file")), t.path(), build, true);
        let src = PieceSources {
            dirs: vec![tier.dir()],
            tier: Some(Arc::new(tier)),
            cache_root: t.path().to_path_buf(),
            download: true,
            linux: false,
            needs: RUN_NEEDS.to_vec(),
        };
        let p = pieces(&src, &|_: &NugetPiece| -> FileSource { panic!("not opened") });
        let err = fetch_outcome(p).expect_err("a feed error is not a fact of the build");
        assert!(err.starts_with("the merge set was not fetched"), "{err}");
    }
}
