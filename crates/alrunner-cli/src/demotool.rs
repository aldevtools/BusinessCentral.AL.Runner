//! `--company-data demotool`: the company of Microsoft's Legacy test lane, built the way BCApps' CI builds it
//! (`build/scripts/ImportTestDataInBcContainer.ps1`, `Invoke-LegacyDemoDataTool`): a new company, `Company-Initialize`,
//! then the DemoTool's "Create Demo Data from Config" (codeunit 101899, the action of page 101900), which reads
//! `DemoDataConfig.xml` and the W1 resource files from the server's user folder.
//!
//! The DemoTool is not in the artifact. Its AL source (`src/Layers/W1/DemoTool`) and its files (`src/DemoTool`) come from
//! the BCApps repository (MIT) of the release, fetched sparse with `git` into `<cache>/bcapps-demotool/<major>.<minor>/`
//! once, like the recipe files (`bcapps.rs`). The app is compiled with the Microsoft apps as one more dependency app; the
//! install baseline runs the codeunit (`al_testrun::company_data`). Nothing from BCApps is committed to this repository.
//!
//! The resource files are staged as BCApps' `Initialize-DemoToolResources` stages them for W1: the content of
//! `src/DemoTool/Pictures`, the folder `src/DemoTool/O365`, and `DemoDataConfig.xml` with `DataType` `Extended`.
//!
//! This file does the OS work (git, folders) that the core crates must not do (ADR-006).

use crate::bcapps;
use al_deps::load::AppJson;
use al_deps::select::DepDecl;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// The parts of BCApps the DemoTool needs. `build/scripts` brings, as a sparse checkout in cone mode does, the files of
/// `build` itself, among them [`GROUPS`].
pub const SPARSE_PATHS: [&str; 3] = ["src/DemoTool", "src/Layers/W1/DemoTool", "build/scripts"];

/// The app groups of BCApps' build, which say which apps are installed before the DemoTool runs.
const GROUPS: &str = "build/groups.json";

/// The country of the DemoTool's layer (`src/Layers/W1`).
const COUNTRY: &str = "W1";

/// Written last into a finished cache folder.
const COMPLETE: &str = ".alrunner-complete";

/// The staged resource folder inside the cache folder.
/// `-crlf`: the text files have the line ends of a Windows checkout (`checked_out_as_text`); a folder staged before
/// that (`.alrunner-resources`) is left unused and staged again.
const STAGED: &str = ".alrunner-resources-crlf";

/// The DemoTool of a release, ready to compile and run.
#[derive(Debug, Clone)]
pub struct DemoTool {
    /// The app folder (`src/Layers/W1/DemoTool`, with its `app.json`).
    pub app_dir: PathBuf,
    /// The app's manifest.
    pub app: AppJson,
    /// The staged resource files.
    pub resources: PathBuf,
    /// What the company depends on: the fetch and a hash of the app's source and the staged files.
    pub identity: String,
    /// One line: where it came from.
    pub how: String,
    /// The apps installed before the DemoTool runs (names).
    pub installed_first: Vec<String>,
}

impl DemoTool {
    /// The dependency the project gets, so the resolver loads the app from [`DemoTool::app_dir`].
    pub fn dependency(&self) -> DepDecl {
        DepDecl {
            id: self.app.id.clone(),
            name: self.app.name.clone(),
            publisher: self.app.publisher.clone(),
            min: self.app.version,
            optional: false,
        }
    }

    /// What the install baseline runs.
    pub fn company_data(&self) -> al_testrun::company_data::CompanyData {
        al_testrun::company_data::CompanyData {
            codeunit: al_testrun::company_data::DEMOTOOL_CODEUNIT,
            name: al_testrun::company_data::DEMOTOOL_CODEUNIT_NAME.to_string(),
            resources: Some(self.resources.clone()),
            identity: self.identity.clone(),
            // Microsoft's Legacy lane builds the company in an on-prem container (the DemoTool app is `"target":
            // "OnPrem"`); only the company build runs so, the tests keep `--environment`.
            environment: Some(al_host::Environment::OnPrem),
            installed_first: Some(self.installed_first.clone()),
            method: None,
            more_steps: Vec::new(),
            no_client: false,
            evaluation_company: false,
        }
    }
}

/// The cache folder of a release.
pub fn cache_dir(cache_root: &Path, major: u32, minor: u32) -> PathBuf {
    cache_root.join("bcapps-demotool").join(format!("{major}.{minor}"))
}

