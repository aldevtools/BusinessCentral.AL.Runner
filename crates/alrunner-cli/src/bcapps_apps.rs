//! The test apps Microsoft's CI runs that the artifact does not hold (ADR-052): their source from BCApps.
//!
//! Microsoft's CI installs every app of the projects its `Test Apps <COUNTRY>` project tests (`projectsToTest`, for W1
//! `build/projects/Apps W1`) and runs the tests of all of them (`runTestsInAllInstalledTestApps`). For BC 29 that
//! dispatches 15 test apps the artifact has no source archive of: AlCosting, the Intelligent Cloud tests, the *-Internal
//! apps and others (BCApps releases/29.x, run 39519586834). Their source is public in BCApps, in the folders the
//! project lists. This module reads those folder lists, reads each folder's `app.json` (fetched sparsely into the cache
//! with `bcapps::ensure_sources`), and decides which test apps come from BCApps and which of their dependencies the
//! artifact lacks too. Nothing from BCApps is committed to this repository (hard rule 7).

use std::path::{Component, Path, PathBuf};

/// One app folder of the project, as its `app.json` says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BcApp {
    /// The folder below the repository root (`src/Layers/W1/AlCosting`).
    pub folder: String,
    pub name: String,
    pub id: String,
    /// The ids of its dependencies.
    pub deps: Vec<String>,
}

/// One folder of a project's settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Folder {
    /// Below the repository root, with `/` (`src/Layers/W1/AlCosting`).
    pub path: String,
    /// The project that lists it (`build/projects/Apps W1`).
    pub project: String,
    /// Listed in `testFolders`: Microsoft's CI runs its tests. An `appFolders` app with tests is not dispatched
    /// (run 39519586834 did not run Agent Samples Tests, an `appFolders` entry).
    pub test: bool,
}

/// The folders of the projects the country's test project tests. `None` when the test project names no project
/// (`projectsToTest`): then nothing comes from BCApps.
pub fn project_folders(root: &Path, country: &str) -> Result<Option<Vec<Folder>>, String> {
    let test_project = root
        .join("build/projects")
        .join(format!("Test Apps {}", country.to_ascii_uppercase()))
        .join(".AL-Go/settings.json");
    let read = |p: &Path| -> Result<serde_json::Value, String> {
        let text = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
        serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| format!("{}: {e}", p.display()))
    };
    let strings = |v: &serde_json::Value, k: &str| -> Vec<String> {
        v.get(k)
            .and_then(|a| a.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
            .unwrap_or_default()
    };
    let projects = strings(&read(&test_project)?, "projectsToTest");
    if projects.is_empty() {
        return Ok(None);
    }
    let mut out: Vec<Folder> = Vec::new();
    for project in projects {
        let settings = read(&root.join(&project).join(".AL-Go/settings.json"))?;
        let tests = strings(&settings, "testFolders");
        for (f, test) in strings(&settings, "appFolders")
            .into_iter()
            .map(|f| (f, false))
            .chain(tests.into_iter().map(|f| (f, true)))
        {
            let Some(path) = normalize(&project, &f) else { continue };
            match out.iter_mut().find(|o| o.path.eq_ignore_ascii_case(&path)) {
                Some(o) => o.test |= test,
                None => out.push(Folder {
                    path,
                    project: project.clone(),
                    test,
                }),
            }
        }
    }
    Ok(Some(out))
}

