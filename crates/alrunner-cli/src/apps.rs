//! Reading the APPs given on the command line: a folder, a source archive or an `.app`.

use al_deps::load::{AppJson, read_package_at_with};
use al_deps::select::{APPLICATION_APP_ID, DepDecl, SYSTEM_APP_ID};
use al_sema::workspace::AppSource;
use al_testrun::runtime::Manifest;
use al_version::AppVersions;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// One app whose tests run.
pub struct AppInput {
    pub path: PathBuf,
    pub source: AppSource,
    pub json: AppJson,
    pub manifest: Manifest,
    pub versions: AppVersions,
    /// `preprocessorSymbols` of the app's app.json (an app folder only).
    pub symbols: Vec<String>,
}

/// Reads `path`. `build` is an exact build that fills Microsoft's build variables in a source
/// archive that names none itself.
pub fn load(path: &Path, build: Option<&str>) -> Result<AppInput, String> {
    if !path.exists() {
        return Err(format!("{}: no such file or folder", path.display()));
    }
    if path.is_dir() && !path.join("app.json").is_file() {
        return Err(format!(
            "{}: an app folder needs an app.json (to run a folder of packages, name each package)",
            path.display()
        ));
    }
    let (m, files) = read_package_at_with(path, build)?;
    let mut manifest = Manifest {
        publisher: m.publisher.clone(),
        version: m.version.to_string(),
        resources: Vec::new(),
        include_source: false,
        // Not read here: a RecordRef.Open that depends on the target refuses.
        target: String::new(),
        layout_files: Vec::new(),
        id_ranges: Vec::new(),
        symbols: Vec::new(),
    };
    let mut versions = AppVersions::parse(
        None,
        m.platform.map(|v| v.to_string()).as_deref(),
        m.application.map(|v| v.to_string()).as_deref(),
    )
    .map_err(|e| format!("{}: {e}", path.display()))?;
    let mut symbols = Vec::new();
    if path.is_dir() {
        // The folder's own app.json: it can say more than the package manifest does.
        manifest = al_testrun::runtime::load_manifest(path)?;
        versions = al_testrun::target::read_app_versions(path)?;
        let text = std::fs::read_to_string(path.join("app.json")).map_err(|e| e.to_string())?;
        let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("app.json: {e}"))?;
        symbols = json["preprocessorSymbols"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|s| s.as_str().map(String::from))
            .collect();
    }
    let source = AppSource {
        id: m.id.clone(),
        name: m.name.clone(),
        dependencies: m.dependencies.iter().map(|d| d.id.clone()).collect(),
        files,
    };
    let json = AppJson {
        id: m.id,
        name: m.name,
        publisher: m.publisher,
        version: m.version,
        platform: m.platform,
        application: m.application,
        dependencies: m.dependencies,
    };
    Ok(AppInput {
        path: path.to_path_buf(),
        source,
        json,
        manifest,
        versions,
        symbols,
    })
}

/// The project the dependency resolution works for: all APPs together. Its dependencies are
/// those of the APPs, minus the APPs themselves (they are in the workspace).
pub fn combined_project(apps: &[AppInput]) -> AppJson {
    let own: BTreeSet<&str> = apps.iter().map(|a| a.json.id.as_str()).collect();
    let mut dependencies: Vec<DepDecl> = Vec::new();
    for a in apps {
        for d in &a.json.dependencies {
            if own.contains(d.id.as_str()) {
                continue;
            }
            match dependencies.iter_mut().find(|x| x.id == d.id) {
                Some(x) => x.min = x.min.max(d.min),
                None => dependencies.push(d.clone()),
            }
        }
    }
    let max = |f: fn(&AppJson) -> Option<al_deps::version::Version>| apps.iter().filter_map(|a| f(&a.json)).max();
    // `platform` and `application` in app.json are dependencies on the Microsoft apps `System` and
    // `Application` (which pulls in Base Application and the rest). The resolver treats them as
    // optional roots; declaring them lets the artifact feed supply them when nothing else does.
    for (id, name, floor) in [
        (SYSTEM_APP_ID, "System", max(|j| j.platform)),
        (APPLICATION_APP_ID, "Application", max(|j| j.application)),
    ] {
        if let Some(min) = floor.filter(|_| !dependencies.iter().any(|d| d.id == id)) {
            dependencies.push(DepDecl {
                id: id.to_string(),
                name: name.to_string(),
                publisher: "Microsoft".to_string(),
                min,
                optional: false,
            });
        }
    }
    AppJson {
        id: "alrunner-project".into(),
        name: "alrunner-project".into(),
        publisher: String::new(),
        version: Default::default(),
        platform: max(|j| j.platform),
        application: max(|j| j.application),
        dependencies,
    }
}

