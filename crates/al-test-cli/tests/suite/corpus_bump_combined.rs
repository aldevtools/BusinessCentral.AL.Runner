//! `tools/corpus_bump.sh` for a corpus run that combines fresh and reused results (ADR-077, decisions 4 to 7;
//! `tools/corpus_bump_provenance.py`): one test per refusal, against a fake `gh` and a throwaway corpus repository, as
//! in `corpus_bump_script.rs`. Every case is a dry run: nothing is written.

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn git(dir: &Path, args: &[&str]) -> String {
    let o = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(
        o.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

const APP: &str = "tests/al-language";
const ONPREM: &str = "tests/al-language-onprem";
const LEGS: [&str; 3] = ["27.5", "28.5", "29.0"];

fn build_of(leg: &str) -> String {
    format!("{leg}.100.200")
}

/// The corpus: `c0` declares codeunits 50100, 50101 and 50103 (file D.al) and the OnPrem codeunit 60100; `c1` adds
/// 50102 and deletes D.al; `c2` changes B.al (codeunit 50101); `side` is a commit off master.
struct Corpus {
    dir: tempfile::TempDir,
    c0: String,
    c1: String,
    c2: String,
    side: String,
}

fn put(corpus: &Path, rel: &str, text: &str) {
    let p = corpus.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}

fn corpus() -> Corpus {
    let dir = tempfile::tempdir().unwrap();
    let c = dir.path().join("corpus");
    std::fs::create_dir_all(&c).unwrap();
    git(&c, &["init", "-q", "-b", "master"]);
    put(&c, &format!("{APP}/A.al"), "codeunit 50100 \"A\"\n{\n}\n");
    put(&c, &format!("{APP}/B.al"), "codeunit 50101 \"B\"\n{\n}\n");
    put(&c, &format!("{APP}/D.al"), "codeunit 50103 \"D\"\n{\n}\n");
    put(&c, &format!("{ONPREM}/O.al"), "codeunit 60100 \"O\"\n{\n}\n");
    git(&c, &["add", "."]);
    git(&c, &["commit", "-qm", "c0"]);
    let c0 = git(&c, &["rev-parse", "HEAD"]);
    put(&c, &format!("{APP}/C.al"), "codeunit 50102 \"C\"\n{\n}\n");
    git(&c, &["rm", "-q", &format!("{APP}/D.al")]);
    git(&c, &["add", "."]);
    git(&c, &["commit", "-qm", "c1"]);
    let c1 = git(&c, &["rev-parse", "HEAD"]);
    put(&c, &format!("{APP}/B.al"), "codeunit 50101 \"B\"\n{\n  // changed\n}\n");
    git(&c, &["add", "."]);
    git(&c, &["commit", "-qm", "c2"]);
    let c2 = git(&c, &["rev-parse", "HEAD"]);
    git(&c, &["checkout", "-qb", "side", &c0]);
    put(&c, "README", "side");
    git(&c, &["add", "."]);
    git(&c, &["commit", "-qm", "side"]);
    let side = git(&c, &["rev-parse", "HEAD"]);
    git(&c, &["checkout", "-q", "master"]);
    Corpus { dir, c0, c1, c2, side }
}

/// One `<testsuite>`: codeunit id and, when given, its provenance (source run, source commit, BC build).
#[derive(Clone)]
struct Suite {
    cu: u32,
    prov: Option<(u64, String, String)>,
}

fn junit(suites: &[Suite]) -> String {
    let mut x = String::from("<?xml version='1.0' encoding='utf-8'?>\n<testsuites name=\"hybrid\">");
    for s in suites {
        x.push_str(&format!(
            "<testsuite name=\"Codeunit {}\" tests=\"1\" timestamp=\"2026-11-17T10:00:00Z\">",
            s.cu
        ));
        if let Some((run, commit, build)) = &s.prov {
            x.push_str(&format!(
                "<properties><property name=\"source_run\" value=\"{run}\" /><property name=\"source_attempt\" value=\"1\" /><property name=\"source_commit\" value=\"{commit}\" /><property name=\"bc_build\" value=\"{build}\" /><property name=\"app\" value=\"full\" /></properties>"
            ));
        }
        x.push_str(&format!(
            "<testcase classname=\"Codeunit {}\" name=\"T\" time=\"0.1\" /></testsuite>",
            s.cu
        ));
    }
    x.push_str("</testsuites>\n");
    x
}

/// What the fake corpus run looks like. `Spec::combined` is the accepted run each test changes one fact of.
#[derive(Clone)]
struct Spec {
    run: u64,
    attempt: u32,
    sha: String,
    conclusion: String,
    event: String,
    branch: String,
    /// The suites of the main file and of the OnPrem file, written with BC 27.5's build; each leg gets its own build.
    main: Vec<Suite>,
    onprem: Vec<Suite>,
    /// The results manifest: None for a run without one.
    manifest: Option<serde_json::Value>,
    /// The jobs of the run's latest attempt: name and conclusion.
    jobs: Vec<(String, String)>,
    /// The nightly runs, oldest first.
    nightlies: Vec<Nightly>,
    /// Write the manifest's per-leg counts from the suites (false: keep the counts the manifest has).
    counts: bool,
    /// Each leg also keeps the artifacts of an earlier attempt (so the roles are read from the XML file names).
    rerun: bool,
    /// The bump runs on a clone that has only `c0` and fetches the rest from the corpus (no CORPUS_BUMP_NO_FETCH).
    fetch: bool,
    /// A text replacement applied to every JUnit file of the run (from, to).
    patch: Option<(String, String)>,
}

/// A nightly run: id, commit, its comparison for every version it names (None: no manifest), conclusion, versions.
#[derive(Clone)]
struct Nightly {
    id: u64,
    sha: String,
    cmp: Option<String>,
    conclusion: String,
    versions: Vec<&'static str>,
}

fn night(id: u64, sha: &str, cmp: Option<&str>) -> Nightly {
    Nightly {
        id,
        sha: sha.to_string(),
        cmp: cmp.map(str::to_string),
        conclusion: "success".into(),
        versions: LEGS.to_vec(),
    }
}

fn prov(run: u64, commit: &str, leg: &str) -> Option<(u64, String, String)> {
    Some((run, commit.to_string(), build_of(leg)))
}

/// A results manifest in the corpus's schema 1 (corpus .github/scripts/provenance.py, `manifest`).
fn manifest(run: u64, attempt: u32, sha: &str, mode: &str, chain: u32, reused: u32) -> serde_json::Value {
    let mut versions = serde_json::Map::new();
    for leg in LEGS {
        versions.insert(
            leg.into(),
            serde_json::json!({
                "bc_build": build_of(leg), "bc_artifact_url": "", "mode": mode, "chain": chain, "app": "full",
                "legs": {
                    "junit-al-language.xml": {"test_app_dir": APP, "fresh": 2, "reused": reused, "dropped": 0, "oldest_source_run": 10},
                    "junit-al-language-onprem.xml": {"test_app_dir": ONPREM, "fresh": 0, "reused": reused, "dropped": 0, "oldest_source_run": 10},
                }
            }),
        );
    }
    serde_json::json!({
        "schema": 1, "run_id": run, "run_attempt": attempt, "commit": sha, "event": "push",
        "base_run": if mode == "full" { serde_json::Value::Null } else { serde_json::json!(10) },
        "base_commit": null, "created": "2026-11-17T10:00:00Z", "versions": versions,
    })
}

impl Spec {
    /// Run 20 on `c1`: 50100 and 50102 fresh, 50101 and 60100 reused from run 10 on `c0`; nightly 15 on `c0` identical.
    fn combined(c: &Corpus) -> Spec {
        let leg = "27.5";
        Spec {
            run: 20,
            attempt: 1,
            sha: c.c1.clone(),
            conclusion: "success".into(),
            event: "push".into(),
            branch: "master".into(),
            main: vec![
                Suite {
                    cu: 50100,
                    prov: prov(20, &c.c1, leg),
                },
                Suite {
                    cu: 50101,
                    prov: prov(10, &c.c0, leg),
                },
                Suite {
                    cu: 50102,
                    prov: prov(20, &c.c1, leg),
                },
            ],
            onprem: vec![Suite {
                cu: 60100,
                prov: prov(10, &c.c0, leg),
            }],
            manifest: Some(manifest(20, 1, &c.c1, "subset", 1, 1)),
            jobs: vec![("BC 27.5 / 1".into(), "success".into())],
            nightlies: vec![night(15, &c.c0, Some("identical"))],
            counts: true,
            rerun: false,
            fetch: false,
            patch: None,
        }
    }

    /// Every suite fresh from this run, manifest mode full.
    fn full(c: &Corpus, sha: &str) -> Spec {
        let mut s = Spec::combined(c);
        s.sha = sha.to_string();
        s.main = vec![
            Suite {
                cu: 50100,
                prov: prov(20, sha, "27.5"),
            },
            Suite {
                cu: 50101,
                prov: prov(20, sha, "27.5"),
            },
            Suite {
                cu: 50102,
                prov: prov(20, sha, "27.5"),
            },
        ];
        s.onprem = vec![Suite {
            cu: 60100,
            prov: prov(20, sha, "27.5"),
        }];
        s.manifest = Some(manifest(20, 1, sha, "full", 0, 0));
        s
    }
}

/// Writes the fake `gh`, the artifacts and the old oracle of the spec into the fixture folder; runs the dry bump.
fn bump(c: &Corpus, s: &Spec) -> (i32, String) {
    let d = c.dir.path().join("gh");
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let zip = |id: u64, name: &str, text: &str| {
        let src = d.join(format!("z{id}"));
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join(name), text).unwrap();
        let st = Command::new("zip")
            .current_dir(&src)
            .args(["-q", &format!("{}/{id}.zip", d.display()), name])
            .status()
            .unwrap();
        assert!(st.success());
    };
    // The run's artifacts: per leg the main file and the OnPrem file.
    let mut arts = Vec::new();
    let mut id = 1000;
    for leg in LEGS {
        for (name, suites, size) in [
            ("junit-al-language.xml", &s.main, 2000),
            ("junit-al-language-onprem.xml", &s.onprem, 1000),
        ] {
            id += 1;
            let suites: Vec<Suite> = suites
                .iter()
                .map(|x| Suite {
                    cu: x.cu,
                    prov: x.prov.clone().map(|(r, c, b)| (r, c, b.replacen("27.5", leg, 1))),
                })
                .collect();
            for created in if s.rerun { &["09", "10"][..] } else { &["10"][..] } {
                id += 1;
                let mut x = junit(&suites);
                if let Some((from, to)) = &s.patch {
                    x = x.replace(from.as_str(), to);
                }
                zip(id, name, &x);
                arts.push(format!(r#"{{"id":{id},"name":"junit-test-results-{leg}","size_in_bytes":{size},"created_at":"2026-11-17T{created}:00:00Z"}}"#));
            }
        }
    }
    if let Some(m) = &s.manifest {
        let mut m = m.clone();
        if s.counts {
            // The counts the corpus writes: per leg file, the suites of this run and the others.
            for leg in LEGS {
                for (name, suites) in [
                    ("junit-al-language.xml", &s.main),
                    ("junit-al-language-onprem.xml", &s.onprem),
                ] {
                    let fresh = suites
                        .iter()
                        .filter(|x| x.prov.as_ref().is_some_and(|p| p.0 == s.run))
                        .count();
                    if let Some(l) = m["versions"].get_mut(leg).and_then(|v| v["legs"].get_mut(name)) {
                        l["fresh"] = serde_json::json!(fresh);
                        l["reused"] = serde_json::json!(suites.len() - fresh);
                    }
                }
            }
        }
        id += 1;
        zip(id, "results-manifest.json", &m.to_string());
        arts.push(format!(
            r#"{{"id":{id},"name":"results-manifest","size_in_bytes":10,"created_at":"2026-11-17T11:00:00Z"}}"#
        ));
    }
    std::fs::write(
        d.join(format!("arts-{}.json", s.run)),
        format!(r#"{{"artifacts":[{}]}}"#, arts.join(",")),
    )
    .unwrap();
    std::fs::write(
        d.join(format!("view-{}.json", s.run)),
        serde_json::json!({
            "databaseId": s.run, "status": "completed", "conclusion": s.conclusion, "headSha": s.sha,
            "attempt": s.attempt, "event": s.event, "headBranch": s.branch, "workflowDatabaseId": 77,
        })
        .to_string(),
    )
    .unwrap();
    let jobs: Vec<_> = s
        .jobs
        .iter()
        .map(|(n, c)| serde_json::json!({"name": n, "conclusion": c}))
        .collect();
    std::fs::write(
        d.join(format!("jobs-{}.json", s.run)),
        serde_json::json!({ "jobs": jobs }).to_string(),
    )
    .unwrap();
    let mut list = Vec::new();
    for (k, n) in s.nightlies.iter().enumerate() {
        let (nid, nsha) = (&n.id, &n.sha);
        list.push(serde_json::json!({"databaseId": nid, "headSha": nsha, "attempt": 1, "conclusion": n.conclusion, "createdAt": format!("2026-11-1{k}T02:00:00Z")}));
        let mut a = Vec::new();
        if let Some(cmp) = &n.cmp {
            let mut m = manifest(*nid, 1, nsha, "full", 0, 0);
            for leg in LEGS {
                if n.versions.contains(&leg) {
                    m["versions"][leg]["comparison"] = serde_json::json!(cmp);
                } else {
                    m["versions"].as_object_mut().unwrap().remove(leg);
                }
            }
            id += 1;
            zip(id, "results-manifest.json", &m.to_string());
            a.push(format!(
                r#"{{"id":{id},"name":"results-manifest","size_in_bytes":10,"created_at":"2026-11-17T11:00:00Z"}}"#
            ));
        }
        if *nid != s.run {
            std::fs::write(
                d.join(format!("arts-{nid}.json")),
                format!(r#"{{"artifacts":[{}]}}"#, a.join(",")),
            )
            .unwrap();
        }
    }
    std::fs::write(d.join("list.json"), serde_json::Value::Array(list).to_string()).unwrap();
    // The old oracle: codeunits 50100, 50101 and 50103 (D.al, deleted by c1) and 60100; oracle.toml with the CI builds.
    let oracle = d.join("oracle");
    let mut toml = String::new();
    for leg in LEGS {
        let major = &leg[..2];
        std::fs::create_dir_all(oracle.join(major)).unwrap();
        let old = |cus: &[u32]| junit(&cus.iter().map(|&cu| Suite { cu, prov: None }).collect::<Vec<_>>());
        std::fs::write(oracle.join(major).join("junit.xml"), old(&[50100, 50101, 50103])).unwrap();
        std::fs::write(oracle.join(major).join("junit-onprem.xml"), old(&[60100])).unwrap();
        toml.push_str(&format!(
            "[[version]]\nmajor = {major}\nbc_build = \"{}\"\n\n",
            build_of(leg)
        ));
    }
    std::fs::write(oracle.join("oracle.toml"), toml).unwrap();
    let dd = d.display();
    let gh = d.join("gh");
    std::fs::write(
        &gh,
        format!(
            r#"#!/bin/bash
echo "$*" >> {dd}/calls.log
case "$*" in
  *"run view"*"--json jobs"*) cat {dd}/jobs-$3.json ;;
  *"run view"*) cat {dd}/view-$3.json ;;
  *"run list"*) cat {dd}/list.json ;;
  */zip*) id=$(echo "$*" | sed 's#.*/artifacts/\([0-9]*\)/zip.*#\1#'); cat {dd}/$id.zip ;;
  *"/artifacts?"*) id=${{2#*runs/}}; cat {dd}/arts-${{id%%/*}}.json ;;
  *) echo "unexpected gh $*" >&2; exit 9 ;;
esac
"#
        ),
    )
    .unwrap();
    Command::new("chmod").arg("+x").arg(&gh).status().unwrap();
    // The fetch case: a clone of the corpus that has only c0, whose origin has every commit.
    let origin = c.dir.path().join("corpus");
    let local = if s.fetch {
        let clone = c.dir.path().join("clone");
        let _ = std::fs::remove_dir_all(&clone);
        git(
            c.dir.path(),
            &["clone", "-q", "--no-local", &origin.display().to_string(), "clone"],
        );
        git(&clone, &["checkout", "-q", "--detach", &c.c0]);
        git(&clone, &["branch", "-f", "master", &c.c0]);
        git(&clone, &["checkout", "-q", "master"]);
        git(&clone, &["remote", "set-head", "origin", "-d"]);
        for r in ["refs/remotes/origin/master", "refs/remotes/origin/side"] {
            git(&clone, &["update-ref", "-d", r]);
        }
        git(&clone, &["reflog", "expire", "--expire=now", "--all"]);
        git(&clone, &["gc", "-q", "--prune=now"]);
        let has_c1 = Command::new("git")
            .arg("-C")
            .arg(&clone)
            .args(["cat-file", "-e", &format!("{}^{{commit}}", c.c1)])
            .output()
            .unwrap();
        assert!(!has_c1.status.success(), "the clone still has c1");
        clone
    } else {
        origin
    };
    for _ in 0..20 {
        let o = Command::new("bash")
            .arg(root().join("tools/corpus_bump.sh"))
            .args(["--dry-run", &s.run.to_string()])
            .env("CORPUS_BUMP_GH", &gh)
            .env("CORPUS_BUMP_CORPUS", &local)
            .env("CORPUS_BUMP_NO_FETCH", if s.fetch { "0" } else { "1" })
            .env("CORPUS_BUMP_PIN", &c.c0)
            .env("CORPUS_BUMP_ORACLE", &oracle)
            .output()
            .unwrap();
        let all = format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        if !all.contains("Text file busy") {
            return (o.status.code().unwrap_or(-1), all);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("{} stayed busy", gh.display());
}

fn refused(c: &Corpus, s: &Spec, why: &str) {
    let (code, out) = bump(c, s);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("corpus_bump: refused:") && out.contains(why),
        "expected {why:?}:\n{out}"
    );
    assert!(!out.contains("dry run: nothing changed"), "{out}");
}

fn accepted(c: &Corpus, s: &Spec) -> String {
    let (code, out) = bump(c, s);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("dry run: nothing changed"), "{out}");
    out
}

#[test]
fn a_combined_run_is_accepted_and_reports_the_oracle_toml_fields() {
    let c = corpus();
    let out = accepted(&c, &Spec::combined(&c));
    assert!(
        out.contains(
            "BC 27.5: mode subset, 2 fresh and 2 reused codeunits, oldest source run 10, CI build 27.5.100.200"
        ),
        "{out}"
    );
    assert!(
        out.contains("nightly run 15 at or before its commit reports no difference"),
        "{out}"
    );
    assert!(!out.contains("WARNING: BC"), "{out}");
}

#[test]
fn a_run_whose_legs_succeeded_but_whose_manifest_job_failed_is_refused() {
    let c = corpus();
    let mut s = Spec::combined(&c);
    s.conclusion = "failure".into();
    s.manifest = None;
    s.jobs = vec![
        ("BC 27.5 / 1".into(), "success".into()),
        ("Results manifest".into(), "failure".into()),
    ];
    refused(&c, &s, "is not green (conclusion: failure)");
}

#[test]
fn a_manifest_of_an_earlier_attempt_is_refused() {
    let c = corpus();
    let mut s = Spec::combined(&c);
    s.attempt = 2;
    refused(
        &c,
        &s,
        "its results manifest is of attempt 1, the run's latest attempt is 2",
    );
}

#[test]
fn a_manifest_of_another_schema_is_refused() {
    let c = corpus();
    let mut s = Spec::combined(&c);
    s.manifest.as_mut().unwrap()["schema"] = serde_json::json!(2);
    refused(&c, &s, "results manifest schema 2");
}

#[test]
fn a_full_run_without_manifest_and_without_provenance_is_accepted() {
    let c = corpus();
    let mut s = Spec::full(&c, &c.c1);
    s.manifest = None;
    s.nightlies = vec![];
    for x in s.main.iter_mut().chain(s.onprem.iter_mut()) {
        x.prov = None;
    }
    let out = accepted(&c, &s);
    assert!(out.contains("no results manifest"), "{out}");
}

#[test]
fn a_run_without_manifest_whose_suites_carry_provenance_is_refused() {
    let c = corpus();
    let mut s = Spec::full(&c, &c.c1);
    s.manifest = None;
    refused(
        &c,
        &s,
        "has no results manifest, but junit-al-language.xml of BC 27.5 carries provenance properties",
    );
}

#[test]
fn a_run_on_another_branch_is_refused() {
    let c = corpus();
    let mut s = Spec::combined(&c);
    s.branch = "staging".into();
    refused(&c, &s, "ran on branch 'staging', not on master");
}

#[test]
fn a_suite_without_its_properties_is_refused() {
    let c = corpus();
    let mut s = Spec::combined(&c);
    s.main[2].prov = None;
    refused(&c, &s, "suite Codeunit 50102: provenance properties []");
}

#[test]
fn a_suite_of_another_bc_build_is_refused() {
    let c = corpus();
    let mut s = Spec::combined(&c);
    s.main[0].prov = Some((20, c.c1.clone(), "27.5.100.199".into()));
    refused(
        &c,
        &s,
        "BC build 27.5.100.199, the manifest names 27.5.100.200 for BC 27.5",
    );
}

#[test]
fn a_source_commit_that_is_not_an_ancestor_is_refused() {
    let c = corpus();
    let mut s = Spec::combined(&c);
    s.main[1].prov = prov(10, &c.side, "27.5");
    refused(&c, &s, "is not an ancestor of the run's commit");
}

#[test]
fn a_reused_codeunit_whose_declaring_file_changed_is_refused() {
    // Run 20 on c2, which changed B.al: codeunit 50101 cannot be reused from c0.
    let c = corpus();
    let mut s = Spec::combined(&c);
    s.sha = c.c2.clone();
    s.main[0].prov = prov(20, &c.c2, "27.5");
    s.main[2].prov = prov(20, &c.c2, "27.5");
    s.manifest = Some(manifest(20, 1, &c.c2, "subset", 1, 1));
    refused(&c, &s, "codeunit 50101 is reused from commit");
}

#[test]
fn a_chain_of_more_than_twenty_runs_is_refused() {
    let c = corpus();
    let mut s = Spec::combined(&c);
    s.manifest.as_mut().unwrap()["versions"]["28.5"]["chain"] = serde_json::json!(21);
    refused(&c, &s, "BC 28.5: chain 21, at most 20 runs");
}

#[test]
fn a_chain_of_twenty_runs_is_accepted() {
    let c = corpus();
    let mut s = Spec::combined(&c);
    for leg in LEGS {
        s.manifest.as_mut().unwrap()["versions"][leg]["chain"] = serde_json::json!(20);
    }
    accepted(&c, &s);
}

#[test]
fn results_from_more_runs_than_the_chain_allows_are_refused() {
    let c = corpus();
    let mut s = Spec::combined(&c);
    s.onprem[0].prov = prov(12, &c.c0, "27.5");
    refused(&c, &s, "results from 3 runs, more than its chain 1 allows");
}

#[test]
fn a_missing_codeunit_whose_file_the_diff_kept_is_refused() {
    // Codeunit 50101 (B.al, unchanged) was in the old oracle and is missing from the new file. The default spec also
    // lacks 50103, whose file D.al the diff deleted: that one passes.
    let c = corpus();
    let mut s = Spec::combined(&c);
    s.main.remove(1);
    refused(&c, &s, "codeunit 50101 of the current oracle is missing");
}

#[test]
fn a_nightly_difference_blocks_a_combined_run() {
    let c = corpus();
    let mut s = Spec::combined(&c);
    s.nightlies = vec![night(15, &c.c0, Some("differs"))];
    refused(&c, &s, "the nightly run 15 reports comparison 'differs'");
}

#[test]
fn a_nightly_difference_blocks_a_full_push_run() {
    let c = corpus();
    let mut s = Spec::full(&c, &c.c1);
    s.nightlies = vec![night(15, &c.c0, Some("differs"))];
    refused(&c, &s, "the nightly run 15 reports comparison 'differs'");
}

#[test]
fn a_nightly_without_a_manifest_blocks_the_bump() {
    let c = corpus();
    let mut s = Spec::combined(&c);
    s.nightlies = vec![night(15, &c.c0, None)];
    refused(
        &c,
        &s,
        "the newest nightly run at or before its commit, 15, has no results manifest",
    );
}

#[test]
fn only_the_newest_nightly_at_or_before_the_commit_counts() {
    // Nightly 16 on the side branch (not before the commit) says differs; nightly 15 on c0 is the one that counts.
    let c = corpus();
    let mut s = Spec::combined(&c);
    s.nightlies = vec![night(15, &c.c0, Some("identical")), night(16, &c.side, Some("differs"))];
    accepted(&c, &s);
}

#[test]
fn a_combined_run_with_no_nightly_before_it_is_refused() {
    let c = corpus();
    let mut s = Spec::combined(&c);
    s.nightlies = vec![];
    refused(
        &c,
        &s,
        "reuses results and no nightly run at or before its commit compared them",
    );
}

#[test]
fn a_full_push_run_with_no_nightly_before_it_is_accepted() {
    let c = corpus();
    let mut s = Spec::full(&c, &c.c1);
    s.nightlies = vec![];
    let out = accepted(&c, &s);
    assert!(
        out.contains("no nightly run at or before its commit, a full run"),
        "{out}"
    );
}

/// The nightly full run on c1 whose comparison failed: all its results are fresh (decision 6).
fn nightly(c: &Corpus) -> Spec {
    let mut s = Spec::full(c, &c.c1);
    s.event = "schedule".into();
    s.conclusion = "failure".into();
    s.jobs = vec![
        ("BC 27.5 / 1".into(), "success".into()),
        ("Results manifest".into(), "success".into()),
        ("Nightly comparison".into(), "failure".into()),
        ("Coverage facts must not shrink".into(), "skipped".into()),
    ];
    s.nightlies = vec![night(20, &c.c1, Some("differs"))];
    s
}

#[test]
fn the_nightly_full_run_whose_only_failed_job_is_the_comparison_is_accepted() {
    let c = corpus();
    let out = accepted(&c, &nightly(&c));
    assert!(
        out.contains("the nightly full run, only its job 'Nightly comparison' failed"),
        "{out}"
    );
}

#[test]
fn a_nightly_run_with_a_failed_test_leg_is_refused() {
    let c = corpus();
    let mut s = nightly(&c);
    s.jobs[0].1 = "failure".into();
    refused(&c, &s, "is not green (conclusion: failure)");
}

#[test]
fn a_nightly_run_that_reused_results_is_refused() {
    let c = corpus();
    let mut s = nightly(&c);
    s.manifest = Some(manifest(20, 1, &c.c1, "subset", 1, 1));
    refused(&c, &s, "is not green (conclusion: failure)");
}

#[test]
fn a_push_run_whose_only_failed_job_is_the_comparison_is_refused() {
    let c = corpus();
    let mut s = nightly(&c);
    s.event = "push".into();
    refused(&c, &s, "is not green (conclusion: failure)");
}

#[test]
fn a_ci_build_that_differs_from_the_loaded_build_is_a_warning() {
    let c = corpus();
    let mut s = Spec::combined(&c);
    for leg in LEGS {
        s.manifest.as_mut().unwrap()["versions"][leg]["bc_build"] = serde_json::json!(format!("{leg}.100.201"));
    }
    for x in s.main.iter_mut().chain(s.onprem.iter_mut()) {
        if let Some(p) = x.prov.as_mut() {
            p.2 = "27.5.100.201".into();
        }
    }
    let out = accepted(&c, &s);
    assert!(out.contains("WARNING: BC 27.5: CI ran build 27.5.100.201, oracle.toml bc_build (the build the runner loads) is 27.5.100.200"), "{out}");
}

/// Decision 7: the edit writes the fields of a provenance summary into each version's oracle.toml table, and a bump
/// without a manifest (empty summary) removes them.
#[test]
fn the_oracle_toml_gets_the_fields_of_decision_7() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("oracle")).unwrap();
    for f in ["oracle.toml", "README.md"] {
        std::fs::copy(root().join("oracle").join(f), dir.path().join("oracle").join(f)).unwrap();
    }
    let sum = dir.path().join("summary.json");
    let edit = |summary: &str| {
        std::fs::write(&sum, summary).unwrap();
        let o = Command::new("python3")
            .env("PROVENANCE", &sum)
            .arg(root().join("tools/corpus_bump_edit.py"))
            .arg(dir.path())
            .arg("0123456789abcdef0123456789abcdef01234567")
            .arg("999")
            .args([
                "27", "1", "2", "5", "6", "28", "3", "4", "5", "6", "29", "5", "6", "5", "6",
            ])
            .output()
            .unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        let t = std::fs::read_to_string(dir.path().join("oracle/oracle.toml")).unwrap();
        toml::from_str::<toml::Value>(&t).unwrap()
    };
    let mut s = serde_json::Map::new();
    for major in [27, 28, 29] {
        s.insert(
            major.to_string(),
            serde_json::json!({"mode": "subset", "fresh": major, "reused": 7, "oldest_source_run": 10, "ci_bc_build": format!("{major}.5.1.2")}),
        );
    }
    let v = edit(&serde_json::Value::Object(s.clone()).to_string());
    for ver in v["version"].as_array().unwrap() {
        let major = ver["major"].as_integer().unwrap();
        assert_eq!(ver["mode"].as_str(), Some("subset"), "{ver:?}");
        assert_eq!(ver["fresh"].as_integer(), Some(major));
        assert_eq!(ver["reused"].as_integer(), Some(7));
        assert_eq!(ver["oldest_source_run"].as_integer(), Some(10));
        assert_eq!(
            ver["ci_bc_build"].as_str().map(str::to_string),
            Some(format!("{major}.5.1.2"))
        );
        assert!(ver["artifact_id"].as_integer().is_some());
    }
    // Applying it again keeps one copy of each field.
    let again = edit(&serde_json::Value::Object(s).to_string());
    assert_eq!(again, v);
    let t = std::fs::read_to_string(dir.path().join("oracle/oracle.toml")).unwrap();
    assert_eq!(t.matches("\nci_bc_build = ").count(), 3, "{t}");
    let none = edit("{}");
    for ver in none["version"].as_array().unwrap() {
        assert!(ver.get("mode").is_none() && ver.get("ci_bc_build").is_none(), "{ver:?}");
    }
}

