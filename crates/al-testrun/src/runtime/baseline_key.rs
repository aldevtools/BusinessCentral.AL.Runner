//! The key of the install baseline kept on disk (`baseline_cache.rs`): everything the baseline of a run depends on.
use super::{Compiled, Manifest, RunOptions};
use sha2::{Digest, Sha256};

/// What the key holds of an app that no layer of the compile cache covers: a workspace app, or a dependency app that
/// stands on one (`dependency_apps_on_workspace`). A layer's own key holds the same of its apps (`al_cache::AppKey`).
#[derive(Debug, Clone)]
pub(crate) struct UncoveredApp {
    /// The app's index in the program.
    pub(crate) app: usize,
    /// [`manifest_digest`] of its manifest.
    pub(crate) manifest: [u8; 32],
    /// The identity of its source when it is read lazily (its files then hold no text to hash).
    pub(crate) identity: Option<[u8; 32]>,
}

/// The uncovered apps of a compile: the program's apps from `n_cached` on. `manifests` and `identities` are in
/// program order.
pub(super) fn uncovered_apps(
    manifests: &[Manifest],
    identities: &[Option<[u8; 32]>],
    n_cached: usize,
) -> Vec<UncoveredApp> {
    manifests
        .iter()
        .enumerate()
        .skip(n_cached)
        .map(|(app, m)| UncoveredApp {
            app,
            manifest: manifest_digest(m),
            identity: identities.get(app).copied().flatten(),
        })
        .collect()
}

/// A hash of what a manifest gives the program (`apply_manifests::apply`) and the compile. The manifest is
/// destructured without `..`, so a field added to it does not compile until it is hashed or named as left out. The
/// version and the publisher are what `ModuleInfo` answers; install code that stores them wrote other rows for
/// another version (al-runner issue 7610).
pub(crate) fn manifest_digest(m: &Manifest) -> [u8; 32] {
    let Manifest {
        publisher,
        version,
        // Hashed from the program for each request: a warm server hands new resource files to a kept compile
        // (al-runner issue 5635), so the files of the compile's manifest are not the ones the install code reads.
        resources: _,
        include_source,
        target,
        logo,
        description,
        symbols,
        layout_files,
        id_ranges,
        translations,
        package_data,
        package_data_files,
        translation_files,
        // What the files say is read on demand from the package, whose content the dependency layers' key already holds.
        translation_source: _,
        runtime,
        app_json_keys,
        platform_internals_visible_to,
    } = m;
    let mut h = Sha256::new();
    let mut put = |name: &str, value: &[u8]| {
        h.update((name.len() as u64).to_le_bytes());
        h.update(name.as_bytes());
        h.update((value.len() as u64).to_le_bytes());
        h.update(value);
    };
    put("publisher", publisher.as_bytes());
    put("version", version.as_bytes());
    put("include_source", &[u8::from(*include_source)]);
    put("target", target.as_bytes());
    put("logo", format!("{logo:?}").as_bytes());
    put("description", format!("{description:?}").as_bytes());
    put("symbols", format!("{symbols:?}").as_bytes());
    for (name, content) in layout_files {
        put("layout_file", name.as_bytes());
        put("layout_file_content", content);
    }
    put("id_ranges", format!("{id_ranges:?}").as_bytes());
    put("translations", &[u8::from(*translations)]);
    put("package_data", &[u8::from(*package_data)]);
    for (name, content) in package_data_files {
        put("package_data_file", name.as_bytes());
        put("package_data_file_content", content);
    }
    put("translation_files", format!("{translation_files:?}").as_bytes());
    put("runtime", format!("{runtime:?}").as_bytes());
    put("app_json_keys", format!("{app_json_keys:?}").as_bytes());
    put(
        "platform_internals_visible_to",
        format!("{platform_internals_visible_to:?}").as_bytes(),
    );
    h.finalize().into()
}

