//! Compile errors that end a run before any test, and test selection by name.

use super::*;

/// al-runner issue 5573: an app that declares runtime 13.0 calls NavApp.GetResourceAsText, which the AL compiler
/// rejects below runtime 14.0 (AL0666). The run ends as the compiler's does: a compile error, exit 3.
#[test]
fn a_call_newer_than_the_apps_declared_runtime_is_a_compile_error() {
    let d = tempfile::tempdir().unwrap();
    let app = d.path().join("app");
    let manifest = |id: &str, name: &str, deps: &[(&str, &str)]| {
        app_json(id, name, deps).replacen('{', r#"{"runtime":"13.0","#, 1)
    };
    write(
        &app,
        "app.json",
        &manifest("6f1d2f8e-5b0a-4c1e-9a43-0d6b8a2c5e11", "Resource Repro App", &[]),
    );
    write(&app, "res/value.txt", "HELLO");
    write(
        &app,
        "src/Reader.Codeunit.al",
        "codeunit 50100 \"Resource Reader\"\n{\n    procedure Value(): Text\n    begin\n        exit(NavApp.GetResourceAsText('value.txt', TextEncoding::UTF8));\n    end;\n\n    procedure Plain(): Text\n    begin\n        exit(UpperCase('hello'));\n    end;\n}\n",
    );
    let test = d.path().join("test");
    write(
        &test,
        "app.json",
        &manifest(
            "6f1d2f8e-5b0a-4c1e-9a43-0d6b8a2c5e12",
            "Resource Repro Tests",
            &[("6f1d2f8e-5b0a-4c1e-9a43-0d6b8a2c5e11", "Resource Repro App")],
        ),
    );
    write(
        &test,
        "src/ReaderTest.Codeunit.al",
        "codeunit 50150 \"Resource Reader Test\"\n{\n    Subtype = Test;\n\n    [Test]\n    procedure ReadsResource()\n    var\n        Reader: Codeunit \"Resource Reader\";\n    begin\n        if Reader.Value() <> 'HELLO' then\n            Error('expected HELLO, got %1', Reader.Value());\n    end;\n\n    [Test]\n    procedure ReadsPlain()\n    var\n        Reader: Codeunit \"Resource Reader\";\n    begin\n        if Reader.Plain() <> 'HELLO' then\n            Error('expected HELLO');\n    end;\n}\n",
    );
    let o = alrunner(&["test", app.to_str().unwrap(), test.to_str().unwrap(), "--no-download"]);
    // As the AL compiler does (catalog/alc-runtimes.txt, measured with AL 30.0.42.60748): AL0666, no test runs, exit 3.
    assert_eq!(code(&o), 3, "{}", text(&o));
    assert!(
        text(&o).contains(
            "src/Reader.Codeunit.al(5,14): error AL0666: 'GetResourceAsText' is not available in runtime version '13.0'. The supported runtime versions are: '14.0' or greater."
        ),
        "{}",
        text(&o)
    );
    assert!(text(&o).contains("Result: COMPILE ERRORS (exit 3)"), "{}", text(&o));
}

/// The checks that depend on the app's `target` run on a real run, not only in the must-not-compile harness: a Cloud
/// app (no `target`, the compiler's default) that calls `File.IsPathTemporary`, a method with scope OnPrem, does not
/// compile (must-not-compile 1310-file-ispathtemporary-cloud.al:1), even in a procedure no test calls. The same app
/// with `"target": "OnPrem"` compiles and its test runs.
#[test]
fn an_onprem_method_called_from_a_cloud_app_is_a_compile_error() {
    let run = |target: Option<&str>| {
        let d = tempfile::tempdir().unwrap();
        let app = d.path().join("app");
        let mut json = app_json("6f1d2f8e-5b0a-4c1e-9a43-0d6b8a2c5e21", "Scope Repro App", &[]);
        if let Some(t) = target {
            json = json.replacen('{', &format!(r#"{{"target":"{t}","#), 1);
        }
        write(&app, "app.json", &json);
        write(
            &app,
            "src/Probe.Codeunit.al",
            "codeunit 50100 \"Scope Probe\"\n{\n    Subtype = Test;\n\n    procedure Temporary(): Boolean\n    begin\n        exit(File.IsPathTemporary('x.tmp'));\n    end;\n\n    [Test]\n    procedure Plain()\n    begin\n        if UpperCase('a') <> 'A' then\n            Error('expected A');\n    end;\n}\n",
        );
        let o = alrunner(&["test", app.to_str().unwrap(), "--no-download"]);
        (code(&o), text(&o))
    };
    let (exit, out) = run(None);
    assert_eq!(exit, 3, "{out}");
    assert!(
        out.contains(
            "src/Probe.Codeunit.al(7,14): error AL0296: The application object or method 'IsPathTemporary' has scope 'OnPrem' and cannot be used for 'Cloud' development."
        ),
        "{out}"
    );
    assert!(out.contains("Result: COMPILE ERRORS (exit 3)"), "{out}");
    let (exit, out) = run(Some("OnPrem"));
    assert_eq!(exit, 0, "{out}");
}

/// The call with scope OnPrem stops a Cloud app's run also when it sits in a test codeunit the selection leaves out:
/// alc compiles the whole app (al-runner issue 5602), so the rule runs on the code the selection leaves out too.
#[test]
fn an_onprem_method_in_code_a_selection_leaves_out_is_a_compile_error() {
    let d = tempfile::tempdir().unwrap();
    let app = d.path().join("app");
    write(
        &app,
        "app.json",
        &app_json("6f1d2f8e-5b0a-4c1e-9a43-0d6b8a2c5e22", "Scope Repro App", &[]),
    );
    write(
        &app,
        "src/Probe.Codeunit.al",
        "codeunit 50101 \"Scope Probe\"\n{\n    Subtype = Test;\n\n    [Test]\n    procedure Temporary()\n    begin\n        if File.IsPathTemporary('x.tmp') then;\n    end;\n}\n",
    );
    write(
        &app,
        "src/Plain.Codeunit.al",
        "codeunit 50100 \"Plain Test\"\n{\n    Subtype = Test;\n\n    [Test]\n    procedure Plain()\n    begin\n        if UpperCase('a') <> 'A' then\n            Error('expected A');\n    end;\n}\n",
    );
    for extra in [&[][..], &["--test", "Plain"][..]] {
        let mut args = vec!["test", app.to_str().unwrap(), "--no-download"];
        args.extend_from_slice(extra);
        let o = alrunner(&args);
        assert_eq!(code(&o), 3, "{extra:?}: {}", text(&o));
        assert!(
            text(&o).contains("src/Probe.Codeunit.al(8,12): error AL0296: The application object or method 'IsPathTemporary' has scope 'OnPrem' and cannot be used for 'Cloud' development."),
            "{extra:?}: {}",
            text(&o)
        );
    }
}

/// Code the AL compiler rejects is not run (al-runner issue 5602): an AL0118 in a procedure no test calls, an error in a
/// test codeunit the selection leaves out each stop the run with exit 3, the
/// errors on stderr in the AL compiler's form, and `compilationErrors` in the JSON document. Before, the selected test
/// passed with exit 0.
#[test]
fn code_that_does_not_compile_runs_no_test_and_exits_3() {
    let d = tempfile::tempdir().unwrap();
    let app = d.path().join("app");
    write(
        &app,
        "app.json",
        &app_json("11111111-0000-0000-0000-000000005602", "Logic", &[]),
    );
    let logic = "codeunit 50600 \"Probe Logic\" {\n    procedure Add(A: Integer; B: Integer): Integer begin exit(A + B); end;\n    procedure Other(): Integer var I: Integer; begin I := NoSuchProc(); exit(I); end;\n}\n";
    write(&app, "src/Logic.al", logic);
    let tests = d.path().join("tests");
    write(
        &tests,
        "app.json",
        &app_json(
            "11111111-0000-0000-0000-000000005603",
            "Logic Tests",
            &[("11111111-0000-0000-0000-000000005602", "Logic")],
        ),
    );
    write(
        &tests,
        "src/Tests.al",
        "codeunit 50650 \"Probe Tests\" { Subtype = Test; [Test] procedure Passes() var L: Codeunit \"Probe Logic\"; begin if L.Add(2, 3) <> 5 then Error('bad'); end; }",
    );
    let json = d.path().join("out.json");
    let run = |extra: &[&str]| {
        let mut args = vec!["test", app.to_str().unwrap(), tests.to_str().unwrap(), "--no-download"];
        args.extend_from_slice(extra);
        alrunner(&args)
    };
    let o = run(&[
        "--test-exact",
        "Codeunit50650.Passes",
        "--output-json",
        json.to_str().unwrap(),
    ]);
    assert_eq!(code(&o), 3, "{}", text(&o));
    assert!(
        text(&o)
            .contains("src/Logic.al(3,59): error AL0118: The name 'NoSuchProc' does not exist in the current context."),
        "{}",
        text(&o)
    );
    assert!(text(&o).contains("Result: COMPILE ERRORS (exit 3)"), "{}", text(&o));
    let doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&json).unwrap()).unwrap();
    assert_eq!(doc["exitCode"], 3);
    assert_eq!(doc["total"], 0);
    let groups = doc["compilationErrors"].as_array().expect("compilationErrors");
    assert_eq!(groups.len(), 1, "{doc}");
    assert!(
        groups[0]["file"].as_str().unwrap().ends_with("app/src/Logic.al"),
        "{doc}"
    );
    assert_eq!(groups[0]["errors"].as_array().unwrap().len(), 1, "{doc}");

    // Fixed, the test runs and passes.
    write(&app, "src/Logic.al", &logic.replace("NoSuchProc()", "1"));
    let o = run(&["--test-exact", "Codeunit50650.Passes"]);
    assert_eq!(code(&o), 0, "{}", text(&o));

    // An error in a test codeunit the selection does not pick.
    write(
        &tests,
        "src/Other.al",
        "codeunit 50651 \"Other Tests\" { Subtype = Test; [Test] procedure Q() begin NoSuchProc(); end; }",
    );
    let o = run(&["--codeunit", "50650"]);
    assert_eq!(code(&o), 3, "{}", text(&o));
    assert!(text(&o).contains("src/Other.al(1,75): error AL0118"), "{}", text(&o));
    std::fs::remove_file(tests.join("src/Other.al")).unwrap();

    // Source our parser does not accept stops the run too (al-runner issue 5916): every file the parser rejects in the
    // corpus is one alc rejects (corpus/tests/must-not-compile), and every other corpus file parses
    // (al-syntax/tests/corpus_parse.rs). alc's code for it is not known, so the line has our parser's message and no
    // AL code. Before, only the tests that needed the file were not run and the others passed.
    write(
        &app,
        "src/Broken.al",
        "codeunit 50601 \"Broken\" {\n    procedure Ok(): Integer begin exit(1); end;\n    procedure Spin(: Integer\n    begin\n    end;\n}\n",
    );
    let o = run(&[
        "--test-exact",
        "Codeunit50650.Passes",
        "--output-json",
        json.to_str().unwrap(),
    ]);
    assert_eq!(code(&o), 3, "{}", text(&o));
    assert!(
        text(&o).contains("src/Broken.al(3,20): error: expected a name, found `:`"),
        "{}",
        text(&o)
    );
    assert!(text(&o).contains("Result: COMPILE ERRORS (exit 3)"), "{}", text(&o));
    let doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&json).unwrap()).unwrap();
    assert_eq!(doc["total"], 0);
    assert!(
        doc["compilationErrors"].to_string().contains("Broken.al(3,20)"),
        "{doc}"
    );
    std::fs::remove_file(app.join("src/Broken.al")).unwrap();

    // A syntax error in the selected test codeunit.
    write(
        &tests,
        "src/Tests.al",
        "codeunit 50650 \"Probe Tests\" { Subtype = Test; [Test] procedure Passes() begin Error('boom %1', 5; end; }",
    );
    let o = run(&["--codeunit", "50650"]);
    assert_eq!(code(&o), 3, "{}", text(&o));
    assert!(text(&o).contains("src/Tests.al(1,98): error: "), "{}", text(&o));
}

