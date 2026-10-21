//! The impact index (ADR-043 step 2): one run of the test apps without any extension that keeps, per test, its
//! outcome and what it raised and touched (`al_testrun::impact_trace`). An impact run with an index runs only the test
//! codeunits that reach an impact point of the extension, once, and takes the outcomes without it from here.
//!
//! The index is internal tooling (ADR-009): a gzip-compressed JSON document with a format number. It is kept under
//! `<cache dir>/impact/<key>/index.json.gz`, where the key hashes everything the outcomes depend on that is known
//! before the run: the runtime build (executable hash), the test apps' source, the country, the build asked for,
//! the company data and test data options, the isolation, the defines and the disabled-tests files. The Microsoft
//! apps the run compiled are stored in the index and checked against the impact run's compile (a different build
//! of the Base App makes the index stale).

use crate::apps;
use crate::args::TestArgs;
use crate::run::{self, Collected};
use al_testrun::Outcome;
use al_testrun::runtime::TestResult;
use serde_json::{Value, json};
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// The format of the index file. A file of another format is not read (it is rebuilt).
pub const FORMAT: u64 = 1;

/// One test of the index run.
#[derive(Debug, Clone, PartialEq)]
pub struct IndexedTest {
    /// The test app (index among the APPs given).
    pub app: usize,
    pub codeunit: u32,
    pub codeunit_name: String,
    pub test: String,
    pub outcome: Outcome,
    /// The ids of the unprobed assumptions it ran through (a pass with any is a flagged pass).
    pub assumptions: Vec<String>,
    /// What it raised and touched: numbers of `Index::keys`, sorted. The first test of a codeunit also holds what
    /// ran before it.
    pub keys: Vec<u32>,
}

/// What `al-runner impact-index build` keeps.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Index {
    /// The apps of the index run's compile: (id, name, version).
    pub apps: Vec<(String, String, String)>,
    /// The keys the tests' numbers stand for.
    pub keys: Vec<String>,
    /// What the install baseline raised and touched; `None` when no baseline ran.
    pub baseline: Option<Vec<u32>>,
    pub tests: Vec<IndexedTest>,
}

impl Index {
    /// The keys of a test, as text.
    pub fn keys_of(&self, t: &IndexedTest) -> Vec<String> {
        t.keys.iter().map(|&k| self.keys[k as usize].clone()).collect()
    }

    pub fn baseline_keys(&self) -> Option<Vec<String>> {
        self.baseline
            .as_ref()
            .map(|b| b.iter().map(|&k| self.keys[k as usize].clone()).collect())
    }

    /// The test as a result of the "without" side of an impact run.
    pub fn result(&self, t: &IndexedTest) -> TestResult {
        TestResult {
            suite: 0,
            app: t.app,
            codeunit: t.codeunit,
            codeunit_name: t.codeunit_name.clone(),
            test: t.test.clone(),
            outcome: t.outcome.clone(),
            call_stack: String::new(),
            assumptions: t
                .assumptions
                .iter()
                .filter_map(|id| al_builtins::assumptions::by_id(id))
                .collect(),
            time: std::time::Duration::ZERO,
            coverage: Vec::new(),
            messages: Vec::new(),
            captured: None,
            impact: t.keys.clone(),
            stubs: Vec::new(),
            entered: Vec::new(),
            dotnet: false,
        }
    }

    pub fn to_json(&self) -> Value {
        // Each test's keys as differences from the one before: small numbers that compress well.
        let delta = |keys: &[u32]| -> Vec<u32> {
            let mut last = 0;
            keys.iter()
                .map(|&k| {
                    let d = k - last;
                    last = k;
                    d
                })
                .collect()
        };
        json!({
            "format": FORMAT,
            "apps": self.apps.iter().map(|(id, name, version)| json!([id, name, version])).collect::<Vec<_>>(),
            "keys": self.keys,
            "baseline": self.baseline.as_deref().map(delta),
            "tests": self.tests.iter().map(|t| {
                let (kind, message) = match &t.outcome {
                    Outcome::Pass => ("pass", ""),
                    Outcome::Fail(m) => ("fail", m.as_str()),
                    Outcome::NotRun(m) => ("notrun", m.as_str()),
                };
                json!({
                    "app": t.app,
                    "codeunit": t.codeunit,
                    "codeunitName": t.codeunit_name,
                    "test": t.test,
                    "outcome": kind,
                    "message": message,
                    "assumptions": t.assumptions,
                    "keys": delta(&t.keys),
                })
            }).collect::<Vec<_>>(),
        })
    }