/// The key under which the install baseline of this run is kept on disk, or `None` when it must run every time.
/// The baseline is what the install code of the dependency apps and `Company-Initialize` write, followed by the
/// workspace's own install codeunits. It is the same for the same program and the same world around it, which is
/// everything the key holds: this runtime build, the dependency layers (the key of the top layer holds every app of
/// them and the target), the source and the manifest of every other app, the test codeunits that were compiled, the BC target, the
/// environment of the host and the day. The rows carry the time the run began (`SystemClock`), and a test may ask how
/// old one is. The file therefore holds that instant, and a run that reads the file starts its own clock there, so
/// the rows and the clock agree as they do in a run that began then and ran the install code. The day only bounds how
/// far in the past a run may think it began.
///
/// The second text is the frame of the key: all of it but the source of the apps (`KeyBuilder::finish_frame`), what a
/// warm server's baseline carried over an edit must match (`baseline_cache::carry`).
///
/// Not kept at all: a run that lets `File` use a real directory (`--files-dir`), reads the real clock as it goes,
/// records, compares two executable forms or has a test runner of its own, and a baseline whose install code made a
/// `DotNet` call that may read more than its arguments (`Engine::dotnet_called`): what such a run reads is not in the key. Test data is in the key by what
/// its provider says it is (`TableProvider::identity`). Nothing the tests write is kept: the file is written before
/// the first test runs, and the login code's records added after the tests are taken only over the baseline as the
/// file holds it (`install::login_work_date`).
pub(super) fn baseline_key(
    compiled: &Compiled,
    only: Option<&[(usize, u32)]>,
    target: Option<&al_version::Target>,
    dotnet: &al_vm::dotnet::DotNetConfig,
    opts: &RunOptions,
) -> Option<(String, String)> {
    let (ws, lowered, suites) = (&compiled.ws, &compiled.lowered, &compiled.suites);
    let report = compiled.cache_report.as_ref()?;
    let uncovered = compiled.uncovered.as_deref()?;
    opts.compile_cache.as_ref()?;
    if opts.trace
        || opts.execute
        || opts.coverage
        || suites.iter().any(|s| s.runner.is_some())
        || crate::host::uses_real_files()
        || crate::host::real_clock()
    {
        return None;
    }
    let mut key = crate::baseline_cache::KeyBuilder::new();
    // A variant of a stability run (ADR-058 section 7) runs the install code under its own host values.
    let host_values = opts.perturb.host_key();
    if !host_values.is_empty() {
        key.text("stability_host", &host_values);
    }
    // The restored database: which backup, as the provider says (file, length, modification time, company). A
    // provider that cannot say gets nothing kept.
    match &opts.test_data {
        Some(td) => key.text("test_data", &td.provider.identity()?),
        None => key.text("test_data", ""),
    };
    // The license (ADR-060): install code may read License Permission. Only its hash goes into the key; the license
    // itself is never written to a cache.
    key.text("license", opts.license.as_ref().map_or("", |l| l.sha256.as_str()));
    // Whether `DotNet` calls of the install code could run. A baseline made without a usable .NET holds what the
    // refused calls left out, and one made with it holds what they did. With .NET, what runs the calls: a baseline is
    // kept only when its calls answer from their arguments alone (`Engine::dotnet_called`), and those answers are
    // the runtime's.
    let dotnet_text = match dotnet.host.availability() {
        Ok(()) => format!("available {}", dotnet.host.identity().unwrap_or_default()),
        Err(_) => "unavailable".to_string(),
    };
    key.text("dotnet", &dotnet_text);
    key.bytes("runtime_build", &crate::compile_cache::runtime_build());
    // The codeunit that builds the company, and what its files and its app's source are.
    // Every setting of how the company is built (steps, methods, environment, client, evaluation company), not only
    // the codeunit and the identity of its app and files.
    match &opts.company_data {
        Some(c) => key.text("company_data", &format!("{c:?}")),
        None => key.text("company_data", ""),
    };
    // `--tdd` lowers a call of a procedure that does not exist to a stub, in install code as in tests (ADR-051).
    key.text("tdd", if opts.tdd { "yes" } else { "no" });
    // A lookup by id of an object no loaded app declares is refused when a skipped app may declare it, and raises
    // the missing-object error otherwise (`al_vm::Engine::skipped_apps`): install code that makes one depends on it.
    key.text("skipped_dependencies", &opts.skipped_dependencies.join("\n"));
    key.text("dependencies", &report.file.file_name()?.to_string_lossy());
    for (i, app) in ws.apps.iter().enumerate() {
        // An app no layer holds: its manifest, and for a dependency app that stands on a workspace app (compiled with
        // the workspace, after the layers) also what the layers hold of the others.
        if let Some(u) = uncovered.iter().find(|u| u.app == i) {
            key.bytes("app_manifest", &u.manifest);
            if ws.kinds.get(i) == Some(&al_sema::AppKind::Dependency) {
                key.text("dependency_app", &app.id);
                key.text("dependency_app_name", &app.name);
                key.text("dependency_app_dependencies", &app.dependencies.join(","));
                let source = u.identity.unwrap_or_else(|| crate::compile_cache::source_hash(app));
                key.bytes("dependency_app_source", &source);
            }
        }
        // The install code of a dependency app loads its data files (NavApp.LoadPackageData, al-runner issue 6649); an app
        // folder's files can change without a new dependency layer.
        if ws.kinds.get(i) == Some(&al_sema::AppKind::Dependency)
            && let Some(a) = lowered.program.apps.get(i)
        {
            for (name, content) in &a.package_data_files {
                key.text("package_data", name);
                key.bytes("package_data_content", content);
            }
        }
        if ws.kinds.get(i) != Some(&al_sema::AppKind::Dependency) {
            key.text("app", &app.id);
            key.text("app_name", &app.name);
            key.text("app_dependencies", &app.dependencies.join(","));
            // The preprocessor symbols the app was parsed with: they select the code that is compiled.
            let symbols: Vec<&str> = ws.symbols_of(i).iter().map(String::as_str).collect();
            key.text("app_symbols", &symbols.join(","));
            key.source("source", &crate::compile_cache::source_hash(app));
            // The install code can read the app's resource files, and a warm server hands it new ones without a new
            // compile (issue 5635).
            for (name, content) in lowered
                .program
                .apps
                .get(i)
                .map(|a| a.resources.as_slice())
                .unwrap_or_default()
            {
                key.text("resource", name);
                key.bytes("resource_content", content);
            }
        }
    }
    let only_set = only.is_some();
    let only: Vec<String> = only.unwrap_or(&[]).iter().map(|(a, id)| format!("{a}/{id}")).collect();
    key.text("only", &only.join(","));
    key.text("only_set", if only_set { "yes" } else { "no" });
    match target {
        Some(t) => key.text("target", &format!("{}/{}", t.bc, t.runtime)),
        None => key.text("target", "none"),
    };
    key.text("environment", crate::host::environment_text());
    // The install code's writes inside a TryFunction follow the setting DisableWriteInsideTryFunctions (al-runner issue 7143).
    key.text(
        "write_in_try_allowed",
        if opts.allow_write_in_try_functions { "yes" } else { "no" },
    );
    // The install code reads "Published As" of the dependency apps (al-runner issue 7169).
    key.text(
        "dependencies_published_global",
        if opts.dependencies_published_global {
            "yes"
        } else {
            "no"
        },
    );
    // GetUrl answers with it, and Company-Initialize stores that answer (ADR-042).
    key.text("server_url", &crate::host::server_url().0);
    // CompanyName() answers it, and Company-Initialize may store that answer.
    key.text("company_name", crate::host::company_name().as_deref().unwrap_or(""));
    // The companies the per-company install code of the apps under test runs in (ADR-046 decision 2).
    key.text("companies", &crate::host::companies().key_text());
    // Whether the apps under test install in the listed order (`install::InstallView`).
    key.text(
        "publish_order_known",
        if crate::host::publish_order_known() {
            "yes"
        } else {
            "no"
        },
    );
    // Read the wall clock without starting the run clock: a run that finds its baseline starts the clock at the
    // instant the baseline's run began (`SystemClock::begin_at`).
    let day = crate::host::SystemClock::wall().div_euclid(86_400_000);
    key.text("day", &day.to_string());
    // The lowering reads it (ADR-023), so the install code it produces depends on it.
    key.text(
        "base_access",
        &std::env::var("AL_UNPINNED_BASE_ACCESS").unwrap_or_default(),
    );
    Some((key.finish(), key.finish_frame()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One change per manifest field the digest holds; `resources` is the one field left out.
    #[test]
    fn every_manifest_field_but_the_resources_changes_the_digest() {
        type Change = fn(&mut Manifest);
        let changes: [(&str, Change); 15] = [
            ("publisher", |m| m.publisher = "Other".into()),
            ("version", |m| m.version = "2.0.0.0".into()),
            ("include_source", |m| m.include_source = true),
            ("target", |m| m.target = "OnPrem".into()),
            ("logo", |m| m.logo = Some(al_ir::AppLogo::File(vec![1]))),
            ("description", |m| m.description = Some("text".into())),
            ("symbols", |m| m.symbols = vec!["CLEAN27".into()]),
            ("layout_files", |m| m.layout_files = vec![("a.rdl".into(), vec![1])]),
            ("id_ranges", |m| m.id_ranges = vec![(50000, 50099)]),
            ("translations", |m| m.translations = true),
            ("package_data", |m| m.package_data = true),
            ("package_data_files", |m| {
                m.package_data_files = vec![("a.navxdata".into(), vec![1])]
            }),
            ("translation_files", |m| {
                m.translation_files = vec![al_ir::TranslationFile::default()]
            }),
            ("runtime", |m| m.runtime = Some((16, 0))),
            ("app_json_keys", |m| m.app_json_keys = vec!["id".into()]),
        ];
        let plain = manifest_digest(&Manifest::default());
        for (name, change) in changes {
            let mut m = Manifest::default();
            change(&mut m);
            assert_ne!(manifest_digest(&m), plain, "{name}");
        }
        let m = Manifest {
            resources: vec![("a.txt".into(), vec![1])],
            ..Default::default()
        };
        assert_eq!(manifest_digest(&m), plain, "resources are hashed from the program");
    }
}