/// Issue 5917: a test that calls a procedure its codeunit does not declare is AL0132 (must-not-compile 1780): the run
/// stops with exit 3 and the error at the member name, as alc gives it; before, that test was not run and the others passed.
#[test]
fn a_call_of_a_procedure_the_codeunit_lacks_exits_3() {
    let d = tempfile::tempdir().unwrap();
    let app = d.path().join("app");
    write(
        &app,
        "app.json",
        &app_json("11111111-0000-0000-0000-000000005917", "Logic", &[]),
    );
    write(
        &app,
        "src/Logic.al",
        "codeunit 50600 \"Probe Logic\"\n{\n    procedure Add(A: Integer; B: Integer): Integer\n    begin\n        exit(A + B);\n    end;\n}\n",
    );
    let tests = d.path().join("tests");
    write(
        &tests,
        "app.json",
        &app_json(
            "11111111-0000-0000-0000-000000005918",
            "Logic Tests",
            &[("11111111-0000-0000-0000-000000005917", "Logic")],
        ),
    );
    write(
        &tests,
        "src/Tests.al",
        "codeunit 50650 \"Probe Tests\"\n{\n    Subtype = Test;\n\n    [Test]\n    procedure Passes()\n    var\n        Logic: Codeunit \"Probe Logic\";\n    begin\n        if Logic.NoSuch(1, 1) <> 2 then\n            Error('must be 2');\n    end;\n\n    [Test]\n    procedure Standalone()\n    begin\n    end;\n}\n",
    );
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        tests.to_str().unwrap(),
        "--no-download",
        "--codeunit",
        "50650",
    ]);
    assert_eq!(code(&o), 3, "{}", text(&o));
    assert!(
        text(&o).contains(
            "src/Tests.al(10,18): error AL0132: 'Codeunit \"Probe Logic\"' does not contain a definition for 'NoSuch'"
        ),
        "{}",
        text(&o)
    );
}

