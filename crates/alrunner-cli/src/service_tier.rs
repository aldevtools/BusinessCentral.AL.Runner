//! Where the .NET sidecar of a run finds BC's service-tier assemblies (ADR-014 section 3, step 2; al-runner issue
//! 6924): the user's `--dotnet-assembly-dir` folders, then an extracted artifact in BcContainerHelper's cache
//! (`al_deps::cache::bcartifacts_sandbox`: `~/.bcartifacts.cache/sandbox`, then on Windows `C:\bcartifacts.cache\sandbox`), then
//! the folder al-runner fetches the assemblies a DotNet declaration names into, by range from the platform artifact
//! of the build in use (`al_deps::service_tier`). The fetch runs only when no extracted artifact serves the major,
//! and only for an assembly the user's folders do not hold.
use crate::args::TestArgs;
use al_deps::artifact::{ArtifactUse, HttpCdn};
use al_deps::service_tier::ServiceTier;
use al_testrun::dotnet::{AssemblyFetch, SidecarConfig};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

/// The sidecar configuration of a run and its report renderer (ADR-040 amendment 2026-11-15): BC's Word merger and Html
/// converter of the build in use, started on the first measured Html save. The renderer's sets are fetched only when no
/// extracted artifact serves the build, as the assemblies of [`dotnet_dirs`].
pub fn sidecar_and_renderer(
    a: &TestArgs,
    build: Option<&ArtifactUse>,
    major: u32,
    source_root: Option<PathBuf>,
) -> (SidecarConfig, Arc<dyn al_host::render::ReportRenderer>) {
    let cfg = SidecarConfig {
        sidecar: a.dotnet_sidecar.clone(),
        source_root,
        // Under the same cache root as the apps when the user chose one (issue 5747).
        cache_dir: al_testrun::dotnet::sidecar_cache_dir_from_env(a.cache_dir.as_deref()),
        allow_roll_forward: a.dotnet_allow_roll_forward,
        timeout: Duration::from_secs(a.dotnet_timeout_secs.unwrap_or(60)),
        // ADR-014 section 3, step 2 and issue 6924: the user's folders, an extracted artifact, then fetched assemblies.
        ..dotnet_dirs(a, build, major)
    };
    let renderer = al_testrun::report_render_pieces::run_renderer(
        cfg.clone(),
        build.filter(|_| cfg.fetch_assembly.is_some()).map(|u| u.build.clone()),
        &al_deps::cache::cache_root_from_env(a.cache_dir.as_deref()),
        !a.no_download,
    );
    (cfg, renderer)
}

/// Gives the run's `dotnet` configuration its report renderer and `--require-report-renderer`.
pub fn set_renderer(
    dotnet: &mut al_vm::dotnet::DotNetConfig,
    renderer: Arc<dyn al_host::render::ReportRenderer>,
    a: &TestArgs,
) {
    dotnet.renderer = renderer;
    dotnet.require_renderer = a.require_report_renderer;
}

/// A sidecar configuration with the probe folders and the fetch set, to fill in the rest of a run's configuration.
pub fn dotnet_dirs(a: &TestArgs, build: Option<&ArtifactUse>, major: u32) -> SidecarConfig {
    let sandbox = al_deps::cache::bcartifacts_sandbox_from_env();
    let name = build.map(|u| u.build.to_string());
    let extracted = sandbox
        .as_deref()
        .and_then(|s| al_testrun::dotnet::default_probe_dir(s, name.as_deref(), major));
    let cache_root = al_deps::cache::cache_root_from_env(a.cache_dir.as_deref());
    let tier = build
        .filter(|_| extracted.is_none())
        .map(|u| ServiceTier::new(HttpCdn::from_env(), &cache_root, u.build.clone(), !a.no_download));
    let fetched = tier.as_ref().map(ServiceTier::dir);
    let assembly_dirs = al_testrun::dotnet::probe_dirs(
        &a.dotnet_assembly_dirs,
        sandbox.as_deref(),
        name.as_deref(),
        major,
        fetched.as_deref(),
    );
    let user = a.dotnet_assembly_dirs.clone();
    SidecarConfig {
        assembly_dirs,
        // The .NET major the sidecar runs on follows the BC major (al_testrun::dotnet_runtime; issues 6924, 5590).
        bc_major: Some(major),
        fetch_assembly: tier.map(|t| {
            let t = Arc::new(t);
            AssemblyFetch(Arc::new(move |assembly: &str| fetch(&t, &user, assembly)))
        }),
        ..Default::default()
    }
}

fn fetch(tier: &ServiceTier<HttpCdn>, user: &[PathBuf], assembly: &str) -> Result<(), String> {
    if user.iter().any(|d| holds(d, assembly)) {
        return Ok(());
    }
    let got = tier.ensure(assembly)?;
    if !got.files.is_empty() {
        eprintln!(
            "fetched {} BC service-tier assemblies ({:.1} MB) for DotNet {assembly} into {}",
            got.files.len(),
            got.bytes as f64 / 1_000_000.0,
            tier.dir().display()
        );
    }
    Ok(())
}

/// Whether `dir`'s top level has `<assembly>.dll` or `.exe`, as the sidecar would find it.
fn holds(dir: &Path, assembly: &str) -> bool {
    ["dll", "exe"]
        .iter()
        .any(|ext| dir.join(format!("{assembly}.{ext}")).is_file())
}
