//! The safety check of `--affected` (ADR-050 decision 12): for a set of edits of the corpus app, the affected
//! selection plus the stored outcomes of the codeunits it skips must equal a full run, test by test. Any difference is
//! a selection miss: `--affected` would have skipped a test whose result changed.
//!
//! Per edit the app runs twice: once narrowed as `--affected` narrows it (the rule against the stored result of the
//! unedited app), once in full. A selected codeunit must end in the narrowed run as in the full run; a skipped one
//! must have its stored outcomes in the full run.
//!
//! The edits are a deterministic sample of the corpus app's routines (an `Error` call at the start of the body, a
//! changed integer literal, a statement that changes nothing when it runs, a blank line above the object) and one
//! blank line above the first object of each object kind. `AL_AFFECTED_DIFF_EDITS` sets the number of routine edits
//! (default 8). The corpus test runs in release only (`--ignored`); the small test next to it checks that the harness
//! finds a miss.

use al_sema::workspace::AppSource;
use al_test_cli::runtime::{self, CorpusRuntime, Narrow, NarrowInput, RunOptions, SuiteRun};
use al_testrun::Outcome;
use al_testrun::affected::{Current, Header, Selection, Stored, TestCodeunits, facts, ran_codeunits};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

type Key = (String, u32);
type Outcomes = BTreeMap<Key, Vec<(String, Outcome)>>;
/// The rule under test: the stored result and the current compile's facts and codeunits to a selection.
type Rule = dyn Fn(&Stored, &Current) -> Selection + Send + Sync;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// The apps of one differential: the workspace apps (all of kind `Workspace`), their manifests, and the caller's
/// index of the app whose test codeunits run and whose routines are edited.
struct Apps {
    apps: Vec<AppSource>,
    manifests: Vec<runtime::Manifest>,
    symbols: BTreeSet<String>,
    tested: usize,
}

/// One run of `apps`; with `narrow`, the callback decides which test codeunits run.
fn run(a: &Apps, apps: Vec<AppSource>, narrow: Option<Narrow>) -> CorpusRuntime {
    let opts = RunOptions {
        object_hashes: true,
        impact_trace: true,
        narrow,
        stop_on_compile_errors: false,
        ..Default::default()
    };
    let kinds = vec![al_sema::AppKind::Workspace; apps.len()];
    CorpusRuntime::run_suites_with(
        apps,
        kinds,
        &a.manifests,
        &a.symbols,
        None,
        Default::default(),
        &[SuiteRun {
            name: String::new(),
            app: Some(a.tested),
            runner: None,
        }],
        &opts,
    )
}

fn outcomes(rt: &CorpusRuntime, ids: &[String]) -> Outcomes {
    let mut out = Outcomes::new();
    for r in rt.results() {
        out.entry((ids[r.app].clone(), r.codeunit))
            .or_default()
            .push((r.test.clone(), r.outcome.clone()));
    }
    out
}

/// The stored result of a full run of the unedited apps, as `--affected` writes it.
fn stored_of(rt: &CorpusRuntime, objects: Vec<al_testrun::affected::ObjectFacts>, ids: &[String]) -> Stored {
    let (ran, _) = ran_codeunits(rt.results(), rt.impact_keys(), |app| ids.get(app).cloned());
    let existing: Vec<Key> = ran.iter().map(|c| c.id_key()).collect();
    Stored::default().merge(Header::default(), objects, &existing, ran, true)
}

/// An edit of one file of the tested app.
#[derive(Debug, Clone)]
struct Edit {
    file: usize,
    what: String,
    text: String,
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// The routines of a file as (header line, `begin` line, `end;` line), by their indentation.
fn routines(text: &str) -> Vec<(usize, usize, usize)> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        let t = l.trim_start().to_ascii_lowercase();
        let header = ["procedure ", "local procedure ", "internal procedure ", "trigger "]
            .iter()
            .any(|p| t.starts_with(p));
        if !header {
            continue;
        }
        let ind = indent(l);
        let Some(begin) = (i + 1..lines.len()).find_map(|j| {
            let t = lines[j].trim();
            if t.eq_ignore_ascii_case("begin") && indent(lines[j]) == ind {
                return Some(Some(j));
            }
            let stop = !t.is_empty()
                && indent(lines[j]) <= ind
                && !t.eq_ignore_ascii_case("var")
                && !t.to_ascii_lowercase().starts_with("var ");
            stop.then_some(None)
        }) else {
            continue;
        };
        let Some(begin) = begin else { continue };
        let Some(end) = (begin + 1..lines.len())
            .find(|&j| indent(lines[j]) == ind && lines[j].trim().to_ascii_lowercase().starts_with("end;"))
        else {
            continue;
        };
        out.push((i, begin, end));
    }
    out
}

