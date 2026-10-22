//! Microsoft's BaseApp test recipe: which company a test runs against (the "lane"), read from BCApps' own settings.
//!
//! BCApps (MIT) decides the company per run, not per test: `build/projects/Test Apps W1/.AL-Go/settings.json` lists the
//! build modes and, for each, a `testType` and a `companyName`; `build/scripts/TestConfiguration.json` lists the test apps
//! of the two Legacy buckets. A test app in a Legacy list runs in the Legacy lane. Every other test app runs once per
//! other lane, and a lane runs the codeunits whose AL property `TestType` is that lane's type (no property: UnitTest).
//! This module only reads those files and plans the lanes. It has no OS access beyond reading the files it is given.
//! Nothing here is Microsoft's data: the files stay in the cache (`bcapps.rs`).

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

/// The company a group of test codeunits runs against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Lane {
    /// The old NAV DemoTool company, the Extended RapidStart package of the artifact (`--company-data package`).
    Legacy,
    /// An empty company: no demo data.
    UnitTest,
    /// Contoso Demo Tool, setup data only.
    Integration,
    /// Contoso Demo Tool, all demo data.
    Uncategorized,
}

impl Lane {
    /// The name BCApps uses for the lane's `testType`.
    pub fn test_type(self) -> &'static str {
        match self {
            Lane::Legacy => "Legacy",
            Lane::UnitTest => "UnitTest",
            Lane::Integration => "IntegrationTest",
            Lane::Uncategorized => "Uncategorized",
        }
    }

    fn from_test_type(s: &str) -> Option<Lane> {
        [Lane::Legacy, Lane::UnitTest, Lane::Integration, Lane::Uncategorized]
            .into_iter()
            .find(|l| l.test_type().eq_ignore_ascii_case(s))
    }

    /// The lane of a test codeunit's `TestType` property. `None`: the type belongs to no lane (`AITest`, or a word
    /// BCApps does not run), so the codeunit is not run.
    ///
    /// No property is UnitTest: in BCApps releases/29.x, run 39519586834, the Default job (UnitTest) ran 242 codeunits
    /// of non-Legacy apps that set no `TestType` (157 of them from System Application Test), and the
    /// UncategorizedTests job ran only codeunits that say `TestType = Uncategorized`.
    pub fn of_property(property: Option<&str>) -> Option<Lane> {
        match property {
            None => Some(Lane::UnitTest),
            Some(p) if p.eq_ignore_ascii_case("Uncategorized") => Some(Lane::Uncategorized),
            Some(p) if p.eq_ignore_ascii_case("UnitTest") => Some(Lane::UnitTest),
            Some(p) if p.eq_ignore_ascii_case("IntegrationTest") => Some(Lane::Integration),
            Some(_) => None,
        }
    }
}

/// One build mode of the Test Apps project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaneDef {
    pub lane: Lane,
    /// `UnitTest` for the default (no build mode), else the build mode (`IntegrationTests`, `LegacyTestsBucket1`).
    pub build_mode: String,
    /// BCApps' `companyName`.
    pub company_name: String,
    /// The Legacy bucket number, when the mode has one.
    pub bucket: Option<u32>,
}

/// What the recipe files say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recipe {
    pub lanes: Vec<LaneDef>,
    /// Test app names of the Legacy buckets (lower case) and the bucket each one is in.
    legacy: BTreeMap<String, u32>,
    /// Where the files were read (the BCApps checkout).
    pub root: PathBuf,
}

/// The project folder of a country's tests: `Test Apps W1`, `Test Apps US`.
fn project_folder(country: &str) -> String {
    format!("Test Apps {}", country.to_ascii_uppercase())
}

