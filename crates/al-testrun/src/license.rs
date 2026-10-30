//! Which BC license a run uses (ADR-060 decision 2), shared by both command lines: `--license FILE` (serve: `license`),
//! else the license of the BC artifacts the run uses, else none. The file is read into memory and parsed by
//! `al_host::license`; it is never written anywhere. A file that cannot be parsed stops the run before any test starts.
use std::path::Path;
use std::sync::Arc;

/// Where the license of a run came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// `--license FILE` (or the serve request's `license`).
    Flag(String),
    /// The license that ships with the BC artifacts of the build (`from`: the file or artifact entry).
    Artifact { build: String, from: String },
    /// No license: why.
    None(String),
}

/// The license of a run and where it came from: what the run report names.
#[derive(Debug, Clone)]
pub struct LicenseUse {
    pub license: Option<Arc<al_host::license::License>>,
    pub source: Source,
}

impl LicenseUse {
    /// No license, for `reason`.
    pub fn none(reason: impl Into<String>) -> LicenseUse {
        LicenseUse {
            license: None,
            source: Source::None(reason.into()),
        }
    }

    /// The license file the user gave. Any error (unreadable, not a license) stops the run.
    pub fn from_file(path: &Path) -> Result<LicenseUse, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("--license {}: {e}", path.display()))?;
        let license = al_host::license::parse(&bytes).map_err(|e| format!("--license {}: {e}", path.display()))?;
        Ok(LicenseUse {
            license: Some(Arc::new(license)),
            source: Source::Flag(path.display().to_string()),
        })
    }

    /// The license of the artifacts of `build`. No file there (offline, an artifact without one) is a run without a
    /// license; a file that cannot be parsed stops the run.
    pub fn from_artifact<C: al_deps::artifact::Cdn>(
        cdn: &C,
        fetch: &al_deps::artifact::Fetch,
        build: &al_deps::build::BuildId,
    ) -> Result<LicenseUse, String> {
        match al_deps::license::artifact_license(cdn, fetch, build) {
            Ok(found) => Self::from_artifact_bytes(&build.to_string(), &found.from, &found.bytes),
            Err(why) => Ok(Self::none(format!("no license from the BC artifacts: {why}"))),
        }
    }

    /// [`LicenseUse::from_artifact`] once the bytes are read.
    pub fn from_artifact_bytes(build: &str, from: &str, bytes: &[u8]) -> Result<LicenseUse, String> {
        let license =
            al_host::license::parse(bytes).map_err(|e| format!("the license of the BC artifacts ({from}): {e}"))?;
        Ok(LicenseUse {
            license: Some(Arc::new(license)),
            source: Source::Artifact {
                build: build.to_string(),
                from: from.to_string(),
            },
        })
    }

    /// One line for the run report.
    pub fn line(&self) -> String {
        match (&self.license, &self.source) {
            (Some(l), source) => {
                let whose = l.header.licensee.as_deref().unwrap_or("(no licensee in the header)");
                let expires = l.header.expires.as_deref().unwrap_or("(no expiry date in the header)");
                let from = match source {
                    Source::Flag(p) => format!("--license {p}"),
                    Source::Artifact { build, .. } => format!("the BC artifacts of build {build}"),
                    Source::None(_) => String::new(),
                };
                format!(
                    "license: {whose}, expires {expires}, from {from}, SHA-256 {}; signature not checked",
                    &l.sha256
                )
            }
            (None, Source::None(why)) => {
                format!("license: none ({why}); License Permission answers only the rows the corpus pins")
            }
            (None, _) => "license: none".into(),
        }
    }

    /// The values the JUnit and JSON reports carry: `source` (`flag`, `artifact` or `none`), `path` (file or artifact
    /// entry), `licensee`, `expires`, `sha256`, `signatureChecked` (always false), `reason` (for none).
    pub fn properties(&self) -> Vec<(&'static str, String)> {
        let mut out = Vec::new();
        match &self.source {
            Source::Flag(p) => {
                out.push(("source", "flag".to_string()));
                out.push(("path", p.clone()));
            }
            Source::Artifact { build, from } => {
                out.push(("source", "artifact".to_string()));
                out.push(("build", build.clone()));
                out.push(("path", from.clone()));
            }
            Source::None(why) => {
                out.push(("source", "none".to_string()));
                out.push(("reason", why.clone()));
            }
        }
        if let Some(l) = &self.license {
            out.push(("licensee", l.header.licensee.clone().unwrap_or_default()));
            out.push(("expires", l.header.expires.clone().unwrap_or_default()));
            out.push(("sha256", l.sha256.clone()));
            out.push(("signatureChecked", "false".to_string()));
        }
        out
    }

    /// [`LicenseUse::properties`] as a JSON object.
    pub fn json(&self) -> serde_json::Value {
        let mut m = serde_json::Map::new();
        for (k, v) in self.properties() {
            let v = if k == "signatureChecked" {
                serde_json::Value::Bool(false)
            } else {
                serde_json::Value::String(v)
            };
            m.insert(k.to_string(), v);
        }
        serde_json::Value::Object(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic() -> Vec<u8> {
        al_host::license::synthetic::file("Synthetic Licensee", "1/2/2040", &[("Codeunit", &[(1, 9, 31)])])
    }

    #[test]
    fn a_license_file_is_named_with_its_licensee_expiry_and_hash() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x.bclicense");
        std::fs::write(&p, synthetic()).unwrap();
        let u = LicenseUse::from_file(&p).unwrap();
        let line = u.line();
        assert!(
            line.contains("Synthetic Licensee") && line.contains("1/2/2040"),
            "{line}"
        );
        assert!(
            line.contains("--license") && line.contains("signature not checked"),
            "{line}"
        );
        let j = u.json();
        assert_eq!(j["source"], "flag");
        assert_eq!(j["sha256"].as_str().unwrap().len(), 64);
        assert_eq!(j["signatureChecked"], false);
    }

    #[test]
    fn a_file_that_is_no_license_stops_the_run() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x.bclicense");
        std::fs::write(&p, b"not a license").unwrap();
        let e = LicenseUse::from_file(&p).unwrap_err();
        assert!(e.contains("--license") && e.contains("byte order mark"), "{e}");
        assert!(LicenseUse::from_file(&dir.path().join("missing")).is_err());
        assert!(LicenseUse::from_artifact_bytes("27.5", "x", b"no").is_err());
    }

    #[test]
    fn the_artifact_license_and_none_are_named() {
        let u = LicenseUse::from_artifact_bytes("27.5.1.2", "https://cdn/x#Cronus.bclicense", &synthetic()).unwrap();
        assert!(u.line().contains("the BC artifacts of build 27.5.1.2"), "{}", u.line());
        assert_eq!(u.json()["source"], "artifact");
        let n = LicenseUse::none("offline");
        assert!(n.line().starts_with("license: none (offline)"), "{}", n.line());
        assert_eq!(n.json()["reason"], "offline");
    }
}