// ---------- review round 1 ----------

#[test]
fn a_run_whose_commit_is_fetched_first_meets_the_open_nightly_difference() {
    // The local corpus has only c0; the run's commit c1 comes with the fetch. The nightly on c0 says differs.
    let c = corpus();
    let mut s = Spec::full(&c, &c.c1);
    s.fetch = true;
    s.nightlies = vec![night(15, &c.c0, Some("differs"))];
    refused(&c, &s, "the nightly run 15 reports comparison 'differs'");
}

#[test]
fn a_combined_run_whose_commit_is_fetched_first_is_accepted() {
    let c = corpus();
    let mut s = Spec::combined(&c);
    s.fetch = true;
    accepted(&c, &s);
}

#[test]
fn a_nightly_whose_commit_git_cannot_read_is_refused() {
    let c = corpus();
    let mut s = Spec::full(&c, &c.c1);
    s.nightlies = vec![night(15, &"f".repeat(40), Some("identical"))];
    refused(&c, &s, "git merge-base --is-ancestor");
}

#[test]
fn a_nightly_manifest_that_lacks_a_version_of_the_run_is_refused() {
    let c = corpus();
    let mut s = Spec::combined(&c);
    s.nightlies[0].versions = vec!["27.5"];
    refused(&c, &s, "the nightly run 15 reports comparison None for BC 28.5");
}

