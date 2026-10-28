//! Finding, choosing and reading the dependency apps of a project (ADR-013).
//!
//! The pool of packages is the project's `.alpackages` first, then any extra
//! directories (`--deps`). Downloading from the artifact feed is a later
//! fallback and is not here yet.

use crate::package::{
    Manifest, PackageError, Sources, Vars, manifest_of_source_zip, read_info_with, read_package_with,
};
use crate::select::{
    APPLICATION_APP_ID, Candidate, DepDecl, MissingPolicy, Options, Project, SYSTEM_APP_ID, SelectError, resolve,
};
use crate::version::Version;
use al_sema::workspace::AppSource;
use std::path::{Path, PathBuf};

/// What `app.json` says about an app: enough to resolve its dependencies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppJson {
    pub id: String,
    pub name: String,
    pub publisher: String,
    pub version: Version,
    pub platform: Option<Version>,
    pub application: Option<Version>,
    pub dependencies: Vec<DepDecl>,
}

/// Parses an `app.json`. Versions are minimums; a missing one is 0.0.0.0.
pub fn parse_app_json(text: &str) -> Result<AppJson, String> {
    let v: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let s = |o: &serde_json::Value, k: &str| o[k].as_str().unwrap_or_default().to_string();
    let ver = |o: &serde_json::Value, k: &str| -> Result<Option<Version>, String> {
        match o[k].as_str().filter(|t| !t.is_empty()) {
            Some(t) => t.parse().map(Some).map_err(|e| format!("`{k}`: {e}")),
            None => Ok(None),
        }
    };
    let mut dependencies = Vec::new();
    for d in v["dependencies"].as_array().map(Vec::as_slice).unwrap_or_default() {
        // Older manifests spell the id `appId`.
        let id = if d["id"].is_string() { s(d, "id") } else { s(d, "appId") };
        dependencies.push(DepDecl {
            id: id.to_ascii_lowercase(),
            name: s(d, "name"),
            publisher: s(d, "publisher"),
            min: ver(d, "version")?.unwrap_or_default(),
            optional: false,
        });
    }
    Ok(AppJson {
        id: s(&v, "id").to_ascii_lowercase(),
        name: s(&v, "name"),
        publisher: s(&v, "publisher"),
        version: ver(&v, "version")?.unwrap_or_default(),
        platform: ver(&v, "platform")?,
        application: ver(&v, "application")?,
        dependencies,
    })
}

/// A package file found in a search directory.
#[derive(Debug, Clone)]
pub struct Found {
    pub path: PathBuf,
    pub manifest: Manifest,
    pub has_source: bool,
}

fn ext_is(p: &Path, ext: &str) -> bool {
    p.extension().is_some_and(|x| x.eq_ignore_ascii_case(ext))
}

/// A file a search directory may hold: an `.app`, or a `.zip` (a source archive such as
/// `System Application Test Library.Source.zip`).
fn is_package_file(p: &Path) -> bool {
    ext_is(p, "app") || ext_is(p, "zip")
}

/// Files that are meant to be packages: every `.app` and every `*.Source.zip`.
fn is_named_package(p: &Path) -> bool {
    ext_is(p, "app")
        || p.file_name()
            .is_some_and(|n| n.to_string_lossy().to_ascii_lowercase().ends_with(".source.zip"))
}

/// The first four-number build (`27.5.46862.48827`) among the folder names of `path`, as the
/// platform artifact cache lays them out (`.../sandbox/27.5.46862.48827/platform/...`).
fn build_in_path(path: &Path) -> Option<Vars> {
    path.components()
        .rev()
        .filter_map(|c| c.as_os_str().to_str())
        .find_map(Vars::for_build)
}

/// The build variables for a package: the build named by the cache's record beside it
/// (`<file>.json`), else a build among the folders it lies in, else `hint` when that is an exact
/// build (a `--bc-version` prefix such as `27.5` is not one), else none.
fn vars_for(path: &Path, hint: Option<&str>) -> Vars {
    let mut record = path.as_os_str().to_owned();
    record.push(".json");
    std::fs::read(PathBuf::from(record))
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .and_then(|v| v["build"].as_str().and_then(Vars::for_build))
        .or_else(|| build_in_path(path))
        .or_else(|| hint.and_then(Vars::for_build))
        .unwrap_or_default()
}

/// The `.al` files under `dir`, as (path relative to `dir` with `/`, text).
fn read_folder_sources(dir: &Path) -> Result<Sources, String> {
    let mut out: Sources = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).map_err(|e| format!("{}: {e}", d.display()))? {
            let p = e.map_err(|e| e.to_string())?.path();
            if p.is_dir() {
                stack.push(p);
            } else if ext_is(&p, "al") {
                let rel = p.strip_prefix(dir).unwrap_or(&p).to_string_lossy().replace('\\', "/");
                let bytes = std::fs::read(&p).map_err(|e| format!("{}: {e}", p.display()))?;
                let text = String::from_utf8_lossy(&bytes);
                out.push((rel, text.strip_prefix('\u{feff}').unwrap_or(&text).to_string()));
            }
        }
    }
    out.sort();
    Ok(out)
}

fn folder_manifest(dir: &Path, vars: &Vars) -> Result<Manifest, String> {
    let p = dir.join("app.json");
    let text = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
    manifest_of_source_zip(&text, vars).map_err(|e| format!("{}: {e}", p.display()))
}

/// Reads a package file or an app folder (`app.json` and `.al` files), filling build variables
/// from the cache's record when it has one.
pub fn read_package_at(path: &Path) -> Result<(Manifest, Sources), String> {
    read_package_at_with(path, None)
}

/// [`read_package_at`] with a build to fill Microsoft's build variables when nothing else names one.
pub fn read_package_at_with(path: &Path, hint: Option<&str>) -> Result<(Manifest, Sources), String> {
    if path.is_dir() {
        let manifest = folder_manifest(path, &vars_for(path, hint))?;
        return Ok((manifest, read_folder_sources(path)?));
    }
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    read_package_with(&bytes, &vars_for(path, hint)).map_err(|e| format!("{}: {e}", path.display()))
}