    pub fn from_json(v: &Value) -> Result<Index, String> {
        if v["format"].as_u64() != Some(FORMAT) {
            return Err(format!(
                "the index has format {}, this runtime reads format {FORMAT}",
                v["format"]
            ));
        }
        let text = |v: &Value| v.as_str().unwrap_or_default().to_string();
        let undelta = |v: &Value| -> Vec<u32> {
            let mut last = 0u32;
            v.as_array()
                .into_iter()
                .flatten()
                .map(|d| {
                    last += d.as_u64().unwrap_or(0) as u32;
                    last
                })
                .collect()
        };
        let apps = v["apps"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|a| (text(&a[0]), text(&a[1]), text(&a[2])))
            .collect();
        let keys: Vec<String> = v["keys"].as_array().into_iter().flatten().map(text).collect();
        let baseline = (!v["baseline"].is_null()).then(|| undelta(&v["baseline"]));
        let mut tests = Vec::new();
        for t in v["tests"].as_array().into_iter().flatten() {
            let message = text(&t["message"]);
            let outcome = match t["outcome"].as_str() {
                Some("pass") => Outcome::Pass,
                Some("fail") => Outcome::Fail(message),
                Some("notrun") => Outcome::NotRun(message),
                other => {
                    return Err(format!(
                        "the index has a test with outcome `{}`",
                        other.unwrap_or_default()
                    ));
                }
            };
            let keys_of = undelta(&t["keys"]);
            if keys_of.iter().any(|&k| k as usize >= keys.len()) {
                return Err("the index has a test with a key it does not list".into());
            }
            tests.push(IndexedTest {
                app: t["app"].as_u64().unwrap_or(0) as usize,
                codeunit: t["codeunit"].as_u64().unwrap_or(0) as u32,
                codeunit_name: text(&t["codeunitName"]),
                test: text(&t["test"]),
                outcome,
                assumptions: t["assumptions"].as_array().into_iter().flatten().map(text).collect(),
                keys: keys_of,
            });
        }
        Ok(Index {
            apps,
            keys,
            baseline,
            tests,
        })
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gz.write_all(self.to_json().to_string().as_bytes())
            .expect("writing to memory");
        gz.finish().expect("writing to memory")
    }

    pub fn decode(bytes: &[u8]) -> Result<Index, String> {
        let mut text = String::new();
        flate2::read::GzDecoder::new(bytes)
            .read_to_string(&mut text)
            .map_err(|e| format!("the index is not readable: {e}"))?;
        let v: Value = serde_json::from_str(&text).map_err(|e| format!("the index is not JSON: {e}"))?;
        Index::from_json(&v)
    }
}

/// Hashes the files of a folder (or one file), in path order.
fn hash_tree(path: &Path, h: &mut impl Hasher) -> Result<(), String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if meta.is_dir() {
        let mut kids: Vec<PathBuf> = std::fs::read_dir(path)
            .map_err(|e| format!("{}: {e}", path.display()))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .collect();
        kids.sort();
        for k in kids {
            k.file_name().hash(h);
            hash_tree(&k, h)?;
        }
    } else {
        std::fs::read(path)
            .map_err(|e| format!("{}: {e}", path.display()))?
            .hash(h);
    }
    Ok(())
}

