//! The workspace layer of the compile cache (ADR-022 amendment 2026-11-15, prototype): a compile read from the entry is
//! byte-equal to a fresh compile of the same inputs (the canonical dump), any change of a key input misses, and an entry
//! that is damaged, cut short or made by another build is a miss that never changes an outcome.

use al_sema::workspace::AppSource;
use al_testrun::compile_cache::{self, CompileCache};
use al_testrun::runtime::{Compiled, CorpusRuntime, Manifest, RunOptions, SuiteRun};
use al_testrun::workspace_cache::{self as wc, WorkspaceCacheOptions, WorkspaceState};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const BASE: &str = r#"
table 50100 Cust { fields { field(1; "No."; Code[20]) { } field(2; Name; Text[50]) { } } keys { key(PK; "No.") { } } }
interface IGreet { procedure Hello(): Text; }
codeunit 50101 DefaultGreet implements IGreet { procedure Hello(): Text begin exit('dep'); end; }
codeunit 50102 Dep
{
    procedure Count(): Integer var C: Record Cust; begin exit(C.Count()); end;
    procedure Say(G: Interface IGreet): Text begin exit(G.Hello()); end;
    procedure Fail() begin Error('from the dependency'); end;
}
"#;

/// Extends and implements dependency objects (relowered blocks), with a page expression (synthetic ranges).
const MAIN: &str = r#"
tableextension 60100 CustExt extends Cust { fields { field(60100; Extra; Text[20]) { } } }
codeunit 60101 WsGreet implements IGreet { procedure Hello(): Text begin exit('ws'); end; }
page 60103 CustCard { SourceTable = Cust; layout { area(Content) { field(Name; Rec.Name) { Visible = Rec.Name <> ''; } } } }
codeunit 60102 Tests
{
    Subtype = Test;
    [Test] procedure Inserts() var C: Record Cust; D: Codeunit Dep; begin C."No." := 'A'; C.Extra := 'x'; C.Insert(); if D.Count() <> 1 then Error('count'); end;
    [Test] procedure Interface_FromTheWorkspace() var D: Codeunit Dep; G: Codeunit WsGreet; begin if D.Say(G) <> 'ws' then Error('say'); end;
    [Test] procedure Fails() var D: Codeunit Dep; begin D.Fail(); end;
}
"#;

/// An app and its test app, no dependency apps: the shape of serve's suite (two `sourcePaths`).
const LIB: &str = r#"
codeunit 50200 Lib { procedure Twice(X: Integer): Integer begin exit(X * 2); end; }
"#;
const LIB_TESTS: &str = r#"
codeunit 50201 LibTests
{
    Subtype = Test;
    [Test] procedure Doubles() var L: Codeunit Lib; begin if L.Twice(2) <> 4 then Error('twice'); end;
    [Test] procedure Fails() begin Error('boom'); end;
}
"#;

fn app(id: &str, deps: &[&str], files: &[(&str, &str)]) -> AppSource {
    AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: files.iter().map(|(p, t)| (p.to_string(), t.to_string())).collect(),
    }
}

fn manifest(publisher: &str) -> Manifest {
    Manifest {
        publisher: publisher.into(),
        version: "1.0.0.0".into(),
        ..Default::default()
    }
}

/// The inputs of one compile.
#[derive(Clone)]
struct Inputs {
    apps: Vec<AppSource>,
    kinds: Vec<al_sema::AppKind>,
    manifests: Vec<Manifest>,
    symbols: BTreeSet<String>,
    target: Option<al_version::Target>,
    opts: RunOptions,
}

fn with_dependency() -> Inputs {
    Inputs {
        apps: vec![
            app("main", &["base"], &[("src/Main.al", MAIN)]),
            app("base", &[], &[("Base.al", BASE)]),
        ],
        kinds: vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        manifests: vec![manifest("Me"), manifest("Microsoft")],
        symbols: BTreeSet::new(),
        target: None,
        opts: RunOptions::default(),
    }
}

fn two_source_paths() -> Inputs {
    Inputs {
        apps: vec![
            app("lib", &[], &[("src/Lib.al", LIB)]),
            app("libtests", &["lib"], &[("test/LibTests.al", LIB_TESTS)]),
        ],
        kinds: vec![al_sema::AppKind::Workspace; 2],
        manifests: vec![manifest("Me"), manifest("Me")],
        symbols: BTreeSet::new(),
        target: None,
        opts: RunOptions::default(),
    }
}

fn cache(dir: &Path) -> Option<CompileCache> {
    Some(CompileCache {
        dir: dir.to_path_buf(),
        require: false,
        rebuild: false,
        country: String::new(),
    })
}

fn layer(write_now: bool) -> Option<WorkspaceCacheOptions> {
    Some(WorkspaceCacheOptions {
        compile_options: "execute=false tdd=false".into(),
        app_json: Vec::new(),
        write_now,
    })
}

fn compile(i: &Inputs, dir: &Path, workspace: bool) -> Compiled {
    let opts = RunOptions {
        compile_cache: cache(dir),
        workspace_cache: if workspace { layer(true) } else { None },
        ..i.opts.clone()
    };
    CorpusRuntime::compile_suites(
        i.apps.clone(),
        i.kinds.clone(),
        &i.manifests,
        &i.symbols,
        i.target.as_ref(),
        &[SuiteRun::everything()],
        &opts,
    )
    .unwrap()
}

fn state(c: &Compiled) -> Option<WorkspaceState> {
    c.workspace_cache().map(|r| r.state)
}

fn entries(dir: &Path) -> Vec<PathBuf> {
    compile_cache::list_workspace(dir).into_iter().map(|(p, _)| p).collect()
}

/// The first line where two dumps differ, for the failure message.
fn first_difference(a: &str, b: &str) -> String {
    for (n, (x, y)) in a.lines().zip(b.lines()).enumerate() {
        if x != y {
            return format!("line {n}:\n fresh: {x:.300}\n read:  {y:.300}");
        }
    }
    format!("lengths {} and {}", a.lines().count(), b.lines().count())
}

/// Fresh, then built, then read: the read compile's dump is the fresh one's.
fn assert_read_equals_fresh(i: &Inputs) {
    let dir = tempfile::tempdir().unwrap();
    let fresh = compile(i, dir.path(), false);
    assert_eq!(state(&fresh), None);
    let built = compile(i, dir.path(), true);
    assert_eq!(
        state(&built),
        Some(WorkspaceState::Built),
        "{:?}",
        built.workspace_cache()
    );
    let read = compile(i, dir.path(), true);
    assert_eq!(state(&read), Some(WorkspaceState::Read), "{:?}", read.workspace_cache());
    let (a, b) = (wc::canonical_dump(&fresh), wc::canonical_dump(&read));
    assert!(
        a == b,
        "the read compile differs from the fresh one: {}",
        first_difference(&a, &b)
    );
    assert_eq!(
        a,
        wc::canonical_dump(&built),
        "the compile that built the entry is the fresh one"
    );
}

#[test]
fn a_read_compile_on_a_dependency_layer_equals_a_fresh_one() {
    assert_read_equals_fresh(&with_dependency());
}

#[test]
fn a_read_compile_of_two_workspace_apps_without_dependencies_equals_a_fresh_one() {
    assert_read_equals_fresh(&two_source_paths());
}