/// Reads the manifest of every `*.app` and `*.zip` file directly inside `dir`. A file that
/// cannot be read is returned as an error line, not dropped: the caller prints it.
pub fn scan_dir(dir: &Path) -> (Vec<Found>, Vec<String>) {
    scan_path(dir, None)
}

/// One entry of a package search list (ADR-013): a directory of packages (as [`scan_dir`]), one
/// package file (`.app`, `*.Source.zip`), or one app folder (it holds an `app.json`). A file
/// named on purpose that is no package is a problem, not skipped. `hint` is the exact build that
/// fills Microsoft's build variables when no record or path names one.
pub fn scan_path(path: &Path, hint: Option<&str>) -> (Vec<Found>, Vec<String>) {
    let mut found = Vec::new();
    let mut problems = Vec::new();
    if path.is_dir() && path.join("app.json").is_file() {
        match folder_manifest(path, &vars_for(path, hint)) {
            Ok(manifest) => found.push(Found {
                path: path.to_path_buf(),
                manifest,
                has_source: true,
            }),
            Err(e) => problems.push(e),
        }
        return (found, problems);
    }
    let explicit = path.is_file();
    let paths: Vec<PathBuf> = if explicit {
        vec![path.to_path_buf()]
    } else {
        let Ok(rd) = std::fs::read_dir(path) else {
            return (found, problems);
        };
        let mut v: Vec<PathBuf> = rd
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| is_package_file(p))
            .collect();
        v.sort();
        v
    };
    // The files are read side by side (a big package is tens of megabytes); the results are
    // handled in path order.
    use rayon::prelude::*;
    let reads: Vec<Result<(Manifest, bool), String>> = paths
        .par_iter()
        .map(|path| {
            let vars = vars_for(path, hint);
            // The manifest and the zip directory are enough, and a big package is not read whole.
            if let Some(l) = crate::lazy::read_lazy(path, &vars) {
                return Ok((l.manifest, !l.names.is_empty()));
            }
            std::fs::read(path)
                .map_err(|e| e.to_string())
                .and_then(|bytes| read_info_with(&bytes, &vars).map_err(|e: PackageError| e.to_string()))
        })
        .collect();
    for (path, read) in paths.into_iter().zip(reads) {
        match read {
            Ok((manifest, has_source)) => found.push(Found {
                path,
                manifest,
                has_source,
            }),
            // A stray `.zip` in a directory is no problem; a broken `.app` or `.Source.zip`, or any
            // file the caller named, is.
            Err(_) if !explicit && !is_named_package(&path) => {}
            Err(e) => problems.push(format!("{}: {e}", path.display())),
        }
    }
    (found, problems)
}

/// An app the caller already has as source (built in this run).
#[derive(Debug, Clone)]
pub struct WorkspaceApp {
    pub json: AppJson,
    /// Shown in messages.
    pub origin: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// The platform's `System` app: system tables, no providers yet (ADR-013 step 4).
    Platform,
    Standard,
}

#[derive(Debug, Clone)]
pub struct DependencyApp {
    pub app: AppSource,
    pub publisher: String,
    pub version: Version,
    pub path: PathBuf,
    pub role: Role,
    /// With `Request::lazy_sources`, a package read lazily: `app.files` holds the paths and empty
    /// texts, and this reads a file and hashes the package (ADR-022, "pure tests").
    pub lazy: Option<al_sema::LazySource>,
    /// The `preprocessorSymbols` of the app's own `app.json`, or of `NavxManifest.xml` for a compiled package. The
    /// app's sources are preprocessed with these and not with the project's.
    pub preprocessor_symbols: Vec<String>,
    /// The packaged resources (`resources/` of a compiled package): name relative to the folder, content.
    pub resources: Resources,
    /// The package ships a translation file (`crate::package::is_translation_file`), ADR-053 step 1.
    pub translations: bool,
}

#[derive(Debug, Default)]
pub struct Loaded {
    /// Dependency apps in dependency order (dependencies first). Workspace apps are not here.
    pub apps: Vec<DependencyApp>,
    /// Lines worth printing: below-minimum Microsoft apps, skipped optional ones, unreadable files.
    pub notes: Vec<String>,
    /// Dependencies no package satisfied.
    pub skipped: Vec<SelectError>,
    /// The ids the chosen `Application` aggregate depends on (it holds no source, so it is not in
    /// `apps`); empty when none was chosen.
    pub application_aggregate: Vec<String>,
}

impl Loaded {
    /// The dependencies an app with a `platform` and/or an `application` floor has without
    /// declaring them: `System`, `Application` and what `Application` propagates.
    pub fn implied_dependencies(&self, platform: bool, application: bool) -> Vec<String> {
        let mut v = Vec::new();
        if application {
            v.push(APPLICATION_APP_ID.to_string());
            v.extend(self.application_aggregate.iter().cloned());
        }
        if platform {
            v.push(SYSTEM_APP_ID.to_string());
        }
        v
    }
}

pub struct Request<'a> {
    pub project: &'a AppJson,
    /// Apps the caller loads from source itself, e.g. the internals fixture.
    pub workspace_apps: &'a [WorkspaceApp],
    /// `<project>/.alpackages` first, then any `--deps` directories.
    pub search_dirs: &'a [PathBuf],
    pub missing: MissingPolicy,
    /// An exact build that fills Microsoft's build variables (`$(app_currentVersion)`) in source
    /// archives and folders that no record or path names a build for.
    pub source_build: Option<&'a str>,
    /// An app with an `application` (or `platform`) floor depends on the Microsoft `Application`
    /// aggregate (`System`), and the aggregate propagates its own dependencies (Base Application,
    /// System Application, Business Foundation), as AL's `app.json` rules say. With this on, the
    /// loaded apps see them without declaring them. Off for the corpus, which declares its own.
    pub application_implies_dependencies: bool,
    /// Leave the text of a package's files unread: the loaded app has the paths and empty texts, and
    /// `Loaded::lazy` says how to read a file later and holds a hash of the package's zip directory
    /// (ADR-022, "pure tests"). A package that cannot be read lazily (a folder, a wrapper) is read as usual.
    pub lazy_sources: bool,
}