/// `project/rel` as a path below the repository root with `/` (`build/projects/Apps W1` + `../../../src/X` is `src/X`);
/// `None` when it leaves the repository.
fn normalize(project: &str, rel: &str) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    for c in Path::new(project).join(rel.replace('\\', "/")).components() {
        match c {
            Component::Normal(p) => parts.push(p.to_string_lossy().into_owned()),
            Component::ParentDir => {
                parts.pop()?;
            }
            Component::CurDir => {}
            _ => return None,
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// The apps of `folders` below `root`, and the folders that have no readable `app.json`.
pub fn index(root: &Path, folders: &[String]) -> (Vec<BcApp>, Vec<String>) {
    let mut apps = Vec::new();
    let mut missing = Vec::new();
    for folder in folders {
        let path = on_disk(root, folder).join("app.json");
        let parsed = std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(t.trim_start_matches('\u{feff}')).ok());
        let text = |v: &serde_json::Value, k: &str| v.get(k).and_then(|x| x.as_str()).map(str::to_string);
        match parsed
            .as_ref()
            .and_then(|v| Some((text(v, "name")?, text(v, "id")?, v)))
        {
            Some((name, id, v)) => apps.push(BcApp {
                folder: folder.clone(),
                name,
                id: id.to_ascii_lowercase(),
                deps: v
                    .get("dependencies")
                    .and_then(|d| d.as_array())
                    .map(|d| {
                        d.iter()
                            .filter_map(|x| text(x, "id").or_else(|| text(x, "appId")))
                            .map(|s| s.to_ascii_lowercase())
                            .collect()
                    })
                    .unwrap_or_default(),
            }),
            None => missing.push(folder.clone()),
        }
    }
    (apps, missing)
}

/// The key two names of the same app share in the artifact and in BCApps: without case, punctuation, spaces and a
/// plural `s` (`E-Document Connector - FORNAV Tests` and `- ForNAV Tests`, `Subcontracting-Tests` and
/// `Subcontracting Test`).
pub fn name_key(name: &str) -> String {
    let k: String = name
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();
    k.strip_suffix('s').map(str::to_string).unwrap_or(k)
}

/// Whether `name` is among `names` by [`name_key`].
pub fn has_name(names: &[&str], name: &str) -> bool {
    let k = name_key(name);
    names.iter().any(|n| name_key(n) == k)
}

/// The apps that must come from BCApps for `app`: the app and, transitively, every dependency in the index whose name
/// `in_artifact` does not hold. Dependencies outside the index (the platform, Base Application) are the artifact's.
pub fn closure<'a>(apps: &'a [BcApp], app: &'a BcApp, in_artifact: &dyn Fn(&str) -> bool) -> Vec<&'a BcApp> {
    let mut out: Vec<&BcApp> = vec![app];
    let mut i = 0;
    while i < out.len() {
        for d in &out[i].deps.clone() {
            if let Some(dep) = apps.iter().find(|a| a.id == *d) {
                if !in_artifact(&dep.name) && !out.iter().any(|o| o.id == dep.id) {
                    out.push(dep);
                }
            }
        }
        i += 1;
    }
    out
}

/// The sparse pattern of a folder's `app.json`, and of a whole folder.
pub fn app_json_pattern(folder: &str) -> String {
    format!("/{folder}/app.json")
}

pub fn folder_pattern(folder: &str) -> String {
    format!("/{folder}/")
}

/// The folder of `app` below `root`.
pub fn path_of(root: &Path, app: &BcApp) -> PathBuf {
    on_disk(root, &app.folder)
}

/// `root/folder` with each part spelled as on disk: the project settings spell folders as Windows reads them, without
/// case (`EDocumentConnectors/ForNav/Test` for `ForNAV/Test`). A part not found keeps its spelling.
pub fn on_disk(root: &Path, folder: &str) -> PathBuf {
    let mut p = root.to_path_buf();
    for part in folder.split('/') {
        let exact = p.join(part);
        if exact.exists() {
            p = exact;
            continue;
        }
        let found = std::fs::read_dir(&p).ok().and_then(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .find(|n| n.eq_ignore_ascii_case(part))
        });
        p = p.join(found.as_deref().unwrap_or(part));
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(folder: &str, name: &str, id: &str, deps: &[&str]) -> BcApp {
        BcApp {
            folder: folder.into(),
            name: name.into(),
            id: id.into(),
            deps: deps.iter().map(|d| d.to_string()).collect(),
        }
    }

    #[test]
    fn a_folder_is_found_on_disk_without_case() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("src/Apps/ForNAV/Test")).unwrap();
        assert_eq!(
            on_disk(root.path(), "src/Apps/ForNav/test"),
            root.path().join("src/Apps/ForNAV/Test")
        );
        assert_eq!(on_disk(root.path(), "src/Gone/x"), root.path().join("src/Gone/x"));
    }

    #[test]
    fn project_folders_are_read_relative_to_their_project() {
        assert_eq!(
            normalize("build/projects/Apps W1", "../../../src/Layers/W1/AlCosting"),
            Some("src/Layers/W1/AlCosting".into())
        );
        assert_eq!(normalize("build/projects/Apps W1", "../../../../x"), None);
    }

    #[test]
    fn names_of_the_same_app_match_without_case_punctuation_and_plural() {
        assert_eq!(
            name_key("E-Document Connector - FORNAV Tests"),
            name_key("E-Document Connector - ForNAV Tests")
        );
        assert_eq!(name_key("Subcontracting-Tests"), name_key("Subcontracting Test"));
        assert!(has_name(&["OnPrem Permissions Tests"], "OnPrem Permissions Test"));
        assert!(!has_name(&["Intelligent Cloud Base Tests"], "Intelligent Cloud Base"));
    }

    #[test]
    fn the_closure_takes_the_dependencies_the_artifact_lacks() {
        let apps = vec![
            app("t", "Cloud Tests", "1", &["2", "3", "9"]),
            app("a", "Cloud Base", "2", &["4"]),
            app("l", "Tests-TestLibraries", "3", &[]),
            app("b", "Cloud Core", "4", &[]),
        ];
        let in_artifact = |n: &str| n == "Tests-TestLibraries";
        let got: Vec<&str> = closure(&apps, &apps[0], &in_artifact)
            .iter()
            .map(|a| a.name.as_str())
            .collect();
        assert_eq!(got, vec!["Cloud Tests", "Cloud Base", "Cloud Core"]);
    }
}