impl Recipe {
    /// Reads the recipe from a BCApps checkout (or the sparse part the cache holds).
    pub fn load(root: &Path, country: &str) -> Result<Recipe, String> {
        let settings_path = root
            .join("build")
            .join("projects")
            .join(project_folder(country))
            .join(".AL-Go")
            .join("settings.json");
        let config_path = root.join("build").join("scripts").join("TestConfiguration.json");
        let read = |p: &Path| std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()));
        let mut r = Recipe::parse(&read(&settings_path)?, &read(&config_path)?)
            .map_err(|e| format!("{}: {e}", settings_path.display()))?;
        r.root = root.to_path_buf();
        Ok(r)
    }

    /// The disabled-tests folder of the checkout.
    pub fn disabled_tests_dir(&self) -> PathBuf {
        self.root.join("src").join("DisabledTests")
    }

    /// Reads the text of `settings.json` of a Test Apps project and of `TestConfiguration.json`.
    pub fn parse(settings: &str, config: &str) -> Result<Recipe, String> {
        let s: serde_json::Value = serde_json::from_str(settings.trim_start_matches('\u{feff}'))
            .map_err(|e| format!("settings.json is not JSON: {e}"))?;
        let text = |v: &serde_json::Value, k: &str| v.get(k).and_then(|x| x.as_str()).map(str::to_string);
        let lane_of = |ty: &str| {
            Lane::from_test_type(ty).ok_or_else(|| {
                format!("testType `{ty}` is not one of Legacy, UnitTest, IntegrationTest, Uncategorized")
            })
        };
        let mut lanes = vec![LaneDef {
            lane: lane_of(&text(&s, "testType").ok_or("settings.json has no testType")?)?,
            build_mode: "UnitTest".into(),
            company_name: text(&s, "companyName").ok_or("settings.json has no companyName")?,
            bucket: None,
        }];
        let modes: Vec<String> = s
            .get("buildModes")
            .and_then(|m| m.as_array())
            .map(|a| a.iter().filter_map(|m| m.as_str().map(str::to_string)).collect())
            .unwrap_or_default();
        for mode in &modes {
            // The ConditionalSettings entry for exactly this mode.
            let entry = s
                .get("ConditionalSettings")
                .and_then(|c| c.as_array())
                .and_then(|c| {
                    c.iter().find(|e| {
                        e.get("buildModes")
                            .and_then(|b| b.as_array())
                            .is_some_and(|b| b.iter().any(|m| m.as_str() == Some(mode)))
                    })
                })
                .and_then(|e| e.get("settings"))
                .ok_or_else(|| format!("build mode {mode} has no ConditionalSettings entry"))?;
            lanes.push(LaneDef {
                lane: lane_of(&text(entry, "testType").ok_or_else(|| format!("build mode {mode} sets no testType"))?)?,
                build_mode: mode.clone(),
                company_name: text(entry, "companyName")
                    .ok_or_else(|| format!("build mode {mode} sets no companyName"))?,
                bucket: entry.get("bucketNumber").and_then(|b| b.as_u64()).map(|b| b as u32),
            });
        }
        let c: serde_json::Value = serde_json::from_str(config.trim_start_matches('\u{feff}'))
            .map_err(|e| format!("TestConfiguration.json is not JSON: {e}"))?;
        let mut legacy = BTreeMap::new();
        for (key, apps) in c.as_object().ok_or("TestConfiguration.json is not an object")? {
            let Some(n) = key
                .strip_prefix("LegacyTests-Bucket")
                .and_then(|n| n.parse::<u32>().ok())
            else {
                continue;
            };
            for app in apps.as_array().into_iter().flatten().filter_map(|a| a.as_str()) {
                legacy.insert(app.to_lowercase(), n);
            }
        }
        if legacy.is_empty() {
            return Err("TestConfiguration.json lists no LegacyTests-Bucket".into());
        }
        Ok(Recipe {
            lanes,
            legacy,
            root: PathBuf::new(),
        })
    }

    /// The Legacy bucket a test app is in, if any.
    pub fn legacy_bucket(&self, app_name: &str) -> Option<u32> {
        self.legacy.get(&app_name.to_lowercase()).copied()
    }

    /// The company name BCApps gives a lane.
    pub fn company_name(&self, lane: Lane) -> Option<&str> {
        self.lanes
            .iter()
            .find(|l| l.lane == lane)
            .map(|l| l.company_name.as_str())
    }

    /// The build mode of Microsoft's CI that runs a lane of `app_name`, as its job is named (`Default`,
    /// `IntegrationTests`, `UncategorizedTests`, `LegacyTestsBucket1`): a Legacy app's bucket picks the Legacy mode.
    pub fn build_mode(&self, app_name: &str, lane: Lane) -> String {
        let bucket = if lane == Lane::Legacy {
            self.legacy_bucket(app_name)
        } else {
            None
        };
        self.lanes
            .iter()
            .find(|l| l.lane == lane && (bucket.is_none() || l.bucket == bucket))
            .map_or_else(
                || lane.test_type().to_string(),
                // The first entry is the project's own settings, the job without a build mode.
                |l| {
                    if std::ptr::eq(l, &self.lanes[0]) {
                        "Default".to_string()
                    } else {
                        l.build_mode.clone()
                    }
                },
            )
    }

    /// The lane of one test codeunit: every codeunit of a Legacy app is Legacy; otherwise its `TestType` decides.
    pub fn lane_of(&self, app_name: &str, test_type: Option<&str>) -> Option<Lane> {
        if self.legacy_bucket(app_name).is_some() {
            return Some(Lane::Legacy);
        }
        Lane::of_property(test_type)
    }
}

/// The test codeunits of one app, with the `TestType` property of each.
#[derive(Debug, Clone)]
pub struct AppUnits {
    pub app_name: String,
    pub codeunits: Vec<(u32, Option<String>)>,
    /// The codeunits that set `RequiredTestIsolation = Disabled`.
    pub isolation_disabled: Vec<u32>,
}