/// The Microsoft build the packages in `search` come from: the highest version, among the packages
/// of Business Foundation, System Application, Base Application and Application (the apps that ship
/// one package per build, `MICROSOFT_PLATFORM_APP_IDS`), that reads as a four-part build. A package
/// with or without AL source counts: a project's `.alpackages` holds what its developer compiled
/// against, which is the build the project really targets, whatever the floor in its app.json says.
/// Returns the build and the package it was read from. Reads manifests only.
pub fn microsoft_packages_build(search: &[PathBuf], hint: Option<&str>) -> Option<(crate::build::BuildId, String)> {
    use crate::select::MICROSOFT_PLATFORM_APP_IDS;
    let mut best: Option<(crate::build::BuildId, String)> = None;
    for dir in search {
        for f in scan_path(dir, hint).0 {
            let m = &f.manifest;
            let ships_per_build = MICROSOFT_PLATFORM_APP_IDS
                .iter()
                .filter(|id| **id != crate::select::SYSTEM_APP_ID)
                .any(|id| id.eq_ignore_ascii_case(&m.id));
            if !ships_per_build || !m.publisher.eq_ignore_ascii_case("Microsoft") {
                continue;
            }
            let Ok(b) = crate::build::BuildId::parse(&m.version.to_string()) else {
                continue;
            };
            if best.as_ref().is_none_or(|(c, _)| b > *c) {
                best = Some((b, format!("{} {} in {}", m.name, m.version, f.path.display())));
            }
        }
    }
    best
}

/// Chooses the packages that satisfy the project's dependencies and reads their source.
pub fn load_dependencies(req: &Request<'_>) -> Result<Loaded, String> {
    read_planned(plan_dependencies(req, &mut ScanCache::new())?, req)
}

/// The packages chosen for the dependencies of a project, not read yet.
pub struct Planned {
    pool: Vec<Candidate>,
    paths: Vec<Option<PathBuf>>,
    notes: Vec<String>,
    order: Vec<usize>,
    /// Dependencies no package satisfied.
    pub skipped: Vec<SelectError>,
    /// Optional dependencies no package satisfied (see `select::Plan::optional_missing`).
    pub optional_missing: Vec<DepDecl>,
    application_aggregate: Vec<String>,
}

/// What `scan_path` found, by search entry: a search that goes round again with one more
/// directory does not read the first ones again.
pub type ScanCache = std::collections::HashMap<PathBuf, (Vec<Found>, Vec<String>)>;

/// Looks at the packages in the search directories and chooses one for every dependency; reads
/// only their manifests. `scans` remembers entries already looked at.
pub fn plan_dependencies(req: &Request<'_>, scans: &mut ScanCache) -> Result<Planned, String> {
    plan_dependencies_labeled(req, scans, &|_| None)
}

/// [`plan_dependencies`] where `label` may name where a package came from for messages (for
/// example "the artifact feed"); `None` names it by its path.
pub fn plan_dependencies_labeled(
    req: &Request<'_>,
    scans: &mut ScanCache,
    label: &dyn Fn(&Path) -> Option<String>,
) -> Result<Planned, String> {
    let mut pool: Vec<Candidate> = Vec::new();
    let mut paths: Vec<Option<PathBuf>> = Vec::new();
    let mut notes = Vec::new();
    for w in req.workspace_apps {
        pool.push(Candidate {
            id: w.json.id.clone(),
            name: w.json.name.clone(),
            publisher: w.json.publisher.clone(),
            version: w.json.version,
            has_source: true,
            tier: 0,
            from_workspace: true,
            origin: w.origin.clone(),
            deps: w.json.dependencies.clone(),
            platform: w.json.platform,
            application: w.json.application,
        });
        paths.push(None);
    }
    for (tier, dir) in req.search_dirs.iter().enumerate() {
        let (found, problems) = scans
            .entry(dir.clone())
            .or_insert_with(|| scan_path(dir, req.source_build))
            .clone();
        notes.extend(problems);
        for f in found {
            let m = f.manifest;
            pool.push(Candidate {
                id: m.id,
                name: m.name,
                publisher: m.publisher,
                version: m.version,
                has_source: f.has_source,
                tier,
                from_workspace: false,
                origin: label(&f.path).unwrap_or_else(|| f.path.display().to_string()),
                deps: m.dependencies,
                platform: m.platform,
                application: m.application,
            });
            paths.push(Some(f.path));
        }
    }
    let project = Project {
        deps: req.project.dependencies.clone(),
        platform: req.project.platform,
        application: req.project.application,
    };
    let opts = Options {
        missing: req.missing,
        searched: req.search_dirs.iter().map(|d| d.display().to_string()).collect(),
    };
    let plan = resolve(&project, &pool, &opts).map_err(|e| e.to_string())?;
    notes.extend(plan.notes);
    let application_aggregate: Vec<String> = plan
        .order
        .iter()
        .find(|&&i| pool[i].id.eq_ignore_ascii_case(APPLICATION_APP_ID))
        .map(|&i| pool[i].deps.iter().map(|d| d.id.clone()).collect())
        .unwrap_or_default();
    Ok(Planned {
        pool,
        paths,
        notes,
        order: plan.order,
        skipped: plan.skipped,
        optional_missing: plan.optional_missing,
        application_aggregate,
    })
}

/// Packaged resources: name relative to the resource folder, content.
pub type Resources = Vec<(String, Vec<u8>)>;