/// Issue 5613: with `--output-json`, a selection that matches no test still writes the document (exit 6, no rows,
/// the reason in `warnings`), on stdout and to a file.
#[test]
fn output_json_writes_the_document_when_the_selection_matches_no_test() {
    let d = tempfile::tempdir().unwrap();
    let app = mixed_app(d.path());
    let a = app.to_str().unwrap();
    let o = alrunner(&[
        "test",
        a,
        "--no-download",
        "--output-json",
        "--test",
        "Codeunit0.NoSuchTest",
    ]);
    assert_eq!(code(&o), 6, "{}", text(&o));
    let doc = json_of(&o);
    assert_eq!(doc["exitCode"], 6, "{doc}");
    assert_eq!(doc["total"], 0, "{doc}");
    assert_eq!(doc["tests"].as_array().map(Vec::len), Some(0), "{doc}");
    assert!(
        doc["warnings"][0].as_str().unwrap().contains("matches no test"),
        "{doc}"
    );
    let file = d.path().join("out.json");
    let o = alrunner(&[
        "test",
        a,
        "--no-download",
        "--output-json",
        file.to_str().unwrap(),
        "--test",
        "NoSuchTest",
    ]);
    assert_eq!(code(&o), 6, "{}", text(&o));
    let doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(doc["exitCode"], 6, "{doc}");
}

