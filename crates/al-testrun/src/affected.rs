//! `--affected` (ADR-050): the stored result of the last run and the rule that picks, from it and the current
//! compile, the test codeunits a change can affect.
//!
//! Pure code: the stored result comes in as JSON and goes out as JSON; reading and writing the file is the command
//! line's (ADR-006). The rule never narrows a run it cannot vouch for: every such case is a full run with a reason
//! (ADR-050 decision 6), so a test whose result could change is never skipped.
//!
//! What a selection keys on:
//! - per workspace object, a hash of its symbolic IR with its source positions (decision 3): an object whose hash
//!   differs is changed;
//! - per test codeunit of the stored run, the trace keys its tests recorded (`impact_trace::keys`: events raised,
//!   tables, pages, reports and enums used) and the workspace objects whose code they entered (decision 4).

use crate::Outcome;
use al_ir::ObjKind;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// The format of the stored result. A file of another format is not read: the run is full.
pub const FORMAT: u64 = 1;

/// What the stored result must agree with before any of it is used (ADR-050 decision 10): a difference in any part is
/// a full run with the part's reason.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Header {
    /// The runtime build (`compile_cache::runtime_build`).
    pub runtime: String,
    /// A hash of the run options that change outcomes (country, BC version, company and test-data options,
    /// isolation, defines, environment options, disabled-test files, `--tdd`).
    pub options: String,
    /// The environment: each dependency app's id, version and content hash, and the BC build, as text, sorted.
    pub environment: Vec<String>,
    /// The workspace apps: id and version, in the order of the run.
    pub apps: Vec<(String, String)>,
    /// A hash of each workspace app's `app.json`, in the same order.
    pub manifests: Vec<String>,
}

/// What the rule needs to know of one workspace object, in the stored run or in the current compile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectFacts {
    /// The id of the declaring app, in lower case.
    pub app: String,
    pub kind: ObjKind,
    pub id: Option<u32>,
    pub name: String,
    /// The hash of its symbolic IR with its source positions (positions relative to the app root).
    pub hash: String,
    /// The trace keys whose use a change of this object can change besides entering its code: `table <id>` for a
    /// table and for the table a table extension extends, `page`, `report` and `enum` alike.
    pub uses: Vec<String>,
    /// The events its subscribers bind, as trace keys (`event <kind> <id> <event>` in lower case).
    pub subscribes: Vec<String>,
    /// It subscribes to an event of `Global Triggers`: any write to any table can run it.
    pub global_triggers: bool,
    /// It is install code: an install codeunit, or it subscribes to an event of `Extension Triggers` (the install
    /// baseline runs it once for all test codeunits, ADR-046).
    pub install: bool,
    /// It is a test codeunit.
    pub test: bool,
    /// The workspace objects (`key`) its symbolic form refers to.
    pub refers: Vec<String>,
    /// Why a change of it cannot be followed by the trace (a subscriber to an event whose raises are not noted, a
    /// test runner, an extension whose base is not in the program): a change of it is a full run.
    pub unfollowed: Vec<String>,
}

impl ObjectFacts {
    /// The identity of the object across runs: app, kind, and id or name.
    pub fn key(&self) -> String {
        match self.id {
            Some(id) => format!("{}/{} {id}", self.app, self.kind.label()),
            None => format!("{}/{} {}", self.app, self.kind.label(), self.name.to_lowercase()),
        }
    }

    /// The object as `changedObjects` lists it: `Kind id Name`.
    pub fn display(&self) -> String {
        match self.id {
            Some(id) => format!("{} {id} {}", self.kind.label(), self.name),
            None => format!("{} {}", self.kind.label(), self.name),
        }
    }
}

/// How the run of a test codeunit ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Ended {
    /// It ran to its end (each test passed, failed, or was not run for a reason decided before it started).
    #[default]
    Normal,
    /// A test hit the time limit, the runtime panicked, the run was cancelled, or a test was refused while it ran:
    /// what it recorded is not all it would reach.
    Cut,
}

