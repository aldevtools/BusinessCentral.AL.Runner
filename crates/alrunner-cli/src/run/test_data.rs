//! Finding and opening the test data of a run: a backup (kept between `al-runner serve` requests), a package, the
//! DemoTool fetch, the companies a run reads, and the choice of a demo backup.

use super::*;

/// With `al-runner serve`: the backup opened last, kept with the tables read from it, for the next request.
pub(super) type KeptBackup = (
    (PathBuf, Option<String>, Vec<String>, Option<al_host::Companies>),
    TestData,
);
pub(super) static KEPT_BACKUP: std::sync::Mutex<Option<KeptBackup>> = std::sync::Mutex::new(None);
pub(super) static KEEP_BACKUPS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// `al-runner serve` keeps the opened backup (and so the tables read from it) between requests.
pub fn keep_backups() {
    KEEP_BACKUPS.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Opens a backup file.
pub(super) fn open_backup(a: &TestArgs, path: PathBuf) -> Result<(TestData, PathBuf), String> {
    if KEEP_BACKUPS.load(std::sync::atomic::Ordering::Relaxed) {
        let key = (
            path.clone(),
            a.test_data_company.clone(),
            a.additional_test_data_companies.clone(),
            a.companies.clone(),
        );
        if let Some((k, d)) = KEPT_BACKUP.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            if *k == key {
                return Ok((d.clone(), path));
            }
        }
        let opened = open_backup_fresh(a, path)?;
        *KEPT_BACKUP.lock().unwrap_or_else(|e| e.into_inner()) = Some((key, opened.0.clone()));
        return Ok(opened);
    }
    open_backup_fresh(a, path)
}

pub(super) fn open_backup_fresh(a: &TestArgs, path: PathBuf) -> Result<(TestData, PathBuf), String> {
    // The page map of the backup is kept next to the compile cache, so a cold or a warm open of a
    // 900 MB backup reads the map instead of verifying every extent.
    let index_dir = (!a.no_compile_cache && !al_testrun::switches::on("ALRUNNER_NO_COMPILE_CACHE"))
        .then(|| al_deps::cache::cache_root_from_env(a.cache_dir.as_deref()).join("backup-index"));
    let all_companies = matches!(a.companies.as_ref(), Some(al_host::Companies::All));
    let additional_companies = if !a.additional_test_data_companies.is_empty() {
        a.additional_test_data_companies.clone()
    } else {
        match &a.companies {
            Some(al_host::Companies::List(names)) => names.iter().skip(1).cloned().collect(),
            _ => Vec::new(),
        }
    };
    let provider = BackupProvider::open_with_index_dir_company_selection(
        &path,
        a.test_data_company.as_deref(),
        &additional_companies,
        all_companies,
        index_dir.as_deref(),
    )
    .map_err(|e| format!("--test-data {}: {e}", path.display()))?;
    if let Some(al_host::Companies::List(names)) = &a.companies {
        provider
            .validate_company_selection(names)
            .map_err(|e| format!("--test-data {}: {e}", path.display()))?;
    }
    Ok((TestData::new(Arc::new(provider)), path))
}

pub(super) fn resolved_companies(a: &TestArgs, data: Option<&TestData>) -> al_host::Companies {
    match (data, &a.companies) {
        (Some(data), Some(al_host::Companies::List(names))) => {
            al_host::Companies::List(data.provider.known_companies().unwrap_or_else(|| names.clone()))
        }
        (Some(data), Some(al_host::Companies::All)) => al_host::Companies::List(
            data.provider
                .known_companies()
                .unwrap_or_else(|| data.provider.companies()),
        ),
        _ => a.companies.clone().unwrap_or_default(),
    }
}

/// `--company-data package`: the artifact's RapidStart package as the company data (the Legacy lane's company: a fresh
/// company, Company-Initialize, then the package). `bak` is the located backup, whose artifact folder holds the package.
pub(super) fn open_package(a: &TestArgs, bak: Option<&Path>) -> Result<(TestData, PathBuf), String> {
    let path = match &a.package_path {
        Some(p) => p.clone(),
        None => {
            let dir = bak
                .and_then(|b| b.parent())
                .and_then(|b| b.parent())
                .map(|root| root.join("platform").join("ConfigurationPackages"))
                .ok_or("no artifact folder to find the package in; name it with --package PATH")?;
            let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
                .map_err(|e| format!("{}: {e}; name the package with --package PATH", dir.display()))?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("rapidstart")))
                .collect();
            found.sort();
            // The Extended English package is the DemoTool's Legacy data; any other is a fallback.
            found
                .iter()
                .find(|p| p.to_string_lossy().to_ascii_uppercase().contains("ENU.EXTENDED"))
                .or(found.first())
                .cloned()
                .ok_or_else(|| {
                    format!(
                        "no *.rapidstart file in {}; name one with --package PATH",
                        dir.display()
                    )
                })?
        }
    };
    let bytes = std::fs::read(&path).map_err(|e| format!("--package {}: {e}", path.display()))?;
    let name = path
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let company = a.test_data_company.as_deref().unwrap_or("CRONUS International Ltd.");
    let provider = al_rapidstart::PackageProvider::from_bytes(&bytes, &name, company)
        .map_err(|e| format!("--package {}: {e}", path.display()))?;
    Ok((TestData::new(Arc::new(provider)), path))
}

