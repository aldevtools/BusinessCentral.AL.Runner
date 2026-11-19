//! One deflate backend for every build (al-runner issue 8388). Cargo joins a dependency's features per build: when two
//! crates ask for different flate2 backends, `cargo build -p al-test-cli` (CI) and the workspace build (the shipped
//! binary, local runs) link different ones, and GZipStream's results differ between them. The root `Cargo.toml` names
//! the one backend; every crate takes flate2 from there.

use std::path::Path;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the workspace root")
}

/// The `flate2` lines of a manifest that do not take the workspace's dependency.
fn own_flate2_lines(manifest: &str) -> Vec<String> {
    manifest
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("flate2") && *l != "flate2.workspace = true")
        .map(String::from)
        .collect()
}

#[test]
fn every_crate_takes_flate2_from_the_workspace() {
    let mut own = Vec::new();
    for entry in std::fs::read_dir(root().join("crates")).expect("read crates/") {
        let manifest = entry.expect("a crates/ entry").path().join("Cargo.toml");
        let Ok(text) = std::fs::read_to_string(&manifest) else {
            continue;
        };
        for line in own_flate2_lines(&text) {
            own.push(format!("{}: {line}", manifest.display()));
        }
    }
    assert!(
        own.is_empty(),
        "flate2 with features of its own (use `flate2.workspace = true`):\n{}",
        own.join("\n")
    );
}

#[test]
fn the_workspace_names_one_flate2_backend() {
    let text = std::fs::read_to_string(root().join("Cargo.toml")).expect("read Cargo.toml");
    let line = text
        .lines()
        .find(|l| l.starts_with("flate2 ="))
        .expect("flate2 in [workspace.dependencies]");
    assert!(line.contains("default-features = false"), "{line}");
    let backends = [
        "zlib-rs",
        "rust_backend",
        "miniz_oxide",
        // Also matches zlib-ng-compat.
        "zlib-ng",
        "zlib\"",
        "cloudflare_zlib",
    ];
    let named: Vec<_> = backends.iter().filter(|b| line.contains(*b)).collect();
    assert_eq!(named.len(), 1, "{line}");
}

#[test]
fn a_crate_with_its_own_backend_is_found() {
    assert_eq!(
        own_flate2_lines(
            "[dependencies]\nflate2 = { version = \"1\", features = [\"rust_backend\"] }\nflate2.workspace = true\n"
        ),
        vec!["flate2 = { version = \"1\", features = [\"rust_backend\"] }".to_string()]
    );
}
