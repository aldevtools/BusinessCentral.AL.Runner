//! The Microsoft test apps of one BC build (ADR-052 section 1.3): which `*.Source.zip` entries of the artifact can hold
//! a test app, and getting the ones a run needs into the cache.
//!
//! The listing reads only the central directory of the artifact (saved in the cache, else by range from the feed), or
//! the files of an extracted artifact on disk. An app is fetched only when [`Listing::get`] asks for it. Whether an app
//! holds a test codeunit is not decided here: that needs its AL source, which the caller reads.

use crate::artifact::{Cdn, CountryZip, Fetch, is_w1, open_country, pick_local, published_builds};
use crate::build::{BuildId, Origin, select_build};
use crate::cache::{BuildDir, Store, local_country_files, local_platform_files};
use crate::remote::{RangeSource, RemoteZip};
use crate::zip::Entry;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Whether an archive entry can hold a Microsoft test app: a `*.Source.zip` under `Applications/` (the platform
/// artifact) or `Applications.<COUNTRY>/` (a country artifact) whose path names a test. The platform artifact keeps test
/// apps in `Test`, `Tests` and `Test Library` folders (`Applications/BaseApp/Test/Tests-ERM.Source.zip`); a country
/// artifact keeps all its apps in one folder, and there the file name says it (`Applications.US/Tests-ERM.Source.zip`).
/// Test libraries and the test framework match too; the caller tells them apart by their test codeunits.
pub fn is_candidate(entry: &str) -> bool {
    let lower = entry.to_ascii_lowercase();
    (lower.starts_with("applications/") || lower.starts_with("applications."))
        && lower.ends_with(".source.zip")
        && lower.contains("test")
}

/// The app's name in the artifact: the file name without `.Source.zip` (`Tests-ERM`).
pub fn app_name(entry: &str) -> &str {
    let base = entry.rsplit('/').next().unwrap_or(entry);
    base.get(..base.len().saturating_sub(".source.zip".len()))
        .filter(|_| base.to_ascii_lowercase().ends_with(".source.zip"))
        .unwrap_or(base)
}

/// The candidate entries of a build, one per app name: the country artifact's first (a localized `Tests-ERM` replaces
/// the w1 one, as the country's Base Application does), then the platform's. Sorted by app name, without case.
pub fn candidates<'a>(platform: &[&'a str], country: &[&'a str]) -> Vec<&'a str> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<&str> = Vec::new();
    for e in country.iter().chain(platform).copied().filter(|e| is_candidate(e)) {
        if seen.insert(app_name(e).to_ascii_lowercase()) {
            out.push(e);
        }
    }
    out.sort_by_key(|e| (app_name(e).to_ascii_lowercase(), e.to_string()));
    out
}

/// The app names of every `*.Source.zip` of the platform and country entries, sorted and without repeats.
pub fn source_apps(platform: &[&str], country: &[&str]) -> Vec<String> {
    let mut out: Vec<String> = country
        .iter()
        .chain(platform)
        .filter(|e| e.to_ascii_lowercase().ends_with(".source.zip"))
        .map(|e| app_name(e).to_string())
        .collect();
    out.sort();
    out.dedup();
    out
}

/// The package the Legacy lane uses among the artifact's `ConfigurationPackages/*.rapidstart` entries: the Extended
/// English one (the DemoTool's Legacy data, as `--company-data package` picks it), else the first.
pub fn pick_package<'a>(names: &[&'a str]) -> Option<&'a str> {
    let mut found: Vec<&str> = names
        .iter()
        .copied()
        .filter(|n| {
            let l = n.to_ascii_lowercase();
            l.starts_with("configurationpackages/") && l.ends_with(".rapidstart")
        })
        .collect();
    found.sort();
    found
        .iter()
        .find(|n| n.to_ascii_uppercase().contains("ENU.EXTENDED"))
        .or(found.first())
        .copied()
}

enum Place {
    /// A file of an extracted artifact.
    Local(PathBuf),
    Platform,
    Country,
}

struct Candidate {
    name: String,
    entry: String,
    place: Place,
}

/// The test-app candidates of the chosen build.
pub struct Listing<S> {
    pub build: BuildId,
    pub origin: Origin,
    /// `w1` or the country code.
    pub country: String,
    /// Where the listing came from, for the run's header.
    pub from: String,
    /// Lines for the log: the build chosen and warnings.
    pub log: Vec<String>,
    items: Vec<Candidate>,
    /// The app name (file name without `.Source.zip`) of every source archive of the build, test apps or not, sorted.
    pub source_apps: Vec<String>,
    /// The `ConfigurationPackages` folder of an extracted artifact.
    local_packages: Option<PathBuf>,
    platform: Option<(String, RemoteZip<S>, BuildDir)>,
    country_zip: Option<(CountryZip<S>, BuildDir)>,
}