/// The codeunits one lane runs, and those the recipe does not run at all.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Plan {
    pub lanes: BTreeMap<Lane, Vec<u32>>,
    /// Codeunits whose `TestType` is in no lane (`AITest`): BCApps does not run them in these builds.
    pub unrun: Vec<u32>,
}

/// Sorts every codeunit into its lane.
pub fn plan(recipe: &Recipe, apps: &[AppUnits]) -> Plan {
    let mut p = Plan::default();
    let mut seen: HashSet<u32> = HashSet::new();
    for app in apps {
        for (id, ty) in &app.codeunits {
            if !seen.insert(*id) {
                continue;
            }
            // `RequiredTestIsolation = Disabled` runs only in the UnitTest lane: BCApps
            // `build/scripts/RunTestsInBcContainer.ps1` (releases/29.0) runs a second pass with
            // `requiredTestIsolation = Disabled` for `UnitTest` only, and in run 39519586834 the IntegrationTests job ran
            // none of its 13 such codeunits while the Default job ran 27.
            let lane = recipe
                .lane_of(&app.app_name, ty.as_deref())
                .filter(|l| *l == Lane::UnitTest || !app.isolation_disabled.contains(id));
            match lane {
                Some(lane) => p.lanes.entry(lane).or_default().push(*id),
                None => p.unrun.push(*id),
            }
        }
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    const SETTINGS: &str = r#"{
      "testType": "UnitTest", "companyName": "Empty Company",
      "buildModes": ["IntegrationTests", "UncategorizedTests", "LegacyTestsBucket1", "LegacyTestsBucket2"],
      "ConditionalSettings": [
        {"buildModes": ["IntegrationTests"], "settings": {"testType": "IntegrationTest", "companyName": "My Company"}},
        {"buildModes": ["UncategorizedTests"], "settings": {"testType": "Uncategorized", "companyName": "CRONUS International Ltd."}},
        {"buildModes": ["LegacyTestsBucket1"], "settings": {"testType": "Legacy", "bucketNumber": 1, "companyName": "CRONUS International Ltd."}},
        {"buildModes": ["LegacyTestsBucket2"], "settings": {"testType": "Legacy", "bucketNumber": 2, "companyName": "CRONUS International Ltd."}}
      ]}"#;
    const CONFIG: &str = r#"{
      "LegacyTests-Bucket1": ["Tests-ERM", "Tests-Cost Accounting"],
      "LegacyTests-Bucket2": ["Tests-SCM"],
      "Other": ["Not-A-Bucket"] }"#;

    fn recipe() -> Recipe {
        Recipe::parse(SETTINGS, CONFIG).unwrap()
    }

    #[test]
    fn the_build_modes_give_the_four_lanes_and_their_companies() {
        let r = recipe();
        let got: Vec<(Lane, &str, &str, Option<u32>)> = r
            .lanes
            .iter()
            .map(|l| (l.lane, l.build_mode.as_str(), l.company_name.as_str(), l.bucket))
            .collect();
        assert_eq!(
            got,
            vec![
                (Lane::UnitTest, "UnitTest", "Empty Company", None),
                (Lane::Integration, "IntegrationTests", "My Company", None),
                (
                    Lane::Uncategorized,
                    "UncategorizedTests",
                    "CRONUS International Ltd.",
                    None
                ),
                (Lane::Legacy, "LegacyTestsBucket1", "CRONUS International Ltd.", Some(1)),
                (Lane::Legacy, "LegacyTestsBucket2", "CRONUS International Ltd.", Some(2)),
            ]
        );
        assert_eq!(r.company_name(Lane::Integration), Some("My Company"));
    }

    #[test]
    fn a_lane_is_named_by_the_build_mode_of_microsofts_job() {
        let r = recipe();
        assert_eq!(r.build_mode("Tests-Misc", Lane::UnitTest), "Default");
        assert_eq!(r.build_mode("Tests-Misc", Lane::Integration), "IntegrationTests");
        assert_eq!(r.build_mode("Tests-Misc", Lane::Uncategorized), "UncategorizedTests");
        assert_eq!(r.build_mode("tests-erm", Lane::Legacy), "LegacyTestsBucket1");
        assert_eq!(r.build_mode("Tests-SCM", Lane::Legacy), "LegacyTestsBucket2");
    }

    #[test]
    fn a_legacy_app_runs_every_codeunit_in_the_legacy_lane() {
        let r = recipe();
        assert_eq!(r.legacy_bucket("tests-erm"), Some(1));
        assert_eq!(r.legacy_bucket("Tests-SCM"), Some(2));
        assert_eq!(r.legacy_bucket("Not-A-Bucket"), None);
        // The property of a codeunit in a Legacy app does not matter.
        assert_eq!(r.lane_of("Tests-ERM", Some("UnitTest")), Some(Lane::Legacy));
        assert_eq!(r.lane_of("Tests-ERM", None), Some(Lane::Legacy));
    }

    #[test]
    fn another_app_is_sorted_by_the_test_type_of_each_codeunit() {
        let r = recipe();
        assert_eq!(r.lane_of("Tests-New", Some("UnitTest")), Some(Lane::UnitTest));
        assert_eq!(r.lane_of("Tests-New", Some("integrationtest")), Some(Lane::Integration));
        assert_eq!(r.lane_of("Tests-New", Some("Uncategorized")), Some(Lane::Uncategorized));
        // No property: UnitTest, the Default job (BCApps releases/29.x, run 39519586834: the Default job ran 242
        // codeunits without a TestType, the UncategorizedTests job none).
        assert_eq!(r.lane_of("Tests-New", None), Some(Lane::UnitTest));
        // AITest is in none of the four build modes.
        assert_eq!(r.lane_of("Tests-New", Some("AITest")), None);
    }

    #[test]
    fn a_codeunit_that_requires_isolation_disabled_runs_only_in_the_unit_test_lane() {
        // BCApps `build/scripts/RunTestsInBcContainer.ps1` (releases/29.0): only for `UnitTest` a second pass runs with
        // `requiredTestIsolation = Disabled` (runner 130451). Run 39519586834: the Default job ran 27 such codeunits,
        // the IntegrationTests job none of the 13 it has (`_Exclude_APIV1_ Tests` 139700, `TestType = IntegrationTest`).
        let r = recipe();
        let apps = vec![AppUnits {
            app_name: "Tests-New".into(),
            codeunits: vec![
                (1, Some("UnitTest".into())),
                (2, None),
                (3, Some("IntegrationTest".into())),
                (4, Some("Uncategorized".into())),
            ],
            isolation_disabled: vec![1, 2, 3, 4],
        }];
        let p = plan(&r, &apps);
        assert_eq!(p.lanes[&Lane::UnitTest], vec![1, 2]);
        assert!(!p.lanes.contains_key(&Lane::Integration));
        assert!(!p.lanes.contains_key(&Lane::Uncategorized));
        assert_eq!(p.unrun, vec![3, 4]);
    }

    #[test]
    fn the_plan_puts_each_codeunit_in_one_lane() {
        let r = recipe();
        let apps = vec![
            AppUnits {
                app_name: "Tests-ERM".into(),
                codeunits: vec![(1, None), (2, Some("UnitTest".into()))],
                isolation_disabled: vec![],
            },
            AppUnits {
                app_name: "Tests-New".into(),
                codeunits: vec![
                    (3, Some("UnitTest".into())),
                    (4, Some("IntegrationTest".into())),
                    (5, None),
                    (6, Some("AITest".into())),
                ],
                isolation_disabled: vec![],
            },
        ];
        let p = plan(&r, &apps);
        assert_eq!(p.lanes[&Lane::Legacy], vec![1, 2]);
        assert_eq!(p.lanes[&Lane::UnitTest], vec![3, 5]);
        assert_eq!(p.lanes[&Lane::Integration], vec![4]);
        assert!(!p.lanes.contains_key(&Lane::Uncategorized));
        assert_eq!(p.unrun, vec![6]);
    }

    #[test]
    fn a_settings_file_that_does_not_say_what_it_must_is_refused_with_the_reason() {
        assert!(Recipe::parse("{}", CONFIG).unwrap_err().contains("testType"));
        let bad = SETTINGS.replace("\"IntegrationTest\"", "\"Smoke\"");
        assert!(Recipe::parse(&bad, CONFIG).unwrap_err().contains("Smoke"));
        assert!(Recipe::parse(SETTINGS, "{}").unwrap_err().contains("Bucket"));
        assert!(Recipe::parse("nope", CONFIG).unwrap_err().contains("not JSON"));
    }

    #[test]
    fn load_reads_the_files_of_a_checkout() {
        let dir = tempfile::tempdir().unwrap();
        let proj = dir.path().join("build/projects/Test Apps W1/.AL-Go");
        std::fs::create_dir_all(&proj).unwrap();
        std::fs::create_dir_all(dir.path().join("build/scripts")).unwrap();
        std::fs::write(proj.join("settings.json"), SETTINGS).unwrap();
        std::fs::write(dir.path().join("build/scripts/TestConfiguration.json"), CONFIG).unwrap();
        let r = Recipe::load(dir.path(), "w1").unwrap();
        assert_eq!(r.lanes.len(), 5);
        assert_eq!(r.disabled_tests_dir(), dir.path().join("src/DisabledTests"));
        assert!(Recipe::load(dir.path(), "us").unwrap_err().contains("Test Apps US"));
    }
}
