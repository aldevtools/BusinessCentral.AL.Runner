//! The report render process (ADR-040 amendment 2026-11-13b) against the real `tools/dotnet-sidecar` and BC's own
//! assemblies from the artifact cache. Needs .NET and a build whose merge set is in `<cache>/artifacts/<build>/service`
//! (fetched by an earlier run); without them the test skips with a message. Nothing is downloaded.
//!
//! The fixture layout is hand-made (a table-row repeater over one data item plus static text and a label) and the dataset
//! is one the runner wrote for that report; neither is Microsoft's.

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

/// The newest build of each BC major in the artifact cache whose service folder holds the merge set.
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
        if !out.iter().any(|(m, _)| *m == major) {
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
