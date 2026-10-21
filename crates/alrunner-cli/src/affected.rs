//! `al-runner test --affected` (ADR-050): the command line's side of affected test selection. The rule and the stored
//! result are `al_testrun::affected` (pure); here the stored result is read and written (gzip JSON, a temporary file
//! and a rename), the header of this run is made from its options and apps, the narrow callback picks the test
//! codeunits once the compile is done, and the report says what was run and what was skipped.
//!
//! A skipped test is never reported as passed or failed in this run: it is counted in `selection`, shown in JUnit as
//! `<skipped>` with its stored outcome, and one whose stored outcome is not a pass keeps the exit code at 1.

use crate::apps::AppInput;
use crate::args::TestArgs;
use al_testrun::Outcome;
use al_testrun::affected::{Current, Ended, Header, ObjectFacts, Selection, Stored, StoredCodeunit};
use al_testrun::runtime::{Narrow, NarrowInput, TestResult};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// The reason of the full run when there is no stored result (ADR-050 test plan 1).
pub const NO_STORED_RESULT: &str = "no stored result";

/// At most this many changed objects are listed (decision 11); `changedObjectsTruncated` counts the rest.
const CHANGED_SHOWN: usize = 50;

/// What the narrow callback decided, for the report and the stored result after the run.
struct Decided {
    selection: Selection,
    objects: Vec<ObjectFacts>,
    /// Every test codeunit of the run, as (app id in lower case, codeunit id).
    codeunits: Vec<(String, u32)>,
    /// The caller's app index of each workspace app id.
    caller: BTreeMap<String, usize>,
}

/// An `--affected` run: prepared before the run, finished after it.
pub struct Affected {
    store: PathBuf,
    header: Header,
    stored: Result<Stored, String>,
    include_failing: bool,
    /// The number of workspace apps: the caller's app indexes below it are theirs.
    workspace: usize,
    decided: Arc<Mutex<Option<Decided>>>,
}

/// One test of a skipped codeunit, with its stored outcome.
#[derive(Debug, Clone)]
pub struct Skipped {
    pub app: usize,
    pub codeunit: u32,
    pub codeunit_name: String,
    pub test: String,
    pub outcome: Outcome,
}

/// What an `--affected` run reports about its selection.
#[derive(Debug, Clone)]
pub struct Report {
    pub selection: Selection,
    /// The tests that ran.
    pub ran: usize,
    /// The tests of the codeunits that did not run, in app, codeunit and stored order.
    pub skipped: Vec<Skipped>,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn sha(text: &[u8]) -> String {
    hex(&Sha256::digest(text))
}

/// The files of a folder (or one file), with their names relative to it, in path order.
fn hash_tree(path: &Path, rel: &str, h: &mut Sha256) -> Result<(), String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if meta.is_dir() {
        let mut kids: Vec<PathBuf> = std::fs::read_dir(path)
            .map_err(|e| format!("{}: {e}", path.display()))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .collect();
        kids.sort();
        for k in kids {
            let name = k
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            hash_tree(&k, &format!("{rel}/{name}"), h)?;
        }
    } else {
        h.update(rel.as_bytes());
        h.update([0]);
        h.update(std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?);
        h.update([0]);
    }
    Ok(())
}

/// The workspace apps as (id in lower case, version), in the order of the run.
fn workspace_apps(inputs: &[AppInput]) -> Vec<(String, String)> {
    inputs
        .iter()
        .map(|i| (i.source.id.to_lowercase(), i.json.version.to_string()))
        .collect()
}

/// The hash of the run options that change outcomes (the set the impact index hashes, ADR-043, with `--tdd`). Paths
/// count by their file name only, so the hash does not depend on where a checkout is (decision 10).
fn options_hash(a: &TestArgs) -> Result<String, String> {
    let name = |p: &Option<PathBuf>| {
        p.as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
    };
    let text = format!(
        "{:?}",
        (
            a.country(),
            &a.bc_version,
            (
                a.wants_test_data(),
                a.no_test_data,
                name(&a.test_data_path),
                &a.test_data_company,
                a.company_package,
                name(&a.package_path),
                &a.company_name,
            ),
            a.isolation,
            &a.defines,
            a.environment.as_ref().map(std::mem::discriminant),
            (&a.server_url, a.test_timeout_secs),
            a.tdd,
        )
    );
    let mut h = Sha256::new();
    h.update(text.as_bytes());
    if let Some(dir) = &a.disabled_tests_dir {
        hash_tree(dir, "", &mut h).map_err(|e| format!("--disabled-tests {e}"))?;
    }
    Ok(hex(&h.finalize()))
}

