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