impl<S: RangeSource> Listing<S> {
    /// The app names, sorted.
    pub fn names(&self) -> Vec<&str> {
        self.items.iter().map(|c| c.name.as_str()).collect()
    }

    /// The entry name of an app, matched without case.
    pub fn entry(&self, name: &str) -> Option<&str> {
        self.find(name).map(|c| c.entry.as_str())
    }

    fn find(&self, name: &str) -> Option<&Candidate> {
        self.items.iter().find(|c| c.name.eq_ignore_ascii_case(name))
    }

    /// The RapidStart package of the platform artifact that Microsoft's Legacy test lane builds its company from
    /// (`ConfigurationPackages/NAV<v>.W1.ENU.EXTENDED.rapidstart`, else the first package there), from the extracted
    /// artifact, the cache or fetched now. `None` when the artifact has none, or for a country other than w1: the
    /// package is the w1 one.
    pub fn rapidstart_package(&self) -> Result<Option<PathBuf>, String> {
        if !is_w1(&self.country) {
            return Ok(None);
        }
        if let Some(dir) = &self.local_packages {
            let Ok(rd) = std::fs::read_dir(dir) else {
                return Ok(None);
            };
            let found: Vec<(String, PathBuf)> = rd
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter_map(|p| Some((format!("ConfigurationPackages/{}", p.file_name()?.to_string_lossy()), p)))
                .collect();
            let names: Vec<&str> = found.iter().map(|(n, _)| n.as_str()).collect();
            return Ok(pick_package(&names).and_then(|n| found.iter().find(|(f, _)| f == n).map(|(_, p)| p.clone())));
        }
        let Some((url, zip, dir)) = &self.platform else {
            return Ok(None);
        };
        let names: Vec<&str> = zip.entries.iter().map(|e| e.name.as_str()).collect();
        let Some(entry) = pick_package(&names) else {
            return Ok(None);
        };
        if let Some(p) = dir.cached_in("packages", entry) {
            return Ok(Some(p));
        }
        let e = zip.find(entry).expect("the entry was listed");
        let bytes = zip.read(e).map_err(|err| format!("{url}: entry `{entry}`: {err}"))?;
        dir.put_entry_in("packages", entry, url, &bytes).map(Some)
    }

    /// The path of an app's `*.Source.zip`: the file of the extracted artifact, the cached copy, or a copy fetched into
    /// the cache now (CRC-checked).
    pub fn get(&self, name: &str) -> Result<PathBuf, String> {
        let c = self
            .find(name)
            .ok_or_else(|| format!("BC build {} has no test app named `{name}`", self.build))?;
        let (url, zip, dir) = match &c.place {
            Place::Local(p) => return Ok(p.clone()),
            Place::Platform => {
                let (url, zip, dir) = self.platform.as_ref().expect("a platform entry has its archive open");
                (url.as_str(), zip, dir)
            }
            Place::Country => {
                let (cz, dir) = self.country_zip.as_ref().expect("a country entry has its archive open");
                (cz.url.as_str(), &cz.zip, dir)
            }
        };
        if let Some(p) = dir.cached(&c.entry) {
            return Ok(p);
        }
        let e: &Entry = zip
            .find(&c.entry)
            .ok_or_else(|| format!("{url}: no entry `{}`", c.entry))?;
        let bytes = zip
            .read(e)
            .map_err(|err| format!("{url}: entry `{}`: {err}", c.entry))?;
        dir.put_entry(&c.entry, url, &bytes)
    }
}