/// The DemoTool of release `major.minor`: `explicit` (`--demotool DIR`, a BCApps checkout) as it is, else the cache
/// folder, else a fetch of `releases/<major>.<minor>` into it unless `offline`.
pub fn resolve(
    explicit: Option<&Path>,
    major: u32,
    minor: u32,
    cache_root: &Path,
    offline: bool,
    url: &str,
    fetch: &dyn Fn(&str, &str, &Path) -> Result<(), String>,
) -> Result<DemoTool, String> {
    let (dir, how) = match explicit {
        Some(d) => {
            if !d.is_dir() {
                return Err(format!("--demotool {}: not a folder", d.display()));
            }
            (d.to_path_buf(), format!("DemoTool: {} (--demotool)", d.display()))
        }
        None => {
            let dir = cache_dir(cache_root, major, minor);
            let own = bcapps::branch(major, minor);
            // A cache folder from before the install groups were part of the fetch is fetched again.
            if !dir.join(COMPLETE).is_file() || !dir.join(GROUPS).is_file() {
                if offline {
                    return Err(format!(
                        "the DemoTool of {major}.{minor} is not in the cache ({}) and --no-download forbids fetching it; run once without --no-download, or name a BCApps checkout with --demotool DIR",
                        dir.display()
                    ));
                }
                let parent = dir.parent().ok_or("bad cache folder")?;
                std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
                let tmp = parent.join(format!(".fetch-{major}.{minor}-{}", std::process::id()));
                let _ = std::fs::remove_dir_all(&tmp);
                let done = (|| -> Result<(), String> {
                    fetch(url, &own, &tmp).map_err(|e| format!("fetching the DemoTool of {own} of {url}: {e}"))?;
                    if !tmp.join(SPARSE_PATHS[1]).join("app.json").is_file() {
                        return Err(format!("{own} of {url} has no {}/app.json", SPARSE_PATHS[1]));
                    }
                    stage(&tmp)?;
                    std::fs::write(tmp.join(COMPLETE), format!("fetched {own} of {url}\n"))
                        .map_err(|e| e.to_string())?;
                    let _ = std::fs::remove_dir_all(&dir);
                    std::fs::rename(&tmp, &dir).map_err(|e| format!("{}: {e}", dir.display()))
                })();
                if let Err(e) = done {
                    let _ = std::fs::remove_dir_all(&tmp);
                    return Err(e);
                }
            }
            let marker = std::fs::read_to_string(dir.join(COMPLETE)).unwrap_or_default();
            let how = format!("DemoTool: {} ({})", dir.display(), marker.trim());
            (dir, how)
        }
    };
    let app_dir = dir.join(SPARSE_PATHS[1]);
    let text = std::fs::read_to_string(app_dir.join("app.json"))
        .map_err(|e| format!("{}: {e}", app_dir.join("app.json").display()))?;
    let app = al_deps::load::parse_app_json(&text).map_err(|e| format!("{}: {e}", app_dir.display()))?;
    let resources = dir.join(STAGED);
    if !resources.join("DemoDataConfig.xml").is_file() {
        // A named checkout is staged into a folder of its own in the cache, never into the checkout.
        if explicit.is_some() {
            let staged = cache_root
                .join("bcapps-demotool")
                .join(format!("staged-{}", short_hash(dir.to_string_lossy().as_bytes())));
            let _ = std::fs::remove_dir_all(&staged);
            stage_into(&dir, &staged)?;
            return finish(&dir, app_dir, app, staged, how);
        }
        stage(&dir)?;
    }
    finish(&dir, app_dir, app, resources, how)
}

/// The apps BCApps' `Install-BaseAppsForDemoTool` installs before the DemoTool runs (build/scripts/
/// ImportTestDataInBcContainer.ps1): the apps of the groups `Base`, `TestFramework` and `LocalBaseExtensions` of the
/// country (`Get-ApplicationGroup`, ALAppBuild.psm1 `GetApplicationsInGroup`: an app is in a group when the group lists
/// no `countries` or lists the country, and does not list it in `exclude_countries`), and the DemoTool. Like the script,
/// it takes the names and leaves `doNotInstall` aside.
fn installed_first(groups_json: &str, country: &str) -> Result<Vec<String>, String> {
    let apps: Vec<serde_json::Value> = serde_json::from_str(groups_json).map_err(|e| format!("{GROUPS}: {e}"))?;
    let lists = |g: &serde_json::Value, key: &str| {
        g.get(key)
            .and_then(|v| v.as_array())
            .map(|a| a.iter().any(|c| c.as_str() == Some(country)))
    };
    let mut names: Vec<String> = apps
        .iter()
        .filter(|a| {
            a.get("groups").and_then(|g| g.as_array()).is_some_and(|groups| {
                groups.iter().any(|g| {
                    let named = g.get("name").and_then(|n| n.as_str());
                    matches!(named, Some("Base" | "TestFramework" | "LocalBaseExtensions"))
                        && lists(g, "countries").unwrap_or(true)
                        && !lists(g, "exclude_countries").unwrap_or(false)
                })
            })
        })
        .filter_map(|a| a.get("name").and_then(|n| n.as_str()).map(String::from))
        .collect();
    names.push("DemoTool".to_string());
    names.sort();
    names.dedup();
    Ok(names)
}