/// The packaged resources of the package at `path`, and whether it ships a translation file ([`ships_translations`]). A
/// folder has no resources; a package file is read a second time here.
fn read_resources_at(path: &Path, hint: Option<&str>) -> Result<(Resources, bool), String> {
    if !path.is_file() {
        return Ok((Vec::new(), ships_translations(path)));
    }
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let vars = vars_for(path, hint);
    let at = |e: crate::package::PackageError| format!("{}: {e}", path.display());
    Ok((
        crate::package::read_resources(&bytes, &vars).map_err(at)?,
        crate::package::ships_translations(&bytes, &vars).map_err(at)?,
    ))
}

/// Whether the app at `path` ships a translation file (`crate::package::is_translation_file`): a folder's `Translations/`
/// folder holds one, or a package file has such an entry. A package that cannot be read ships none here (reading its
/// sources reports the error).
pub fn ships_translations(path: &Path) -> bool {
    if path.is_dir() {
        let mut stack = vec![path.join("Translations")];
        while let Some(d) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&d) else { continue };
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if let Ok(rel) = p.strip_prefix(path) {
                    if crate::package::is_translation_file(&rel.to_string_lossy()) {
                        return true;
                    }
                }
            }
        }
        return false;
    }
    std::fs::read(path)
        .ok()
        .and_then(|bytes| crate::package::ships_translations(&bytes, &vars_for(path, None)).ok())
        .unwrap_or(false)
}