/// The build of `fetch.query`, as [`list`] picks it: a matching build in the artifact cache, an extracted artifact on
/// disk that the query names exactly, else the newest matching published build; offline, or when the feed has no
/// match, any matching build on disk (no download of the build
/// itself). Returns the selection, the local artifacts root when the build comes from there, and the log lines.
pub fn choose<'f, C: Cdn>(
    fetch: &'f Fetch,
    cdn: &C,
) -> Result<(crate::build::Selection, Option<&'f std::path::Path>, Vec<String>), String> {
    let store = Store::new(&fetch.cache_root);
    let cache_dir = store.dir().display().to_string();
    let mut log: Vec<String> = Vec::new();
    let local_root = fetch.local_artifacts.as_deref();
    let local_pick = |exact_only: bool| {
        local_root.and_then(|r| pick_local(r, &fetch.query, exact_only, &fetch.country).map(|s| (r, s)))
    };
    let cached = store.list_builds();
    let cached_sel = fetch
        .query
        .exact()
        .is_none_or(|e| cached.contains(&e))
        .then(|| select_build(&fetch.query, &cached, None, &cache_dir).ok())
        .flatten();
    let (sel, local) = if let Some(sel) = cached_sel {
        (sel, None)
    } else if let Some((root, sel)) = local_pick(true) {
        (sel, Some(root))
    } else if !fetch.download {
        match local_pick(false) {
            Some((root, sel)) => (sel, Some(root)),
            None => {
                return Err(format!(
                    "no BC build matching `{}` is in the artifact cache ({cache_dir}) or on disk, and --no-download is set",
                    fetch.query.prefix
                ));
            }
        }
    } else {
        let picked = published_builds(cdn, &fetch.country)
            .and_then(|p| select_build(&fetch.query, &cached, Some(&p), &cache_dir).map_err(|e| e.to_string()));
        match picked {
            Ok(sel) => (sel, None),
            Err(e) => match local_pick(false) {
                Some((root, sel)) => {
                    log.push(format!("artifact: {e}; using the local artifacts instead"));
                    (sel, Some(root))
                }
                None => return Err(e),
            },
        }
    };
    Ok((sel, local, log))
}

/// Lists the test-app candidates of the build `fetch.query` picks, the way a test run picks the build of its
/// dependencies: a cached build first, then a build of the extracted artifacts on disk that the query names exactly,
/// then the newest published build; offline, or when the feed has no match, any matching build on disk.
pub fn list<C: Cdn>(fetch: &Fetch, cdn: &C) -> Result<Listing<C::Src>, String> {
    let store = Store::new(&fetch.cache_root);
    let country = if is_w1(&fetch.country) {
        "w1".to_string()
    } else {
        fetch.country.to_ascii_lowercase()
    };
    let (sel, local, mut log) = choose(fetch, cdn)?;
    log.extend(sel.warnings.iter().map(|w| format!("warning: {w}")));
    let build = sel.build.clone();
    if let Some(root) = local {
        return Ok(local_listing(root, sel.build, sel.origin, country, &fetch.country, log));
    }
    // The platform artifact's central directory: saved, else by range from the feed (and saved).
    let bd = store.build(&build);
    let (url, src) = cdn.platform(&build);
    let zip = match bd.load_tail() {
        Some((total, tail)) => RemoteZip::from_tail(src, total, tail).map_err(|e| format!("{url}: {e}"))?,
        None if !fetch.download => {
            return Err(format!(
                "the artifact of BC build {build} is not cached and --no-download is set"
            ));
        }
        None => {
            let zip = RemoteZip::open(src).map_err(|e| format!("{url}: {e}"))?;
            let (total, tail) = zip.tail();
            bd.save_tail(&url, total, tail)?;
            zip
        }
    };
    let country_zip = if is_w1(&fetch.country) {
        None
    } else {
        let cbd = store.country_build(&build, &fetch.country);
        Some((open_country(cdn, fetch, &cbd, &build)?, cbd))
    };
    let platform_names: Vec<&str> = zip.entries.iter().map(|e| e.name.as_str()).collect();
    let country_names: Vec<&str> = country_zip
        .as_ref()
        .map(|(cz, _)| cz.entries.iter().map(|e| e.name.as_str()).collect())
        .unwrap_or_default();
    let in_country: HashSet<&str> = country_names.iter().copied().collect();
    let source_apps = source_apps(&platform_names, &country_names);
    let items: Vec<Candidate> = candidates(&platform_names, &country_names)
        .into_iter()
        .map(|e| Candidate {
            name: app_name(e).to_string(),
            entry: e.to_string(),
            place: if in_country.contains(e) {
                Place::Country
            } else {
                Place::Platform
            },
        })
        .collect();
    let from = match sel.origin {
        Origin::Cache => format!("the artifact cache at {}", bd.path().display()),
        _ => format!("the artifact feed, {url}"),
    };
    Ok(Listing {
        build,
        origin: sel.origin,
        country,
        from,
        log,
        items,
        source_apps,
        local_packages: None,
        platform: Some((url, zip, bd)),
        country_zip,
    })
}