fn finish(dir: &Path, app_dir: PathBuf, app: AppJson, resources: PathBuf, how: String) -> Result<DemoTool, String> {
    let groups = dir.join(GROUPS);
    let text = std::fs::read_to_string(&groups).map_err(|e| format!("{}: {e}", groups.display()))?;
    let installed_first = installed_first(&text, COUNTRY)?;
    let mut h = Sha256::new();
    hash_tree(&app_dir, &app_dir, &mut h)?;
    hash_tree(&resources, &resources, &mut h)?;
    // What the company is built with: the apps installed first, and the server kind.
    h.update(installed_first.join("\n").as_bytes());
    h.update(b"onprem");
    let identity = format!("{} {}", app.version, hex(&h.finalize()));
    Ok(DemoTool {
        app_dir,
        app,
        resources,
        identity,
        how,
        installed_first,
    })
}

fn short_hash(b: &[u8]) -> String {
    hex(&Sha256::digest(b))[..16].to_string()
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Every file below `dir`, by its path relative to `root` and its bytes, in name order.
fn hash_tree(root: &Path, dir: &Path, h: &mut Sha256) -> Result<(), String> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            hash_tree(root, &p, h)?;
        } else {
            let rel = p.strip_prefix(root).unwrap_or(&p).to_string_lossy().into_owned();
            let bytes = std::fs::read(&p).map_err(|e| format!("{}: {e}", p.display()))?;
            h.update((rel.len() as u64).to_le_bytes());
            h.update(rel.as_bytes());
            h.update((bytes.len() as u64).to_le_bytes());
            h.update(&bytes);
        }
    }
    Ok(())
}

/// Stages the resource files of the checkout `dir` into `dir/.alrunner-resources`.
fn stage(dir: &Path) -> Result<(), String> {
    stage_into(dir, &dir.join(STAGED))
}

/// What `Initialize-DemoToolResources` stages for W1: the content of `src/DemoTool/Pictures`, the folder
/// `src/DemoTool/O365` as `O365`, and `DemoDataConfig.xml`, whose `DataType` it sets to `Extended` (the W1 file has it
/// already; another value is refused rather than rewritten).
fn stage_into(dir: &Path, out: &Path) -> Result<(), String> {
    let src = dir.join(SPARSE_PATHS[0]);
    let config = src.join("DemoDataConfig.xml");
    let text = std::fs::read_to_string(&config).map_err(|e| format!("{}: {e}", config.display()))?;
    if !text.contains("<DataType>Extended</DataType>") {
        return Err(format!(
            "{}: its DataType is not Extended; AL Runner does not rewrite the file",
            config.display()
        ));
    }
    let tmp = out.with_extension("tmp");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).map_err(|e| format!("{}: {e}", tmp.display()))?;
    copy_tree(&src.join("Pictures"), &tmp)?;
    let o365 = ["O365", "o365"].iter().map(|n| src.join(n)).find(|p| p.is_dir());
    if let Some(o) = o365 {
        copy_tree(&o, &tmp.join("O365"))?;
    }
    std::fs::write(tmp.join("DemoDataConfig.xml"), text).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_dir_all(out);
    std::fs::rename(&tmp, out).map_err(|e| format!("{}: {e}", out.display()))
}

/// The patterns BCApps' `.gitattributes` marks `binary` (lines 3-16); every other file is `* text=auto` (line 1).
const BINARY: [&str; 14] = [
    "app", "png", "jpg", "gif", "ico", "ttf", "woff", "woff2", "eot", "pdf", "zip", "dll", "exe", "snk",
];

