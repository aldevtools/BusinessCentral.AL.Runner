//! `oracle/oracle.toml`: the corpus pin and one `[[version]]` table per BC major
//! (ADR-015 3.5).

use al_version::BcVersion;

#[derive(Debug, serde::Deserialize)]
pub struct OracleFile {
    pub corpus_repo: String,
    pub corpus_app: String,
    pub corpus_commit: String,
    /// The companies of the oracle's database (ADR-046 decision 2 and amendment 1, A6): the corpus run declares them as
    /// `--companies` does. Absent: one company.
    pub companies: Option<CompaniesValue>,
    #[serde(rename = "version")]
    pub versions: Vec<VersionEntry>,
}

/// `companies` in `oracle.toml`: a list of names, the session company first, or `"all"`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(untagged)]
pub enum CompaniesValue {
    Names(Vec<String>),
    Word(String),
}

impl CompaniesValue {
    /// The setting it stands for; a word other than `all`, an empty list or a name given twice is an error.
    pub fn companies(&self) -> Result<al_host::Companies, String> {
        match self {
            CompaniesValue::Word(w) if w == "all" => Ok(al_host::Companies::All),
            CompaniesValue::Word(w) => Err(format!("companies = \"{w}\": a list of names or \"all\"")),
            CompaniesValue::Names(n) if n.is_empty() => Err("companies = []: name at least the session company".into()),
            CompaniesValue::Names(n) => al_host::Companies::parse(&n.join(",")).map_err(|e| format!("companies: {e}")),
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct VersionEntry {
    pub major: u16,
    /// The corpus CI leg the oracle came from (the newest minor of the major).
    pub bc_version: String,
    /// The corpus commit this oracle was recorded for, when it is older than the pin. Absent
    /// means the pin. Accepted only when the corpus diff since then adds files and changes none
    /// (`pin::check`, ADR-007).
    pub corpus_commit: Option<String>,
    /// The full build, once the corpus records it.
    pub bc_build: Option<String>,
    /// The companies of this major's database, when they differ from the top-level `companies`.
    pub companies: Option<CompaniesValue>,
    pub workflow_run: Option<u64>,
    pub artifact_id: Option<u64>,
    /// The `runtime` the corpus apps were compiled and published with on this leg. The apps in the
    /// corpus declare the newest runtime; the CI patches it per leg to the newest the service tier
    /// accepts (27.5: 16.1, 28.5: 17.1, 29.0: 18.0, corpus run 39110861115). The corpus job compiles with this
    /// value, so what the runtime sees is what real BC ran. Absent: the app.json's own runtime.
    pub app_runtime: Option<String>,
    /// JUnit path, relative to the directory of `oracle.toml`.
    pub junit: String,
    /// Ratchet path, relative to the repository root.
    pub baseline: String,
    /// The other test apps the corpus holds (`[[version.suite]]`): each has its own oracle file,
    /// ratchet and report line. The main corpus app is the version's own `junit` and `baseline`.
    #[serde(default, rename = "suite")]
    pub suites: Vec<SuiteEntry>,
}

/// How an oracle file is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OracleFormat {
    /// The corpus CI's JUnit (Linux legs).
    #[default]
    Junit,
    /// BcContainerHelper's XUnit (the Windows nightly).
    Xunit,
}

/// A test app beyond the main corpus app, with the real-BC run that is its oracle.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SuiteEntry {
    /// The report name and the suffix of its output files; unique within a version.
    pub name: String,
    /// The app folder, relative to the repository root.
    pub app: String,
    /// Oracle path, relative to the directory of `oracle.toml`.
    pub oracle: String,
    #[serde(default)]
    pub format: OracleFormat,
    /// The test runner codeunit of the app the real run used (`-testRunnerCodeunitId`). Its
    /// `TestIsolation` property is how this suite is run. Absent: Microsoft's `Test Runner - Isol.
    /// Codeunit`, as for the main app.
    pub runner: Option<u32>,
    /// The BC build the oracle came from, when it is not the version's own leg (the isolation
    /// probe only runs in the Windows nightly).
    pub bc_version: Option<String>,
    /// The corpus commit of the oracle run, when it is older than the version's. Accepted only
    /// like `VersionEntry::corpus_commit` (ADR-007).
    pub corpus_commit: Option<String>,
    pub workflow_run: Option<u64>,
    pub artifact_id: Option<u64>,
    /// Ratchet path, relative to the repository root.
    pub baseline: String,
    /// The apps the corpus CI installs for this suite although no test app declares them (corpus
    /// `.github/workflows/ci.yml`, `extra_apps` of the job that runs the suite). They change what AL code sees (the
    /// test runner runs codeunits of Tests-TestLibraries around every test, `al-testrun` `runner_reset.rs`), so a
    /// suite that names any runs in an environment of its own: its own compile and run, with these apps loaded like
    /// declared dependencies, and without the main app's tests.
    #[serde(default)]
    pub extra_apps: Vec<ExtraApp>,
}

/// An app of `SuiteEntry::extra_apps`: the id it is found by, and its name for messages.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct ExtraApp {
    pub id: String,
    pub name: String,
}