/// The hash of each workspace app's `app.json` (the file of an app folder; the manifest read from a package).
fn manifests(inputs: &[AppInput]) -> Vec<String> {
    inputs
        .iter()
        .map(|i| match std::fs::read(i.path.join("app.json")) {
            Ok(bytes) => sha(&bytes),
            Err(_) => sha(format!("{:?} {:?} {:?}", i.json, i.manifest, i.symbols).as_bytes()),
        })
        .collect()
}

/// The environment: each dependency app's id, version and content hash, and the BC build, sorted (decision 7).
fn environment(dependencies: &al_testrun::deps::Dependencies, bc_build: Option<&str>) -> Vec<String> {
    let mut out: Vec<String> = dependencies
        .apps
        .iter()
        .enumerate()
        .map(|(i, app)| {
            let version = dependencies.manifests.get(i).map(|m| m.version.as_str()).unwrap_or("");
            let content = match dependencies.lazy.get(i).and_then(Option::as_ref) {
                Some(l) => hex(&l.identity),
                None => {
                    let mut h = Sha256::new();
                    let mut files: Vec<&(String, String)> = app.files.iter().collect();
                    files.sort();
                    for (path, text) in files {
                        h.update(path.as_bytes());
                        h.update([0]);
                        h.update(text.as_bytes());
                        h.update([0]);
                    }
                    hex(&h.finalize())
                }
            };
            format!("{} {version} {content}", app.id.to_lowercase())
        })
        .collect();
    if let Some(b) = bc_build {
        out.push(format!("bc {b}"));
    }
    out.sort();
    out
}

/// Where the stored result of the run is: `--affected-store`, else `<cache dir>/affected/<key>.json.gz` with the key
/// made of the workspace apps' ids and versions (never their paths).
pub fn store_path(a: &TestArgs, inputs: &[AppInput]) -> PathBuf {
    if let Some(p) = &a.affected_store {
        return p.clone();
    }
    let mut apps = workspace_apps(inputs);
    apps.sort();
    let key = sha(format!("{apps:?}").as_bytes());
    al_deps::cache::cache_root_from_env(a.cache_dir.as_deref())
        .join("affected")
        .join(format!("{}.json.gz", &key[..16]))
}

/// The stored result at `path`; an error is the reason of the full run.
fn read(path: &Path) -> Result<Stored, String> {
    let Ok(bytes) = std::fs::read(path) else {
        return Err(NO_STORED_RESULT.into());
    };
    let mut text = String::new();
    flate2::read::GzDecoder::new(&bytes[..])
        .read_to_string(&mut text)
        .map_err(|e| format!("the stored result is not readable: {e}"))?;
    let v: Value = serde_json::from_str(&text).map_err(|e| format!("the stored result is not JSON: {e}"))?;
    Stored::from_json(&v)
}

/// Writes `stored` at `path` by a temporary file and a rename: the last writer wins.
fn write(path: &Path, stored: &Stored) -> Result<(), String> {
    let dir = path
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gz.write_all(stored.to_json().to_string().as_bytes())
        .and_then(|()| gz.finish())
        .map_err(|e| e.to_string())
        .and_then(|bytes| {
            let tmp = dir.join(format!(".affected.tmp-{}", std::process::id()));
            let written = std::fs::write(&tmp, bytes).and_then(|()| std::fs::rename(&tmp, path));
            if written.is_err() {
                let _ = std::fs::remove_file(&tmp);
            }
            written.map_err(|e| e.to_string())
        })
        .map_err(|e| format!("{}: {e}", path.display()))
}

impl Affected {
    /// The header of this run and the stored result of the last one.
    pub fn prepare(
        a: &TestArgs,
        inputs: &[AppInput],
        dependencies: &al_testrun::deps::Dependencies,
        bc_build: Option<&str>,
    ) -> Result<Affected, String> {
        let header = Header {
            runtime: hex(&al_testrun::compile_cache::runtime_build()),
            options: options_hash(a)?,
            environment: environment(dependencies, bc_build),
            apps: workspace_apps(inputs),
            manifests: manifests(inputs),
        };
        let store = store_path(a, inputs);
        Ok(Affected {
            stored: read(&store),
            store,
            header,
            include_failing: a.include_failing,
            workspace: inputs.len(),
            decided: Arc::default(),
        })
    }