#[test]
fn a_nightly_exception_whose_files_hold_reused_results_is_refused() {
    // The manifest says full and reused 0, but the files hold suites of run 10.
    let c = corpus();
    let mut s = nightly(&c);
    s.counts = false;
    s.main[1].prov = prov(10, &c.c0, "27.5");
    refused(
        &c,
        &s,
        "junit-al-language.xml: 2 fresh and 1 reused codeunits, the manifest names 2 and 0",
    );
}

#[test]
fn a_full_push_run_whose_files_are_all_reused_is_refused() {
    let c = corpus();
    let mut s = Spec::full(&c, &c.c1);
    s.nightlies = vec![];
    s.counts = false;
    for x in s.main.iter_mut().chain(s.onprem.iter_mut()) {
        x.prov = prov(10, &c.c0, "27.5");
    }
    s.main.retain(|x| x.cu != 50102);
    refused(&c, &s, "the manifest names");
}

#[test]
fn a_full_version_with_a_chain_is_refused() {
    let c = corpus();
    let mut s = Spec::full(&c, &c.c1);
    s.manifest.as_mut().unwrap()["versions"]["29.0"]["chain"] = serde_json::json!(1);
    refused(&c, &s, "BC 29.0: mode full with chain 1 and 0 reused codeunits");
}

