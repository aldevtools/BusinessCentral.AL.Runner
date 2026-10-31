//! Test data backups, failure ranking, explain, call stacks and warm.

use super::*;

const TP_TESTS: &str = r#"
table 50100 "exttest" { fields { field(1; id; Integer) { } field(2; own; Text[50]) { } } keys { key(PK; id) { } } }
table 50101 "stray" { fields { field(1; id; Integer) { } } keys { key(PK; id) { } } }
codeunit 50100 "Data Tests" { Subtype = Test;
  [Test] procedure RestoredRows() var E: Record "exttest"; begin
    if E.Count() <> 3 then Error('count %1', E.Count());
    E.Get(2);
    if E.own <> 'base-two' then Error(E.own);
    if CompanyName() <> 'TP' then Error(CompanyName());
  end;
  [Test] procedure AnEmptyTable() var S: Record "stray"; begin if not S.IsEmpty() then Error('rows'); end; }
"#;

fn typeprobe() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../al-bcdb/tests/fixtures/typeprobe.bak")
}

#[test]
fn test_data_restores_a_company_from_a_backup_and_the_summary_says_what_was_loaded() {
    let d = tempfile::tempdir().unwrap();
    let dir = d.path().join("tp");
    write(
        &dir,
        "app.json",
        &app_json("11111111-0000-0000-0000-000000000002", "TP", &[]),
    );
    write(&dir, "src/Tests.al", TP_TESTS);
    let bak = typeprobe();
    let o = alrunner(&[
        "test",
        dir.to_str().unwrap(),
        "--no-download",
        "--verbose",
        "--test-data",
        bak.to_str().unwrap(),
        "--test-data-company",
        "TP",
    ]);
    let t = text(&o);
    assert_eq!(code(&o), 0, "{t}");
    assert!(t.contains("Tests: 2 total, 2 passed, 0 failed, 0 not run"), "{t}");
    assert!(
        !t.contains("no test data"),
        "a run with test data carries no such warning: {t}"
    );
    assert!(
        t.contains("Test data: 3 rows loaded from 1 tables of company 'TP' in typeprobe.bak"),
        "{t}"
    );
    assert!(
        t.contains("test data: ") && t.contains("typeprobe.bak (company 'TP'"),
        "the header names the source: {t}"
    );
}

#[test]
fn a_company_the_backup_does_not_hold_is_a_start_up_error_that_lists_the_companies() {
    let d = tempfile::tempdir().unwrap();
    let app = mixed_app(d.path());
    let bak = typeprobe();
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--no-download",
        "--test-data",
        bak.to_str().unwrap(),
        "--test-data-company",
        "Nope",
    ]);
    assert_eq!(code(&o), 2);
    assert!(
        text(&o).contains("'ProbeCo'") && text(&o).contains("'TP'"),
        "{}",
        text(&o)
    );
}

#[test]
fn top_failures_ranks_the_failure_messages_with_numbers_folded() {
    let d = tempfile::tempdir().unwrap();
    let app = mixed_app(d.path());
    let o = alrunner(&["test", app.to_str().unwrap(), "--no-download", "--top-failures", "5"]);
    let t = text(&o);
    assert!(
        t.contains("Top failures, failed tests: 1 failed, 1 distinct reasons"),
        "{t}"
    );
    assert!(t.contains("boom N"), "{t}");
}

#[test]
fn explain_shows_where_a_failing_test_raised_its_error() {
    let d = tempfile::tempdir().unwrap();
    let app = mixed_app(d.path());
    let o = alrunner(&["test", app.to_str().unwrap(), "--no-download", "--explain", "Fails"]);
    let t = text(&o);
    assert!(t.contains("runtime: FAIL"), "{t}");
    assert!(t.contains("error: boom 5"), "{t}");
    assert!(t.contains("raised in (innermost first):"), "{t}");
    assert!(t.contains("Mixed Tests") && t.contains("Fails"), "{t}");
    assert!(t.contains("src/Tests.al:"), "the source position is named: {t}");
    assert_eq!(code(&o), 1, "{t}");
}

#[test]
fn explain_shows_where_a_refused_test_was_refused() {
    let d = tempfile::tempdir().unwrap();
    let app = mixed_app(d.path());
    let o = alrunner(&["test", app.to_str().unwrap(), "--no-download", "--explain", "Refused"]);
    let t = text(&o);
    assert!(t.contains("runtime: NOT RUN (refused)"), "{t}");
    assert!(t.contains("reason: "), "{t}");
    assert_eq!(code(&o), 7, "{t}");
}