/// The translation files of the app folder `dir` (`crate::package::is_translation_file`, ADR-053 step 2): name relative to the
/// app (`/` separators) and the parsed file, or why it could not be read. Sorted by name. A package gives none here.
pub fn translation_files(dir: &Path) -> Vec<(String, Result<crate::xliff::Xliff, String>)> {
    let mut out = Vec::new();
    if !dir.is_dir() {
        return out;
    }
    let mut stack = vec![dir.join("Translations")];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else { continue };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if let Ok(rel) = p.strip_prefix(dir) {
                let rel = rel.to_string_lossy().replace('\\', "/");
                if crate::package::is_translation_file(&rel) {
                    let parsed = std::fs::read(&p)
                        .map_err(|e| e.to_string())
                        .and_then(|b| crate::xliff::parse(&b));
                    out.push((rel, parsed));
                }
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Reads the sources of the packages a plan chose, side by side.
pub fn read_planned(planned: Planned, req: &Request<'_>) -> Result<Loaded, String> {
    use rayon::prelude::*;
    let Planned {
        pool,
        paths,
        mut notes,
        order,
        skipped,
        optional_missing: _,
        application_aggregate,
    } = planned;
    let to_read: Vec<&PathBuf> = order
        .iter()
        .filter_map(|&i| paths[i].as_ref().filter(|_| pool[i].has_source))
        .collect();
    // A package read lazily has no sources; its files are named and read later.
    enum Read {
        Eager(Box<Manifest>, Sources),
        Lazy(Box<crate::lazy::LazyPackage>),
    }
    let mut reads: std::collections::VecDeque<Result<Read, String>> = to_read
        .par_iter()
        .map(|path| {
            if req.lazy_sources && path.is_file() {
                if let Some(l) = crate::lazy::read_lazy(path, &vars_for(path, req.source_build)) {
                    return Ok(Read::Lazy(Box::new(l)));
                }
            }
            read_package_at_with(path, req.source_build).map(|(m, s)| Read::Eager(Box::new(m), s))
        })
        .collect();
    // The resources of the same packages (`NavApp.GetResource` of a dependency app reads its own).
    let mut resources: std::collections::VecDeque<Result<(Resources, bool), String>> = to_read
        .par_iter()
        .map(|path| read_resources_at(path, req.source_build))
        .collect();
    let mut loaded = Loaded {
        application_aggregate,
        ..Default::default()
    };
    for i in order {
        let Some(path) = paths[i].clone() else { continue };
        let c = &pool[i];
        if !c.has_source {
            // Symbols only (or an aggregate such as `Application`): nothing to compile. Its
            // objects stay unresolved, so what needs them is refused. Say so.
            // The `Application` aggregate never has any; with its dependencies propagated, that is
            // expected and not worth a line.
            if req.application_implies_dependencies && c.id.eq_ignore_ascii_case(APPLICATION_APP_ID) {
                continue;
            }
            notes.push(format!(
                "{}: `{}` {} carries no AL source; the runtime compiles source (ADR-013), so its objects are unavailable",
                path.display(),
                c.name,
                c.version
            ));
            continue;
        }
        let (manifest, sources, lazy): (Manifest, Option<Sources>, Option<crate::lazy::LazyPackage>) =
            match reads.pop_front().expect("one read per package")? {
                Read::Eager(m, s) => (*m, Some(s), None),
                Read::Lazy(l) => (l.manifest.clone(), None, Some(*l)),
            };
        let (app_resources, translations) = resources.pop_front().expect("one read per package")?;
        let role = if manifest.id.eq_ignore_ascii_case(SYSTEM_APP_ID) {
            Role::Platform
        } else {
            Role::Standard
        };
        let mut dependencies: Vec<String> = manifest.dependencies.iter().map(|d| d.id.clone()).collect();
        if req.application_implies_dependencies {
            for id in loaded.implied_dependencies(manifest.platform.is_some(), manifest.application.is_some()) {
                if !id.eq_ignore_ascii_case(&manifest.id) && !dependencies.contains(&id) {
                    dependencies.push(id);
                }
            }
        }
        loaded.apps.push(DependencyApp {
            app: AppSource {
                id: manifest.id.clone(),
                name: manifest.name.clone(),
                dependencies,
                files: match (&lazy, sources) {
                    (Some(l), _) => l
                        .names
                        .iter()
                        .map(|p| (format!("{}/{p}", manifest.name), String::new()))
                        .collect(),
                    (None, sources) => sources
                        .unwrap_or_default()
                        .into_iter()
                        .map(|(p, text)| (format!("{}/{p}", manifest.name), text))
                        .collect(),
                },
            },
            publisher: manifest.publisher.clone(),
            version: manifest.version,
            path,
            role,
            lazy: lazy.map(|l| l.source),
            preprocessor_symbols: manifest.preprocessor_symbols.clone(),
            resources: app_resources,
            translations,
        });
    }
    loaded.notes = notes;
    loaded.skipped = skipped;
    Ok(loaded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zip::testutil::{Builder, navx};

    fn manifest(id: &str, name: &str, version: &str, deps: &[(&str, &str, &str)]) -> String {
        let deps: String = deps
            .iter()
            .map(|(id, name, min)| {
                format!(r#"<Dependency Id="{id}" Name="{name}" Publisher="P" MinVersion="{min}" />"#)
            })
            .collect();
        format!(
            r#"<Package><App Id="{id}" Name="{name}" Publisher="P" Version="{version}" /><Dependencies>{deps}</Dependencies></Package>"#
        )
    }

    fn write_app(
        dir: &Path,
        file: &str,
        id: &str,
        name: &str,
        version: &str,
        deps: &[(&str, &str, &str)],
        al: &[(&str, &str)],
    ) {
        let mut b = Builder::new().add("NavxManifest.xml", manifest(id, name, version, deps).as_bytes(), true);
        for (p, t) in al {
            b = b.add(p, t.as_bytes(), true);
        }
        std::fs::write(dir.join(file), navx(&b.finish())).unwrap();
    }

    fn project(deps: &[(&str, &str, &str)]) -> AppJson {
        AppJson {
            id: "p".into(),
            name: "Proj".into(),
            publisher: "P".into(),
            version: Version(1, 0, 0, 0),
            platform: None,
            application: None,
            dependencies: deps
                .iter()
                .map(|(id, name, min)| DepDecl {
                    id: id.to_string(),
                    name: name.to_string(),
                    publisher: "P".into(),
                    min: min.parse().unwrap(),
                    optional: false,
                })
                .collect(),
        }
    }

    #[test]
    fn app_json_dependencies_and_floors_are_read() {
        let a = parse_app_json(
            r#"{"id":"A1B2","name":"N","publisher":"P","version":"1.2.0.0","platform":"27.0.0.0","application":"27.0.0.0",
                "dependencies":[{"id":"C3D4","name":"D","publisher":"Q","version":"27.0.0.0"},{"appId":"e5","name":"O","publisher":"Q","version":"1.0.0.0"}]}"#,
        )
        .unwrap();
        assert_eq!(a.id, "a1b2");
        assert_eq!(a.platform, Some(Version(27, 0, 0, 0)));
        assert_eq!(a.dependencies.len(), 2);
        assert_eq!(a.dependencies[0].id, "c3d4");
        assert_eq!(a.dependencies[1].id, "e5");
        assert!(parse_app_json("{").is_err());
    }

    #[test]
    fn loads_the_chosen_packages_in_dependency_order_with_prefixed_paths() {
        let dir = tempfile::tempdir().unwrap();
        write_app(
            dir.path(),
            "top.app",
            "top",
            "Top",
            "2.0.0.0",
            &[("base", "Base", "1.0.0.0")],
            &[("src/T.al", "codeunit 2 T { }")],
        );
        write_app(
            dir.path(),
            "base.app",
            "base",
            "Base",
            "1.0.0.0",
            &[],
            &[("src/B.al", "codeunit 1 B { }")],
        );
        write_app(
            dir.path(),
            "unused.app",
            "unused",
            "Unused",
            "1.0.0.0",
            &[],
            &[("src/U.al", "codeunit 3 U { }")],
        );
        std::fs::write(dir.path().join("readme.txt"), "x").unwrap();
        let loaded = load_dependencies(&Request {
            project: &project(&[("top", "Top", "2.0.0.0")]),
            workspace_apps: &[],
            search_dirs: &[dir.path().to_path_buf()],
            missing: MissingPolicy::Error,
            source_build: None,
            application_implies_dependencies: false,
            lazy_sources: false,
        })
        .unwrap();
        let names: Vec<_> = loaded.apps.iter().map(|a| a.app.name.as_str()).collect();
        assert_eq!(names, ["Base", "Top"]);
        assert_eq!(loaded.apps[1].app.dependencies, ["base"]);
        assert_eq!(loaded.apps[0].app.files[0].0, "Base/src/B.al");
        assert_eq!(loaded.apps[0].app.files[0].1, "codeunit 1 B { }");
    }

    #[test]
    fn a_workspace_app_satisfies_its_dependency_and_is_not_loaded_again() {
        let dir = tempfile::tempdir().unwrap();
        write_app(
            dir.path(),
            "base.app",
            "base",
            "Base",
            "1.0.0.0",
            &[],
            &[("src/B.al", "codeunit 1 B { }")],
        );
        let fixture = WorkspaceApp {
            json: AppJson {
                dependencies: vec![DepDecl {
                    id: "base".into(),
                    name: "Base".into(),
                    publisher: "P".into(),
                    min: Version(1, 0, 0, 0),
                    optional: false,
                }],
                ..project(&[])
            },
            origin: "fixture".into(),
        };
        let mut fixture = fixture;
        fixture.json.id = "fix".into();
        let loaded = load_dependencies(&Request {
            project: &project(&[("fix", "Proj", "1.0.0.0")]),
            workspace_apps: &[fixture],
            search_dirs: &[dir.path().to_path_buf()],
            missing: MissingPolicy::Error,
            source_build: None,
            application_implies_dependencies: false,
            lazy_sources: false,
        })
        .unwrap();
        let names: Vec<_> = loaded.apps.iter().map(|a| a.app.name.as_str()).collect();
        assert_eq!(names, ["Base"]);
    }

    #[test]
    fn a_planned_search_reads_no_sources_and_a_scan_cache_does_not_look_twice() {
        let dir = tempfile::tempdir().unwrap();
        write_app(
            dir.path(),
            "base.app",
            "base",
            "Base",
            "1.0.0.0",
            &[],
            &[("src/B.al", "codeunit 1 B { }")],
        );
        let req = Request {
            project: &project(&[("base", "Base", "1.0.0.0")]),
            workspace_apps: &[],
            search_dirs: &[dir.path().to_path_buf()],
            missing: MissingPolicy::Skip,
            source_build: None,
            application_implies_dependencies: false,
            lazy_sources: false,
        };
        let mut scans = ScanCache::new();
        let first = plan_dependencies(&req, &mut scans).unwrap();
        assert!(first.skipped.is_empty());
        // A package added after the first look is not seen by a second plan that shares the scans...
        write_app(
            dir.path(),
            "later.app",
            "later",
            "Later",
            "1.0.0.0",
            &[],
            &[("src/L.al", "codeunit 9 L { }")],
        );
        let again = plan_dependencies(&req, &mut scans).unwrap();
        assert_eq!(again.pool.len(), 1);
        // ...and is seen by one that scans afresh.
        assert_eq!(plan_dependencies(&req, &mut ScanCache::new()).unwrap().pool.len(), 2);
        let loaded = read_planned(again, &req).unwrap();
        assert_eq!(loaded.apps.len(), 1);
        assert_eq!(loaded.apps[0].app.files.len(), 1);
    }

    #[test]
    fn a_dependency_with_no_package_is_an_error_naming_the_searched_directory() {
        let dir = tempfile::tempdir().unwrap();
        let err = load_dependencies(&Request {
            project: &project(&[("zzz", "Zed", "1.0.0.0")]),
            workspace_apps: &[],
            search_dirs: &[dir.path().to_path_buf()],
            missing: MissingPolicy::Error,
            source_build: None,
            application_implies_dependencies: false,
            lazy_sources: false,
        })
        .unwrap_err();
        assert!(
            err.contains("Zed") && err.contains(&dir.path().display().to_string()),
            "{err}"
        );
    }

    #[test]
    fn the_skip_policy_reports_what_is_missing_and_loads_the_rest() {
        let dir = tempfile::tempdir().unwrap();
        write_app(
            dir.path(),
            "base.app",
            "base",
            "Base",
            "1.0.0.0",
            &[],
            &[("src/B.al", "codeunit 1 B { }")],
        );
        let loaded = load_dependencies(&Request {
            project: &project(&[("zzz", "Zed", "1.0.0.0"), ("base", "Base", "1.0.0.0")]),
            workspace_apps: &[],
            search_dirs: &[dir.path().to_path_buf()],
            missing: MissingPolicy::Skip,
            source_build: None,
            application_implies_dependencies: false,
            lazy_sources: false,
        })
        .unwrap();
        assert_eq!(loaded.apps.len(), 1);
        assert_eq!(loaded.skipped.len(), 1);
    }

    #[test]
    fn a_corrupt_package_is_reported_and_does_not_hide_the_others() {
        let dir = tempfile::tempdir().unwrap();
        write_app(
            dir.path(),
            "base.app",
            "base",
            "Base",
            "1.0.0.0",
            &[],
            &[("src/B.al", "codeunit 1 B { }")],
        );
        std::fs::write(dir.path().join("broken.app"), b"NAVX nonsense").unwrap();
        let loaded = load_dependencies(&Request {
            project: &project(&[("base", "Base", "1.0.0.0")]),
            workspace_apps: &[],
            search_dirs: &[dir.path().to_path_buf()],
            missing: MissingPolicy::Error,
            source_build: None,
            application_implies_dependencies: false,
            lazy_sources: false,
        })
        .unwrap();
        assert_eq!(loaded.apps.len(), 1);
        assert!(
            loaded.notes.iter().any(|n| n.contains("broken.app")),
            "{:?}",
            loaded.notes
        );
    }

    #[test]
    fn a_package_without_source_is_not_found_and_says_where_the_search_went() {
        let dir = tempfile::tempdir().unwrap();
        write_app(dir.path(), "sym.app", "sym", "Sym", "1.0.0.0", &[], &[]);
        let proj = project(&[("sym", "Sym", "1.0.0.0")]);
        let searched = [dir.path().to_path_buf()];
        let req = |missing| Request {
            project: &proj,
            workspace_apps: &[],
            search_dirs: &searched,
            missing,
            source_build: None,
            application_implies_dependencies: false,
            lazy_sources: false,
        };
        let loaded = load_dependencies(&req(MissingPolicy::Skip)).unwrap();
        assert!(
            loaded.apps.is_empty(),
            "a package without source is not loaded as a dependency"
        );
        let text = loaded.skipped[0].to_string();
        assert!(
            text.contains("Sym") && text.contains("without AL source") && text.contains("sym.app"),
            "{text}"
        );
        assert!(
            text.contains(&dir.path().display().to_string()),
            "names the place searched: {text}"
        );
        // With the default policy it is an error, as a missing package is.
        assert!(load_dependencies(&req(MissingPolicy::Error)).is_err());
    }

    /// The owner's case: the symbol package is in the project's `.alpackages`, the source is elsewhere
    /// (a source archive in another search directory): the search goes on and finds it.
    #[test]
    fn a_symbol_package_in_the_first_place_does_not_hide_the_source_in_the_second() {
        let alpackages = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        write_app(alpackages.path(), "sym.app", "sym", "Sym", "2.0.0.0", &[], &[]);
        let app_json = r#"{"id":"sym","name":"Sym","publisher":"P","version":"1.0.0.0","dependencies":[]}"#;
        let zip = Builder::new()
            .add("app.json", app_json.as_bytes(), true)
            .add("S.Codeunit.al", b"codeunit 9 S { }", true)
            .finish();
        std::fs::write(elsewhere.path().join("Sym.Source.zip"), zip).unwrap();
        let loaded = load_dependencies(&Request {
            project: &project(&[("sym", "Sym", "1.0.0.0")]),
            workspace_apps: &[],
            search_dirs: &[alpackages.path().to_path_buf(), elsewhere.path().to_path_buf()],
            missing: MissingPolicy::Skip,
            source_build: None,
            application_implies_dependencies: false,
            lazy_sources: false,
        })
        .unwrap();
        assert_eq!(loaded.apps.len(), 1);
        assert!(loaded.skipped.is_empty());
        assert!(loaded.apps[0].app.files.iter().any(|(_, t)| t.contains("codeunit 9 S")));
    }

    #[test]
    fn the_build_of_a_project_is_the_build_of_the_microsoft_packages_it_carries() {
        let dir = tempfile::tempdir().unwrap();
        let put = |file: &str, id: &str, name: &str, version: &str, publisher: &str| {
            let m = format!(
                r#"<Package><App Id="{id}" Name="{name}" Publisher="{publisher}" Version="{version}" /><Dependencies/></Package>"#
            );
            let z = Builder::new().add("NavxManifest.xml", m.as_bytes(), true).finish();
            std::fs::write(dir.path().join(file), navx(&z)).unwrap();
        };
        // Symbol packages, as VS Code downloads them: no AL source.
        put(
            "ba.app",
            "437dbf0e-84ff-417a-965d-ed2bb9650972",
            "Base Application",
            "28.4.53241.53504",
            "Microsoft",
        );
        put(
            "sa.app",
            "63ca2fa4-4f03-4f2b-a480-172fef340d3f",
            "System Application",
            "28.3.52162.52931",
            "Microsoft",
        );
        // System has the platform's own number; a test library and another publisher's app do not count.
        put(
            "sys.app",
            "8874ed3a-0643-4247-9ced-7a7002f7135d",
            "System",
            "28.0.53445.0",
            "Microsoft",
        );
        put(
            "lib.app",
            "dd0be2ea-f733-4d65-bb34-a28f4624fb14",
            "Library Assert",
            "99.9.99999.99999",
            "Microsoft",
        );
        put(
            "other.app",
            "437dbf0e-84ff-417a-965d-ed2bb9650973",
            "Mine",
            "99.9.99999.99999",
            "Me",
        );
        let (build, from) = microsoft_packages_build(&[dir.path().to_path_buf()], None).unwrap();
        assert_eq!(build.to_string(), "28.4.53241.53504");
        assert!(from.contains("Base Application") && from.contains("ba.app"), "{from}");
        assert!(microsoft_packages_build(&[tempfile::tempdir().unwrap().path().to_path_buf()], None).is_none());
    }

    #[test]
    fn a_source_zip_in_a_search_directory_satisfies_a_dependency() {
        let dir = tempfile::tempdir().unwrap();
        let app_json = r#"{"id":"zz","name":"Zed","publisher":"P","version":"2.0.0.0","dependencies":[]}"#;
        let zip = Builder::new()
            .add("app.json", app_json.as_bytes(), true)
            .add("Sub/Z.Codeunit.al", b"codeunit 9 Z { }", true)
            .finish();
        std::fs::write(dir.path().join("Zed.Source.zip"), zip).unwrap();
        std::fs::write(dir.path().join("notes.zip"), b"not an archive").unwrap();
        let loaded = load_dependencies(&Request {
            project: &project(&[("zz", "Zed", "2.0.0.0")]),
            workspace_apps: &[],
            search_dirs: &[dir.path().to_path_buf()],
            missing: MissingPolicy::Error,
            source_build: None,
            application_implies_dependencies: false,
            lazy_sources: false,
        })
        .unwrap();
        assert_eq!(loaded.apps.len(), 1);
        assert_eq!(loaded.apps[0].app.files[0].0, "Zed/Sub/Z.Codeunit.al");
        // A stray .zip that is no package is ignored; only `*.Source.zip` files are reported when broken.
        assert!(
            loaded.notes.iter().all(|n| !n.contains("notes.zip")),
            "{:?}",
            loaded.notes
        );
    }

    #[test]
    fn a_broken_source_zip_is_reported_like_a_broken_app() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Broken.Source.zip"), b"not an archive").unwrap();
        let (found, problems) = scan_dir(dir.path());
        assert!(found.is_empty());
        assert!(problems.iter().any(|p| p.contains("Broken.Source.zip")), "{problems:?}");
    }

    #[test]
    fn a_templated_source_zip_in_the_cache_is_read_with_the_build_its_record_names() {
        let dir = tempfile::tempdir().unwrap();
        let app_json = r#"{"id":"zz","name":"Zed","publisher":"P","version":"$(app_currentVersion)",
            "dependencies":[{"id":"yy","name":"Why","publisher":"P","version":"$(app_minimumVersion)"}]}"#;
        let zip = Builder::new()
            .add("app.json", app_json.as_bytes(), true)
            .add("Z.al", b"codeunit 9 Z { }", true)
            .finish();
        std::fs::write(dir.path().join("Zed.Source.zip"), &zip).unwrap();
        // Without a record of the build the variables cannot be filled: reported, not guessed.
        let (found, problems) = scan_dir(dir.path());
        assert!(found.is_empty());
        assert!(problems.iter().any(|p| p.contains("$(app_")), "{problems:?}");
        // The cache writes such a record next to every entry.
        std::fs::write(
            dir.path().join("Zed.Source.zip.json"),
            r#"{"build":"27.5.46862.56249","entry":"x","size":1,"crc":1}"#,
        )
        .unwrap();
        let (found, problems) = scan_dir(dir.path());
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(found[0].manifest.version, "27.5.46862.56249".parse().unwrap());
        assert_eq!(found[0].manifest.dependencies[0].min, "27.5.0.0".parse().unwrap());
        let (m, _) = read_package_at(&found[0].path).unwrap();
        assert_eq!(m.version, found[0].manifest.version);
    }

    #[test]
    fn a_package_file_as_a_search_path_is_the_only_candidate() {
        let dir = tempfile::tempdir().unwrap();
        let app_json = r#"{"id":"zz","name":"Zed","publisher":"P","version":"2.0.0.0","dependencies":[]}"#;
        let zip = Builder::new()
            .add("app.json", app_json.as_bytes(), true)
            .add("Z.al", b"codeunit 9 Z { }", true)
            .finish();
        std::fs::write(dir.path().join("Zed.Source.zip"), &zip).unwrap();
        std::fs::write(dir.path().join("Other.Source.zip"), &zip).unwrap();
        let (found, problems) = scan_path(&dir.path().join("Zed.Source.zip"), None);
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(found.len(), 1);
        assert!(found[0].path.ends_with("Zed.Source.zip"));
        // A file that is no package is a problem when it was named on purpose.
        std::fs::write(dir.path().join("notes.zip"), b"not an archive").unwrap();
        let (found, problems) = scan_path(&dir.path().join("notes.zip"), None);
        assert!(found.is_empty());
        assert_eq!(problems.len(), 1, "{problems:?}");
    }

    #[test]
    fn an_app_folder_as_a_search_path_is_a_source_package() {
        let dir = tempfile::tempdir().unwrap();
        let app = dir.path().join("zed");
        std::fs::create_dir_all(app.join("Sub")).unwrap();
        std::fs::write(
            app.join("app.json"),
            r#"{"id":"ZZ","name":"Zed","publisher":"P","version":"2.0.0.0","dependencies":[]}"#,
        )
        .unwrap();
        std::fs::write(app.join("Sub/Z.al"), "codeunit 9 Z { }").unwrap();
        std::fs::write(app.join("notes.txt"), "not al").unwrap();
        let (found, problems) = scan_path(&app, None);
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(found.len(), 1);
        assert!(found[0].has_source);
        assert_eq!(found[0].manifest.name, "Zed");
        let (m, sources) = read_package_at(&app).unwrap();
        assert_eq!(m.id, "zz");
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].0, "Sub/Z.al");
        // The folder is not a directory of packages: nothing else under it is scanned.
        let (found, _) = scan_path(dir.path(), None);
        assert!(found.is_empty());
    }

    #[test]
    fn build_variables_are_filled_from_a_build_in_the_path_or_from_the_hint() {
        let dir = tempfile::tempdir().unwrap();
        let app_json =
            r#"{"id":"zz","name":"Zed","publisher":"P","version":"$(app_currentVersion)","dependencies":[]}"#;
        let zip = Builder::new()
            .add("app.json", app_json.as_bytes(), true)
            .add("Z.al", b"codeunit 9 Z { }", true)
            .finish();
        let built = dir.path().join("27.5.46862.48827").join("Applications");
        std::fs::create_dir_all(&built).unwrap();
        std::fs::write(built.join("Zed.Source.zip"), &zip).unwrap();
        let (found, problems) = scan_path(&built.join("Zed.Source.zip"), None);
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(found[0].manifest.version, "27.5.46862.48827".parse().unwrap());
        // Elsewhere the caller's hint (an exact `--bc-version`) names the build; a prefix does not.
        std::fs::write(dir.path().join("Zed.Source.zip"), &zip).unwrap();
        let p = dir.path().join("Zed.Source.zip");
        assert!(scan_path(&p, None).0.is_empty());
        assert!(scan_path(&p, Some("27.5")).0.is_empty());
        let (found, _) = scan_path(&p, Some("27.5.46862.56249"));
        assert_eq!(found[0].manifest.version, "27.5.46862.56249".parse().unwrap());
    }

    #[test]
    fn an_application_floor_implies_the_application_and_what_it_propagates() {
        use crate::select::{APPLICATION_APP_ID, SYSTEM_APP_ID};
        let dir = tempfile::tempdir().unwrap();
        // The `Application` aggregate holds no source; its dependency is Base Application.
        write_app(
            dir.path(),
            "application.app",
            APPLICATION_APP_ID,
            "Application",
            "27.5.0.0",
            &[("ba", "Base", "27.0.0.0")],
            &[],
        );
        write_app(
            dir.path(),
            "ba.app",
            "ba",
            "Base",
            "27.5.0.0",
            &[],
            &[("B.al", "codeunit 1 B { }")],
        );
        // An app that names an `application` floor and a `platform`, and declares no dependency.
        let manifest = r#"<Package><App Id="top" Name="Top" Publisher="P" Version="1.0.0.0" Platform="27.0.0.0" Application="27.5.0.0" /><Dependencies /></Package>"#;
        let b =
            Builder::new()
                .add("NavxManifest.xml", manifest.as_bytes(), true)
                .add("T.al", b"codeunit 2 T { }", true);
        std::fs::write(dir.path().join("top.app"), crate::zip::testutil::navx(&b.finish())).unwrap();
        let load = |implies: bool| {
            load_dependencies(&Request {
                project: &project(&[("top", "Top", "1.0.0.0")]),
                workspace_apps: &[],
                search_dirs: &[dir.path().to_path_buf()],
                missing: MissingPolicy::Skip,
                source_build: None,
                application_implies_dependencies: implies,
                lazy_sources: false,
            })
            .unwrap()
        };
        let top_deps = |l: &Loaded| {
            l.apps
                .iter()
                .find(|a| a.app.name == "Top")
                .unwrap()
                .app
                .dependencies
                .clone()
        };
        assert!(top_deps(&load(false)).is_empty());
        let l = load(true);
        assert_eq!(top_deps(&l), [APPLICATION_APP_ID, "ba", SYSTEM_APP_ID]);
        // A caller's own app (not in `apps`) asks for the same set.
        assert_eq!(
            l.implied_dependencies(true, true),
            [APPLICATION_APP_ID, "ba", SYSTEM_APP_ID]
        );
        assert_eq!(l.implied_dependencies(false, false), Vec::<String>::new());
    }
}