fn local_listing<S>(
    root: &Path,
    build: BuildId,
    origin: Origin,
    country: String,
    fetch_country: &str,
    log: Vec<String>,
) -> Listing<S> {
    let mut files = if is_w1(fetch_country) {
        Vec::new()
    } else {
        local_country_files(root, &build, fetch_country)
    };
    let n_country = files.len();
    files.extend(local_platform_files(root, &build));
    let country_names: Vec<&str> = files[..n_country].iter().map(|(n, _)| n.as_str()).collect();
    let platform_names: Vec<&str> = files[n_country..].iter().map(|(n, _)| n.as_str()).collect();
    let source_apps = source_apps(&platform_names, &country_names);
    let items = candidates(&platform_names, &country_names)
        .into_iter()
        .filter_map(|e| {
            let (_, path) = files.iter().find(|(n, _)| n == e)?;
            Some(Candidate {
                name: app_name(e).to_string(),
                entry: e.to_string(),
                place: Place::Local(path.clone()),
            })
        })
        .collect();
    Listing {
        local_packages: Some(root.join(build.as_str()).join("platform").join("ConfigurationPackages")),
        from: format!("the local artifacts at {}", root.join(build.as_str()).display()),
        build,
        origin,
        country,
        log,
        items,
        source_apps,
        platform: None,
        country_zip: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::BuildQuery;
    use crate::remote::{FileSource, RemoteError};
    use crate::zip::testutil::Builder;
    use std::cell::Cell;

    #[test]
    fn every_source_archive_names_an_app_test_or_not() {
        assert_eq!(
            source_apps(
                &[
                    "Applications/BaseApp/Source/Base Application.Source.zip",
                    "Applications/BaseApp/Test/Tests-ERM.Source.zip",
                    "ModernDev/System.app"
                ],
                &["Applications.US/Base Application.Source.zip"],
            ),
            vec!["Base Application".to_string(), "Tests-ERM".to_string()]
        );
    }

    #[test]
    fn test_apps_are_the_source_archives_whose_path_names_a_test() {
        assert!(is_candidate("Applications/BaseApp/Test/Tests-ERM.Source.zip"));
        assert!(is_candidate(
            "Applications/EU3PartyTradePurchase/test/EU 3-Party Trade Purchase Tests.Source.zip"
        ));
        assert!(is_candidate("Applications.US/Tests-ERM.Source.zip"));
        assert!(is_candidate(
            "Applications/TestFramework/TestLibraries/Any/Any.Source.zip"
        ));
        assert!(!is_candidate("Applications/BaseApp/Source/Base Application.Source.zip"));
        assert!(!is_candidate("Applications/BaseApp/Test/Tests-ERM.app"));
        assert!(!is_candidate("Extensions/Tests-ERM.Source.zip"));
    }

    #[test]
    fn the_name_is_the_file_name_without_the_archive_suffix() {
        assert_eq!(app_name("Applications/BaseApp/Test/Tests-ERM.Source.zip"), "Tests-ERM");
        assert_eq!(app_name("Applications.US/Tests-Bank.Source.zip"), "Tests-Bank");
        assert_eq!(app_name("x.app"), "x.app");
    }

    #[test]
    fn a_country_test_app_replaces_the_w1_one_of_the_same_name() {
        let platform = [
            "Applications/BaseApp/Test/Tests-ERM.Source.zip",
            "Applications/BaseApp/Test/Tests-Bank.Source.zip",
            "Applications/BaseApp/Source/Base Application.Source.zip",
        ];
        let country = [
            "Applications.US/Tests-ERM.Source.zip",
            "Applications.US/Base Application.Source.zip",
        ];
        assert_eq!(
            candidates(&platform, &country),
            vec![
                "Applications/BaseApp/Test/Tests-Bank.Source.zip",
                "Applications.US/Tests-ERM.Source.zip"
            ]
        );
    }

    fn artifact() -> Vec<u8> {
        Builder::new()
            .add(
                "Applications/BaseApp/Source/Base Application.Source.zip",
                b"base",
                false,
            )
            .add("Applications/BaseApp/Test/Tests-A.Source.zip", b"tests a", false)
            .add("Applications/BaseApp/Test/Tests-B.Source.zip", b"tests b", true)
            .add(
                "Applications/BaseApp/Test/Tests-TestLibraries.Source.zip",
                b"lib",
                false,
            )
            .add(
                "ConfigurationPackages/NAV29.0.W1.ENU.EXTENDED.rapidstart",
                b"package",
                true,
            )
            .finish()
    }

    struct Feed {
        platform: PathBuf,
        reads: Cell<u32>,
    }

    impl Cdn for Feed {
        type Src = FileSource;
        fn base_url(&self) -> String {
            "https://cdn.test".into()
        }
        fn index(&self) -> Result<Vec<u8>, RemoteError> {
            Ok(br#"[{"Version":"29.0.1.2","CreationTime":"2026-10-01T00:00:00Z"},{"Version":"29.0.1.3","CreationTime":"2026-10-02T00:00:00Z"},{"Version":"28.5.1.1","CreationTime":"2026-10-01T00:00:00Z"}]"#.to_vec())
        }
        fn platform(&self, build: &BuildId) -> (String, FileSource) {
            self.reads.set(self.reads.get() + 1);
            (
                format!("https://cdn.test/{build}/platform"),
                FileSource(self.platform.clone()),
            )
        }
        fn country(&self, build: &BuildId, country: &str) -> (String, FileSource) {
            (
                format!("https://cdn.test/{build}/{country}"),
                FileSource(PathBuf::from("/nonexistent")),
            )
        }
    }

    fn fetch(cache: &Path, download: bool, requested: Option<&str>) -> Fetch {
        Fetch {
            cache_root: cache.to_path_buf(),
            download,
            query: BuildQuery::new(requested, 29, None),
            local_artifacts: None,
            country: String::new(),
        }
    }

    #[test]
    fn the_feed_listing_names_every_test_app_and_fetches_only_what_is_asked() {
        let dir = tempfile::tempdir().unwrap();
        let platform = dir.path().join("platform.zip");
        std::fs::write(&platform, artifact()).unwrap();
        let feed = Feed {
            platform,
            reads: Cell::new(0),
        };
        let cache = dir.path().join("cache");
        let l = list(&fetch(&cache, true, None), &feed).unwrap();
        assert_eq!(l.build.as_str(), "29.0.1.3", "the newest published build of the major");
        assert_eq!(l.names(), vec!["Tests-A", "Tests-B", "Tests-TestLibraries"]);
        let b = l.get("tests-b").unwrap();
        assert_eq!(std::fs::read(&b).unwrap(), b"tests b");
        let apps = cache.join("artifacts/29.0.1.3/apps");
        assert!(apps.join("Tests-B.Source.zip").is_file());
        assert!(
            !apps.join("Tests-A.Source.zip").exists(),
            "nothing is fetched that was not asked for"
        );
        assert!(l.get("Tests-Z").unwrap_err().contains("no test app named `Tests-Z`"));
        let pkg = l.rapidstart_package().unwrap().unwrap();
        assert_eq!(
            pkg,
            cache.join("artifacts/29.0.1.3/packages/NAV29.0.W1.ENU.EXTENDED.rapidstart")
        );
        assert_eq!(std::fs::read(&pkg).unwrap(), b"package");
        // Offline, the cached build and its saved central directory answer; the cached copy is used.
        let l = list(&fetch(&cache, false, None), &feed).unwrap();
        assert_eq!(l.origin, Origin::Cache);
        assert_eq!(l.get("Tests-B").unwrap(), apps.join("Tests-B.Source.zip"));
    }

    #[test]
    fn the_legacy_package_is_the_extended_english_one() {
        let names = [
            "ConfigurationPackages/NAV29.0.W1.ENU.EVALUATION.rapidstart",
            "ConfigurationPackages/NAV29.0.W1.ENU.EXTENDED.rapidstart",
            "Applications/X.Source.zip",
        ];
        assert_eq!(pick_package(&names), Some(names[1]));
        assert_eq!(pick_package(&names[..1]), Some(names[0]));
        assert_eq!(pick_package(&names[2..]), None);
    }

    #[test]
    fn offline_without_a_cached_build_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let feed = Feed {
            platform: dir.path().join("none"),
            reads: Cell::new(0),
        };
        let e = list(&fetch(&dir.path().join("cache"), false, Some("29.0")), &feed)
            .err()
            .unwrap();
        assert!(e.contains("--no-download"), "{e}");
    }

    #[test]
    fn an_extracted_artifact_on_disk_is_listed_from_its_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("sandbox");
        let test = root.join("29.0.1.2/platform/Applications/BaseApp/Test");
        std::fs::create_dir_all(&test).unwrap();
        std::fs::write(test.join("Tests-A.Source.zip"), b"a").unwrap();
        let mut f = fetch(&dir.path().join("cache"), false, Some("29.0.1.2"));
        f.local_artifacts = Some(root);
        let feed = Feed {
            platform: dir.path().join("none"),
            reads: Cell::new(0),
        };
        let l = list(&f, &feed).unwrap();
        assert_eq!(l.origin, Origin::Local);
        assert_eq!(l.names(), vec!["Tests-A"]);
        assert_eq!(l.get("Tests-A").unwrap(), test.join("Tests-A.Source.zip"));
        assert_eq!(feed.reads.get(), 0);
    }
}