/// Whether a Windows checkout of BCApps has CRLF line ends in this file. Microsoft's CI checks BCApps out on Windows, and
/// BCApps' `.gitattributes` says `* text=auto`: git treats a file as text unless a `binary` pattern names it (matched
/// without case, as on Windows) or its first 8000 bytes hold a NUL byte, and checks text out with CRLF. BC needs them:
/// a VariableText XmlPort import with LF-only line ends fails on Windows BC 27.5 and 29.0 as on Linux (the DemoTool's
/// CurrencyData.txt, coordinator's Windows check of 2026-10-22).
fn checked_out_as_text(name: &str, bytes: &[u8]) -> bool {
    let ext = name
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    !BINARY.contains(&ext.as_str()) && !bytes[..bytes.len().min(8000)].contains(&0)
}

/// `bytes` with every LF that has no CR before it turned into CRLF.
fn with_crlf(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len() + bytes.len() / 20);
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'\n' && (i == 0 || bytes[i - 1] != b'\r') {
            out.push(b'\r');
        }
        out.push(b);
    }
    out
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|e| format!("{}: {e}", to.display()))?;
    for e in std::fs::read_dir(from).map_err(|e| format!("{}: {e}", from.display()))? {
        let e = e.map_err(|e| e.to_string())?;
        let p = e.path();
        let dest = to.join(e.file_name());
        if p.is_dir() {
            copy_tree(&p, &dest)?;
        } else {
            let bytes = std::fs::read(&p).map_err(|e| format!("{}: {e}", p.display()))?;
            let bytes = if checked_out_as_text(&e.file_name().to_string_lossy(), &bytes) {
                with_crlf(&bytes)
            } else {
                bytes
            };
            std::fs::write(&dest, bytes).map_err(|e| format!("{}: {e}", dest.display()))?;
        }
    }
    Ok(())
}