/// One test codeunit of the stored run.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StoredCodeunit {
    /// The id of its app, in lower case.
    pub app: String,
    pub id: u32,
    pub name: String,
    /// What its tests (and what ran before its first test) raised and touched: `impact_trace::keys`, sorted.
    pub keys: Vec<String>,
    /// The workspace objects (`ObjectFacts::key`) whose code its tests entered, sorted.
    pub entered: Vec<String>,
    /// A test reached a `DotNet` value or a .NET static: state the trace does not key.
    pub dotnet: bool,
    /// `--tdd`: a test of it ran a stub (ADR-051); it runs on every run (decision 8).
    pub stubbed: bool,
    pub ended: Ended,
    /// Its tests in the order they ran, with their outcome.
    pub tests: Vec<(String, Outcome)>,
}

impl StoredCodeunit {
    pub fn id_key(&self) -> (String, u32) {
        (self.app.clone(), self.id)
    }

    /// Whether a test of it did not pass.
    pub fn failing(&self) -> bool {
        self.tests.iter().any(|(_, o)| *o != Outcome::Pass)
    }
}

/// The stored result of the last run with `--affected`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Stored {
    pub header: Header,
    pub objects: Vec<ObjectFacts>,
    pub codeunits: Vec<StoredCodeunit>,
}

/// What the current compile knows, for the rule.
#[derive(Debug, Clone)]
pub struct Current<'a> {
    pub header: &'a Header,
    pub objects: &'a [ObjectFacts],
    /// The test codeunits the run would run, as (app id in lower case, codeunit id).
    pub codeunits: &'a [(String, u32)],
    /// `--include-failing`: every codeunit with a test that did not pass last time runs.
    pub include_failing: bool,
    /// `--tdd`: the codeunits with a test that calls a stub; they run every time (decision 8).
    pub stubbed: &'a [(String, u32)],
}

/// The answer of the rule.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Selection {
    /// `Some(reason)`: every test codeunit runs (`forcedFull`).
    pub full: Option<String>,
    /// The test codeunits to run, as (app id, codeunit id). With `full`, every codeunit of `Current::codeunits`.
    pub run: BTreeSet<(String, u32)>,
    /// The changed objects as `Kind id Name`, sorted.
    pub changed: Vec<String>,
}

/// Table ids from this one up are the platform's: the metadata virtual tables (AllObj, Field, Key, ...) among them.
/// A test that read one reads rows about the objects, so any change of an object can change what it reads.
const PLATFORM_TABLES: u32 = 2_000_000_000;

/// The object kinds a change of which the trace can follow (decision 4). A change of any other kind is a full run.
fn covered(kind: ObjKind) -> bool {
    matches!(
        kind,
        ObjKind::Codeunit
            | ObjKind::Table
            | ObjKind::TableExtension
            | ObjKind::Page
            | ObjKind::PageExtension
            | ObjKind::Report
            | ObjKind::ReportExtension
            | ObjKind::Enum
            | ObjKind::EnumExtension
    )
}

/// Why the stored result cannot be used with the current header; `None` when it can.
pub fn header_mismatch(stored: &Header, current: &Header) -> Option<String> {
    if stored.runtime != current.runtime {
        return Some("the stored result was written by another runtime build".into());
    }
    if stored.apps != current.apps {
        return Some("the workspace apps (ids or versions) changed".into());
    }
    if stored.manifests != current.manifests {
        return Some("app.json changed".into());
    }
    if stored.options != current.options {
        return Some("a run option changed".into());
    }
    if stored.environment != current.environment {
        return Some("the dependency apps or the BC build changed".into());
    }
    None
}

impl Stored {
    /// The codeunits of the stored run with a test that ran a stub (`Current::stubbed`).
    pub fn stubbed(&self) -> Vec<(String, u32)> {
        self.codeunits
            .iter()
            .filter(|c| c.stubbed)
            .map(StoredCodeunit::id_key)
            .collect()
    }