/// Whether `publisher` is Microsoft.
pub fn is_microsoft(publisher: &str) -> bool {
    publisher.eq_ignore_ascii_case("Microsoft")
}

/// The build the tests themselves come from: Microsoft's test archives and packages carry it in
/// the folder they lie in (`.../<build>/platform/...`) or as their version. Ahead of the feed's
/// newest build, the apps, `System.app` and the demo data should come from this one.
pub fn own_build(inputs: &[AppInput]) -> Option<al_deps::build::BuildId> {
    use al_deps::build::BuildId;
    inputs
        .iter()
        .filter(|i| is_microsoft(&i.manifest.publisher))
        .find_map(|i| {
            i.path
                .components()
                .rev()
                .filter_map(|c| c.as_os_str().to_str())
                .find_map(|c| BuildId::parse(c).ok())
                .or_else(|| BuildId::parse(&i.manifest.version).ok())
        })
}

/// What decided the Microsoft build of a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildChoice {
    /// The build the feed and the test data should come from when it is cached, on disk or published.
    pub preferred: Option<al_deps::build::BuildId>,
    /// One line for the run header: the rule that applied.
    pub rule: String,
    /// The target's BC minor, when the packages say it.
    pub minor: Option<u32>,
    /// The packages named a build that was not used, and why.
    pub note: Option<String>,
    /// `preferred` is the build of the Microsoft packages the project carries.
    pub from_packages: bool,
}

/// The rule for the Microsoft build of a run (owner decision 2026-10-15): in this order,
/// 1. `--bc-version`;
/// 2. the build of the tests, when they are Microsoft's (`own_build`);
/// 3. the build of the Microsoft packages the project carries in its own package folders
///    (`.alpackages`, `--dep`, `--package-cache`; symbols only or with source): what its developer
///    compiled against, so the build the project really targets. Used when it is of the target's
///    major and not below the app.json floor (`application`, `platform`);
/// 4. the newest published build of the target's major at or above the floor.
///
/// The build wanted may be neither cached, on disk nor published; then the feed's usual rules pick one
/// and the run says so.
pub fn choose_build(
    bc_version: Option<&str>,
    own: Option<al_deps::build::BuildId>,
    packages: Option<(al_deps::build::BuildId, String)>,
    floor: Option<(u32, u32)>,
    target_major: u32,
) -> BuildChoice {
    // What app.json asks for, in words that read right with and without a floor.
    let (asks, above, above_floor) = match floor {
        Some((a, b)) => (
            format!("asks for {a}.{b}"),
            format!(" at or above the floor {a}.{b}"),
            format!(" at or above the app.json floor {a}.{b}"),
        ),
        None => (
            "sets no floor".to_string(),
            " (app.json sets no floor)".to_string(),
            String::new(),
        ),
    };
    if let Some(v) = bc_version {
        return BuildChoice {
            preferred: None,
            rule: format!("--bc-version {v}"),
            minor: None,
            note: None,
            from_packages: false,
        };
    }
    if let Some(b) = own {
        return BuildChoice {
            rule: format!("the build of the tests ({b})"),
            preferred: Some(b),
            minor: None,
            note: None,
            from_packages: false,
        };
    }
    if let Some((b, from)) = packages {
        let floor_ok = floor.is_none_or(|f| (b.major(), b.minor()) >= f);
        if b.major() == target_major && floor_ok {
            return BuildChoice {
                rule: format!("the build of the Microsoft packages the project carries ({from}); app.json {asks}"),
                minor: Some(b.minor()),
                preferred: Some(b),
                note: None,
                from_packages: true,
            };
        }
        return BuildChoice {
            preferred: None,
            rule: format!("newest published build of BC {target_major}{above}"),
            minor: None,
            note: Some(format!(
                "the Microsoft packages in the project's package folders are build {b} ({from}), which is not of BC {target_major}{above_floor}; not used to choose the build"
            )),
            from_packages: false,
        };
    }
    BuildChoice {
        preferred: None,
        rule: format!("newest published build of BC {target_major}{above}"),
        minor: None,
        note: None,
        from_packages: false,
    }
}