/// What the test data of a run came to.
pub(super) struct DataChoice {
    pub(super) data: Option<(TestData, PathBuf)>,
    /// Lines for stderr: where the data is from when that is not the build of the apps, why it is
    /// off, what failed on the way.
    pub(super) notes: Vec<String>,
}

/// The run's target once the Microsoft apps are loaded: the build they come from (`BcVersion::resolved_to`, al-runner
/// issues 8209 and 8243). `fetched_demotool`: the release of the DemoTool the run fetched before this call
/// (`fetch_demotool`), or `None` (`--demotool DIR`, no DemoTool). The run goes on when that release is the build's
/// major and minor; one of another release stops it.
pub(super) fn settle_target(
    bc: &mut al_version::BcVersion,
    build: &str,
    fetched_demotool: Option<(u32, u32)>,
) -> Result<(), String> {
    bc.resolved_to(build)?;
    let release = match fetched_demotool {
        None => return Ok(()),
        Some(r) if r == (u32::from(bc.major), u32::from(bc.minor)) => return Ok(()),
        Some(r) => r,
    };
    Err(format!(
        "--company-data demotool: the DemoTool of release {}.{} was chosen before the Microsoft apps were found, and they come from build {build}; name the build to run on with --bc-version",
        release.0, release.1
    ))
}

/// `--company-data demotool`: the DemoTool, and the release fetched (`None` for `--demotool DIR`). `minor_from_packages`:
/// the Microsoft packages the project carries set the target's minor (`apps::BuildChoice::minor` is `Some`); the release
/// is then the target's, as before. Else it is the minor of the build the artifact cache, the local artifacts or the feed
/// name before the dependencies load (`Provision::build_before_load`), or the target's when they name none or a build of
/// another major (al-runner issue 8243). When the load ends on a build other than this release, `settle_target` stops the
/// run; when every Microsoft app comes from disk no build is loaded and nothing is checked (issue 8440).
pub(super) fn fetch_demotool(
    a: &TestArgs,
    bc: &al_version::BcVersion,
    minor_from_packages: bool,
    provision: &Provision,
) -> Result<(crate::demotool::DemoTool, Option<(u32, u32)>), String> {
    // A lane of `--recipe microsoft` gets the DemoTool without the check of the command line (args.rs).
    if !al_deps::artifact::is_w1(a.country()) {
        return Err("--company-data demotool supports only the w1 country (the DemoTool's W1 layer)".into());
    }
    let url = std::env::var("ALRUNNER_BCAPPS_URL").unwrap_or_else(|_| crate::bcapps::DEFAULT_URL.to_string());
    let target = (u32::from(bc.major), u32::from(bc.minor));
    let fetched = a.demotool_dir.is_none().then(|| {
        if minor_from_packages {
            return target;
        }
        match provision.build_before_load() {
            Some(b) if b.major() == target.0 => (target.0, b.minor()),
            _ => target,
        }
    });
    let (major, minor) = fetched.unwrap_or(target);
    let dt = crate::demotool::resolve(
        a.demotool_dir.as_deref(),
        major,
        minor,
        &al_deps::cache::cache_root_from_env(a.cache_dir.as_deref()),
        a.no_download,
        &url,
        &crate::demotool::git_fetch,
    )
    .map_err(|e| format!("--company-data demotool: {e}"))?;
    eprintln!("company data: {}", dt.how);
    Ok((dt, fetched))
}