/// Issue 5572: a test whose AL name is a quoted identifier is selected by that name, with --test-exact, --test
/// and --explain; issue 5612: --test with --test-exact selects only the tests both match.
#[test]
fn a_quoted_test_name_is_selected_by_its_al_name() {
    let d = tempfile::tempdir().unwrap();
    let dir = d.path().join("quoted");
    write(
        &dir,
        "app.json",
        &app_json("11111111-0000-0000-0000-000000000072", "Quoted", &[]),
    );
    write(
        &dir,
        "src/Tests.al",
        "codeunit 50650 \"Probe Tests\"\n{\n    Subtype = Test;\n\n    [Test]\n    procedure \"Space Step\"()\n    begin\n    end;\n\n    [Test]\n    procedure \"Dot.Name\"()\n    begin\n    end;\n\n    [Test]\n    procedure Passes()\n    begin\n    end;\n\n    [Test]\n    procedure PassesToo()\n    begin\n    end;\n}\n",
    );
    let a = dir.to_str().unwrap();
    let names = |args: &[&str]| -> Vec<String> {
        let mut all = vec!["test", a, "--no-download", "--output-json"];
        all.extend_from_slice(args);
        let o = alrunner(&all);
        assert_eq!(code(&o), 0, "{args:?}: {}", text(&o));
        let doc = json_of(&o);
        let mut n: Vec<String> = doc["tests"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap().to_string())
            .collect();
        n.sort();
        n
    };
    let space = vec!["Codeunit50650.Space Step".to_string()];
    assert_eq!(names(&["--test-exact", "Codeunit50650.Space Step"]), space);
    assert_eq!(names(&["--test-exact", "space step"]), space);
    assert_eq!(names(&["--test", "Space Step"]), space);
    assert_eq!(
        names(&["--test-exact", "Codeunit50650.Dot.Name"]),
        vec!["Codeunit50650.Dot.Name".to_string()]
    );
    assert_eq!(names(&["--test", "Dot"]), vec!["Codeunit50650.Dot.Name".to_string()]);
    assert_eq!(
        names(&["--test", "Codeunit50650.Passes", "--test-exact", "Codeunit50650.Passes"]),
        vec!["Codeunit50650.Passes".to_string()]
    );
    let o = alrunner(&["test", a, "--no-download", "--explain", "Space Step"]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    assert!(text(&o).contains("Space Step"), "{}", text(&o));
}
