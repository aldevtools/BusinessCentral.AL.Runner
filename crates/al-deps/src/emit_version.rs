//! The emit version of a BC build: the `const int` `EmitVersion.CodeEmitVersion` (namespace
//! `Microsoft.Dynamics.Nav.CodeAnalysis.Emit`) of `Microsoft.Dynamics.Nav.CodeAnalysis.dll` in the service folder of the
//! platform artifact. BC writes `NavEnvironment.EmitVersion` in the error for a missing object; that property is this
//! constant unless the server setting `EmitVersionOverride` is set, which a container leaves at its default (-1).
//!
//! Read from the artifacts on 2026-11-20 (the BC runtime's use is assumption `missing-object-emit-version-from-artifact`):
//! 27.5.46862.48827 27024, 27.5.46862.58057 27025, 27.5.54899.58548 27025, 28.5.54151.54365 28014, 28.5.54151.58194 28015,
//! 28.5.54976.58543 28015, 29.0.54011.55816 29022, 29.0.54011.56262 29022. So the number is a fact of the build, not of
//! the BC minor.
//!
//! The emit version of a BC build, the constant `EmitVersion.CodeEmitVersion` of
//! `Microsoft.Dynamics.Nav.CodeAnalysis.dll` in its platform artifact (assumption `missing-object-
//! emit-version-from-artifact`). Entry points: `emit_version` (the file's bytes), `in_dir`, `of_tier`
//! (through the artifact cache).
use crate::artifact::Cdn;
use crate::service_tier::ServiceTier;
use std::path::Path;

/// The service-folder file that holds the constant.
pub const CODE_ANALYSIS: &str = "Microsoft.Dynamics.Nav.CodeAnalysis.dll";

/// The emit version `bytes` (the file [`CODE_ANALYSIS`]) defines.
pub fn emit_version(bytes: &[u8]) -> Result<u32, String> {
    let n = crate::assembly_refs::literal_i32(
        bytes,
        "Microsoft.Dynamics.Nav.CodeAnalysis.Emit",
        "EmitVersion",
        "CodeEmitVersion",
    )
    .ok_or_else(|| format!("{CODE_ANALYSIS} defines no int32 constant EmitVersion.CodeEmitVersion"))?;
    u32::try_from(n).map_err(|_| format!("{CODE_ANALYSIS} defines EmitVersion.CodeEmitVersion as {n}"))
}

/// The emit version of the service folder `dir` of an extracted artifact; `None` when `dir` holds no [`CODE_ANALYSIS`].
pub fn in_dir(dir: &Path) -> Option<Result<u32, String>> {
    let bytes = std::fs::read(dir.join(CODE_ANALYSIS)).ok()?;
    Some(emit_version(&bytes).map_err(|e| format!("{}: {e}", dir.display())))
}

/// The emit version of `tier`'s build, its [`CODE_ANALYSIS`] read from the cache or else from the platform artifact
/// (and kept in the cache).
pub fn of_tier<C: Cdn>(tier: &ServiceTier<C>) -> Result<u32, String> {
    match tier.service_file(CODE_ANALYSIS)? {
        Some(bytes) => emit_version(&bytes),
        None => Err(format!(
            "the platform artifact of build {} has no {CODE_ANALYSIS}",
            tier.build()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assembly_refs::testutil::literal;
    use crate::service_tier::tests::{Platform, SVC, build};
    use crate::zip::testutil::Builder;

    fn code_analysis(n: i32) -> Vec<u8> {
        literal(
            "Microsoft.Dynamics.Nav.CodeAnalysis.Emit",
            "EmitVersion",
            "CodeEmitVersion",
            0x08,
            &n.to_le_bytes(),
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
    fn the_emit_version_is_the_code_emit_version_constant() {
        assert_eq!(emit_version(&code_analysis(28015)), Ok(28015));
        assert!(emit_version(&code_analysis(-1)).unwrap_err().contains("as -1"));
        let other = literal(
            "Microsoft.Dynamics.Nav.CodeAnalysis.Emit",
            "EmitVersion",
            "Other",
            0x08,
            &1i32.to_le_bytes(),
        );
        assert!(emit_version(&other).unwrap_err().contains("defines no int32 constant"));
        assert!(emit_version(b"not an assembly").is_err());
    }

    #[test]
    fn the_file_of_a_fetched_tier_is_read_once_from_the_feed_and_then_from_the_cache() {
        let t = tempfile::tempdir().unwrap();
        let dll = code_analysis(27025);
        let zip = platform(t.path(), &[(CODE_ANALYSIS, &dll)]);
        let cache = t.path().join("c");
        let tier = ServiceTier::new(Platform(zip), &cache, build(), true);
        assert_eq!(of_tier(&tier), Ok(27025));
        // Offline, with the feed gone, the cached file still answers.
        let off = ServiceTier::new(Platform(t.path().join("gone.zip")), &cache, build(), false);
        assert_eq!(of_tier(&off), Ok(27025));
        assert_eq!(in_dir(&tier.dir()), Some(Ok(27025)));
    }

    #[test]
    fn a_platform_without_the_file_or_offline_without_it_cached_is_an_error() {
        let t = tempfile::tempdir().unwrap();
        let zip = platform(t.path(), &[("Microsoft.Dynamics.Nav.Ncl.dll", b"x")]);
        let tier = ServiceTier::new(Platform(zip.clone()), &t.path().join("c"), build(), true);
        assert!(
            of_tier(&tier)
                .unwrap_err()
                .contains("has no Microsoft.Dynamics.Nav.CodeAnalysis.dll")
        );
        let off = ServiceTier::new(Platform(zip), &t.path().join("d"), build(), false);
        assert!(of_tier(&off).unwrap_err().contains("downloading is off"));
        assert_eq!(in_dir(t.path()), None);
    }
}
