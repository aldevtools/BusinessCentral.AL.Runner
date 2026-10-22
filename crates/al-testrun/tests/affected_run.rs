//! `--affected` (ADR-050) asks a run for the per-object hashes (`RunOptions::object_hashes`): the narrow callback then
//! sees a hash for every workspace object, also one no test names, and each test result carries the workspace objects
//! its code entered.

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, Narrow, NarrowInput, RunOptions, SuiteRun};
use std::sync::{Arc, Mutex};

const SRC: &str = r#"
codeunit 50500 "Helper"
{
    procedure Two(): Integer
    begin
        exit(2);
    end;
}
codeunit 50501 "Unused" { procedure Nothing() begin end; }
codeunit 50502 "Affected Tests"
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
"#;

#[test]
fn the_narrow_callback_sees_the_hash_of_every_workspace_object() {
    let seen: Arc<Mutex<Vec<String>>> = Arc::default();
    let out = seen.clone();
    let opts = RunOptions {
        object_hashes: true,
        impact_trace: true,
        narrow: Some(Narrow(Arc::new(move |input: &NarrowInput| {
            let facts = al_testrun::affected::facts(input.lowered);
            *out.lock().unwrap() = facts.iter().map(|f| f.key()).collect();
            None
        }))),
        ..Default::default()
    };
    let rt = CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "A5".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), SRC.into())],
        }],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &opts,
    );
    let mut keys = seen.lock().unwrap().clone();
    keys.sort();
    assert_eq!(
        keys,
        vec!["a5/codeunit 50500", "a5/codeunit 50501", "a5/codeunit 50502"]
    );
    let r = &rt.results()[0];
    assert!(r.entered.contains(&"a5/codeunit 50500".to_string()), "{:?}", r.entered);
}

/// The hash of each workspace object (key to hash) that the narrow callback of a run with `opts` sees.
fn hashing(out: Arc<Mutex<Vec<(String, String)>>>) -> RunOptions {
    RunOptions {
        object_hashes: true,
        impact_trace: true,
        narrow: Some(Narrow(Arc::new(move |input: &NarrowInput| {
            let facts = al_testrun::affected::facts(input.lowered);
            let mut v: Vec<(String, String)> = facts.iter().map(|f| (f.key(), f.hash.clone())).collect();
            v.sort();
            *out.lock().unwrap() = v;
            None
        }))),
        ..Default::default()
    }
}

/// The workspace app with `text`, on a dependency app: the incremental update needs the compile cache, which holds
/// the dependency apps.
fn apps(
    text: &str,
) -> (
    Vec<AppSource>,
    Vec<al_sema::AppKind>,
    Vec<al_testrun::runtime::Manifest>,
) {
    (
        vec![
            AppSource {
                id: "A5".into(),
                name: "a".into(),
                dependencies: vec!["base".into()],
                files: vec![("main.al".into(), text.into())],
            },
            AppSource {
                id: "base".into(),
                name: "base".into(),
                dependencies: vec![],
                files: vec![(
                    "base.al".into(),
                    "codeunit 50590 \"Base\" { procedure B() begin end; }".into(),
                )],
            },
        ],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        ["Me", "Microsoft"]
            .iter()
            .map(|p| al_testrun::runtime::Manifest {
                publisher: p.to_string(),
                version: "1.0.0.0".into(),
                resources: vec![],
                include_source: false,
                target: String::new(),
                layout_files: Vec::new(),
                symbols: vec![],
                ..Default::default()
            })
            .collect(),
    )
}

/// [`SRC`] with a test that reads a table: tests that touch data are compiled with the compile cache, which the
/// incremental update needs.
fn with_data() -> String {
    format!(
        "{SRC}{}",
        r#"
table 50503 "Row" { fields { field(1; "No."; Integer) { } } keys { key(PK; "No.") { } } }
codeunit 50504 "Data Tests"
{
    Subtype = Test;
    [Test]
    procedure ReadsRow()
    var
        R: Record "Row";
    begin
        if not R.IsEmpty() then Error('rows');
    end;
}
"#
    )
}

#[test]
fn the_incremental_update_of_serve_refreshes_the_hashes() {
    use al_testrun::runtime::{FileChange, Warm, WarmUse};
    let seen: Arc<Mutex<Vec<(String, String)>>> = Arc::default();
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("affected_run_cache");
    let _ = std::fs::remove_dir_all(&dir);
    let opts = RunOptions {
        compile_cache: Some(al_testrun::compile_cache::CompileCache {
            dir,
            require: false,
            rebuild: false,
            country: String::new(),
        }),
        ..hashing(seen.clone())
    };
    let mut warm = Warm::new();
    let run = |warm: &mut Warm, text: &str| {
        let files = [FileChange {
            app: 0,
            path: "main.al",
            text,
        }];
        let (_, used) = warm
            .run(
                "k",
                &files,
                || apps(text),
                &Default::default(),
                None,
                Default::default(),
                &[SuiteRun::everything()],
                &opts,
            )
            .unwrap();
        used
    };
    let src = with_data();
    assert_eq!(run(&mut warm, &src), WarmUse::Compiled);
    let before = seen.lock().unwrap().clone();
    let edited = src.replacen("exit(2);", "exit(1 + 1);", 1);
    assert_eq!(run(&mut warm, &edited), WarmUse::Updated);
    let after = seen.lock().unwrap().clone();
    // A complete compile of the edited text gives the same hashes as the update.
    let (a, k, m) = apps(&edited);
    CorpusRuntime::run_suites_with(
        a,
        k,
        &m,
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &opts,
    );
    let complete = seen.lock().unwrap().clone();
    assert_eq!(after, complete);
    let changed: Vec<&String> = before
        .iter()
        .zip(&after)
        .filter(|(b, a)| b.1 != a.1)
        .map(|(b, _)| &b.0)
        .collect();
    assert_eq!(changed, vec!["a5/codeunit 50500"], "{before:?} {after:?}");
}

#[test]
fn with_a_dependency_app_every_workspace_object_has_its_hash_too() {
    // With a dependency app, tests free of data are compiled without the install baseline's roots (`text_looks_pure`):
    // that compile must still hash every workspace object.
    let seen: Arc<Mutex<Vec<(String, String)>>> = Arc::default();
    let (a, k, m) = apps(SRC);
    CorpusRuntime::run_suites_with(
        a,
        k,
        &m,
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &hashing(seen.clone()),
    );
    let keys: Vec<String> = seen.lock().unwrap().iter().map(|(k, _)| k.clone()).collect();
    assert_eq!(
        keys,
        vec!["a5/codeunit 50500", "a5/codeunit 50501", "a5/codeunit 50502"]
    );
}