impl VersionEntry {
    /// The corpus commit this version's oracle belongs to.
    pub fn oracle_commit<'a>(&'a self, pin: &'a str) -> &'a str {
        self.corpus_commit.as_deref().unwrap_or(pin)
    }

    /// The app versions of the corpus app for this leg: its own, with `runtime` replaced by `app_runtime`.
    pub fn app_versions_for_leg(&self, mut app: al_version::AppVersions) -> Result<al_version::AppVersions, String> {
        if let Some(r) = &self.app_runtime {
            app.runtime = Some(
                al_version::Runtime::parse(r).map_err(|e| format!("version {}: app_runtime `{r}`: {e}", self.major))?,
            );
        }
        Ok(app)
    }

    /// The BC version of the oracle leg, with the build when it is known.
    pub fn bc(&self) -> BcVersion {
        let mut v = BcVersion::parse(&self.bc_version).unwrap_or_else(|_| BcVersion::new(self.major, 0));
        if let Some(b) = self.bc_build.as_deref().and_then(|b| BcVersion::parse(b).ok()) {
            v.build = b.build;
        }
        v
    }
}

impl OracleFile {
    /// The companies of `version`'s database: its own `companies`, else the top-level one, else one company.
    pub fn companies_of(&self, version: &VersionEntry) -> Result<al_host::Companies, String> {
        match version.companies.as_ref().or(self.companies.as_ref()) {
            Some(c) => c.companies(),
            None => Ok(al_host::Companies::One),
        }
    }

    pub fn parse(text: &str) -> Result<OracleFile, String> {
        let f: OracleFile = toml::from_str(text).map_err(|e| e.to_string())?;
        if let Some(c) = &f.companies {
            c.companies()?;
        }
        for v in &f.versions {
            if let Some(c) = &v.companies {
                c.companies().map_err(|e| format!("version {}: {e}", v.major))?;
            }
        }
        if f.versions.is_empty() {
            return Err("no [[version]] table".into());
        }
        let mut seen = std::collections::BTreeSet::new();
        for v in &f.versions {
            let leg = BcVersion::parse(&v.bc_version)
                .map_err(|e| format!("version {}: bc_version `{}`: {e}", v.major, v.bc_version))?;
            if leg.major != v.major {
                return Err(format!("bc_version `{}` is not in major {}", v.bc_version, v.major));
            }
            if !seen.insert(v.major) {
                return Err(format!("major {} is listed twice", v.major));
            }
            let mut names = std::collections::BTreeSet::new();
            for suite in &v.suites {
                if !names.insert(suite.name.as_str()) {
                    return Err(format!("version {}: suite `{}` is listed twice", v.major, suite.name));
                }
            }
        }
        Ok(f)
    }