/// Finds and opens the demo backup (ADR-020): the build of the Microsoft apps first, on disk, then
/// in the cache, then downloaded; see [`testdata::locate`]. A default run whose backup cannot be
/// found or opened has test data off and says why; a run that asked for test data fails instead.
pub(super) fn choose_test_data(
    a: &TestArgs,
    builds: &[String],
    downloadable: &[al_deps::build::BuildId],
    major: u16,
    minor: Option<u16>,
) -> Result<DataChoice, String> {
    // A named package needs no backup at all.
    if a.company_package && a.package_path.is_some() {
        return match open_package(a, None) {
            Ok(d) => Ok(DataChoice {
                data: Some(d),
                notes: Vec::new(),
            }),
            Err(e) => Err(format!("--company-data package: {e}")),
        };
    }
    let asked = a.test_data_asked_for() || a.company_package;
    let fail = |why: String| -> Result<DataChoice, String> {
        if asked {
            Err(format!("--test-data: {why}"))
        } else {
            Ok(DataChoice {
                data: None,
                notes: vec![format!(
                    "test data is off: {why}. Microsoft's tests expect a CRONUS company; failures of such a run are mostly missing setup data. Name a backup with --test-data PATH.bak"
                )],
            })
        }
    };
    let (path, mut notes) = match &a.test_data_path {
        Some(p) => (p.clone(), Vec::new()),
        None => {
            let cache_root = al_deps::cache::cache_root_from_env(a.cache_dir.as_deref());
            let local = al_deps::cache::bcartifacts_sandbox_from_env();
            let download = (!a.no_download).then_some(|build: &al_deps::build::BuildId| -> Result<PathBuf, String> {
                let cdn = al_deps::artifact::HttpCdn::from_env();
                let mut printed = 0u64;
                let t = Instant::now();
                eprintln!(
                    "test data: downloading the {} demo data of build {build} (about 0.5 GB, once) ...",
                    a.country()
                );
                let got =
                    al_deps::demo::ensure_demo_backup(&cdn, &cache_root, build, a.country(), &mut |read, total| {
                        let pct = (read * 100).checked_div(total).unwrap_or(100);
                        if pct >= printed + 10 || (pct == 100 && printed < 100) {
                            printed = pct - pct % 10;
                            eprintln!(
                                "test data: {pct}% ({} of {} MB, {:.0} s)",
                                read >> 20,
                                total >> 20,
                                t.elapsed().as_secs_f64()
                            );
                        }
                    })?;
                Ok(got.path)
            });
            let found = testdata::locate(
                builds,
                downloadable,
                major,
                minor,
                testdata::Where {
                    local_root: local.as_deref(),
                    cache_root: &cache_root,
                    download,
                    explicit_country: a.country.is_some(),
                },
                a.country(),
            );
            match found.path {
                Some(p) => (p, found.notes),
                None => return fail(found.off.unwrap_or_default()),
            }
        }
    };
    if a.company_package {
        return match open_package(a, Some(&path)) {
            Ok(d) => Ok(DataChoice {
                data: Some(d),
                notes: std::mem::take(&mut notes),
            }),
            Err(e) => Err(format!("--company-data package: {e}")),
        };
    }
    match open_backup(a, path) {
        Ok(d) => Ok(DataChoice {
            data: Some(d),
            notes: std::mem::take(&mut notes),
        }),
        Err(e) if asked => Err(e),
        Err(e) => fail(e),
    }
}

/// Whether the run looks for a demo backup (and so may download one): never with `--no-test-data`, whatever the
/// country (owner 2026-10-17: partners who test across countries write tests that need no data); not when a backup
/// is open already; a default run with no Microsoft app has no use for the CRONUS company.
pub(super) fn should_choose_test_data(a: &TestArgs, already_open: bool, microsoft: bool) -> bool {
    a.wants_test_data() && !already_open && (microsoft || a.test_data_asked_for())
}

#[cfg(test)]
mod settle_target_tests {
    use super::settle_target;
    use al_version::BcVersion;

    // al-runner issue 8243: a fetched DemoTool of another release than the build stops the run; one of the build's
    // release, or none at all (`--demotool DIR`, no DemoTool), keeps it going. The target asked (`28`, minor 0) does not
    // matter: an app floor of 28.0 on build 28.5 with the DemoTool of 28.5 runs.
    #[test]
    fn a_demotool_of_another_release_than_the_build_stops_the_run() {
        let build = "28.5.54151.57283";
        let mut bc = BcVersion::parse("28").unwrap();
        let e = settle_target(&mut bc, build, Some((28, 0))).unwrap_err();
        assert!(e.contains("release 28.0") && e.contains(build), "{e}");
        for (asked, fetched) in [("28", Some((28, 5))), ("28.5", Some((28, 5))), ("28", None)] {
            let mut bc = BcVersion::parse(asked).unwrap();
            settle_target(&mut bc, build, fetched).unwrap();
            assert_eq!((bc.major, bc.minor), (28, 5), "{asked}");
        }
        // A build of another major stops it too (`BcVersion::resolved_to`).
        let mut bc = BcVersion::parse("28").unwrap();
        assert!(settle_target(&mut bc, "27.5.46862.58057", None).is_err());
    }
}
