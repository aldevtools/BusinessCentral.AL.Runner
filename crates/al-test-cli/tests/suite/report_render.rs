//! The report render process (ADR-040 amendment 2026-11-13b) against the real `tools/dotnet-sidecar` and BC's own
//! assemblies from the artifact cache. Needs .NET and a build of BC 27 to 29 whose merge set is in
//! `<cache>/artifacts/<build>/service` (fetched by an earlier run); without them the test skips with a message. Nothing
//! is downloaded. The conversion test also takes the service folders listed in `ALRT_RENDER_SERVICE_DIRS` (a path list),
//! and the pinned Linux Skia natives from `<cache>/render-extras`.
//!
//! The fixtures are hand-made: a layout (a table-row repeater over one data item plus static text and a label), the
//! dataset the runner wrote for that report, and a document with one 16x16 picture. None is Microsoft's.

use al_deps::render_set::{SKIA_LINUX, piece_dir, skia_for};
use al_host::render::{RenderNeed, ReportRenderer, WordMerge};
use al_test_cli::dotnet::SidecarConfig;
use al_testrun::report_render::{RenderHost, RenderPieces};
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// The newest build of each BC major from 27 to 29 (the majors the expected exception text is measured on) in the
/// artifact cache whose service folder holds the merge set.
fn cached_builds() -> Vec<(u32, PathBuf)> {
    let cache = al_deps::cache::cache_root_from_env(None);
    let Ok(rd) = std::fs::read_dir(cache.join("artifacts")) else {
        return Vec::new();
    };
    let mut builds: Vec<(Vec<u64>, PathBuf)> = rd
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let parts: Vec<u64> = name.split('.').map(|p| p.parse().ok()).collect::<Option<_>>()?;
            let svc = e.path().join("service");
            let set = al_deps::render_set::set_files(al_deps::render_set::RenderSet::Merge, cfg!(windows));
            set.iter()
                .all(|f| svc.join(format!("{f}.dll")).is_file())
                .then_some((parts, svc))
        })
        .collect();
    builds.sort();
    let mut out: Vec<(u32, PathBuf)> = Vec::new();
    for (parts, svc) in builds.into_iter().rev() {
        let major = parts[0] as u32;
        if (27..=29).contains(&major) && !out.iter().any(|(m, _)| *m == major) {
            out.push((major, svc));
        }
    }
    out
}

fn text_of(docx: &[u8], part: &str) -> String {
    let zip = al_deps::zip::Zip::parse(docx).expect("a zip");
    let e = zip.find(part).expect("the part");
    String::from_utf8(zip.read(e).unwrap()).unwrap()
}

#[test]
fn bcs_merger_merges_a_hand_made_layout_with_a_runner_dataset() {
    let builds = cached_builds();
    if builds.is_empty() {
        eprintln!(
            "SKIPPED: no build in the artifact cache has the report merge set in its service folder \
             (a run that renders a Word layout fetches it)"
        );
        return;
    }
    let fixtures = root().join("crates/al-test-cli/tests/fixtures/render");
    let layout = std::fs::read(fixtures.join("table-row-repeater.docx")).unwrap();
    let dataset = std::fs::read(fixtures.join("table-row-repeater.dataset.xml")).unwrap();
    for (major, svc) in builds {
        let cfg = SidecarConfig {
            source_root: Some(root()),
            cache_dir: Some(Path::new(env!("CARGO_TARGET_TMPDIR")).join("dotnet-sidecar-cache")),
            bc_major: Some(major),
            ..Default::default()
        };
        let dirs = vec![svc.clone()];
        let host = RenderHost::new(
            cfg,
            Box::new(move || RenderPieces {
                dirs: dirs.clone(),
                ..RenderPieces::default()
            }),
        );
        if let Err(why) = host.availability(RenderNeed::Merge) {
            assert!(
                std::env::var("ALRT_REQUIRE_DOTNET").is_err() || why.piece != "the render process",
                "ALRT_REQUIRE_DOTNET is set but the render process did not start: {why}"
            );
            eprintln!("SKIPPED BC {major} ({}): merge unavailable: {why}", svc.display());
            continue;
        }
        let merged = host
            .merge_word(&WordMerge {
                layout: layout.clone(),
                dataset: dataset.clone(),
                title: "Spike Invoice".into(),
                company: String::new(),
                pictures: false,
            })
            .unwrap_or_else(|e| panic!("BC {major}: {e}"));
        assert_eq!(&merged[..4], b"PK\x03\x04", "BC {major}");
        let doc = text_of(&merged, "word/document.xml");
        // The three rows of the repeater, the header values, the static text and the label.
        for want in [
            "INV-1001",
            "Spike line 1",
            "Spike line 2",
            "Spike line 3",
            "Static: document ",
            "Spike Invoice",
        ] {
            assert!(doc.contains(want), "BC {major}: `{want}` not in document.xml");
        }
        // A dataset BC's merger cannot read comes back as the merger's own exception, by CLR type.
        let bad = host
            .merge_word(&WordMerge {
                layout: layout.clone(),
                ..WordMerge::default()
            })
            .unwrap_err();
        assert!(
            bad.to_string()
                .starts_with("Microsoft.Dynamics.Nav.OpenXml.Exceptions.OfficeDocumentMergeException:"),
            "BC {major}: {bad}"
        );
        eprintln!("BC {major} ({}): merged {} bytes", svc.display(), merged.len());
    }
}