#[test]
fn a_suite_of_this_run_from_another_commit_is_refused() {
    // Run 20 on c2: suite 50101 names run 20 but commit c0, so it would skip the reused-file check.
    let c = corpus();
    let mut s = Spec::combined(&c);
    s.sha = c.c2.clone();
    s.main[0].prov = prov(20, &c.c2, "27.5");
    s.main[1].prov = prov(20, &c.c0, "27.5");
    s.main[2].prov = prov(20, &c.c2, "27.5");
    s.manifest = Some(manifest(20, 1, &c.c2, "subset", 1, 1));
    refused(&c, &s, "names this run but source commit");
}

#[test]
fn a_nightly_run_with_a_skipped_test_leg_is_refused() {
    let c = corpus();
    let mut s = nightly(&c);
    s.jobs[0].1 = "skipped".into();
    refused(&c, &s, "is not green (conclusion: failure)");
}

#[test]
fn a_pull_request_run_with_a_manifest_is_refused() {
    let c = corpus();
    let mut s = Spec::combined(&c);
    s.event = "pull_request".into();
    refused(&c, &s, "event 'pull_request'");
}

#[test]
fn a_workflow_dispatch_run_with_a_manifest_is_refused() {
    let c = corpus();
    let mut s = Spec::combined(&c);
    s.event = "workflow_dispatch".into();
    refused(&c, &s, "event 'workflow_dispatch'");
}