/// The cache key of the index of the run `a` (its APPs are the test apps): see the module text.
pub fn key(a: &TestArgs) -> Result<String, String> {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    FORMAT.hash(&mut h);
    al_testrun::compile_cache::runtime_build().hash(&mut h);
    for p in &a.apps {
        let input = apps::load(p, a.bc_version.as_deref())?;
        input.source.id.hash(&mut h);
        input.manifest.version.hash(&mut h);
        let mut files: Vec<&(String, String)> = input.source.files.iter().collect();
        files.sort();
        files.hash(&mut h);
    }
    a.country().hash(&mut h);
    a.bc_version.hash(&mut h);
    (
        a.wants_test_data(),
        a.no_test_data,
        &a.test_data_path,
        &a.test_data_company,
        a.company_package,
        &a.package_path,
        &a.company_name,
    )
        .hash(&mut h);
    a.isolation.hash(&mut h);
    a.defines.hash(&mut h);
    a.environment.as_ref().map(std::mem::discriminant).hash(&mut h);
    (&a.server_url, a.test_timeout_secs).hash(&mut h);
    if let Some(dir) = &a.disabled_tests_dir {
        hash_tree(dir, &mut h).map_err(|e| format!("--disabled-tests {e}"))?;
    }
    Ok(format!("{:016x}", h.finish()))
}

/// Where the index of the run `a` is kept.
pub fn path(a: &TestArgs) -> Result<PathBuf, String> {
    let root = al_deps::cache::cache_root_from_env(a.cache_dir.as_deref());
    Ok(root.join("impact").join(key(a)?).join("index.json.gz"))
}

/// The cached index of the run `a`, if there is one this runtime reads. A file that cannot be read is reported and
/// treated as missing.
pub fn lookup(a: &TestArgs) -> Result<Option<(Index, PathBuf)>, String> {
    let path = path(a)?;
    let Ok(bytes) = std::fs::read(&path) else {
        return Ok(None);
    };
    match Index::decode(&bytes) {
        Ok(i) => Ok(Some((i, path))),
        Err(e) => {
            eprintln!("warning: impact index {} is not used: {e}; rebuild it", path.display());
            Ok(None)
        }
    }
}

/// Writes by a temporary file and a rename, as the compile cache does.
fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let dir = path.parent().expect("the index has a folder");
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let tmp = dir.join(format!(".index.tmp-{}", std::process::id()));
    let written = std::fs::write(&tmp, bytes).and_then(|()| std::fs::rename(&tmp, path));
    if written.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    written.map_err(|e| format!("{}: {e}", path.display()))
}

/// The index of a run's results.
pub fn of_run(c: &Collected) -> Index {
    let tests = c
        .results
        .iter()
        .map(|r| IndexedTest {
            app: r.app,
            codeunit: r.codeunit,
            codeunit_name: r.codeunit_name.clone(),
            test: r.test.clone(),
            outcome: r.outcome.clone(),
            assumptions: r.assumptions.iter().map(|a| a.info().id.to_string()).collect(),
            keys: r.impact.clone(),
        })
        .collect();
    Index {
        apps: c.program_apps.clone(),
        keys: c.impact_keys.clone(),
        baseline: c.impact_baseline.clone(),
        tests,
    }
}

/// `al-runner impact-index build APP...`: runs the tests of the APPs with the trace on and writes the index.
pub fn build(a: &TestArgs) -> Result<u8, String> {
    // The index must hold every test codeunit an impact run of these APPs can run.
    if !a.codeunits.is_empty() || !a.filters.is_empty() || !a.exact.is_empty() || !a.excludes.is_empty() {
        return Err("al-runner impact-index build runs every test of the APPs: --codeunit, --test, --exact and --exclude do not apply (an impact run selects from the whole index)".into());
    }
    build_run(a)
}

