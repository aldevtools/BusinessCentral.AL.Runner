//! Coverage, the server URL and colored output (docs/integrations/alchemist-lethal.md, step 2).

use super::*;

// ---- coverage (docs/integrations/alchemist-lethal.md, step 2) ----

const COVERED: &str = r#"
codeunit 50500 "Covered Tests"
{
    Subtype = Test;

    [Test]
    procedure TakesTheThenBranch()
    var
        N: Integer;
    begin
        N := 1;
        if N = 1 then
            N := 2
        else
            N := 3;
    end;

    [Test]
    procedure TakesTheElseBranch()
    var
        N: Integer;
    begin
        N := 5;
        if N = 1 then
            N := 2
        else
            N := 3;
    end;
}
"#;

fn covered_app(root: &Path) -> PathBuf {
    one_file_app(
        root,
        "covered",
        "11111111-0000-0000-0000-000000000005",
        "Covered",
        "src/Covered.al",
        COVERED,
    )
}

#[test]
fn coverage_writes_cobertura_with_the_runs_of_each_line_and_json_with_each_tests_statements() {
    let d = tempfile::tempdir().unwrap();
    let app = covered_app(d.path());
    let xml = d.path().join("out/c.xml");
    let js = d.path().join("out/c.json");
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--no-download",
        "--no-test-data",
        "--coverage-out",
        xml.to_str().unwrap(),
        "--coverage-json",
        js.to_str().unwrap(),
    ]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    assert!(text(&o).contains("coverage: "), "{}", text(&o));
    // Cobertura: a class for the file with a line per statement line. Test 1: `N := 1;` (11), the `if` (12), the then
    // branch (13) run, the else branch (15) does not. Test 2: `N := 5;` (23), the `if` (24), the else branch (27) run,
    // the then branch (25) does not.
    let x = std::fs::read_to_string(&xml).unwrap();
    assert!(x.contains("filename=\"") && x.contains("Covered.al\""), "{x}");
    assert!(x.contains("<line number=\"11\" hits=\"1\""), "{x}");
    assert!(x.contains("<line number=\"12\" hits=\"1\""), "{x}");
    let line_hits = |n: u32| -> u64 {
        let tag = format!("<line number=\"{n}\" hits=\"");
        let at = x.find(&tag).unwrap_or_else(|| panic!("no line {n} in {x}")) + tag.len();
        x[at..].split('"').next().unwrap().parse().unwrap()
    };
    assert_eq!(
        (line_hits(13), line_hits(15), line_hits(25), line_hits(27)),
        (1, 0, 0, 1)
    );
    // JSON: every statement, and per test the statements it ran.
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&js).unwrap()).unwrap();
    let file = &v["coverage"][0];
    assert!(file["file"].as_str().unwrap().ends_with("/src/Covered.al"), "{file}");
    let statements = file["statements"].as_array().unwrap();
    assert!(
        statements
            .iter()
            .any(|s| s["scope"] == "TakesTheThenBranch" && s["line"] == 13 && s["hits"] == 1)
    );
    let per_test = v["perTestCoverage"].as_array().unwrap();
    let then = per_test
        .iter()
        .find(|t| t["test"] == "Codeunit50500.TakesTheThenBranch")
        .unwrap();
    let ran: Vec<u64> = then["coverage"][0]["statements"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["line"].as_u64().unwrap())
        .collect();
    assert!(ran.contains(&13) && !ran.contains(&15), "{ran:?}");
}

#[test]
fn coverage_off_writes_nothing_and_a_selection_counts_only_the_selected_tests() {
    let d = tempfile::tempdir().unwrap();
    let app = covered_app(d.path());
    let xml = d.path().join("c.xml");
    let o = alrunner(&["test", app.to_str().unwrap(), "--no-download", "--no-test-data"]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    assert!(!text(&o).contains("coverage:"), "{}", text(&o));
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--no-download",
        "--no-test-data",
        "--test",
        "Codeunit50500.TakesTheElseBranch",
        "--coverage-out",
        xml.to_str().unwrap(),
    ]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    let x = std::fs::read_to_string(&xml).unwrap();
    // Only the second test counts: its else branch ran, the first test's then branch did not.
    assert!(x.contains("<line number=\"27\" hits=\"1\""), "{x}");
    assert!(x.contains("<line number=\"13\" hits=\"0\""), "{x}");
}

#[test]
fn server_url_is_checked_printed_and_taken_from_the_variable() {
    let d = tempfile::tempdir().unwrap();
    let app = mixed_app(d.path());
    let a = app.to_str().unwrap();
    let o = alrunner(&["test", a, "--no-download", "--server-url", "bc.example"]);
    assert_eq!(code(&o), 2, "{}", text(&o));
    assert!(text(&o).contains("--server-url"), "{}", text(&o));
    let o = alrunner(&[
        "test",
        a,
        "--no-download",
        "--verbose",
        "--server-url",
        "https://bc.example:7048/BC1/",
    ]);
    assert!(
        text(&o).contains("server address: https://bc.example:7048/BC1/ (--server-url)"),
        "{}",
        text(&o)
    );
    let o = alrunner(&["test", a, "--no-download", "--verbose"]);
    assert!(
        text(&o).contains("server address: http://localhost:8080/BC (default)"),
        "{}",
        text(&o)
    );
    let o = std::process::Command::new(env!("CARGO_BIN_EXE_al-runner"))
        .env("ALRUNNER_NO_PATH_NOTE", "1")
        .env("ALRUNNER_SERVER_URL", "http://env.example/BC/")
        .args(["test", a, "--no-download", "--verbose"])
        .output()
        .unwrap();
    assert!(
        text(&o).contains("server address: http://env.example/BC/ (ALRUNNER_SERVER_URL)"),
        "{}",
        text(&o)
    );
}

#[test]
fn color_always_paints_the_labels_and_never_or_no_color_keeps_them_plain() {
    let d = tempfile::tempdir().unwrap();
    let app = mixed_app(d.path());
    let a = app.to_str().unwrap();
    let t = text(&alrunner(&[
        "test",
        a,
        "--no-download",
        "--verbose",
        "--color",
        "always",
    ]));
    assert!(t.contains("\x1b[32mPASS"), "{t}");
    assert!(t.contains("\x1b[31mFAIL"), "{t}");
    assert!(t.contains("\x1b[33mNOT RUN"), "{t}");
    assert!(t.contains("\x1b[1m"), "headings are bold: {t}");
    let t = text(&alrunner(&["test", a, "--no-download", "--color", "never"]));
    assert!(!t.contains('\x1b'), "{t}");
    let o = std::process::Command::new(env!("CARGO_BIN_EXE_al-runner"))
        .env("ALRUNNER_NO_PATH_NOTE", "1")
        .env("NO_COLOR", "1")
        .args(["test", a, "--no-download"])
        .output()
        .unwrap();
    assert!(!text(&o).contains('\x1b'));
    // A JUnit file is plain even with --color always.
    let xml = d.path().join("r.xml");
    alrunner(&[
        "test",
        a,
        "--no-download",
        "--color",
        "always",
        "--junit",
        xml.to_str().unwrap(),
    ]);
    assert!(!std::fs::read_to_string(&xml).unwrap().contains('\x1b'));
}