#[test]
fn explain_needs_exactly_one_test() {
    let d = tempfile::tempdir().unwrap();
    let app = mixed_app(d.path());
    let o = alrunner(&["test", app.to_str().unwrap(), "--no-download", "--explain", "Pass"]);
    let t = text(&o);
    assert!(t.contains("matches 2 tests"), "{t}");
    assert_eq!(code(&o), 2, "{t}");
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--no-download",
        "--explain",
        "NoSuchTest",
    ]);
    assert_eq!(code(&o), 6, "{}", text(&o));
}

const DEEP: &str = r#"
codeunit 50200 "Deep Tests"
{
    Subtype = Test;

    local procedure Inner()
    begin
        Error('deep boom');
    end;

    local procedure Middle()
    begin
        Inner();
    end;

    [Test]
    procedure DeepFails()
    begin
        Middle();
    end;
}
"#;

fn deep_app(root: &Path) -> PathBuf {
    let dir = root.join("deep");
    write(
        &dir,
        "app.json",
        &app_json("11111111-0000-0000-0000-000000000002", "Deep", &[]),
    );
    write(&dir, "src/Deep.al", DEEP);
    dir
}

#[test]
fn a_failure_shows_the_al_call_stack_in_the_console_and_in_junit() {
    let d = tempfile::tempdir().unwrap();
    let app = deep_app(d.path());
    let junit = d.path().join("junit.xml");
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--no-download",
        "--junit",
        junit.to_str().unwrap(),
    ]);
    let t = text(&o);
    assert_eq!(code(&o), 1, "{t}");
    // Innermost frame first: Inner, Middle, then the test itself.
    let frames: Vec<&str> = t
        .lines()
        .filter(|l| l.trim_start().starts_with("\"Deep Tests\"(CodeUnit 50200)"))
        .collect();
    assert_eq!(frames.len(), 3, "{t}");
    assert!(frames[0].contains(".Inner line 2"), "{t}");
    assert!(frames[1].contains(".Middle line 2"), "{t}");
    assert!(frames[2].contains(".DeepFails line 2"), "{t}");
    assert!(
        t.lines().any(|l| l.starts_with("FAIL") && l.contains("deep boom")),
        "{t}"
    );
    let x = std::fs::read_to_string(&junit).unwrap();
    assert!(x.contains("<failure message=\"deep boom\">"), "{x}");
    assert!(x.contains(".Inner line 2"), "{x}");
    assert!(x.contains("</failure>"), "{x}");
}

#[test]
fn quiet_leaves_the_call_stack_out_of_the_console() {
    let d = tempfile::tempdir().unwrap();
    let app = deep_app(d.path());
    let o = alrunner(&["test", app.to_str().unwrap(), "--no-download", "--quiet"]);
    assert!(!text(&o).contains("CodeUnit 50200"), "{}", text(&o));
}

#[test]
fn warm_builds_the_index_of_a_backup_so_that_the_first_run_does_not() {
    let backup = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../al-bcdb/tests/fixtures/typeprobe.bak");
    let cache = tempfile::tempdir().unwrap();
    let args = [
        "warm",
        backup.to_str().unwrap(),
        "--test-data-company",
        "TP",
        "--cache-dir",
        cache.path().to_str().unwrap(),
    ];
    let o = alrunner(&args);
    assert_eq!(code(&o), 0, "{}", text(&o));
    assert!(text(&o).contains("built and saved"), "{}", text(&o));
    let dir = cache.path().join("backup-index");
    let files: Vec<_> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()).collect();
    assert_eq!(files.len(), 1, "{files:?}");
    let o = alrunner(&args);
    assert_eq!(code(&o), 0, "{}", text(&o));
    assert!(text(&o).contains("kept map and catalog used"), "{}", text(&o));
}

#[test]
fn warm_without_a_backup_or_a_build_says_what_it_needs() {
    let o = alrunner(&["warm"]);
    assert_eq!(code(&o), 2);
    assert!(
        text(&o).contains("PATH.bak") && text(&o).contains("--bc-version"),
        "{}",
        text(&o)
    );
}