fn with_line(text: &str, at: usize, insert: &str) -> String {
    let mut lines: Vec<&str> = text.lines().collect();
    lines.insert(at, insert);
    lines.join("\n") + "\n"
}

/// `exit(N)` or `:= N;` in the line, with N one larger.
fn bump_literal(line: &str) -> Option<String> {
    for (open, close) in [("exit(", ")"), (":= ", ";")] {
        let Some(at) = line.find(open) else { continue };
        let rest = &line[at + open.len()..];
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if digits.is_empty() || !rest[digits.len()..].starts_with(close) || digits.len() > 9 {
            continue;
        }
        let n: u64 = digits.parse().ok()?;
        return Some(format!(
            "{}{}{}",
            &line[..at + open.len()],
            n + 1,
            &rest[digits.len()..]
        ));
    }
    None
}

/// The first line of the first object declaration of a file, with its kind.
fn first_object(text: &str) -> Option<(usize, String)> {
    const KINDS: &[&str] = &[
        "codeunit",
        "table",
        "tableextension",
        "page",
        "pageextension",
        "report",
        "reportextension",
        "enum",
        "enumextension",
        "query",
        "xmlport",
        "interface",
        "permissionset",
        "permissionsetextension",
        "profile",
        "controladdin",
        "entitlement",
    ];
    text.lines().enumerate().find_map(|(i, l)| {
        let first = l.split_whitespace().next()?.to_ascii_lowercase();
        KINDS.contains(&first.as_str()).then_some((i, first))
    })
}

/// The edits of the tested app: `routine_edits` routines spread evenly over all of them, and one blank line above the
/// first object of each kind.
fn edits(app: &AppSource, routine_edits: usize) -> Vec<Edit> {
    let mut files: Vec<usize> = (0..app.files.len()).collect();
    files.sort_by(|&a, &b| app.files[a].0.cmp(&app.files[b].0));
    let all: Vec<(usize, (usize, usize, usize))> = files
        .iter()
        .flat_map(|&f| routines(&app.files[f].1).into_iter().map(move |r| (f, r)))
        .collect();
    let mut out = Vec::new();
    let step = (all.len() / routine_edits.max(1)).max(1);
    for (n, &(f, (header, begin, end))) in all.iter().step_by(step).take(routine_edits).enumerate() {
        let (name, text) = &app.files[f];
        let lines: Vec<&str> = text.lines().collect();
        let pad = " ".repeat(indent(lines[begin]) + 4);
        let routine = lines[header].trim();
        let insert = |what: &str, stmt: &str| Edit {
            file: f,
            what: format!("{what} in {name}:{} `{routine}`", header + 1),
            text: with_line(text, begin + 1, &format!("{pad}{stmt}")),
        };
        let edit = match n % 4 {
            0 => insert("an Error call", "Error('affected differential edit');"),
            1 => (begin + 1..end)
                .find_map(|j| bump_literal(lines[j]).map(|l| (j, l)))
                .map(|(j, l)| {
                    let mut lines = lines.clone();
                    lines[j] = &l;
                    Edit {
                        file: f,
                        what: format!("a changed literal in {name}:{} `{routine}`", j + 1),
                        text: lines.join("\n") + "\n",
                    }
                })
                .unwrap_or_else(|| insert("an Error call", "Error('affected differential edit');")),
            2 => insert("a statement that changes nothing", "if 1 = 0 then Error('');"),
            _ => match first_object(text) {
                Some((at, _)) => Edit {
                    file: f,
                    what: format!("a blank line above the object of {name}"),
                    text: with_line(text, at, ""),
                },
                None => continue,
            },
        };
        out.push(edit);
    }
    let mut kinds = BTreeSet::new();
    for &f in &files {
        let (name, text) = &app.files[f];
        if let Some((at, kind)) = first_object(text) {
            if kinds.insert(kind.clone()) {
                out.push(Edit {
                    file: f,
                    what: format!("a blank line above the {kind} of {name}"),
                    text: with_line(text, at, ""),
                });
            }
        }
    }
    out
}

