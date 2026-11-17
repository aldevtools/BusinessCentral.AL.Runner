//! The `Microsoft.NETCore.App` major a BC build's service tier runs on (ADR-014 section 2): the version of that framework
//! in `Microsoft.Dynamics.Nav.Server.runtimeconfig.json`, the file the server is started with, in the service folder.
//!
//! Measured on the platform artifacts of the feed on 2026-11-17 (`runtimeOptions.frameworks`; the
//! `TargetFrameworkAttribute` of `Microsoft.Dynamics.Nav.OpenXml.dll` agrees in each):
//! - `8.0.0` (`"tfm": "net8.0"`): 26.5.38752.58303, 27.5.46862.56249, 27.5.46862.58057, 27.5.46862.58294,
//!   28.5.54151.56267, 28.5.54151.58194.
//! - `10.0.0` (`"tfm": "net10.0"`): 27.0.54905.58351, 27.5.54899.58349, 28.5.54976.58348, 29.0.54011.56262,
//!   29.0.54011.58356.
//!
//! The same list, as data: `al_version::MEASURED_SERVICE_RUNTIMES`. So the major is a fact of the build, not of the BC
//! major: the BC 27 and 28 builds published on 2026-11-17 run on .NET 10, the builds of the same minors before them on
//! .NET 8.
use crate::artifact::Cdn;
use crate::service_tier::ServiceTier;
use std::path::Path;

/// The service folder's runtime configuration of the BC server.
pub const SERVER_RUNTIME_CONFIG: &str = "Microsoft.Dynamics.Nav.Server.runtimeconfig.json";

/// The major of the `Microsoft.NETCore.App` entry of `runtimeOptions.frameworks` in a runtime configuration, the form
/// every measured build has.
pub fn netcore_major(json: &[u8]) -> Result<u32, String> {
    let v: serde_json::Value =
        serde_json::from_slice(json).map_err(|e| format!("{SERVER_RUNTIME_CONFIG} is no JSON: {e}"))?;
    let version = v["runtimeOptions"]["frameworks"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|f| f["name"].as_str() == Some("Microsoft.NETCore.App"))
        .and_then(|f| f["version"].as_str())
        .ok_or_else(|| format!("{SERVER_RUNTIME_CONFIG} names no Microsoft.NETCore.App framework version"))?;
    version
        .split('.')
        .next()
        .and_then(|m| m.parse().ok())
        .ok_or_else(|| format!("{SERVER_RUNTIME_CONFIG} names Microsoft.NETCore.App version '{version}'"))
}

/// The major the service folder `dir` runs on (an extracted artifact); `None` when `dir` holds no runtime configuration.
pub fn major_in_dir(dir: &Path) -> Option<Result<u32, String>> {
    let bytes = std::fs::read(dir.join(SERVER_RUNTIME_CONFIG)).ok()?;
    Some(netcore_major(&bytes).map_err(|e| format!("{}: {e}", dir.display())))
}

/// The major the service tier of `tier`'s build runs on, its runtime configuration read from the cache or else from the
/// platform artifact (and kept in the cache).
pub fn major_of_tier<C: Cdn>(tier: &ServiceTier<C>) -> Result<u32, String> {
    match tier.service_file(SERVER_RUNTIME_CONFIG)? {
        Some(bytes) => netcore_major(&bytes),
        None => Err(format!(
            "the platform artifact of build {} has no {SERVER_RUNTIME_CONFIG}",
            tier.build()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service_tier::tests::{Platform, SVC, build};
    use crate::zip::testutil::Builder;

    /// The `frameworks` part of the measured files, with the version under test.
    fn config(netcore: &str) -> String {
        format!(
            r#"{{"runtimeOptions":{{"tfm":"net10.0","frameworks":[{{"name":"Microsoft.NETCore.App","version":"{netcore}"}},{{"name":"Microsoft.AspNetCore.App","version":"{netcore}"}}]}}}}"#
        )
    }

    fn platform(dir: &Path, files: &[(&str, &[u8])]) -> std::path::PathBuf {
        let mut b = Builder::new();
        for (name, bytes) in files {
            b = b.add(&format!("{SVC}/{name}"), bytes, true);
        }
        let path = dir.join("platform.zip");
        std::fs::write(&path, b.finish()).unwrap();
        path
    }

    #[test]
    fn the_major_is_the_netcore_framework_version_of_the_server_runtime_config() {
        assert_eq!(netcore_major(config("10.0.0").as_bytes()), Ok(10));
        assert_eq!(netcore_major(config("8.0.0").as_bytes()), Ok(8));
        // Another form than the measured `frameworks` list is not read.
        let single = r#"{"runtimeOptions":{"framework":{"name":"Microsoft.NETCore.App","version":"8.0.0"}}}"#;
        assert!(
            netcore_major(single.as_bytes())
                .unwrap_err()
                .contains("names no Microsoft.NETCore.App")
        );
        assert!(netcore_major(b"not json").unwrap_err().contains("is no JSON"));
    }

    #[test]
    fn the_config_of_a_fetched_tier_is_read_once_from_the_feed_and_then_from_the_cache() {
        let t = tempfile::tempdir().unwrap();
        let cfg = config("10.0.0");
        let zip = platform(t.path(), &[(SERVER_RUNTIME_CONFIG, cfg.as_bytes())]);
        let cache = t.path().join("c");
        let tier = ServiceTier::new(Platform(zip), &cache, build(), true);
        assert_eq!(major_of_tier(&tier), Ok(10));
        assert!(tier.dir().join(SERVER_RUNTIME_CONFIG).is_file());
        // Offline, with the feed gone, the cached file still answers.
        let off = ServiceTier::new(Platform(t.path().join("gone.zip")), &cache, build(), false);
        assert_eq!(major_of_tier(&off), Ok(10));
        assert_eq!(major_in_dir(&tier.dir()), Some(Ok(10)));
    }

    #[test]
    fn a_platform_without_the_config_or_offline_without_it_cached_is_an_error() {
        let t = tempfile::tempdir().unwrap();
        let zip = platform(t.path(), &[("Microsoft.Dynamics.Nav.Ncl.dll", b"x")]);
        let tier = ServiceTier::new(Platform(zip.clone()), &t.path().join("c"), build(), true);
        let e = major_of_tier(&tier).unwrap_err();
        assert!(
            e.contains("has no Microsoft.Dynamics.Nav.Server.runtimeconfig.json"),
            "{e}"
        );
        let off = ServiceTier::new(Platform(zip), &t.path().join("d"), build(), false);
        assert!(major_of_tier(&off).unwrap_err().contains("downloading is off"));
        assert_eq!(major_in_dir(t.path()), None);
    }
}