    /// The callback that picks the test codeunits once the compile is done (`RunOptions::narrow`): `None` runs all.
    pub fn narrow(&self) -> Narrow {
        let decided = self.decided.clone();
        let header = self.header.clone();
        let stored = self.stored.clone();
        let (include_failing, workspace) = (self.include_failing, self.workspace);
        Narrow(Arc::new(move |input: &NarrowInput| {
            let program = input.program;
            let mut caller: BTreeMap<String, usize> = BTreeMap::new();
            let mut by_key: BTreeMap<(String, u32), (usize, u32)> = BTreeMap::new();
            for o in &program.objects {
                let (Some(&app), Some(id)) = (input.order.get(o.app as usize), o.id) else {
                    continue;
                };
                if app >= workspace
                    || !al_testrun::impact_static::is_test_codeunit(o)
                    || input.only.is_some_and(|only| !only.contains(&(app, id)))
                {
                    continue;
                }
                let app_id = program.apps[o.app as usize].id.to_lowercase();
                caller.insert(app_id.clone(), app);
                by_key.insert((app_id, id), (app, id));
            }
            let codeunits: Vec<(String, u32)> = by_key.keys().cloned().collect();
            let objects = al_testrun::affected::facts(input.lowered);
            let selection = match &stored {
                Ok(s) => s.select(&Current {
                    header: &header,
                    objects: &objects,
                    codeunits: &codeunits,
                    include_failing,
                    stubbed: &s.stubbed(),
                }),
                Err(reason) => Selection {
                    full: Some(reason.clone()),
                    run: codeunits.iter().cloned().collect(),
                    changed: Vec::new(),
                },
            };
            let only = selection
                .full
                .is_none()
                .then(|| selection.run.iter().filter_map(|k| by_key.get(k).copied()).collect());
            *decided.lock().expect("the decision") = Some(Decided {
                selection,
                objects,
                codeunits,
                caller,
            });
            only
        }))
    }

    /// After the run: the stored result for the next run is written (a warning when it cannot be), and the report
    /// is made. `None` when the run stopped before its compile was done. `impact_keys` are the keys
    /// `TestResult::impact` numbers.
    pub fn finish(self, results: &[TestResult], impact_keys: &[String]) -> Option<Report> {
        let d = self.decided.lock().expect("the decision").take()?;
        let id_of: BTreeMap<usize, &String> = d.caller.iter().map(|(id, &app)| (app, id)).collect();
        let mut ran: BTreeMap<(String, u32), StoredCodeunit> = BTreeMap::new();
        let mut ran_tests = 0;
        for r in results {
            let Some(app_id) = id_of.get(&r.app) else {
                continue;
            };
            ran_tests += 1;
            let c = ran
                .entry(((*app_id).clone(), r.codeunit))
                .or_insert_with(|| StoredCodeunit {
                    app: (*app_id).clone(),
                    id: r.codeunit,
                    name: r.codeunit_name.clone(),
                    ..Default::default()
                });
            c.keys
                .extend(r.impact.iter().filter_map(|&k| impact_keys.get(k as usize).cloned()));
            c.entered.extend(r.entered.iter().cloned());
            c.dotnet |= r.dotnet;
            c.stubbed |= !r.stubs.is_empty();
            // A test refused while it ran, stopped by the time limit or ended by a panic is not run (`NotRun`): what
            // its codeunit recorded is not all it would reach.
            if matches!(r.outcome, Outcome::NotRun(_)) {
                c.ended = Ended::Cut;
            }
            c.tests.push((r.test.clone(), r.outcome.clone()));
        }
        let ran: Vec<StoredCodeunit> = ran
            .into_values()
            .map(|mut c| {
                c.keys.sort();
                c.keys.dedup();
                c.entered.sort();
                c.entered.dedup();
                c
            })
            .collect();
        let before = self.stored.unwrap_or_default();
        let skipped: Vec<Skipped> = before
            .codeunits
            .iter()
            .filter(|c| !d.selection.run.contains(&c.id_key()) && d.codeunits.contains(&c.id_key()))
            .flat_map(|c| {
                let app = d.caller.get(&c.app).copied().unwrap_or(0);
                c.tests.iter().map(move |(test, outcome)| Skipped {
                    app,
                    codeunit: c.id,
                    codeunit_name: c.name.clone(),
                    test: test.clone(),
                    outcome: outcome.clone(),
                })
            })
            .collect();
        let after = before.merge(self.header, d.objects, &d.codeunits, ran, d.selection.full.is_some());
        if let Err(e) = write(&self.store, &after) {
            eprintln!("warning: the stored result of --affected was not written: {e}");
        }
        Some(Report {
            selection: d.selection,
            ran: ran_tests,
            skipped,
        })
    }
}

impl Report {
    /// The skipped tests whose stored outcome is not a pass.
    pub fn still_failing(&self) -> Vec<&Skipped> {
        self.skipped.iter().filter(|s| s.outcome != Outcome::Pass).collect()
    }

    /// The exit code of the run: a skipped test that failed last time keeps it at 1, and a run that skipped every
    /// test is not "nothing selected" (decision 9).
    pub fn exit_code(&self, code: u8) -> u8 {
        let failing = !self.still_failing().is_empty();
        match code {
            6 if self.skipped.is_empty() => 6,
            0 | 6..=8 if failing => 1,
            6 => 0,
            c => c,
        }
    }