    /// The test codeunits to run (ADR-050 decisions 4 to 8).
    pub fn select(&self, current: &Current) -> Selection {
        let all: BTreeSet<(String, u32)> = current.codeunits.iter().cloned().collect();
        let full = |reason: String, changed: Vec<String>| Selection {
            full: Some(reason),
            run: all.clone(),
            changed,
        };
        if let Some(reason) = header_mismatch(&self.header, current.header) {
            return full(reason, Vec::new());
        }
        let before: BTreeMap<String, &ObjectFacts> = self.objects.iter().map(|o| (o.key(), o)).collect();
        let now: BTreeMap<String, &ObjectFacts> = current.objects.iter().map(|o| (o.key(), o)).collect();
        if before.len() != self.objects.len() || now.len() != current.objects.len() {
            // Two objects with one key: the rule cannot tell their changes apart.
            return full("two workspace objects have the same kind and id".into(), Vec::new());
        }
        // The changed objects, with their facts before and after.
        let mut changed: Vec<(Option<&ObjectFacts>, Option<&ObjectFacts>)> = Vec::new();
        for (k, o) in &before {
            match now.get(k) {
                Some(n) if n.hash == o.hash => {}
                Some(n) => changed.push((Some(o), Some(n))),
                None => changed.push((Some(o), None)),
            }
        }
        for (k, n) in &now {
            if !before.contains_key(k) {
                changed.push((None, Some(n)));
            }
        }
        let mut names: Vec<String> = changed
            .iter()
            .map(|(b, n)| n.or(*b).expect("one side").display())
            .collect();
        names.sort();
        for (b, n) in &changed {
            let o = n.or(*b).expect("one side");
            match (b, n) {
                (Some(_), None) => return full(format!("an object was removed: {}", o.display()), names),
                // A new test codeunit just runs (owner decision 4); it is in `current.codeunits` and not stored.
                (None, Some(n)) if n.test && n.kind == ObjKind::Codeunit => {}
                (None, Some(_)) => return full(format!("an object was added: {}", o.display()), names),
                _ => {}
            }
            if !covered(o.kind) {
                return full(
                    format!(
                        "a changed {} is not followed by the trace: {}",
                        o.kind.label(),
                        o.display()
                    ),
                    names,
                );
            }
            if let Some(why) = [b, n].into_iter().flatten().find_map(|f| f.unfollowed.first()) {
                return full(
                    format!("a change of {} is not followed by the trace: {why}", o.display()),
                    names,
                );
            }
            if b.is_some_and(|f| f.install) || n.is_some_and(|f| f.install) {
                return full(format!("install code changed: {}", o.display()), names);
            }
        }
        if !changed.is_empty() && self.codeunits.iter().any(|c| c.dotnet) {
            return full(
                "a test reached a DotNet value, whose state the trace does not follow".into(),
                names,
            );
        }
        // What a change reaches: the changed objects, and the objects that refer to one (their lowering reads the
        // declarations of what they refer to).
        let changed_keys: BTreeSet<String> = changed.iter().map(|(b, n)| n.or(*b).expect("one side").key()).collect();
        let mut reached = changed_keys.clone();
        for o in current.objects {
            if o.refers.iter().any(|r| changed_keys.contains(r)) {
                reached.insert(o.key());
            }
        }
        let mut uses: BTreeSet<&str> = BTreeSet::new();
        for k in &reached {
            for f in [before.get(k), now.get(k)].into_iter().flatten() {
                uses.extend(f.uses.iter().map(String::as_str));
            }
        }
        let mut events: BTreeSet<&str> = BTreeSet::new();
        let mut global_triggers = false;
        for (b, n) in &changed {
            for f in [b, n].into_iter().flatten() {
                events.extend(f.subscribes.iter().map(String::as_str));
                global_triggers |= f.global_triggers;
            }
        }
        let stored: BTreeMap<(String, u32), &StoredCodeunit> = self.codeunits.iter().map(|c| (c.id_key(), c)).collect();
        let stubbed: BTreeSet<&(String, u32)> = current.stubbed.iter().collect();
        let mut run = BTreeSet::new();
        for cu in &all {
            let Some(s) = stored.get(cu) else {
                run.insert(cu.clone()); // new, or not recorded
                continue;
            };
            let refused = s.tests.iter().any(|(_, o)| matches!(o, Outcome::NotRun(_)));
            let selected = s.ended == Ended::Cut
                || refused
                || (current.include_failing && s.failing())
                || stubbed.contains(cu)
                || changed_keys.iter().any(|k| {
                    now.get(k).is_some_and(|n| {
                        n.test && n.kind == ObjKind::Codeunit && (n.app.clone(), n.id.unwrap_or(0)) == *cu
                    })
                })
                || s.entered.iter().any(|e| reached.contains(e))
                || s.keys
                    .iter()
                    .any(|k| uses.contains(k.as_str()) || events.contains(k.as_str()))
                || (global_triggers && s.keys.iter().any(|k| k.starts_with("write ")))
                || (!changed.is_empty() && s.keys.iter().any(|k| platform_table(k)));
            if selected {
                run.insert(cu.clone());
            }
        }
        Selection {
            full: None,
            run,
            changed: names,
        }
    }