/// Builds the index of the run `a` as given: one lane of a test app of the `microsoft` source names its own
/// codeunits (ADR-052 1.4, `crate::sources::build_index`).
pub fn build_run(a: &TestArgs) -> Result<u8, String> {
    let started = Instant::now();
    let path = path(a)?;
    let mut side = a.clone();
    side.quiet = !a.verbose;
    // Microsoft's disabled tests are left out of impact runs, and so of their index (ADR-043 owner decision 2).
    side.parity = a.disabled_tests_dir.is_some();
    side.test_apps = Some(a.apps.len());
    println!("building the impact index of {} test app(s)", a.apps.len());
    let mut c = Collected {
        impact_trace: true,
        ..Default::default()
    };
    let code = run::test_collect(&side, &mut c)?;
    if !matches!(code, 0 | 1 | 7 | 8) {
        return Ok(code);
    }
    let index = of_run(&c);
    let bytes = index.encode();
    write(&path, &bytes)?;
    let codeunits: std::collections::BTreeSet<(usize, u32)> = index.tests.iter().map(|t| (t.app, t.codeunit)).collect();
    let keys: usize = index.tests.iter().map(|t| t.keys.len()).sum();
    println!(
        "impact index: {} tests in {} codeunits, {} distinct keys ({:.0} per test on average), {} bytes, built in {:.1} s: {}",
        index.tests.len(),
        codeunits.len(),
        index.keys.len(),
        keys as f64 / index.tests.len().max(1) as f64,
        bytes.len(),
        started.elapsed().as_secs_f64(),
        path.display()
    );
    Ok(0)
}

/// `al-runner impact-index check-static APP... [OPTIONS]` (ADR-043 step 5, the cross-check): compiles the APPs as the
/// index's run did, runs nothing, and checks that every key each test recorded in the cached index is in the test's
/// static reach (with "cannot rule out" sites). A key outside it is a bug in static reach: exit 1. Also prints how
/// much more static reach selects than the index, per test and per codeunit.
pub fn check_static(a: &TestArgs) -> Result<u8, String> {
    use std::sync::{Arc, Mutex};
    let mut side = a.clone();
    side.quiet = !a.verbose;
    side.parity = a.disabled_tests_dir.is_some();
    side.test_apps = Some(a.apps.len());
    let Some((index, path)) = lookup(&side)? else {
        return Err("no impact index for these test apps and options: build it first with al-runner impact-index build and the same APPs and options".into());
    };
    println!("checking static reach against the impact index {}", path.display());
    let (index, verbose, tests) = (Arc::new(index), a.verbose, a.apps.len());
    let verdict: Arc<Mutex<Option<u8>>> = Arc::new(Mutex::new(None));
    let (ix, out) = (index.clone(), verdict.clone());
    let mut c = Collected {
        narrow: Some(al_testrun::runtime::Narrow(Arc::new(
            move |input: &al_testrun::runtime::NarrowInput| {
                *out.lock().expect("the verdict") = Some(cross_check(&ix, input, tests, verbose));
                Some(Vec::new())
            },
        ))),
        ..Default::default()
    };
    let code = run::test_collect(&side, &mut c)?;
    Ok(verdict.lock().expect("the verdict").take().unwrap_or(code))
}