/// What a differential found.
#[derive(Debug, Default)]
struct Found {
    /// One line per selection miss: edit, codeunit, test, the outcome kept or narrowed, the full run's.
    misses: Vec<String>,
    edits: usize,
    /// Edits whose selection was not a full run, and the codeunits they skipped in all.
    narrowed: usize,
    skipped: usize,
    /// Tests whose outcome in the full run after a narrowed edit differs from the unedited run: what the edits broke.
    changed: usize,
    /// Tests whose outcome differs between two runs of the unedited apps; they are not compared.
    unstable: usize,
}

fn differential(a: &Apps, edits: &[Edit], rule: Arc<Rule>) -> Found {
    let ids: Vec<String> = a.apps.iter().map(|s| s.id.to_lowercase()).collect();
    let seen: Arc<Mutex<Option<Vec<al_testrun::affected::ObjectFacts>>>> = Arc::default();
    let out = seen.clone();
    let record = Narrow(Arc::new(move |input: &NarrowInput| {
        *out.lock().unwrap() = Some(facts(input.lowered));
        None
    }));
    let first = run(a, a.apps.clone(), Some(record));
    let objects = seen.lock().unwrap().take().expect("the narrow callback ran");
    let stored = Arc::new(stored_of(&first, objects, &ids));
    let base = outcomes(&first, &ids);
    let again = outcomes(&run(a, a.apps.clone(), None), &ids);
    let mut unstable: BTreeSet<(Key, String)> = BTreeSet::new();
    for (k, tests) in &base {
        for (t, o) in tests {
            let other = again
                .get(k)
                .and_then(|v| v.iter().find(|(n, _)| n == t))
                .map(|(_, o)| o);
            if other != Some(o) {
                unstable.insert((k.clone(), t.clone()));
            }
        }
    }
    let mut found = Found {
        unstable: unstable.len(),
        ..Default::default()
    };
    let workspace = a.apps.len();
    for e in edits {
        let mut apps = a.apps.clone();
        apps[a.tested].files[e.file].1 = e.text.clone();
        let decided: Arc<Mutex<Option<(Selection, Vec<Key>)>>> = Arc::default();
        let (d, s, r) = (decided.clone(), stored.clone(), rule.clone());
        let narrow = Narrow(Arc::new(move |input: &NarrowInput| {
            let tc = TestCodeunits::of(input, workspace);
            let objects = facts(input.lowered);
            let header = Header::default();
            let selection = r(
                &s,
                &Current {
                    header: &header,
                    objects: &objects,
                    codeunits: &tc.codeunits,
                    include_failing: false,
                    stubbed: &[],
                },
            );
            let only = tc.narrow(&selection);
            *d.lock().unwrap() = Some((selection, tc.codeunits));
            only
        }));
        let narrowed = outcomes(&run(a, apps.clone(), Some(narrow)), &ids);
        let (selection, codeunits) = decided.lock().unwrap().take().expect("the narrow callback ran");
        found.edits += 1;
        if selection.full.is_some() {
            continue;
        }
        found.narrowed += 1;
        let full = outcomes(&run(a, apps, None), &ids);
        let kept: BTreeMap<Key, &Vec<(String, Outcome)>> =
            stored.codeunits.iter().map(|c| (c.id_key(), &c.tests)).collect();
        for k in &codeunits {
            let ran = selection.run.contains(k);
            found.skipped += usize::from(!ran);
            let ours: Vec<(String, Outcome)> = if ran {
                narrowed.get(k).cloned().unwrap_or_default()
            } else {
                kept.get(k).map(|v| (*v).clone()).unwrap_or_default()
            };
            let theirs = full.get(k).cloned().unwrap_or_default();
            found.changed += theirs
                .iter()
                .filter(|(n, o)| base.get(k).and_then(|v| v.iter().find(|(b, _)| b == n)).map(|(_, b)| b) != Some(o))
                .count();
            let names: BTreeSet<&String> = ours.iter().chain(&theirs).map(|(n, _)| n).collect();
            for n in names {
                if unstable.contains(&(k.clone(), n.clone())) {
                    continue;
                }
                let get = |v: &[(String, Outcome)]| v.iter().find(|(t, _)| t == n).map(|(_, o)| format!("{o:?}"));
                let (o, t) = (get(&ours), get(&theirs));
                if o != t {
                    found.misses.push(format!(
                        "{}: codeunit {} test {n}: {} {}, full run {}",
                        e.what,
                        k.1,
                        if ran { "narrowed run" } else { "kept" },
                        o.unwrap_or_else(|| "absent".into()),
                        t.unwrap_or_else(|| "absent".into()),
                    ));
                }
            }
        }
    }
    found
}