    /// The stored result after a run: a full run replaces the codeunits; a narrowed one replaces those it ran and
    /// keeps the others that still exist. The header and the objects become the current ones.
    pub fn merge(
        self,
        header: Header,
        objects: Vec<ObjectFacts>,
        existing: &[(String, u32)],
        ran: Vec<StoredCodeunit>,
        full: bool,
    ) -> Stored {
        let ran_keys: BTreeSet<(String, u32)> = ran.iter().map(StoredCodeunit::id_key).collect();
        let existing: BTreeSet<&(String, u32)> = existing.iter().collect();
        let mut codeunits: Vec<StoredCodeunit> = if full {
            Vec::new()
        } else {
            self.codeunits
                .into_iter()
                .filter(|c| !ran_keys.contains(&c.id_key()) && existing.contains(&c.id_key()))
                .collect()
        };
        codeunits.extend(ran);
        codeunits.sort_by_key(StoredCodeunit::id_key);
        Stored {
            header,
            objects,
            codeunits,
        }
    }

    pub fn to_json(&self) -> Value {
        let h = &self.header;
        json!({
            "format": FORMAT,
            "runtime": h.runtime,
            "options": h.options,
            "environment": h.environment,
            "apps": h.apps.iter().map(|(id, v)| json!([id, v])).collect::<Vec<_>>(),
            "manifests": h.manifests,
            "objects": self.objects.iter().map(|o| json!({
                "app": o.app,
                "kind": o.kind.label(),
                "id": o.id,
                "name": o.name,
                "hash": o.hash,
                "uses": o.uses,
                "subscribes": o.subscribes,
                "globalTriggers": o.global_triggers,
                "install": o.install,
                "test": o.test,
                "refers": o.refers,
                "unfollowed": o.unfollowed,
            })).collect::<Vec<_>>(),
            "codeunits": self.codeunits.iter().map(|c| json!({
                "app": c.app,
                "id": c.id,
                "name": c.name,
                "keys": c.keys,
                "entered": c.entered,
                "dotnet": c.dotnet,
                "stubbed": c.stubbed,
                "cut": c.ended == Ended::Cut,
                "tests": c.tests.iter().map(|(name, o)| {
                    let (kind, message) = match o {
                        Outcome::Pass => ("pass", ""),
                        Outcome::Fail(m) => ("fail", m.as_str()),
                        Outcome::NotRun(m) => ("notrun", m.as_str()),
                    };
                    json!([name, kind, message])
                }).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        })
    }

    /// The stored result in `v`; an error says why it cannot be used (the run is then full with that reason).
    pub fn from_json(v: &Value) -> Result<Stored, String> {
        if v["format"].as_u64() != Some(FORMAT) {
            return Err(format!(
                "the stored result has format {}, this runtime reads format {FORMAT}",
                v["format"]
            ));
        }
        let text = |v: &Value| v.as_str().unwrap_or_default().to_string();
        let texts = |v: &Value| -> Vec<String> { v.as_array().into_iter().flatten().map(text).collect() };
        let list = |v: &Value, what: &str| -> Result<Vec<Value>, String> {
            v.as_array()
                .cloned()
                .ok_or_else(|| format!("the stored result has no `{what}` list"))
        };
        let header = Header {
            runtime: text(&v["runtime"]),
            options: text(&v["options"]),
            environment: texts(&v["environment"]),
            apps: list(&v["apps"], "apps")?
                .iter()
                .map(|a| (text(&a[0]), text(&a[1])))
                .collect(),
            manifests: texts(&v["manifests"]),
        };
        let mut objects = Vec::new();
        for o in list(&v["objects"], "objects")? {
            let kind: ObjKind = text(&o["kind"])
                .parse()
                .map_err(|_| format!("the stored result has an object of kind `{}`", o["kind"]))?;
            objects.push(ObjectFacts {
                app: text(&o["app"]),
                kind,
                id: o["id"].as_u64().map(|i| i as u32),
                name: text(&o["name"]),
                hash: text(&o["hash"]),
                uses: texts(&o["uses"]),
                subscribes: texts(&o["subscribes"]),
                global_triggers: o["globalTriggers"].as_bool().unwrap_or(true),
                install: o["install"].as_bool().unwrap_or(true),
                test: o["test"].as_bool().unwrap_or(false),
                refers: texts(&o["refers"]),
                unfollowed: texts(&o["unfollowed"]),
            });
        }
        let mut codeunits = Vec::new();
        for c in list(&v["codeunits"], "codeunits")? {
            let mut tests = Vec::new();
            for t in c["tests"].as_array().into_iter().flatten() {
                let message = text(&t[2]);
                let outcome = match t[1].as_str() {
                    Some("pass") => Outcome::Pass,
                    Some("fail") => Outcome::Fail(message),
                    Some("notrun") => Outcome::NotRun(message),
                    other => {
                        return Err(format!(
                            "the stored result has a test with outcome `{}`",
                            other.unwrap_or_default()
                        ));
                    }
                };
                tests.push((text(&t[0]), outcome));
            }
            codeunits.push(StoredCodeunit {
                app: text(&c["app"]),
                id: c["id"]
                    .as_u64()
                    .ok_or("the stored result has a codeunit without an id")? as u32,
                name: text(&c["name"]),
                keys: texts(&c["keys"]),
                entered: texts(&c["entered"]),
                // A missing flag is read as the safe answer.
                dotnet: c["dotnet"].as_bool().unwrap_or(true),
                stubbed: c["stubbed"].as_bool().unwrap_or(true),
                ended: if c["cut"].as_bool().unwrap_or(true) {
                    Ended::Cut
                } else {
                    Ended::Normal
                },
                tests,
            });
        }
        Ok(Stored {
            header,
            objects,
            codeunits,
        })
    }
}

/// The test codeunits a narrow callback sees, as the rule names them.
#[derive(Debug, Clone, Default)]
pub struct TestCodeunits {
    /// (app id in lower case, codeunit id), sorted: [`Current::codeunits`].
    pub codeunits: Vec<(String, u32)>,
    /// Each of them as (caller's app index, codeunit id), the form `RunOptions::narrow` answers in.
    pub by_key: BTreeMap<(String, u32), (usize, u32)>,
    /// The caller's app index of each app id that has one of them.
    pub caller: BTreeMap<String, usize>,
}

impl TestCodeunits {
    /// The test codeunits of the workspace apps (the caller's app indexes below `workspace`) that the run would run.
    pub fn of(input: &crate::runtime::NarrowInput, workspace: usize) -> TestCodeunits {
        let program = input.program;
        let mut out = TestCodeunits::default();
        for o in &program.objects {
            let (Some(&app), Some(id)) = (input.order.get(o.app as usize), o.id) else {
                continue;
            };
            if app >= workspace
                || !crate::impact_static::is_test_codeunit(o)
                || input.only.is_some_and(|only| !only.contains(&(app, id)))
            {
                continue;
            }
            let app_id = program.apps[o.app as usize].id.to_lowercase();
            out.caller.insert(app_id.clone(), app);
            out.by_key.insert((app_id, id), (app, id));
        }
        out.codeunits = out.by_key.keys().cloned().collect();
        out
    }

    /// The answer of a narrow callback for `selection`: `None` (every codeunit) for a full run.
    pub fn narrow(&self, selection: &Selection) -> Option<Vec<(usize, u32)>> {
        selection.full.is_none().then(|| {
            selection
                .run
                .iter()
                .filter_map(|k| self.by_key.get(k).copied())
                .collect()
        })
    }
}

/// What the test codeunits of a run recorded, for [`Stored::merge`], and the number of tests that ran. `app_id` gives
/// the app id (lower case) of a caller's app index, `None` for an app whose tests are not stored. `impact_keys` are
/// the keys the results number (`CorpusRuntime::impact_keys`).
pub fn ran_codeunits(
    results: &[crate::runtime::TestResult],
    impact_keys: &[String],
    app_id: impl Fn(usize) -> Option<String>,
) -> (Vec<StoredCodeunit>, usize) {
    let mut ran: BTreeMap<(String, u32), StoredCodeunit> = BTreeMap::new();
    let mut tests = 0;
    for r in results {
        let Some(app) = app_id(r.app) else {
            continue;
        };
        tests += 1;
        let c = ran.entry((app.clone(), r.codeunit)).or_insert_with(|| StoredCodeunit {
            app,
            id: r.codeunit,
            name: r.codeunit_name.clone(),
            ..Default::default()
        });
        c.keys
            .extend(r.impact.iter().filter_map(|&k| impact_keys.get(k as usize).cloned()));
        c.entered.extend(r.entered.iter().cloned());
        c.dotnet |= r.dotnet;
        c.stubbed |= !r.stubs.is_empty();
        // A test refused while it ran, stopped by the time limit or ended by a panic is not run (`NotRun`): what its
        // codeunit recorded is not all it would reach.
        if matches!(r.outcome, Outcome::NotRun(_)) {
            c.ended = Ended::Cut;
        }
        c.tests.push((r.test.clone(), r.outcome.clone()));
    }
    let ran = ran
        .into_values()
        .map(|mut c| {
            c.keys.sort();
            c.keys.dedup();
            c.entered.sort();
            c.entered.dedup();
            c
        })
        .collect();
    (ran, tests)
}

/// The key of object `oi` of `program` (as [`ObjectFacts::key`] makes it).
pub fn object_key(program: &al_ir::Program, oi: usize) -> Option<String> {
    let o = program.objects.get(oi)?;
    let app = program.apps.get(o.app as usize)?.id.to_lowercase();
    Some(match o.id {
        Some(id) => format!("{app}/{} {id}", o.kind.label()),
        None => format!("{app}/{} {}", o.kind.label(), o.name.to_lowercase()),
    })
}

/// The keys of the workspace objects a test entered (`Touches::entered`), sorted; `dependency` marks the objects of
/// dependency apps, which are left out (a change of a dependency app is a full run).
pub fn entered_keys(program: &al_ir::Program, t: &al_vm::impact::Touches, dependency: &[bool]) -> Vec<String> {
    let mut out: Vec<String> = t
        .entered
        .iter()
        .filter(|&&oi| !dependency.get(oi).copied().unwrap_or(false))
        .filter_map(|&oi| object_key(program, oi))
        .collect();
    out.sort();
    out.dedup();
    out
}

/// The facts of the workspace objects of a compile made with `Roots::object_hashes`: what [`Stored::select`] reads
/// of the current compile and keeps for the next one.
pub fn facts(lowered: &al_sema::lower::Lowered) -> Vec<ObjectFacts> {
    use crate::impact_static::GLOBAL_WRITE_EVENTS;
    use crate::impact_trace::{PAGE_EVENTS, TABLE_EVENTS, publisher};
    let program = &lowered.program;
    let registry = al_vm::events::Registry::build(program);
    let workspace = |oi: usize| !lowered.dependency.get(oi).copied().unwrap_or(true);
    let has = |o: &al_ir::Object, name: &str, value: &str| {
        o.properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case(name) && v.trim().eq_ignore_ascii_case(value))
    };
    // Each object by its symbolic key, for `refers`.
    let by_key: std::collections::HashMap<al_ir::sym::ObjKey, usize> = program
        .objects
        .iter()
        .enumerate()
        .map(|(oi, o)| (al_ir::sym::ObjKey::of(o, &program.apps), oi))
        .collect();
    // The base of each extension.
    let mut base: BTreeMap<usize, usize> = BTreeMap::new();
    for (oi, o) in program.objects.iter().enumerate() {
        for e in o.table.iter().flat_map(|t| t.extensions.iter()) {
            base.insert(*e, oi);
        }
        for e in o.page.iter().flat_map(|p| p.extensions.iter()) {
            base.insert(*e, oi);
        }
        if let Some(x) = &o.report_ext {
            base.insert(oi, x.base);
        }
        if o.kind == ObjKind::EnumExtension {
            if let Some(b) = o.extends_id.and_then(|id| {
                program
                    .objects
                    .iter()
                    .position(|e| e.kind == ObjKind::Enum && e.id == Some(id))
            }) {
                base.insert(oi, b);
            }
        }
    }
    let mut out: BTreeMap<usize, ObjectFacts> = BTreeMap::new();
    for (oi, h) in lowered.object_hashes.iter().enumerate() {
        let (Some(h), Some(o)) = (h, program.objects.get(oi)) else {
            continue;
        };
        if !workspace(oi) {
            continue;
        }
        let mut unfollowed = Vec::new();
        let used = |kind: ObjKind, oi: usize| -> Option<String> {
            let id = program.objects.get(oi)?.id?;
            let what = match kind {
                ObjKind::Table | ObjKind::TableExtension => "table",
                ObjKind::Page | ObjKind::PageExtension => "page",
                ObjKind::Report | ObjKind::ReportExtension => "report",
                ObjKind::Enum | ObjKind::EnumExtension => "enum",
                _ => return None,
            };
            Some(format!("{what} {id}"))
        };
        let uses = match o.kind {
            ObjKind::Table | ObjKind::Page | ObjKind::Report | ObjKind::Enum => used(o.kind, oi).into_iter().collect(),
            ObjKind::TableExtension | ObjKind::PageExtension | ObjKind::ReportExtension | ObjKind::EnumExtension => {
                match base.get(&oi).and_then(|&b| used(o.kind, b)) {
                    Some(k) => vec![k],
                    None => {
                        unfollowed.push(format!("its base {} is not in the program", o.kind.label()));
                        Vec::new()
                    }
                }
            }
            _ => Vec::new(),
        };
        if has(o, "Subtype", "TestRunner") {
            unfollowed.push("it is a test runner".into());
        }
        let refers: BTreeSet<String> = h
            .refers
            .iter()
            .filter_map(|k| by_key.get(k).copied())
            .filter(|&r| r != oi && workspace(r))
            .filter_map(|r| object_key(program, r))
            .collect();
        let key = object_key(program, oi).unwrap_or_default();
        out.insert(
            oi,
            ObjectFacts {
                app: key.split('/').next().unwrap_or_default().to_string(),
                kind: o.kind,
                id: o.id,
                name: o.name.clone(),
                hash: h.hash.clone(),
                uses,
                subscribes: Vec::new(),
                global_triggers: false,
                install: o.kind == ObjKind::Codeunit && (has(o, "Subtype", "Install") || has(o, "Subtype", "Upgrade")),
                test: o.kind == ObjKind::Codeunit && has(o, "Subtype", "Test"),
                refers: refers.into_iter().collect(),
                unfollowed,
            },
        );
    }
    // Subscribers: the event each binds, as the trace keys a raise; install code; what the trace does not note.
    for &(so, _, _) in &registry.install_subscribers {
        if let Some(f) = out.get_mut(&so) {
            f.install = true;
        }
    }
    for ((object, event), list) in &registry.subscribers {
        for s in list {
            let Some(f) = out.get_mut(&s.object) else {
                continue;
            };
            if f.install
                && registry
                    .install_subscribers
                    .iter()
                    .any(|(o, r, _)| (*o, *r) == (s.object, s.routine))
            {
                continue;
            }
            let kind = program.objects.get(*object).map(|o| o.kind);
            // `Company Triggers` runs the subscribers of the apps under test in the install session (ADR-046).
            if program
                .objects
                .get(*object)
                .is_some_and(|o| o.kind == ObjKind::Codeunit && o.id == Some(2_000_000_003))
                || *object == al_vm::events::COMPANY_TRIGGERS
            {
                f.install = true;
                continue;
            }
            if *object == registry.global_key {
                if GLOBAL_WRITE_EVENTS.contains(&event.as_str()) {
                    f.global_triggers = true;
                } else {
                    f.unfollowed.push(format!(
                        "a subscriber to the Global Triggers event {event}, whose raises are not noted"
                    ));
                }
                continue;
            }
            let routine_publisher = registry
                .publishers
                .values()
                .any(|p| p.object == *object && p.event == *event);
            let keyed = *object == al_vm::events::CAPTION_CLASS
                || routine_publisher
                || (kind == Some(ObjKind::Table) && TABLE_EVENTS.contains(&event.as_str()))
                || (kind == Some(ObjKind::Page) && PAGE_EVENTS.contains(&event.as_str()));
            match publisher(program, *object) {
                Some(name) if keyed => {
                    let element = s.element.to_lowercase();
                    f.subscribes.push(if element.is_empty() {
                        format!("event {name} {event}")
                    } else {
                        format!("event {name} {event}/{element}")
                    });
                }
                _ => f
                    .unfollowed
                    .push(format!("a subscriber to the event {event}, whose raises are not noted")),
            }
        }
    }
    out.into_values()
        .map(|mut f| {
            f.subscribes.sort();
            f.subscribes.dedup();
            f.unfollowed.sort();
            f.unfollowed.dedup();
            f
        })
        .collect()
}

/// `table <id>` of a platform table (see [`PLATFORM_TABLES`]).
fn platform_table(key: &str) -> bool {
    key.strip_prefix("table ")
        .and_then(|id| id.parse::<u32>().ok())
        .is_some_and(|id| id >= PLATFORM_TABLES)
}

#[cfg(test)]
#[path = "affected_tests.rs"]
mod tests;