#[test]
fn a_red_nightly_does_not_count() {
    let c = corpus();
    let mut s = Spec::combined(&c);
    s.nightlies[0].conclusion = "failure".into();
    refused(&c, &s, "the nightly run 15 ended with conclusion failure");
}

#[test]
fn a_version_without_a_mode_is_refused() {
    let c = corpus();
    let mut s = Spec::combined(&c);
    s.manifest.as_mut().unwrap()["versions"]["27.5"]
        .as_object_mut()
        .unwrap()
        .remove("mode");
    refused(&c, &s, "BC 27.5: mode None, expected one of full, subset");
}

#[test]
fn a_version_with_an_unknown_mode_is_refused() {
    let c = corpus();
    let mut s = Spec::combined(&c);
    s.manifest.as_mut().unwrap()["versions"]["27.5"]["mode"] = serde_json::json!("banana");
    refused(&c, &s, "BC 27.5: mode 'banana', expected one of full, subset");
}

#[test]
fn a_suite_of_a_later_attempt_than_the_run_is_refused() {
    let c = corpus();
    let mut s = Spec::combined(&c);
    s.patch = Some((
        r#"name="source_attempt" value="1""#.into(),
        r#"name="source_attempt" value="9""#.into(),
    ));
    refused(&c, &s, "source attempt 9, the run is at attempt 1");
}

