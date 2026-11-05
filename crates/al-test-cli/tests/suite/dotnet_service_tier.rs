//! al-runner issue 6924: a DotNet type of BC's service tier (`Microsoft.Dynamics.Nav.OpenXml`) loads on a machine
//! with only a .NET runtime, because the sidecar host fetches the assembly and the service-tier assemblies it
//! references from the platform artifact into the artifact cache. Needs .NET (skips without it, as
//! `dotnet_sidecar.rs` does) and Microsoft's artifact feed (skips with a message when offline).

use al_deps::artifact::{Cdn, HttpCdn};
use al_deps::service_tier::ServiceTier;
use al_host::dotnet::*;
use al_test_cli::dotnet::{AssemblyFetch, SidecarConfig, SidecarHost};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

#[test]
fn workbook_writer_create_loads_from_the_fetched_service_tier_assemblies() {
    let cdn = HttpCdn::from_env();
    let index = match cdn.index() {
        Ok(i) => i,
        Err(e) => {
            eprintln!(
                "SKIPPED (offline: the artifact feed {} cannot be reached): {e}",
                cdn.base_url()
            );
            return;
        }
    };
    // The newest BC 27 build the feed publishes: the feed drops old builds, so none is named here.
    let builds = al_deps::build::parse_index(&String::from_utf8_lossy(&index)).unwrap();
    let build = builds
        .into_iter()
        .filter(|b| b.major() == 27)
        .max()
        .expect("a BC 27 build on the feed");
    let cache = tempfile::tempdir().unwrap();
    let tier = Arc::new(ServiceTier::new(cdn, cache.path(), build, true));
    let bytes = Arc::new(AtomicU64::new(0));
    let (t, b) = (tier.clone(), bytes.clone());
    let cfg = SidecarConfig {
        source_root: Some(root()),
        cache_dir: Some(Path::new(env!("CARGO_TARGET_TMPDIR")).join("dotnet-sidecar-cache")),
        assembly_dirs: vec![tier.dir()],
        fetch_assembly: Some(AssemblyFetch(Arc::new(move |name: &str| {
            b.fetch_add(t.ensure(name)?.bytes, Ordering::Relaxed);
            Ok(())
        }))),
        ..Default::default()
    };
    let host = Arc::new(SidecarHost::new(cfg));
    if let Err(why) = host.availability() {
        assert!(
            std::env::var("ALRT_REQUIRE_DOTNET").is_err(),
            "ALRT_REQUIRE_DOTNET is set but .NET is unavailable: {why}"
        );
        eprintln!("SKIPPED (no usable .NET): DotNet {why}");
        return;
    }
    let mut sess = host.open_session(&DotNetSessionConfig::default());
    let writer = DotNetType {
        assembly: "Microsoft.Dynamics.Nav.OpenXml".into(),
        name: "Microsoft.Dynamics.Nav.OpenXml.Spreadsheet.WorkbookWriter".into(),
        ..Default::default()
    };
    // BC's ConfigExcelExchange calls `WrkbkWriter.Create(InStream)`. An empty stream is no workbook, so OpenXml may
    // throw; what matters is that the assembly loads, so a refusal (not loaded, not found) is the only failure.
    let got = sess.invoke(
        DotNetTarget::Static(writer),
        "Create",
        vec![DotNetArg {
            value: DotNetValue::InStream(Vec::new()),
            by_ref: false,
        }],
    );
    if let Err(DotNetFailure::Refused(why)) = &got {
        panic!("WorkbookWriter.Create was refused: {why}");
    }
    let fetched = bytes.load(Ordering::Relaxed);
    eprintln!(
        "fetched {:.1} MB of service-tier assemblies for WorkbookWriter",
        fetched as f64 / 1e6
    );
    assert!(tier.dir().join("Microsoft.Dynamics.Nav.OpenXml.dll").is_file());
    assert!(tier.dir().join("DocumentFormat.OpenXml.dll").is_file());
    assert!(fetched > 0);
}
