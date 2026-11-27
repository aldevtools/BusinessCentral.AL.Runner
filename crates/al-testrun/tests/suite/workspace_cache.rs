//! The workspace layer of the compile cache (ADR-022 amendment 2026-11-15): a compile read from the entry is
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
        verbose: false,
    })
}

fn compile(i: &Inputs, dir: &Path, workspace: bool) -> Compiled {
    let opts = RunOptions {
        compile_cache: cache(dir),
        workspace_cache: if workspace {
            i.opts.workspace_cache.clone().or_else(|| layer(true))
        } else {
            None
        },
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

/// Builds the entry for `i`, then compiles `changed`: it must not read the entry (built, a new key), and `i` still reads
/// its own.
fn assert_misses(i: &Inputs, changed: &Inputs, what: &str) {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        state(&compile(i, dir.path(), true)),
        Some(WorkspaceState::Built),
        "{what}"
    );
    let other = compile(changed, dir.path(), true);
    assert_eq!(
        state(&other),
        Some(WorkspaceState::Built),
        "{what}: {:?}",
        other.workspace_cache()
    );
    let fresh = compile(changed, dir.path(), false);
    assert_eq!(wc::canonical_dump(&other), wc::canonical_dump(&fresh), "{what}");
    assert_eq!(
        state(&compile(i, dir.path(), true)),
        Some(WorkspaceState::Read),
        "{what}"
    );
    assert_eq!(entries(dir.path()).len(), 2, "{what}: one entry per key");
}

/// Decision 2 through a run: a byte of source, a renamed file, a symbol, the target, a compile option, the layer below
/// and a field of the roots each miss.
#[test]
fn each_input_of_the_key_misses_through_a_compile() {
    type Change = fn(&mut Inputs);
    let changes: Vec<(&str, Change)> = vec![
        ("a byte of source", |i| i.apps[0].files[0].1.push(' ')),
        ("a renamed file", |i| i.apps[0].files[0].0 = "src/Renamed.al".into()),
        ("a symbol", |i| {
            i.symbols.insert("CLEAN27".into());
        }),
        ("the target", |i| {
            i.target = Some(al_version::Target::for_bc(
                al_version::BcVersion::parse("28.0").unwrap(),
            ))
        }),
        ("a compile option", |i| {
            i.opts.workspace_cache = layer(true).map(|mut o| {
                o.compile_options = "execute=true".into();
                o
            })
        }),
        ("the layer below", |i| {
            i.apps[1].files[0].1.push_str("\ncodeunit 50199 More { }\n")
        }),
        ("a field of the roots (the selection)", |i| {
            i.opts.only = Some(vec![(0, 60102)])
        }),
        ("a field of the roots (--affected hashes)", |i| {
            i.opts.object_hashes = true
        }),
        ("a manifest", |i| i.manifests[0].version = "2.0.0.0".into()),
    ];
    for (what, change) in changes {
        let base = with_dependency();
        let mut changed = base.clone();
        change(&mut changed);
        assert_misses(&base, &changed, what);
    }
}

/// `compile` with the options of `i`, keeping its own `workspace_cache` when it set one.
fn compile_as_set(i: &Inputs, dir: &Path) -> Compiled {
    let opts = RunOptions {
        compile_cache: cache(dir),
        workspace_cache: i.opts.workspace_cache.clone().or_else(|| layer(true)),
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

/// What a run decided, test by test.
fn outcomes(i: &Inputs, dir: &Path, workspace: bool) -> (Vec<String>, Option<WorkspaceState>) {
    let opts = RunOptions {
        compile_cache: cache(dir),
        workspace_cache: if workspace { layer(true) } else { None },
        ..i.opts.clone()
    };
    let rt = CorpusRuntime::try_run_suites_with(
        i.apps.clone(),
        i.kinds.clone(),
        &i.manifests,
        &i.symbols,
        i.target.as_ref(),
        Default::default(),
        &[SuiteRun::everything()],
        &opts,
    )
    .unwrap();
    let list = rt
        .results()
        .iter()
        .map(|r| format!("{} {} {} {:?}", r.app, r.codeunit, r.test, r.outcome))
        .collect();
    (list, rt.phases.workspace_cache.as_ref().map(|r| r.state))
}

/// Decision 10: an entry that is damaged, cut short, or made for other inputs under this key's name is removed and
/// rebuilt; the run's outcomes are those of a run without the layer.
#[test]
fn a_damaged_truncated_or_foreign_entry_is_a_miss_that_never_changes_an_outcome() {
    let i = with_dependency();
    let plain_dir = tempfile::tempdir().unwrap();
    let (plain, _) = outcomes(&i, plain_dir.path(), false);
    type Spoil = fn(&Path, &Path);
    let spoils: Vec<(&str, Spoil)> = vec![
        ("a byte flipped in the blocks", |p, _| {
            let mut b = std::fs::read(p).unwrap();
            let at = b.len() / 3;
            b[at] ^= 0x5a;
            std::fs::write(p, b).unwrap();
        }),
        ("a byte flipped in the extra section", |p, _| {
            let mut b = std::fs::read(p).unwrap();
            let at = b.len() - 10;
            b[at] ^= 0x5a;
            std::fs::write(p, b).unwrap();
        }),
        ("cut short", |p, _| {
            let b = std::fs::read(p).unwrap();
            std::fs::write(p, &b[..b.len() * 2 / 3]).unwrap();
        }),
        ("empty", |p, _| std::fs::write(p, b"").unwrap()),
        ("another key's entry under this name", |p, other| {
            std::fs::copy(other, p).unwrap();
        }),
    ];
    for (what, spoil) in spoils {
        let dir = tempfile::tempdir().unwrap();
        // Another key's entry, to copy over this one.
        let mut other = i.clone();
        other.apps[0].files[0].1.push(' ');
        compile(&other, dir.path(), true);
        let other_file = entries(dir.path())[0].clone();
        let (first, s) = outcomes(&i, dir.path(), true);
        assert_eq!(s, Some(WorkspaceState::Built), "{what}");
        let file = entries(dir.path()).into_iter().find(|p| *p != other_file).unwrap();
        spoil(&file, &other_file);
        let (after, s) = outcomes(&i, dir.path(), true);
        assert_eq!(s, Some(WorkspaceState::Built), "{what}: refused and built again");
        assert_eq!(first, plain, "{what}");
        assert_eq!(after, plain, "{what}");
        let (again, s) = outcomes(&i, dir.path(), true);
        assert_eq!(
            (again, s),
            (plain.clone(), Some(WorkspaceState::Read)),
            "{what}: the rebuilt entry reads"
        );
    }
}

/// The outcomes of a run that reads the entry are those of a run without the layer.
#[test]
fn a_run_on_a_read_compile_has_the_outcomes_of_a_fresh_run() {
    for i in [with_dependency(), two_source_paths()] {
        let dir = tempfile::tempdir().unwrap();
        let (plain, s) = outcomes(&i, dir.path(), false);
        assert_eq!(s, None);
        let (built, s) = outcomes(&i, dir.path(), true);
        assert_eq!((built, s), (plain.clone(), Some(WorkspaceState::Built)));
        let (read, s) = outcomes(&i, dir.path(), true);
        assert_eq!((read, s), (plain, Some(WorkspaceState::Read)));
    }
}

/// The modes of the test plan: `--tdd`, `--affected` (object hashes), a workspace with compile errors, and a dependency
/// app that stands on a workspace app (compiled with the workspace, al-runner issue 6044). Relowered blocks (the
/// workspace extends and implements dependency objects) and synthetic ranges are in `with_dependency` itself.
#[test]
fn the_modes_read_equal_to_fresh() {
    let mut tdd = with_dependency();
    tdd.opts.tdd = true;
    tdd.apps[0].files.push((
        "test/Stub.al".into(),
        "codeunit 60110 StubTests { Subtype = Test; [Test] procedure T() var D: Codeunit Dep; begin D.NotThere(); end; }"
            .into(),
    ));
    let mut affected = with_dependency();
    affected.opts.object_hashes = true;
    let mut errors = with_dependency();
    errors.apps[0].files.push((
        "src/Bad.al".into(),
        "codeunit 60120 Bad { procedure P() begin Unknown(); end; }".into(),
    ));
    let mut on_workspace = with_dependency();
    on_workspace.apps.push(app(
        "ondep",
        &["main"],
        &[(
            "OnDep.al",
            "codeunit 70100 OnDep { procedure P(): Text var G: Codeunit WsGreet; begin exit(G.Hello()); end; }",
        )],
    ));
    on_workspace.kinds.push(al_sema::AppKind::Dependency);
    on_workspace.manifests.push(manifest("Partner"));
    for (what, i) in [
        ("tdd", tdd),
        ("affected", affected),
        ("compile errors", errors),
        ("a dependency on the workspace", on_workspace),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let fresh = compile(&i, dir.path(), false);
        let built = compile_as_set(&i, dir.path());
        let read = compile_as_set(&i, dir.path());
        assert_eq!(
            state(&built),
            Some(WorkspaceState::Built),
            "{what}: {:?}",
            built.workspace_cache()
        );
        assert_eq!(
            state(&read),
            Some(WorkspaceState::Read),
            "{what}: {:?}",
            read.workspace_cache()
        );
        let (a, b) = (wc::canonical_dump(&fresh), wc::canonical_dump(&read));
        assert!(a == b, "{what}: {}", first_difference(&a, &b));
    }
}

/// The incremental compile of serve after a read compile gives what it gives after a fresh compile.
#[test]
fn an_incremental_compile_after_a_read_equals_one_after_a_fresh_compile() {
    let mut i = with_dependency();
    const EXTRA: &str = "codeunit 60130 Extra { procedure P(): Integer begin exit(1); end; }";
    i.apps[0].files.push(("src/Extra.al".into(), EXTRA.into()));
    let dir = tempfile::tempdir().unwrap();
    let mut fresh = compile(&i, dir.path(), false);
    compile(&i, dir.path(), true);
    let mut read = compile(&i, dir.path(), true);
    assert_eq!(state(&read), Some(WorkspaceState::Read));
    let edited = EXTRA.replace("exit(1)", "exit(2)");
    let change = [al_testrun::runtime::FileChange {
        app: 0,
        path: "src/Extra.al",
        text: &edited,
    }];
    fresh.update_files(&change).unwrap();
    read.update_files(&change).unwrap();
    let (a, b) = (wc::canonical_dump(&fresh), wc::canonical_dump(&read));
    assert!(a == b, "{}", first_difference(&a, &b));
    fresh.same_lowering_as(&read).unwrap();
}

/// Selected tests that are free of data are compiled without the dependency layers and without the install baseline
/// (`compile_suites_once`, the weighing); that compile is stored too, and read back equal to a fresh one.
#[test]
fn a_compile_of_selected_tests_free_of_data_reads_equal_to_fresh() {
    let mut i = with_dependency();
    i.apps[0].files = vec![(
        "test/Pure.al".into(),
        "codeunit 60140 PureTests { Subtype = Test; [Test] procedure Adds() var D: Codeunit DefaultGreet; begin if D.Hello() <> 'dep' then Error('hello'); end; }"
            .into(),
    )];
    i.opts.only = Some(vec![(0, 60140)]);
    let dir = tempfile::tempdir().unwrap();
    let fresh = compile(&i, dir.path(), false);
    let built = compile(&i, dir.path(), true);
    assert_eq!(
        state(&built),
        Some(WorkspaceState::Built),
        "{:?}",
        built.workspace_cache()
    );
    let read = compile(&i, dir.path(), true);
    assert_eq!(state(&read), Some(WorkspaceState::Read), "{:?}", read.workspace_cache());
    let (a, b) = (wc::canonical_dump(&fresh), wc::canonical_dump(&read));
    assert!(a.contains("skipped true"), "the compile of the weighing is the compile");
    assert!(a == b, "{}", first_difference(&a, &b));
    // A run compiles with the lean selection (another key than `compile_suites`): built once, then read.
    let (plain, _) = outcomes(&i, dir.path(), false);
    outcomes(&i, dir.path(), true);
    let (cached, s) = outcomes(&i, dir.path(), true);
    assert_eq!((cached, s), (plain, Some(WorkspaceState::Read)));
}

/// Review HIGH 2: a dependency file that does not parse (its objects are unusable) is in the read compile as in the fresh
/// one, for the compile on the dependency layers, the compile of selected tests free of data, and a dependency app that
/// stands on a workspace app (compiled with the workspace).
#[test]
fn a_dependency_file_that_does_not_parse_reads_as_fresh() {
    const BAD: &str =
        "codeunit 50901 \"Dep Bad\"\n{\n    procedure Two(): Integer\n    begin\n        exit(2 +);\n    end\n}\n";
    const USES_BAD: &str = "codeunit 60150 BadTests { Subtype = Test; [Test] procedure Bad() var B: Codeunit \"Dep Bad\"; begin if B.Two() <> 2 then Error('two'); end; }";
    let mut layered = with_dependency();
    layered.apps[1].files.push(("Bad.al".into(), BAD.into()));
    layered.apps[0].files.push(("test/UsesBad.al".into(), USES_BAD.into()));
    let mut pure = layered.clone();
    pure.apps[0].files = vec![("test/UsesBad.al".into(), USES_BAD.into())];
    pure.opts.only = Some(vec![(0, 60150)]);
    let mut on_workspace = with_dependency();
    on_workspace.apps.push(app(
        "ondep",
        &["main"],
        &[("OnDep.al", "codeunit 70100 OnDep { }"), ("Bad.al", BAD)],
    ));
    on_workspace.kinds.push(al_sema::AppKind::Dependency);
    on_workspace.manifests.push(manifest("Partner"));
    on_workspace.apps[0]
        .files
        .push(("test/UsesBad.al".into(), USES_BAD.into()));
    for (what, i) in [
        ("layered", layered),
        ("free of data", pure),
        ("on the workspace", on_workspace),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let fresh = compile(&i, dir.path(), false);
        let a = wc::canonical_dump(&fresh);
        compile(&i, dir.path(), true);
        let read = compile(&i, dir.path(), true);
        assert_eq!(state(&read), Some(WorkspaceState::Read), "{what}");
        let b = wc::canonical_dump(&read);
        assert!(a == b, "{what}: {}", first_difference(&a, &b));
    }
}

/// Review MED 3: the apps the platform package's `InternalsVisibleTo` grants (read from every manifest, a dependency's
/// too, which the dependency layer's key does not hold) are in the key.
#[test]
fn a_grant_of_internals_misses() {
    let base = with_dependency();
    let mut granted = base.clone();
    granted.manifests[1].platform_internals_visible_to = vec![al_deps::package::InternalsGrant {
        id: "main".into(),
        publisher: "Me".into(),
    }];
    assert_misses(&base, &granted, "a grant");
}

/// A run of `i` under a hard memory limit of 1000 MB whose resident memory reads as `rss` (in MB), with the layer's
/// writes deferred to the end of the run as `al-runner test` defers them.
fn run_under_limit(i: &Inputs, dir: &Path, rss: impl Fn() -> u64 + Send + Sync + 'static) -> CorpusRuntime {
    let opts = RunOptions {
        compile_cache: cache(dir),
        workspace_cache: layer(false),
        memory: Some(al_testrun::runtime::memory::MemoryBudget {
            limit: 1000 << 20,
            source: "--max-memory".into(),
            rss: std::sync::Arc::new(move || Some(rss() << 20)),
            hard: true,
        }),
        ..i.opts.clone()
    };
    let rt = CorpusRuntime::try_run_suites_with(
        i.apps.clone(),
        i.kinds.clone(),
        &i.manifests,
        &i.symbols,
        i.target.as_ref(),
        Default::default(),
        &[SuiteRun::everything()],
        &opts,
    )
    .unwrap();
    wc::wait_for_writes();
    rt
}

/// Decision 6: only a compile the run goes on with is stored. A compile the memory limit stops after its last check
/// (the stop line is passed once the compile has ended, before its watcher stops) reports every test not run and
/// leaves no entry, so the next run of the same inputs compiles fresh, as it would without the first run.
#[test]
fn a_compile_the_memory_limit_stops_after_its_last_check_stores_no_entry() {
    let i = with_dependency();
    let dir = tempfile::tempdir().unwrap();
    let (plain, _) = outcomes(&i, tempfile::tempdir().unwrap().path(), false);
    // The watcher's first two reads (when it starts) are under the line. Its next read, on its own thread, waits two
    // seconds, then reads over the line; the compile of these two small apps has ended by then, and the run stops when
    // the watcher stops (`compile_halt.rs`). A compile the stop reached before its end leaves no `after_compile`
    // reading, so a slow machine fails the assert below instead of testing a stop during the compile.
    let reads = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let stopped = run_under_limit(&i, dir.path(), move || {
        let n = reads.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if n < 2 {
            return 100;
        }
        if n == 2 {
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
        990
    });
    assert!(stopped.phases.memory.halted.is_some(), "{:?}", stopped.phases.memory);
    assert!(
        stopped.phases.memory.after_compile.is_some(),
        "the compile ended before the halt: {:?}",
        stopped.phases.memory
    );
    assert_eq!(stopped.results().len(), plain.len());
    for r in stopped.results() {
        assert_eq!(
            format!("{:?}", r.outcome),
            "NotRun(\"memory limit (--max-memory 1000 MB)\")",
            "{}",
            r.test
        );
    }
    let left = entries(dir.path());
    let next = run_under_limit(&i, dir.path(), || 100);
    let list: Vec<String> = next
        .results()
        .iter()
        .map(|r| format!("{} {} {} {:?}", r.app, r.codeunit, r.test, r.outcome))
        .collect();
    let state = next.phases.workspace_cache.as_ref().map(|r| r.state);
    assert_eq!(
        (left, list, state),
        (Vec::new(), plain, Some(WorkspaceState::Built)),
        "a stopped compile writes no entry, and the next run compiles fresh"
    );
}

/// Rewrites the extra section of the workspace entry at `path` with `change` applied, and makes its hashes again
/// (`al_cache::build_workspace`'s layout): an entry that passes every check of the file but holds lists that do not fit
/// its own compile.
fn rewrite_extra(path: &Path, change: impl FnOnce(&mut wc::WorkspaceExtra)) {
    const HEADER: usize = 8 + 4 + 32 + 8 + 8 + 8 + 32 + 32;
    let b = std::fs::read(path).unwrap();
    let word = |at: usize| u64::from_le_bytes(b[at..at + 8].try_into().unwrap()) as usize;
    let at = HEADER + word(44);
    let raw = zstd::bulk::decompress(&b[at..], word(60)).unwrap();
    let mut extra: wc::WorkspaceExtra = postcard::from_bytes(&raw).unwrap();
    change(&mut extra);
    let raw = postcard::to_stdvec(&extra).unwrap();
    let packed = zstd::bulk::compress(&raw, 3).unwrap();
    let mut out = b[..at].to_vec();
    out[52..60].copy_from_slice(&(packed.len() as u64).to_le_bytes());
    out[60..68].copy_from_slice(&(raw.len() as u64).to_le_bytes());
    out[68..100].copy_from_slice(&al_cache::digest::sha(&packed));
    let header_hash = al_cache::digest::sha(&out[..HEADER - 32]);
    out[HEADER - 32..HEADER].copy_from_slice(&header_hash);
    out.extend_from_slice(&packed);
    std::fs::write(path, out).unwrap();
}

/// Decision 10: an entry whose per-object lists do not fit its compile (one entry too few or too many, or a routine
/// left out as unreached in no object of the workspace) is refused at read, removed and built again, even when every
/// check of the file holds; the run's outcomes are those of a run without the layer.
#[test]
fn an_entry_whose_lists_do_not_fit_its_compile_is_a_miss() {
    let i = with_dependency();
    let (plain, _) = outcomes(&i, tempfile::tempdir().unwrap().path(), false);
    type Change = fn(&mut wc::WorkspaceExtra);
    let changes: Vec<(&str, Change)> = vec![
        ("unresolved_calls one short", |e| {
            e.lowered.unresolved_calls.pop();
        }),
        ("skipped of one object", |e| e.lowered.skipped = vec![false]),
        ("left_out one short", |e| {
            e.lowered.left_out.pop();
        }),
        ("object_hashes of one object", |e| e.lowered.object_hashes = vec![None]),
        ("an unreached routine of no object", |e| {
            e.workspace.unreached = Some(vec![(1 << 30, 0)])
        }),
        ("an unreached routine past its file", |e| {
            e.workspace.unreached = Some(vec![(0, 1 << 30)])
        }),
    ];
    let mut wrong = Vec::new();
    for (what, change) in changes {
        let dir = tempfile::tempdir().unwrap();
        let (first, s) = outcomes(&i, dir.path(), true);
        assert_eq!((first, s), (plain.clone(), Some(WorkspaceState::Built)), "{what}");
        let file = entries(dir.path()).pop().unwrap();
        rewrite_extra(&file, change);
        let after = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| outcomes(&i, dir.path(), true)));
        match after {
            Ok((after, Some(WorkspaceState::Built))) if after == plain => {}
            Ok((after, s)) => wrong.push(format!("{what}: {s:?}, outcomes equal {}", after == plain)),
            Err(_) => wrong.push(format!("{what}: the run panicked")),
        }
    }
    assert!(
        wrong.is_empty(),
        "refused and built again, with the outcomes of a run without the layer: {wrong:#?}"
    );
}

/// A dependency app given without a manifest, whose source depends on a preprocessor symbol the project defines: a
/// dependency compiles with its own symbols (none here), never the project's (hard rule 6). The run that reads the
/// entry gives the dependency the fresh run's symbols: the same compile (canonical dump) and the same outcomes.
#[test]
fn a_read_compile_gives_a_dependency_without_a_manifest_the_fresh_symbols() {
    const SYMBOL_BASE: &str = r#"
codeunit 50110 SymbolDep
{
    procedure Which(): Text
    begin
#if CLEAN99
        exit('project symbol');
#else
        exit('own symbols');
#endif
    end;
}
"#;
    const SYMBOL_MAIN: &str = r#"
codeunit 60110 SymbolTests
{
    Subtype = Test;
    [Test] procedure DependencyUsesItsOwnSymbols() var D: Codeunit SymbolDep; begin if D.Which() <> 'own symbols' then Error('%1', D.Which()); end;
}
"#;
    let i = Inputs {
        apps: vec![
            app("main", &["base"], &[("src/Main.al", SYMBOL_MAIN)]),
            app("base", &[], &[("Base.al", SYMBOL_BASE)]),
        ],
        kinds: vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        // The dependency has no manifest.
        manifests: vec![manifest("Me")],
        symbols: ["CLEAN99".to_string()].into_iter().collect(),
        target: None,
        opts: RunOptions::default(),
    };
    assert_read_equals_fresh(&i);
    let dir = tempfile::tempdir().unwrap();
    let (plain, s) = outcomes(&i, dir.path(), false);
    assert_eq!(s, None);
    assert!(plain.iter().all(|o| o.ends_with("Pass")), "{plain:?}");
    let (built, s) = outcomes(&i, dir.path(), true);
    assert_eq!((built, s), (plain.clone(), Some(WorkspaceState::Built)));
    let (read, s) = outcomes(&i, dir.path(), true);
    assert_eq!((read, s), (plain, Some(WorkspaceState::Read)));
}