fn host(major: u32, dirs: Vec<PathBuf>, skia: Option<PathBuf>) -> RenderHost {
    let cfg = SidecarConfig {
        source_root: Some(root()),
        cache_dir: Some(Path::new(env!("CARGO_TARGET_TMPDIR")).join("dotnet-sidecar-cache")),
        bc_major: Some(major),
        ..Default::default()
    };
    RenderHost::new(
        cfg,
        Box::new(move || RenderPieces {
            dirs: dirs.clone(),
            skia: skia.clone(),
            ..RenderPieces::default()
        }),
    )
}

/// A folder of links to every entry of `svc` except `leave_out`, plus `extra` (name, target).
fn linked_copy(svc: &Path, leave_out: &[&str], extra: &[(&str, &Path)]) -> tempfile::TempDir {
    let t = tempfile::tempdir().unwrap();
    for e in std::fs::read_dir(svc).unwrap().flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if !leave_out.contains(&name.as_str()) {
            std::os::unix::fs::symlink(e.path(), t.path().join(&name)).unwrap();
        }
    }
    for (name, target) in extra {
        std::os::unix::fs::symlink(target, t.path().join(name)).unwrap();
    }
    t
}

fn pictures_in_pdf(pdf: &[u8]) -> usize {
    let text = String::from_utf8_lossy(pdf);
    text.matches("/Subtype /Image").count() + text.matches("/Subtype/Image").count()
}

/// The cached Linux Skia native of `v`'s pin, and a cached one of another pin, when both are there.
fn cached_natives(v: [u16; 4]) -> Option<(PathBuf, PathBuf)> {
    let cache = al_deps::cache::cache_root_from_env(None);
    let path = |p: &al_deps::render_set::NugetPiece| piece_dir(&cache, p).join(p.file_name());
    let pinned = skia_for(v).map(|p| path(&p)).filter(|p| p.is_file())?;
    let other = SKIA_LINUX
        .iter()
        .map(|(_, p)| path(p))
        .find(|p| *p != pinned && p.is_file())?;
    Some((pinned, other))
}

/// The probe requires every file of a set by name, and the conversion uses the pinned Skia native even when another one
/// lies beside `SkiaSharp.dll` (where .NET's own search would find it first).
#[test]
fn the_probe_requires_every_set_file_and_the_conversion_binds_the_pinned_skia_native() {
    if !cfg!(target_os = "linux") {
        eprintln!("SKIPPED: the Skia native and the links of this test are the Linux ones");
        return;
    }
    let mut folders: Vec<(u32, PathBuf)> = cached_builds();
    if let Some(list) = std::env::var_os("ALRT_RENDER_SERVICE_DIRS") {
        folders.extend(std::env::split_paths(&list).map(|p| (0, p)));
    }
    let picture = std::fs::read(root().join("crates/al-test-cli/tests/fixtures/render/one-picture.docx")).unwrap();
    let mut ran = 0;
    for (major, svc) in folders {
        let Some(v) = std::fs::read(svc.join("SkiaSharp.dll"))
            .ok()
            .and_then(|b| al_deps::assembly_refs::assembly_version(&b))
        else {
            eprintln!("SKIPPED {}: no SkiaSharp.dll", svc.display());
            continue;
        };
        let Some((pinned, other)) = cached_natives(v) else {
            eprintln!(
                "SKIPPED {}: the pinned Skia native or a second one is not cached",
                svc.display()
            );
            continue;
        };
        // The folders of ALRT_RENDER_SERVICE_DIRS run as BC 28 (the .NET 8 runtime).
        let major = if major == 0 { 28 } else { major };
        let whole = host(major, vec![svc.clone()], Some(pinned.clone()));
        if let Err(why) = whole.availability(RenderNeed::Pdf) {
            eprintln!("SKIPPED {}: pdf unavailable: {why}", svc.display());
            continue;
        }
        let right = whole
            .word_to_pdf(&picture)
            .unwrap_or_else(|e| panic!("{}: {e}", svc.display()));
        assert_eq!(pictures_in_pdf(&right), 1, "{}", svc.display());

        // Another native beside SkiaSharp.dll: the pinned one is still the one used, so the picture is kept.
        let beside = linked_copy(&svc, &[], &[("libSkiaSharp.so", &other)]);
        let h = host(major, vec![beside.path().to_path_buf()], Some(pinned.clone()));
        let pdf = h
            .word_to_pdf(&picture)
            .unwrap_or_else(|e| panic!("{}: {e}", svc.display()));
        assert_eq!(pictures_in_pdf(&pdf), 1, "{}: the picture was dropped", svc.display());

        // The pinned path holds a native of another version: SkiaSharp's own check fails, pdf and html are unavailable.
        let h = host(major, vec![svc.clone()], Some(other.clone()));
        for need in [RenderNeed::Pdf, RenderNeed::Html] {
            let e = h.availability(need).unwrap_err();
            assert!(e.piece.starts_with("libSkiaSharp"), "{}: {e}", svc.display());
        }

        // A file of each set is missing: the probe names it.
        let partial = linked_copy(&svc, &["Aspose.PDF.dll", "DocumentFormat.OpenXml.Framework.dll"], &[]);
        let h = host(major, vec![partial.path().to_path_buf()], Some(pinned.clone()));
        assert_eq!(
            h.availability(RenderNeed::Merge).unwrap_err().piece,
            "DocumentFormat.OpenXml.Framework.dll"
        );
        assert_eq!(h.availability(RenderNeed::Pdf).unwrap_err().piece, "Aspose.PDF.dll");
        assert_eq!(h.availability(RenderNeed::Html).unwrap_err().piece, "Aspose.PDF.dll");
        eprintln!("{}: Skia binding and set checks hold", svc.display());
        ran += 1;
    }
    if ran == 0 {
        eprintln!("SKIPPED: no service folder with a working conversion set and two cached Skia natives");
    }
}