/// The real fetch: the DemoTool's parts of the branch.
pub fn git_fetch(url: &str, branch: &str, dest: &Path) -> Result<(), String> {
    bcapps::git_fetch_parts(url, branch, dest, &SPARSE_PATHS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_checkout(dest: &Path) -> Result<(), String> {
        let app = dest.join("src/Layers/W1/DemoTool");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::write(
            app.join("app.json"),
            r#"{"id":"3f63a3de-6597-4097-9f34-b0ff7f92e6b0","name":"DemoTool","publisher":"Microsoft","version":"29.0.0.0","application":"29.0.0.0"}"#,
        )
        .unwrap();
        std::fs::write(app.join("X.Codeunit.al"), "codeunit 101899 X {}").unwrap();
        let src = dest.join("src/DemoTool");
        std::fs::create_dir_all(src.join("Pictures/Images/Item")).unwrap();
        std::fs::create_dir_all(src.join("o365")).unwrap();
        std::fs::write(src.join("Pictures/CurrencyData.TXT"), "AED;784\nAFN;971\r\n").unwrap();
        std::fs::write(src.join("Pictures/1896-S.BMP"), b"BM\0\n\x01").unwrap();
        std::fs::write(src.join("Pictures/Item.png"), b"png\n").unwrap();
        std::fs::write(src.join("Pictures/Images/Item/1000.jpg"), "j").unwrap();
        std::fs::write(src.join("o365/HyperLink.xml"), "h").unwrap();
        std::fs::write(
            src.join("DemoDataConfig.xml"),
            "<DemoDataSetup><DataType>Extended</DataType></DemoDataSetup>",
        )
        .unwrap();
        std::fs::create_dir_all(dest.join("build")).unwrap();
        std::fs::write(dest.join("build/groups.json"), GROUPS_JSON).unwrap();
        Ok(())
    }

    /// The shape of BCApps' build/groups.json: an app is in a group, for some countries only, or not in it.
    const GROUPS_JSON: &str = r#"[
        {"name": "System Application", "groups": [{"name": "Base"}, {"name": "Financials"}]},
        {"name": "Test Runner", "groups": [{"name": "Base", "doNotInstall": true}, {"name": "TestFramework"}]},
        {"name": "FI Core", "groups": [{"name": "LocalBaseExtensions"}]},
        {"name": "EU 3-Party Trade Purchase", "groups": [{"name": "LocalBaseExtensions", "countries": ["CZ"]}]},
        {"name": "Not For W1", "groups": [{"name": "Base", "exclude_countries": ["W1"]}]},
        {"name": "Shopify Connector", "groups": [{"name": "Financials"}]}
    ]"#;

    #[test]
    fn the_company_is_built_on_prem_with_only_the_base_apps_installed_first() {
        // BCApps build/scripts/ImportTestDataInBcContainer.ps1, Install-BaseAppsForDemoTool: the groups Base,
        // TestFramework and LocalBaseExtensions of the country, and the DemoTool.
        let root = tempfile::tempdir().unwrap();
        let d = resolve(
            None,
            29,
            0,
            root.path(),
            false,
            "u",
            &|_: &str, _: &str, dest: &Path| fake_checkout(dest),
        )
        .unwrap();
        let c = d.company_data();
        assert_eq!(c.environment, Some(al_host::Environment::OnPrem));
        let mut first = c.installed_first.unwrap();
        first.sort();
        assert_eq!(first, vec!["DemoTool", "FI Core", "System Application", "Test Runner"]);
    }

    #[test]
    fn a_cache_folder_from_before_the_install_groups_is_fetched_again() {
        let root = tempfile::tempdir().unwrap();
        let fetched = std::cell::Cell::new(0);
        let fetch = |_: &str, _: &str, dest: &Path| {
            fetched.set(fetched.get() + 1);
            fake_checkout(dest)
        };
        resolve(None, 29, 0, root.path(), false, "u", &fetch).unwrap();
        std::fs::remove_file(cache_dir(root.path(), 29, 0).join(GROUPS)).unwrap();
        let e = resolve(None, 29, 0, root.path(), true, "u", &fetch).unwrap_err();
        assert!(e.contains("--no-download"), "{e}");
        resolve(None, 29, 0, root.path(), false, "u", &fetch).unwrap();
        assert_eq!(fetched.get(), 2);
    }

    #[test]
    fn the_first_run_fetches_and_stages_the_files_as_bcapps_does_and_the_next_reads_the_cache() {
        let root = tempfile::tempdir().unwrap();
        let fetched = std::cell::Cell::new(0);
        let fetch = |_: &str, branch: &str, dest: &Path| {
            assert_eq!(branch, "releases/29.0");
            fetched.set(fetched.get() + 1);
            fake_checkout(dest)
        };
        let d = resolve(None, 29, 0, root.path(), false, "u", &fetch).unwrap();
        assert_eq!(d.app.name, "DemoTool");
        assert_eq!(d.dependency().id, "3f63a3de-6597-4097-9f34-b0ff7f92e6b0");
        assert!(
            d.resources.join("CurrencyData.TXT").is_file(),
            "the content of Pictures is at the top"
        );
        assert!(d.resources.join("Images/Item/1000.jpg").is_file());
        assert!(
            d.resources.join("O365/HyperLink.xml").is_file(),
            "O365 is a folder of its own"
        );
        assert!(d.resources.join("DemoDataConfig.xml").is_file());
        // `* text=auto` (BCApps .gitattributes:1): a Windows checkout has CRLF in text files; binary ones stay as they are.
        assert_eq!(
            std::fs::read(d.resources.join("CurrencyData.TXT")).unwrap(),
            b"AED;784\r\nAFN;971\r\n"
        );
        assert_eq!(std::fs::read(d.resources.join("1896-S.BMP")).unwrap(), b"BM\0\n\x01");
        assert_eq!(
            std::fs::read(d.resources.join("Item.png")).unwrap(),
            b"png\n",
            "*.png binary"
        );
        let again = resolve(None, 29, 0, root.path(), true, "u", &fetch).unwrap();
        assert_eq!(fetched.get(), 1, "the second run reads the cache");
        assert_eq!(again.identity, d.identity);
        assert_eq!(d.company_data().codeunit, 101_899);
    }

    #[test]
    fn offline_without_the_cache_is_refused() {
        let root = tempfile::tempdir().unwrap();
        let fetch = |_: &str, _: &str, _: &Path| -> Result<(), String> { panic!("no fetch offline") };
        let e = resolve(None, 29, 0, root.path(), true, "u", &fetch).unwrap_err();
        assert!(e.contains("--no-download"), "{e}");
    }

    #[test]
    fn a_config_whose_data_type_is_not_extended_is_refused() {
        let root = tempfile::tempdir().unwrap();
        let fetch = |_: &str, _: &str, dest: &Path| {
            fake_checkout(dest)?;
            std::fs::write(
                dest.join("src/DemoTool/DemoDataConfig.xml"),
                "<DemoDataSetup><DataType>Standard</DataType></DemoDataSetup>",
            )
            .unwrap();
            Ok(())
        };
        let e = resolve(None, 29, 0, root.path(), false, "u", &fetch).unwrap_err();
        assert!(e.contains("Extended"), "{e}");
        assert!(
            !cache_dir(root.path(), 29, 0).exists(),
            "a failed fetch leaves no cache folder"
        );
    }
}
