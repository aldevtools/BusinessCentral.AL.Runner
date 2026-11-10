//! The corpus's missing test libraries come from Microsoft's artifact feed (ADR-013 Layer A).
//! `#[ignore]`d: it talks to the CDN, and `cargo test` never uses the network.
//!
//! `cargo test --release -p al-test-cli --test network_deps -- --ignored --nocapture`
//!
//! Cold: the build is chosen from the index, the central directory (about 1 MB of a 1.3 GB file)
//! and the nine Microsoft apps the corpus runs on (about 41 MB: the platform's System app, four source archives of
//! the application, three test libraries and the Test Runner app) are read by range and saved in a fresh cache.
//! The corpus tree holds no Microsoft package and its `.alpackages` is not searched (ADR-013 amendment of
//! 2026-11-10), so every one of them comes from the feed.
//! Warm: the same call with downloading off, served from that cache.

use al_deps::build::{BuildQuery, Origin};
use al_test_cli::deps::{self, Provision};
use std::path::{Path, PathBuf};
use std::time::Instant;

fn corpus_app() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/tests/al-language")
}

fn provision(cache: &Path, download: bool) -> Provision {
    Provision {
        cache_dir: Some(cache.to_path_buf()),
        download,
        query: BuildQuery::new(Some("27.5"), 27, Some((27, 0))),
        local_artifacts: None,
        country: String::new(),
    }
}

#[test]
#[ignore = "needs the network: cargo test --release -p al-test-cli --test network_deps -- --ignored"]
fn the_corpus_test_libraries_download_cache_and_load_offline() {
    let app = corpus_app();
    assert!(app.join("app.json").exists(), "corpus submodule not checked out");
    let corpus_deps = deps::corpus_source_dirs(&app).unwrap();
    let cache = tempfile::tempdir().unwrap();

    let t = Instant::now();
    let cold = deps::load_corpus(&app, &corpus_deps, &[], Some(&provision(cache.path(), true)), &[]).unwrap();
    let cold_time = t.elapsed();
    let used = cold
        .build
        .as_ref()
        .expect("no Microsoft app is on disk, so the feed was used");
    println!("cold: {cold_time:?}, build {}, fetched {:?}", used.build, used.fetched);
    assert!(used.build.matches_prefix("27.5"));
    assert_eq!(used.origin, Origin::Published);
    // Each app once, however often the list names it: the set, not the count (an app several loaded apps depend on
    // was listed once per dependent before the fetch list was made distinct).
    let fetched: std::collections::BTreeSet<&str> = used
        .fetched
        .iter()
        .map(|entry| entry.rsplit('/').next().unwrap_or(entry))
        .collect();
    assert_eq!(
        fetched.into_iter().collect::<Vec<_>>(),
        [
            "Any.Source.zip",
            "Application.Source.zip",
            "Base Application.Source.zip",
            "Business Foundation.Source.zip",
            "Permissions Mock.Source.zip",
            "System Application Test Library.Source.zip",
            "System Application.Source.zip",
            "System.app",
            "Test Runner.Source.zip",
        ],
        "the nine Microsoft apps the corpus app runs on, the Test Runner app every run loads among them (ADR-013, \"The test environment's apps\")"
    );
    let names: Vec<&str> = cold.apps.iter().map(|a| a.name.as_str()).collect();
    for want in [
        "System",
        "System Application",
        "Business Foundation",
        "Base Application",
        "System Application Test Library",
        "Permissions Mock",
        "Any",
        "Test Runner",
    ] {
        assert!(names.contains(&want), "{want} missing from {names:?}");
    }

    let t = Instant::now();
    let warm = deps::load_corpus(&app, &corpus_deps, &[], Some(&provision(cache.path(), false)), &[]).unwrap();
    println!("warm: {:?}", t.elapsed());
    let used = warm.build.as_ref().unwrap();
    assert_eq!(used.origin, Origin::Cache);
    assert!(used.fetched.is_empty());
    assert!(
        warm.apps.iter().any(|a| a.name == "Test Runner"),
        "the Test Runner app is loaded from the cache offline"
    );
    assert_eq!(
        warm.apps.iter().map(|a| a.name.clone()).collect::<Vec<_>>(),
        cold.apps.iter().map(|a| a.name.clone()).collect::<Vec<_>>()
    );
}