#[cfg(test)]
mod build_choice_tests {
    use super::*;
    use al_deps::build::BuildId;

    fn b(s: &str) -> BuildId {
        BuildId::parse(s).unwrap()
    }

    #[test]
    fn the_packages_the_project_carries_name_the_build() {
        let c = choose_build(
            None,
            None,
            Some((b("28.4.53241.53504"), "Base Application".into())),
            Some((28, 0)),
            28,
        );
        assert_eq!(c.preferred, Some(b("28.4.53241.53504")));
        assert_eq!(c.minor, Some(4));
        assert!(c.from_packages && c.rule.contains("Microsoft packages") && c.rule.contains("asks for 28.0"));
    }

    #[test]
    fn packages_below_the_floor_or_of_another_major_are_not_used_and_the_run_says_so() {
        for (pkg, floor, major) in [("28.0.1.1", (28, 2), 28), ("27.5.46862.48827", (27, 0), 28)] {
            let c = choose_build(None, None, Some((b(pkg), "x".into())), Some(floor), major);
            assert_eq!(c.preferred, None);
            assert!(c.note.as_deref().unwrap().contains("not used"), "{c:?}");
            assert!(c.rule.starts_with("newest published build"));
        }
    }

    #[test]
    fn bc_version_and_the_build_of_the_tests_come_first() {
        let pkg = Some((b("28.4.53241.53504"), "x".into()));
        let c = choose_build(Some("28.1"), Some(b("28.2.1.1")), pkg.clone(), None, 28);
        assert_eq!((c.preferred, c.rule.as_str()), (None, "--bc-version 28.1"));
        let c = choose_build(None, Some(b("28.2.1.1")), pkg, None, 28);
        assert_eq!(c.preferred, Some(b("28.2.1.1")));
        assert!(c.rule.contains("tests"));
    }

    #[test]
    fn without_anything_the_floor_and_the_major_decide() {
        let c = choose_build(None, None, None, Some((28, 0)), 28);
        assert_eq!(c.preferred, None);
        assert!(c.rule.contains("at or above the floor 28.0"));
    }

    #[test]
    fn without_a_floor_in_app_json_the_rule_says_so_in_words() {
        let c = choose_build(None, None, None, None, 28);
        assert_eq!(c.rule, "newest published build of BC 28 (app.json sets no floor)");
        let c = choose_build(None, None, Some((b("28.4.53241.53504"), "x".into())), None, 28);
        assert!(c.rule.ends_with("app.json sets no floor"), "{c:?}");
        let c = choose_build(None, None, Some((b("27.5.46862.48827"), "x".into())), None, 28);
        let note = c.note.clone().unwrap();
        assert!(
            !note.contains("no floor") && note.contains("which is not of BC 28;"),
            "{note}"
        );
        assert!(!c.rule.contains("floor no floor"), "{c:?}");
    }
}