    /// The text lines, as AL Runner v2's watch lines (decision 11).
    pub fn lines(&self) -> Vec<String> {
        let failing = self.still_failing();
        let mut out = vec![format!(
            "affected: ran {} of {}   skipped-unaffected {}   skipped-failing {}",
            self.ran,
            self.ran + self.skipped.len(),
            self.skipped.len() - failing.len(),
            failing.len()
        )];
        match &self.selection.full {
            Some(reason) => out.push(format!("full run: {reason}")),
            None if !self.selection.changed.is_empty() => {
                out.push(format!("changed: {}", self.selection.changed.join(", ")))
            }
            None => {}
        }
        if !failing.is_empty() {
            let names: Vec<String> = failing
                .iter()
                .map(|s| format!("{}::{}", s.codeunit_name, s.test))
                .collect();
            out.push(format!(
                "not re-run, still failing from an earlier run: {}",
                names.join(", ")
            ));
        }
        out
    }

    /// The `selection` object of the summary (serve, `--output-json`).
    pub fn json(&self) -> Value {
        let changed = &self.selection.changed;
        let mut v = json!({
            "mode": "affected",
            "ran": self.ran,
            "skipped": self.skipped.len(),
            "skippedFailing": self.still_failing().len(),
            "changedObjects": changed.iter().take(CHANGED_SHOWN).collect::<Vec<_>>(),
            "forcedFull": self.selection.full.is_some(),
            "reason": self.selection.full,
        });
        if changed.len() > CHANGED_SHOWN {
            v["changedObjectsTruncated"] = json!(changed.len() - CHANGED_SHOWN);
        }
        v
    }

    /// The skipped tests as results for JUnit: not run, with the message that names the stored outcome.
    pub fn junit_results(&self) -> Vec<TestResult> {
        self.skipped
            .iter()
            .map(|s| {
                let stored = match &s.outcome {
                    Outcome::Pass => "pass",
                    Outcome::Fail(_) => "fail",
                    Outcome::NotRun(_) => "not run",
                };
                TestResult {
                    suite: 0,
                    app: s.app,
                    codeunit: s.codeunit,
                    codeunit_name: s.codeunit_name.clone(),
                    test: s.test.clone(),
                    outcome: Outcome::NotRun(format!("not affected by the change; stored outcome: {stored}")),
                    call_stack: String::new(),
                    assumptions: Vec::new(),
                    time: std::time::Duration::ZERO,
                    coverage: Vec::new(),
                    messages: Vec::new(),
                    captured: None,
                    impact: Vec::new(),
                    stubs: Vec::new(),
                    entered: Vec::new(),
                    dotnet: false,
                }
            })
            .collect()
    }
}

/// Adds the `selection` object of `report` to the `--output-json` document `doc`.
pub fn with_selection(doc: String, report: Option<&Report>) -> String {
    let Some(r) = report else {
        return doc;
    };
    let mut v: Value = serde_json::from_str(&doc).unwrap_or_default();
    v["selection"] = r.json();
    serde_json::to_string_pretty(&v).expect("a JSON value serializes")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(skipped: &[Outcome]) -> Report {
        Report {
            selection: Selection::default(),
            ran: 1,
            skipped: skipped
                .iter()
                .enumerate()
                .map(|(i, o)| Skipped {
                    app: 0,
                    codeunit: 50110,
                    codeunit_name: "T".into(),
                    test: format!("Test{i}"),
                    outcome: o.clone(),
                })
                .collect(),
        }
    }

    #[test]
    fn a_skipped_failure_makes_the_exit_code_one() {
        let r = report(&[Outcome::Pass, Outcome::Fail("red".into())]);
        assert_eq!(r.exit_code(0), 1);
        assert_eq!(r.exit_code(6), 1);
        assert_eq!(r.exit_code(7), 1);
        assert_eq!(r.exit_code(3), 3);
    }

    #[test]
    fn a_run_that_skipped_every_test_is_not_nothing_selected() {
        assert_eq!(report(&[Outcome::Pass]).exit_code(6), 0);
        assert_eq!(report(&[]).exit_code(6), 6);
        assert_eq!(report(&[Outcome::Pass]).exit_code(7), 7);
    }

    #[test]
    fn more_than_fifty_changed_objects_are_cut_with_a_count() {
        let mut r = report(&[]);
        r.selection.changed = (0..53).map(|i| format!("codeunit {i} C")).collect();
        let v = r.json();
        assert_eq!(v["changedObjects"].as_array().unwrap().len(), 50);
        assert_eq!(v["changedObjectsTruncated"], 3);
    }
}
