//! The emit version a run's engine names in the missing-object error (al-vm `emit_version`): read from the run build's own
//! extracted artifact when BcContainerHelper's cache holds it, else from its platform artifact through the artifact cache
//! (`al_deps::emit_version`). Never from another build of the major: the number differs between builds of one minor.
//!
//! The emit version of the run's build for al-vm `emit_version`: the extracted service tier first,
//! then the artifact (al-deps `emit_version`). Entry points: `of_build`.
use al_deps::artifact::{Cdn, HttpCdn};
use al_deps::build::BuildId;
use al_deps::service_tier::ServiceTier;
use al_vm::emit_version::EmitVersion;
use std::path::{Path, PathBuf};

/// The emit version of `build`, read on first ask from `sandbox` (BcContainerHelper's cache) or the feed.
pub fn of_build(build: &BuildId, cache_root: &Path, download: bool, sandbox: Option<&Path>) -> EmitVersion {
    of_build_with(HttpCdn::from_env(), build, cache_root, download, sandbox)
}

/// [`of_build`] over the feed `cdn`.
pub fn of_build_with<C>(
    cdn: C,
    build: &BuildId,
    cache_root: &Path,
    download: bool,
    sandbox: Option<&Path>,
) -> EmitVersion
where
    C: Cdn + Send + Sync + 'static,
    C::Src: Send,
{
    let extracted: Option<PathBuf> = sandbox.and_then(|s| crate::dotnet::own_build_probe_dir(s, build.as_str()));
    let tier = ServiceTier::new(cdn, cache_root, build.clone(), download);
    EmitVersion::from_fn(move || {
        if let Some(found) = extracted.as_deref().and_then(al_deps::emit_version::in_dir) {
            return found;
        }
        al_deps::emit_version::of_tier(&tier)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_deps::remote::{FileSource, RemoteError};

    struct Feed(PathBuf);

    impl Cdn for Feed {
        type Src = FileSource;
        fn base_url(&self) -> String {
            "https://cdn.test".into()
        }
        fn index(&self) -> Result<Vec<u8>, RemoteError> {
            panic!("the index is not needed")
        }
        fn platform(&self, build: &BuildId) -> (String, FileSource) {
            (format!("https://cdn.test/{build}/platform"), FileSource(self.0.clone()))
        }
        fn country(&self, _: &BuildId, _: &str) -> (String, FileSource) {
            panic!("the country artifact is not needed")
        }
    }

    #[test]
    fn the_own_extracted_build_is_read_first_and_the_cache_after_it() {
        let t = tempfile::tempdir().unwrap();
        let build = BuildId::parse("28.5.54151.58194").unwrap();
        let svc = t
            .path()
            .join("sandbox/28.5.54151.58194/platform/ServiceTier/PFiles64/Microsoft Dynamics NAV/280/Service");
        std::fs::create_dir_all(&svc).unwrap();
        std::fs::write(svc.join(al_deps::emit_version::CODE_ANALYSIS), b"not an assembly").unwrap();
        let feed = || Feed(t.path().join("gone.zip"));
        let v = of_build_with(
            feed(),
            &build,
            &t.path().join("c"),
            false,
            Some(&t.path().join("sandbox")),
        );
        let e = v.get().unwrap_err();
        assert!(
            e.contains("defines no int32 constant") && e.contains("280/Service"),
            "{e}"
        );
        // Another build of the major in the sandbox is not read: the artifact cache is, and offline it has nothing.
        let other = BuildId::parse("28.5.54976.58543").unwrap();
        let v = of_build_with(
            feed(),
            &other,
            &t.path().join("c"),
            false,
            Some(&t.path().join("sandbox")),
        );
        assert!(v.get().unwrap_err().contains("downloading is off"));
    }
}