    /// The entry for `--bc-version PREFIX`, or the newest entry without a flag.
    pub fn select(&self, requested: Option<&str>) -> Result<&VersionEntry, String> {
        let Some(prefix) = requested else {
            return Ok(self
                .versions
                .iter()
                .max_by_key(|v| v.major)
                .expect("parse checks for a table"));
        };
        let wanted = BcVersion::parse(prefix).map_err(|e| format!("--bc-version `{prefix}`: {e}"))?;
        if wanted.major < al_version::MIN_BC_MAJOR {
            return Err(format!(
                "BC {prefix} is not supported: BC {} and later only",
                al_version::MIN_BC_MAJOR
            ));
        }
        self.versions
            .iter()
            .filter(|v| {
                // A full build names the leg of its major and minor, whatever build the oracle pins: running the corpus
                // on another build is how a newly published one is tried (`main.rs`).
                let leg = BcVersion::parse(&v.bc_version).unwrap_or_else(|_| BcVersion::new(v.major, 0));
                v.bc().matches_prefix(prefix)
                    || (wanted.build.is_some() && (leg.major, leg.minor) == (wanted.major, wanted.minor))
            })
            .max_by_key(|v| v.major)
            .ok_or_else(|| {
                let mut known: Vec<_> = self.versions.iter().map(|v| (v.major, v.bc().to_string())).collect();
                known.sort();
                let known: Vec<_> = known.into_iter().map(|(_, s)| s).collect();
                format!("no oracle for BC {prefix}; known versions: {}", known.join(", "))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOML: &str = r#"
corpus_repo   = "corpus"
corpus_app    = "corpus/tests/al-language"
corpus_commit = "abc"

[[version]]
major = 27
bc_version = "27.5"
junit = "27/junit.xml"
baseline = "metrics/27/passing.txt"

[[version]]
major = 29
bc_version = "29.0"
bc_build = "29.0.54011.55816"
workflow_run = 1
artifact_id = 2
junit = "29/junit.xml"
baseline = "metrics/29/passing.txt"

[[version]]
major = 28
bc_version = "28.5"
junit = "28/junit.xml"
baseline = "metrics/28/passing.txt"
"#;

    fn file() -> OracleFile {
        OracleFile::parse(TOML).unwrap()
    }

    // ADR-046 decision 2: `companies` declares the companies of the oracle's database; a [[version]] table may override it.
    #[test]
    fn companies_are_one_by_default_a_list_or_all_and_a_version_may_override_them() {
        let f = file();
        assert_eq!(f.companies_of(&f.versions[0]).unwrap(), al_host::Companies::One);
        let t = format!("companies = [\"A\", \"B\"]\n{TOML}");
        let f = OracleFile::parse(&t).unwrap();
        assert_eq!(
            f.companies_of(&f.versions[0]).unwrap(),
            al_host::Companies::List(vec!["A".into(), "B".into()])
        );
        let t = format!(
            "companies = [\"A\"]\n{}",
            TOML.replacen("major = 28", "major = 28\ncompanies = \"all\"", 1)
        );
        let f = OracleFile::parse(&t).unwrap();
        let v28 = f.versions.iter().find(|v| v.major == 28).unwrap();
        assert_eq!(f.companies_of(v28).unwrap(), al_host::Companies::All);
        for bad in ["companies = \"some\"", "companies = []", "companies = [\"A\", \"A\"]"] {
            assert!(OracleFile::parse(&format!("{bad}\n{TOML}")).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_version_lists_its_other_test_apps() {
        let t = format!(
            r#"{TOML}
[[version.suite]]
name = "onprem"
app = "corpus/tests/al-language-onprem"
oracle = "28/junit-onprem.xml"
baseline = "metrics/28/passing-onprem.txt"

[[version.suite]]
name = "isolation-function"
app = "corpus/tests/al-language-isolation-probe"
format = "xunit"
runner = 61300
bc_version = "28.4"
corpus_commit = "old"
oracle = "28/xunit-isolation-function.xml"
baseline = "metrics/28/passing-isolation-function.txt"
"#
        );
        let f = OracleFile::parse(&t).unwrap();
        // The suites belong to the last [[version]] table, which is major 28 in TOML.
        let v = f.versions.iter().find(|v| v.major == 28).unwrap();
        assert_eq!(v.suites.len(), 2);
        assert_eq!(v.suites[0].format, OracleFormat::Junit);
        assert_eq!(v.suites[0].runner, None);
        assert_eq!(v.suites[1].format, OracleFormat::Xunit);
        assert_eq!(v.suites[1].runner, Some(61300));
        assert_eq!(v.suites[1].corpus_commit.as_deref(), Some("old"));
        assert!(f.versions.iter().find(|v| v.major == 27).unwrap().suites.is_empty());
    }

    // The corpus CI installs apps no test app declares for some suites (corpus .github/workflows/ci.yml, job
    // test-testlibraries, `extra_apps`): such a suite names them, and runs in an environment of its own.
    #[test]
    fn a_suite_names_the_apps_its_environment_adds() {
        let t = format!(
            r#"{TOML}
[[version.suite]]
name = "onprem"
app = "corpus/tests/al-language-onprem"
oracle = "28/junit-onprem.xml"
baseline = "metrics/28/passing-onprem.txt"

[[version.suite]]
name = "testlibraries"
app = "corpus/tests/al-language-testlibraries"
oracle = "28/junit-testlibraries.xml"
baseline = "metrics/28/passing-testlibraries.txt"
extra_apps = [
  {{ id = "5d86850b-0d76-4eca-bd7b-951ad998e997", name = "Tests-TestLibraries" }},
  {{ id = "7819d79d-feea-4f09-bbed-5bbaca4bf323", name = "Data Archive" }},
]
"#
        );
        let f = OracleFile::parse(&t).unwrap();
        let v = f.versions.iter().find(|v| v.major == 28).unwrap();
        assert!(v.suites[0].extra_apps.is_empty());
        let names: Vec<_> = v.suites[1].extra_apps.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["Tests-TestLibraries", "Data Archive"]);
        assert_eq!(v.suites[1].extra_apps[1].id, "7819d79d-feea-4f09-bbed-5bbaca4bf323");
        // An app without its id cannot be looked up: the dependency apps are found by id.
        let no_id = t.replace(r#"id = "7819d79d-feea-4f09-bbed-5bbaca4bf323", "#, "");
        assert!(OracleFile::parse(&no_id).is_err());
    }

    #[test]
    fn a_suite_name_is_unique_within_its_version() {
        let t = format!(
            "{TOML}\n[[version.suite]]\nname = \"a\"\napp = \"x\"\noracle = \"o\"\nbaseline = \"b\"\n[[version.suite]]\nname = \"a\"\napp = \"y\"\noracle = \"o\"\nbaseline = \"b\"\n"
        );
        assert!(OracleFile::parse(&t).unwrap_err().contains("listed twice"));
    }

    #[test]
    fn parses_every_version_table() {
        let f = file();
        assert_eq!(f.versions.len(), 3);
        assert_eq!(f.versions[1].artifact_id, Some(2));
        assert_eq!(f.versions[0].bc_build, None);
    }

    #[test]
    fn a_version_may_record_an_older_corpus_commit() {
        let t = TOML.replace(
            "bc_build = \"29.0.54011.55816\"",
            "bc_build = \"29.0.54011.55816\"\ncorpus_commit = \"old\"",
        );
        let f = OracleFile::parse(&t).unwrap();
        assert_eq!(f.versions[1].oracle_commit("abc"), "old");
        assert_eq!(f.versions[0].oracle_commit("abc"), "abc");
    }

    #[test]
    fn no_flag_selects_the_newest_version() {
        assert_eq!(file().select(None).unwrap().major, 29);
    }

    #[test]
    fn a_prefix_selects_by_whole_segments() {
        let f = file();
        assert_eq!(f.select(Some("28")).unwrap().major, 28);
        assert_eq!(f.select(Some("27.5")).unwrap().major, 27);
        assert_eq!(f.select(Some("29.0.54011")).unwrap().major, 29);
    }

    #[test]
    fn an_unknown_prefix_lists_the_known_versions() {
        let e = file().select(Some("30")).unwrap_err();
        assert!(e.contains("no oracle for BC 30"), "{e}");
        assert!(e.contains("27.5") && e.contains("28.5") && e.contains("29.0"), "{e}");
        assert!(file().select(Some("27.50")).is_err());
        assert!(file().select(Some("27.0")).is_err(), "the oracle is the 27.5 leg");
    }

    #[test]
    fn a_version_below_27_is_not_supported() {
        let e = file().select(Some("26")).unwrap_err();
        assert!(e.contains("27 and later only"), "{e}");
    }

    #[test]
    fn a_garbage_flag_is_an_error() {
        assert!(file().select(Some("abc")).is_err());
    }

    /// A newly published Microsoft build must not change the corpus results unseen: CI and a local run load the build the
    /// oracle pins.
    #[test]
    fn the_committed_oracle_pins_a_microsoft_build_for_every_major() {
        let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../oracle/oracle.toml")).unwrap();
        let f = OracleFile::parse(&text).unwrap();
        for v in &f.versions {
            let build = v
                .bc()
                .build
                .unwrap_or_else(|| panic!("BC {} has no `bc_build`", v.major));
            assert!(build.starts_with(&format!("{}.", v.major)), "{build}");
            assert_eq!(build.split('.').count(), 4, "{build}");
        }
    }

    #[test]
    fn the_leg_build_is_kept_when_recorded() {
        let f = file();
        assert_eq!(f.versions[1].bc().build.as_deref(), Some("29.0.54011.55816"));
        assert_eq!(f.versions[0].bc().to_string(), "27.5");
    }

    #[test]
    fn rejects_duplicate_majors_and_a_major_that_disagrees_with_the_leg() {
        let dup = TOML.replace("major = 28", "major = 27").replace("28.5", "27.4");
        assert!(OracleFile::parse(&dup).unwrap_err().contains("twice"));
        let bad = TOML.replace("bc_version = \"28.5\"", "bc_version = \"29.5\"");
        assert!(OracleFile::parse(&bad).unwrap_err().contains("28"));
        let none = "corpus_repo=\"c\"\ncorpus_app=\"a\"\ncorpus_commit=\"x\"\n";
        assert!(OracleFile::parse(none).is_err());
    }

    #[test]
    fn app_runtime_replaces_the_runtime_of_the_corpus_app() {
        let t = TOML.replace("bc_version = \"27.5\"", "bc_version = \"27.5\"\napp_runtime = \"16.1\"");
        let f = OracleFile::parse(&t).unwrap();
        let v = f.select(Some("27")).unwrap();
        // The corpus app.json declares 18.0; BC 27.5 refuses that, so the leg's runtime wins.
        let declared = al_version::AppVersions::parse(Some("18.0"), None, None).unwrap();
        let app = v.app_versions_for_leg(declared.clone()).unwrap();
        assert_eq!(app.runtime, Some(al_version::Runtime { major: 16, minor: 1 }));
        let c = al_version::choose_target(Some(&v.bc()), &app, &v.bc()).unwrap();
        assert_eq!(c.target.runtime.to_string(), "16.1");
        // Without app_runtime the app's own runtime stays (and BC 28 refuses nothing at 17.0).
        let v28 = f.select(Some("28")).unwrap();
        assert_eq!(v28.app_versions_for_leg(declared.clone()).unwrap(), declared);
        assert!(
            OracleFile::parse(&TOML.replace("bc_version = \"27.5\"", "bc_version = \"27.5\"\napp_runtime = \"x\""))
                .unwrap()
                .select(Some("27"))
                .unwrap()
                .app_versions_for_leg(declared)
                .is_err()
        );
    }

    /// Run 39110861115 published the corpus apps with these runtimes on the three legs.
    #[test]
    fn the_real_oracle_toml_records_the_runtime_per_major() {
        let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../oracle/oracle.toml")).unwrap();
        let f = OracleFile::parse(&text).unwrap();
        let got: Vec<(u16, Option<&str>)> = f.versions.iter().map(|v| (v.major, v.app_runtime.as_deref())).collect();
        for (major, want) in [(27, "16.1"), (28, "17.1"), (29, "18.0")] {
            assert!(got.contains(&(major, Some(want))), "{got:?}");
        }
    }
}