/// The cross-check of `check_static` over the compiled program; returns the exit code.
fn cross_check(index: &Index, input: &al_testrun::runtime::NarrowInput, tests: usize, verbose: bool) -> u8 {
    use al_testrun::impact_static::{Graph, key_node, test_roots};
    use std::collections::{BTreeMap, HashMap, HashSet};
    let started = Instant::now();
    let program = input.program;
    let graph = Graph::build(input.lowered);
    let built = started.elapsed();
    let routine_of: HashMap<(usize, u32, String), (usize, usize)> = crate::impact::test_routines(input, tests)
        .into_iter()
        .map(|(u, t, n)| ((u.0, u.1, n.to_lowercase()), t))
        .collect();
    let nodes: Vec<Option<usize>> = index.keys.iter().map(|k| key_node(&graph, program, k)).collect();
    let kind = |k: &str| k.split(' ').next().unwrap_or_default().to_string();
    let mut missed: Vec<String> = Vec::new();
    let mut no_node: BTreeMap<String, usize> = BTreeMap::new();
    let mut not_found = 0;
    let mut checked = 0;
    // Per key: tests the index selects, tests static reach reaches, tests it cannot rule out; and the same per codeunit.
    let n = index.keys.len();
    let (mut dynamic, mut precise, mut possible) = (vec![0usize; n], vec![0usize; n], vec![0usize; n]);
    let mut units: [Vec<HashSet<(usize, u32)>>; 3] = [
        vec![HashSet::new(); n],
        vec![HashSet::new(); n],
        vec![HashSet::new(); n],
    ];
    for t in &index.tests {
        let Some(&(o, r)) = routine_of.get(&(t.app, t.codeunit, t.test.to_lowercase())) else {
            not_found += 1;
            continue;
        };
        checked += 1;
        let unit = (t.app, t.codeunit);
        let roots: Vec<usize> = test_roots(program, o, r)
            .into_iter()
            .map(|(o, r)| graph.routine_node(o, r))
            .collect();
        let reach = graph.reached_from(&roots);
        for &k in &t.keys {
            let k = k as usize;
            dynamic[k] += 1;
            units[0][k].insert(unit);
            match nodes[k] {
                Some(node) if !reach.possible[node] => missed.push(format!(
                    "{} {}.{}: {}",
                    t.codeunit, t.codeunit_name, t.test, index.keys[k]
                )),
                Some(_) => {}
                None => *no_node.entry(kind(&index.keys[k])).or_default() += 1,
            }
        }
        for (k, node) in nodes.iter().enumerate() {
            let Some(node) = *node else { continue };
            if reach.precise[node] {
                precise[k] += 1;
                units[1][k].insert(unit);
            }
            if reach.possible[node] {
                possible[k] += 1;
                units[2][k].insert(unit);
            }
        }
    }
    // `AL_IMPACT_WHY=<key>`: why the install baseline and a test that did not record the key reach it statically.
    if let Ok(key) = std::env::var("AL_IMPACT_WHY") {
        why(index, input, &graph, &routine_of, &key);
    }
    if missed.is_empty() {
        println!("static reach covers every key of {checked} tests: missed: 0");
    } else {
        println!(
            "static reach misses {} key(s) tests recorded (a bug in static reach): missed: {}",
            missed.len(),
            missed.len()
        );
        let limit = if verbose { usize::MAX } else { 30 };
        for m in missed.iter().take(limit) {
            println!("  {m}");
        }
        if missed.len() > limit {
            println!("  ... {} more (--verbose lists them)", missed.len() - limit);
        }
    }
    if not_found > 0 {
        println!("  {not_found} test(s) of the index are not test routines of this compile");
    }
    if !no_node.is_empty() {
        let list: Vec<String> = no_node.iter().map(|(k, n)| format!("{k} {n}")).collect();
        println!(
            "  keys with no node in the static graph (test-key pairs, not checked): {}",
            list.join(", ")
        );
    }
    // How much more static reach selects, summed over the keys of a kind (each key standing for an impact point).
    println!(
        "selected per key, summed over the keys of each kind: the index / static reach / with \"cannot rule out\""
    );
    let mut kinds: BTreeMap<String, [usize; 6]> = BTreeMap::new();
    for (k, key) in index.keys.iter().enumerate() {
        if nodes[k].is_none() || dynamic[k] == 0 {
            continue;
        }
        let e = kinds.entry(kind(key)).or_default();
        for (i, v) in [
            dynamic[k],
            precise[k],
            possible[k],
            units[0][k].len(),
            units[1][k].len(),
            units[2][k].len(),
        ]
        .into_iter()
        .enumerate()
        {
            e[i] += v;
        }
    }
    let ratio = |a: usize, b: usize| a as f64 / b.max(1) as f64;
    for (k, v) in &kinds {
        println!(
            "  {k}: tests {} / {} ({:.1}x) / {} ({:.1}x); codeunits {} / {} ({:.1}x) / {} ({:.1}x)",
            v[0],
            v[1],
            ratio(v[1], v[0]),
            v[2],
            ratio(v[2], v[0]),
            v[3],
            v[4],
            ratio(v[4], v[3]),
            v[5],
            ratio(v[5], v[3])
        );
    }
    println!(
        "static graph: {} nodes, built in {:.1} s; check of {checked} tests and {} keys in {:.1} s",
        graph.len(),
        built.as_secs_f64(),
        n,
        started.elapsed().as_secs_f64()
    );
    u8::from(!missed.is_empty())
}

