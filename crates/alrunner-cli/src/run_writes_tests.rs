//! Round 2 MED R2-1: a process that makes several runs (one side after the other of an impact or `--affected-from`
//! run, the lanes of a recipe) writes the workspace layer entry of each run when that run ends, so the compiled objects
//! a queued write holds do not live on into the next run.

/// A run of `test_collect` over a small app leaves no write queued, and its entry is in the cache folder.
#[test]
fn a_collected_run_writes_its_entry_before_it_returns() {
    let d = tempfile::tempdir().unwrap();
    let app = d.path().join("app");
    std::fs::create_dir_all(app.join("src")).unwrap();
    std::fs::write(
        app.join("app.json"),
        r#"{"id":"77777777-0000-0000-0000-000000000001","name":"Writes","publisher":"T","version":"1.0.0.0","dependencies":[]}"#,
    )
    .unwrap();
    std::fs::write(
        app.join("src/t.al"),
        "codeunit 50700 \"W Tests\" { Subtype = Test; [Test] procedure P() begin end; }\n",
    )
    .unwrap();
    let cache = d.path().join("cache");
    let line = format!(
        "test {} --no-download --no-test-data --cache-dir {}",
        app.display(),
        cache.display()
    );
    let a = match crate::args::parse(line.split_whitespace().map(String::from)).unwrap() {
        crate::args::Command::Test(a) => a,
        other => panic!("{other:?}"),
    };
    let mut into = crate::run::Collected::default();
    crate::run::test_collect(&a, &mut into).unwrap();
    assert_eq!(
        al_testrun::workspace_cache::pending_writes(),
        0,
        "a write is still queued"
    );
    let entries = al_testrun::compile_cache::list_workspace(&cache.join("compile"));
    assert_eq!(entries.len(), 1, "{entries:?}");
}