/// The newest BC major of the oracle: the corpus runs as that major's symbols define.
fn newest_major() -> u16 {
    let text = std::fs::read_to_string(root().join("oracle/oracle.toml")).unwrap();
    let cfg = al_test_cli::config::OracleFile::parse(&text).unwrap();
    cfg.select(None).unwrap().major
}

fn the_rule() -> Arc<Rule> {
    Arc::new(|s: &Stored, c: &Current| s.select(c))
}

const SMALL: &str = r#"
codeunit 50500 "Helper"
{
    procedure Two(): Integer
    begin
        exit(2);
    end;
}
codeunit 50501 "Other"
{
    procedure Three(): Integer
    begin
        exit(3);
    end;
}
codeunit 50502 "Helper Tests"
{
    Subtype = Test;
    [Test]
    procedure UsesHelper()
    var
        H: Codeunit "Helper";
    begin
        if H.Two() <> 2 then Error('two');
    end;
}
codeunit 50503 "Other Tests"
{
    Subtype = Test;
    [Test]
    procedure UsesOther()
    var
        O: Codeunit "Other";
    begin
        if O.Three() <> 3 then Error('three');
    end;
}
"#;

fn small() -> Apps {
    Apps {
        apps: vec![AppSource {
            id: "A5".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), SMALL.into())],
        }],
        manifests: vec![runtime::Manifest::default()],
        symbols: BTreeSet::new(),
        tested: 0,
    }
}

#[test]
fn the_harness_finds_a_miss_and_the_rule_has_none_on_a_small_app() {
    let a = small();
    let edits = edits(&a.apps[0], 8);
    assert!(edits.iter().any(|e| e.what.starts_with("an Error call")), "{edits:#?}");
    // A rule that runs nothing after any change misses every test the change breaks.
    let none: Arc<Rule> = Arc::new(|_: &Stored, _: &Current| Selection::default());
    let broken = differential(&a, &edits, none);
    assert!(!broken.misses.is_empty(), "{broken:#?}");
    let found = differential(&a, &edits, the_rule());
    assert!(found.misses.is_empty(), "{:#?}", found.misses);
    assert!(found.narrowed > 0 && found.skipped > 0, "{found:#?}");
}

#[test]
#[ignore = "release: runs the corpus app twice per edit"]
fn the_affected_selection_misses_no_test_on_the_corpus_app() {
    // The compile of the corpus app needs the stack `al-test-cli` gives its main thread.
    std::thread::Builder::new()
        .stack_size(512 << 20)
        .spawn(corpus_differential)
        .unwrap()
        .join()
        .unwrap();
}

fn corpus_differential() {
    let tests = root().join("corpus/tests");
    let dirs = [tests.join("al-language-internals-fixture"), tests.join("al-language")];
    assert!(dirs[1].join("app.json").exists(), "corpus submodule not checked out");
    let a = Apps {
        apps: dirs.iter().map(|d| runtime::load_app(d).unwrap()).collect(),
        manifests: dirs.iter().map(|d| runtime::load_manifest(d).unwrap()).collect(),
        symbols: al_version::corpus_symbols(newest_major()),
        tested: 1,
    };
    let n = std::env::var("AL_AFFECTED_DIFF_EDITS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8);
    let edits = edits(&a.apps[1], n);
    let started = std::time::Instant::now();
    let found = differential(&a, &edits, the_rule());
    println!(
        "affected differential: {} edits, {} narrowed, {} codeunits skipped in all, {} test outcomes changed by the edits, {} unstable tests not compared, {} misses, {:.0} s",
        found.edits,
        found.narrowed,
        found.skipped,
        found.changed,
        found.unstable,
        found.misses.len(),
        started.elapsed().as_secs_f64()
    );
    assert!(
        found.misses.is_empty(),
        "selection misses:\n{}",
        found.misses.join("\n")
    );
    assert!(found.narrowed > 0, "no edit was narrowed: the check checked nothing");
    assert!(
        found.changed > 0,
        "no edit changed an outcome: the check checked nothing"
    );
}