/// Prints a shortest static path to the node of `key` from the install baseline and from the first test that did not
/// record the key (`AL_IMPACT_WHY`, a diagnostic of over-selection).
fn why(
    index: &Index,
    input: &al_testrun::runtime::NarrowInput,
    graph: &al_testrun::impact_static::Graph,
    routine_of: &std::collections::HashMap<(usize, u32, String), (usize, usize)>,
    key: &str,
) {
    use al_testrun::impact_static::{baseline_roots, key_node, test_roots};
    let program = input.program;
    let Some(node) = key_node(graph, program, key) else {
        println!("AL_IMPACT_WHY: {key} has no node in the static graph");
        return;
    };
    let show = |label: &str, roots: Vec<(usize, usize)>| {
        let from: Vec<usize> = roots.into_iter().map(|(o, r)| graph.routine_node(o, r)).collect();
        let path = graph
            .path(&from, node, false)
            .map(|p| (p, "reaches"))
            .or_else(|| graph.path(&from, node, true).map(|p| (p, "cannot rule out")));
        match path {
            Some((p, how)) => {
                println!("AL_IMPACT_WHY {key}: {label} {how}");
                for n in p {
                    println!("    {}", graph.describe(program, n));
                }
            }
            None => println!("AL_IMPACT_WHY {key}: {label} does not reach it"),
        }
    };
    show("the install baseline", baseline_roots(program, |_| true));
    let k = index.keys.iter().position(|k| k == key);
    // Tests and codeunits: recorded in the index / reached statically / with "cannot rule out".
    let mut counts = [0usize; 3];
    let mut units: [std::collections::HashSet<(usize, u32)>; 3] = Default::default();
    for t in &index.tests {
        let Some(&(o, r)) = routine_of.get(&(t.app, t.codeunit, t.test.to_lowercase())) else {
            continue;
        };
        let from: Vec<usize> = test_roots(program, o, r)
            .iter()
            .map(|&(o, r)| graph.routine_node(o, r))
            .collect();
        let reach = graph.reached_from(&from);
        for (i, hit) in [
            k.is_some_and(|k| t.keys.contains(&(k as u32))),
            reach.precise[node],
            reach.possible[node],
        ]
        .into_iter()
        .enumerate()
        {
            if hit {
                counts[i] += 1;
                units[i].insert((t.app, t.codeunit));
            }
        }
    }
    println!(
        "AL_IMPACT_WHY {key}: tests {} / {} / {}, codeunits {} / {} / {} (the index / static reach / with \"cannot rule out\")",
        counts[0],
        counts[1],
        counts[2],
        units[0].len(),
        units[1].len(),
        units[2].len()
    );
    for t in &index.tests {
        if k.is_some_and(|k| t.keys.contains(&(k as u32))) {
            continue;
        }
        let Some(&(o, r)) = routine_of.get(&(t.app, t.codeunit, t.test.to_lowercase())) else {
            continue;
        };
        let roots = test_roots(program, o, r);
        let from: Vec<usize> = roots.iter().map(|&(o, r)| graph.routine_node(o, r)).collect();
        if graph.path(&from, node, false).is_some() {
            show(
                &format!("test {}.{} (it did not record the key)", t.codeunit_name, t.test),
                roots,
            );
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_index_reads_back_what_was_written() {
        let index = Index {
            apps: vec![("id".into(), "Base".into(), "27.5.0.0".into())],
            keys: vec![
                "table 18".into(),
                "event codeunit 80 onbeforepost".into(),
                "write 18".into(),
            ],
            baseline: Some(vec![0, 2]),
            tests: vec![
                IndexedTest {
                    app: 0,
                    codeunit: 134000,
                    codeunit_name: "ERM Tests".into(),
                    test: "Posts".into(),
                    outcome: Outcome::Pass,
                    assumptions: vec!["some-id".into()],
                    keys: vec![0, 1, 2],
                },
                IndexedTest {
                    app: 1,
                    codeunit: 134001,
                    codeunit_name: "ERM Other".into(),
                    test: "Fails".into(),
                    outcome: Outcome::Fail("Assert.AreEqual failed".into()),
                    assumptions: vec![],
                    keys: vec![],
                },
            ],
        };
        assert_eq!(Index::decode(&index.encode()).unwrap(), index);
        // Another format is not read.
        let mut v = index.to_json();
        v["format"] = json!(FORMAT + 1);
        assert!(Index::from_json(&v).is_err());
    }
}
