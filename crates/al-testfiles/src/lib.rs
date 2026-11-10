//! Microsoft's files for the tests of the workspace (ADR-013, amendment of 2026-11-10).
//!
//! A test that needs a file Microsoft ships (the platform's `System.app`, the source archive of Base Application)
//! reads it from the artifact cache, from the folder of the build `oracle/oracle.toml` pins for BC 27:
//! `<cache>/artifacts/<bc_build>/apps/<file>`. The corpus tree holds no Microsoft file, and neither does this
//! repository (hard rule 7).
//!
//! A test never downloads. When the file is not in the cache the test fails and names the command that fills the
//! cache ([`FETCH_COMMAND`]). It never skips: a skipped test is a weaker check.
//!
//! This crate is a dev-dependency only, so no runtime crate reads `oracle/oracle.toml` through it.

use std::path::{Path, PathBuf};

/// The command that fills the artifact cache with the pinned builds, run from the repository root.
pub const FETCH_COMMAND: &str =
    "cargo build --release -p al-test-cli && ./target/release/al-test-cli --root . --all-versions --fetch-deps";

/// The BC major whose pinned build the tests read: the oldest major the oracle has, the one the corpus tree's own
/// packages were of before they were removed.
pub const MAJOR: u32 = 27;

/// The platform's `System` app, a compiled package with AL source.
pub const SYSTEM: &str = "System.app";
pub const SYSTEM_APPLICATION: &str = "System Application.Source.zip";
pub const BUSINESS_FOUNDATION: &str = "Business Foundation.Source.zip";
pub const BASE_APPLICATION: &str = "Base Application.Source.zip";

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `bc_build` of the `[[version]]` table with `major` in the text of `oracle/oracle.toml`.
pub fn build_in(oracle_toml: &str, major: u32) -> Result<String, String> {
    let file: toml::Table = oracle_toml.parse().map_err(|e| format!("oracle.toml: {e}"))?;
    let versions = file
        .get("version")
        .and_then(toml::Value::as_array)
        .ok_or("oracle.toml has no [[version]] table")?;
    let version = versions
        .iter()
        .find(|v| v.get("major").and_then(toml::Value::as_integer) == Some(i64::from(major)))
        .ok_or_else(|| format!("oracle.toml has no [[version]] table with major = {major}"))?;
    version
        .get("bc_build")
        .and_then(toml::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("oracle.toml pins no bc_build for BC {major}"))
}

/// The Microsoft build `oracle/oracle.toml` pins for BC [`MAJOR`].
pub fn pinned_build() -> String {
    let path = repository_root().join("oracle/oracle.toml");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    build_in(&text, MAJOR).unwrap_or_else(|e| panic!("{e}"))
}

/// Where the cache keeps the files of `build` under the cache root `cache`.
pub fn apps_dir_in(cache: &Path, build: &str) -> PathBuf {
    cache.join("artifacts").join(build).join("apps")
}

/// The folder of the pinned build in the artifact cache (`ALRUNNER_CACHE` and the other variables of
/// `al_deps::cache::cache_root` move the cache). It need not exist.
pub fn apps_dir() -> PathBuf {
    apps_dir_in(&al_deps::cache::cache_root_from_env(None), &pinned_build())
}

/// `name` in `dir`, or what a test prints when it is not there. The cache writes a file under a temporary name and
/// renames it, and writes the record `<name>.json` after it: a file without its record is a download that did not end.
pub fn file_in(dir: &Path, name: &str) -> Result<PathBuf, String> {
    let path = dir.join(name);
    if path.is_file() && dir.join(format!("{name}.json")).is_file() {
        return Ok(path);
    }
    Err(format!(
        "Microsoft's `{name}` is not in the artifact cache ({}). Tests do not download. Fill the cache once, from the repository root:\n  {FETCH_COMMAND}",
        dir.display()
    ))
}

/// The path of Microsoft's file `name` of the pinned build (`System.app`, `Base Application.Source.zip`). Fails the
/// test with [`FETCH_COMMAND`] when the cache does not hold it.
pub fn file(name: &str) -> PathBuf {
    file_in(&apps_dir(), name).unwrap_or_else(|e| panic!("{e}"))
}

/// The path of the platform's `System.app` of the pinned build.
pub fn system_app() -> PathBuf {
    file(SYSTEM)
}

/// The AL files of Microsoft's file `name` of the pinned build, as (path in the app, text).
pub fn sources(name: &str) -> Vec<(String, String)> {
    let path = file(name);
    al_deps::load::read_package_at_with(&path, Some(&pinned_build()))
        .unwrap_or_else(|e| panic!("{e}"))
        .1
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORACLE: &str = "corpus_commit = \"c\"\n[[version]]\nmajor = 27\nbc_build = \"27.5.1.2\"\n[[version.suite]]\nname = \"onprem\"\n[[version]]\nmajor = 28\n";

    #[test]
    fn the_build_is_the_pin_of_the_major() {
        assert_eq!(build_in(ORACLE, 27).unwrap(), "27.5.1.2");
    }

    #[test]
    fn a_major_without_a_pin_is_an_error_not_another_build() {
        assert!(build_in(ORACLE, 28).unwrap_err().contains("no bc_build for BC 28"));
        assert!(build_in(ORACLE, 29).unwrap_err().contains("major = 29"));
    }

    #[test]
    fn the_folder_is_the_cache_layout_of_al_deps() {
        assert_eq!(
            apps_dir_in(Path::new("/c"), "27.5.1.2"),
            Path::new("/c/artifacts/27.5.1.2/apps")
        );
    }

    /// A missing file is an error that names the file and the command; it is never a skip.
    #[test]
    fn a_missing_file_names_the_fetch_command() {
        let dir = std::env::temp_dir().join(format!("al-testfiles-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let e = file_in(&dir, "System.app").unwrap_err();
        assert!(
            e.contains("`System.app`") && e.contains("--all-versions --fetch-deps"),
            "{e}"
        );
        // A file without its record is a download that did not end.
        std::fs::write(dir.join("System.app"), b"x").unwrap();
        assert!(file_in(&dir, "System.app").is_err());
        std::fs::write(dir.join("System.app.json"), b"{}").unwrap();
        assert_eq!(file_in(&dir, "System.app").unwrap(), dir.join("System.app"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The pinned oracle file of this repository names a build for the major the tests read.
    #[test]
    fn the_repository_pins_a_build() {
        assert!(pinned_build().starts_with("27."), "{}", pinned_build());
    }
}