#[test]
fn a_suite_of_an_unknown_app_is_refused() {
    let c = corpus();
    let mut s = Spec::combined(&c);
    s.patch = Some((r#"value="full""#.into(), r#"value="bogus""#.into()));
    refused(&c, &s, "app 'bogus', expected one of full, partial");
}

#[test]
fn a_reused_suite_of_a_later_run_is_refused() {
    let c = corpus();
    let mut s = Spec::combined(&c);
    s.main[1].prov = prov(99, &c.c0, "27.5");
    refused(&c, &s, "reused from run 99, which is not older than run 20");
}

#[test]
fn a_manifest_of_another_run_is_refused() {
    let c = corpus();
    let mut s = Spec::combined(&c);
    s.manifest.as_mut().unwrap()["run_id"] = serde_json::json!(21);
    refused(&c, &s, "its results manifest names run 21");
}

#[test]
fn a_manifest_of_another_commit_is_refused() {
    let c = corpus();
    let mut s = Spec::combined(&c);
    s.manifest.as_mut().unwrap()["commit"] = serde_json::json!(c.c0);
    refused(&c, &s, "its results manifest names commit");
}

#[test]
fn a_manifest_without_versions_is_refused() {
    let c = corpus();
    let mut s = Spec::combined(&c);
    s.manifest.as_mut().unwrap()["versions"] = serde_json::json!({});
    refused(&c, &s, "its results manifest names no BC version");
}

#[test]
fn a_file_whose_version_the_manifest_lacks_is_refused() {
    let c = corpus();
    let mut s = Spec::combined(&c);
    s.manifest.as_mut().unwrap()["versions"]
        .as_object_mut()
        .unwrap()
        .remove("29.0");
    refused(&c, &s, "its results manifest has no BC version 29.0");
}

#[test]
fn a_file_whose_leg_the_manifest_lacks_is_refused() {
    let c = corpus();
    let mut s = Spec::combined(&c);
    s.manifest.as_mut().unwrap()["versions"]["28.5"]["legs"]
        .as_object_mut()
        .unwrap()
        .remove("junit-al-language-onprem.xml");
    refused(&c, &s, "has no leg junit-al-language-onprem.xml for BC 28.5");
}

#[test]
fn a_suite_that_is_not_a_codeunit_is_refused() {
    let c = corpus();
    let mut s = Spec::combined(&c);
    s.patch = Some((
        r#"testsuite name="Codeunit 50102""#.into(),
        r#"testsuite name="Page 50102""#.into(),
    ));
    refused(&c, &s, "suite Page 50102: not a codeunit suite");
}

#[test]
fn each_artifact_is_downloaded_once() {
    // With the artifacts of an earlier attempt next to each leg's, the roles come from the XML file names.
    let c = corpus();
    let mut s = Spec::combined(&c);
    s.rerun = true;
    accepted(&c, &s);
    let calls = std::fs::read_to_string(c.dir.path().join("gh/calls.log")).unwrap();
    let zips: Vec<&str> = calls.lines().filter(|l| l.contains("/zip")).collect();
    let mut seen = std::collections::HashSet::new();
    for z in &zips {
        assert!(seen.insert(*z), "downloaded twice: {z}\n{calls}");
    }
}
