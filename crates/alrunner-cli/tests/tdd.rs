//! `al-runner test --tdd` (ADR-051) end to end on the fixture apps in `tests/fixtures/tdd`: a test that calls a
//! procedure that does not exist yet runs against a stub that returns the default of its return type, and is
//! reported as stubbed. Nothing here is Microsoft's code; the fixtures are written for this runner.

use std::path::PathBuf;
use std::process::{Command, Output};

use serde_json::Value;

fn fixture(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/tdd")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

fn alrunner(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_al-runner"))
        .env("AL_CHECK_THREADS", "2")
        .env("AL_LOWER_THREADS", "2")
        .env("AL_TEST_THREADS", "2")
        .env("ALRUNNER_NO_PATH_NOTE", "1")
        .args(args)
        .output()
        .expect("run al-runner")
}

fn text(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

fn code(o: &Output) -> i32 {
    o.status.code().expect("exit code")
}

/// The `--output-json` document of a run of `app` with `extra` flags.
fn json_run(app: &str, extra: &[&str]) -> (Value, Output) {
    let mut args = vec!["test", app, "--no-download", "--no-test-data", "--output-json"];
    args.extend_from_slice(extra);
    let o = alrunner(&args);
    let doc: Value = serde_json::from_slice(&o.stdout).unwrap_or_else(|e| panic!("{e}: {}", text(&o)));
    (doc, o)
}

fn row<'a>(doc: &'a Value, method: &str) -> &'a Value {
    doc["tests"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["method"] == method)
        .unwrap_or_else(|| panic!("no row {method}: {doc:#}"))
}

fn stubs(row: &Value) -> Vec<String> {
    row["generatedStubs"]
        .as_array()
        .map(|a| a.iter().map(|s| s.as_str().unwrap().to_string()).collect())
        .unwrap_or_default()
}

#[test]
fn without_tdd_a_call_of_a_missing_procedure_is_not_run_as_today() {
    let (doc, o) = json_run(&fixture("procs"), &[]);
    let r = row(&doc, "VariantSiblingFailsRed");
    assert_eq!(r["status"], "error", "{r:#}");
    assert_eq!(r["notRun"], true, "{r:#}");
    assert!(r.get("stubbed").is_none(), "{r:#}");
    assert!(r.get("generatedStubs").is_none(), "{r:#}");
    assert!(
        doc.get("stubbed").is_none() && doc.get("generatedStubs").is_none(),
        "{doc:#}"
    );
    assert_eq!(row(&doc, "Healthy")["status"], "pass");
    assert_eq!(code(&o), 7, "{}", text(&o));
}

#[test]
fn a_stubbed_test_runs_and_fails_red_with_the_stub_named() {
    let (doc, _) = json_run(&fixture("procs"), &["--tdd"]);
    let r = row(&doc, "VariantSiblingFailsRed");
    assert_eq!(r["status"], "fail", "{r:#}");
    assert_eq!(r["stubbed"], true, "{r:#}");
    assert_eq!(
        stubs(r),
        vec![r#"Tdd Target: procedure "DoubleIt"(Arg1: Integer): Integer"#],
        "{r:#}"
    );
}

#[test]
fn a_stubbed_pass_counts_as_a_pass_and_is_counted_apart() {
    let (doc, o) = json_run(&fixture("procs"), &["--tdd"]);
    let r = row(&doc, "DefaultHoldsAndPasses");
    assert_eq!(r["status"], "pass", "{r:#}");
    assert_eq!(r["stubbed"], true, "{r:#}");
    // The healthy test is not stubbed.
    let h = row(&doc, "Healthy");
    assert_eq!(h["status"], "pass");
    assert!(h.get("stubbed").is_none() && h.get("generatedStubs").is_none(), "{h:#}");
    // Summary: stubbed passes and failures apart, and every stub of the program.
    assert_eq!(doc["stubbed"]["failed"], 1, "{doc:#}");
    assert_eq!(doc["stubbed"]["error"], 0, "{doc:#}");
    let all: Vec<&str> = doc["generatedStubs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap())
        .collect();
    assert!(all.contains(&r#"Tdd Target: procedure "Never"()"#), "{all:?}");
    // A test failed: the run exits 1, as any run with a failure.
    assert_eq!(code(&o), 1, "{}", text(&o));
}

#[test]
fn each_rule_of_the_return_type_gives_the_default_of_that_type() {
    let (doc, _) = json_run(&fixture("procs"), &["--tdd"]);
    let cases: &[(&str, &str)] = &[
        (
            "AssignTarget",
            r#"Tdd Target: procedure "Price"(Arg1: Text, Arg2: Integer): Decimal"#,
        ),
        ("KnownParameter", r#"Tdd Target: procedure "Name"(): Text"#),
        ("ExitValue", r#"Tdd Target: procedure "CodeOf"(): Code[20]"#),
        ("Comparison", r#"Tdd Target: procedure "Total"(): Integer"#),
        ("CaseSelector", r#"Tdd Target: procedure "Kind"(): Integer"#),
        ("FormatIsText", r#"Tdd Target: procedure "Label"(): Text"#),
        ("BareStatement", r#"Tdd Target: procedure "DoThing"()"#),
        ("VarArgumentUnchanged", r#"Tdd Target: procedure "Fill"(Arg1: Integer)"#),
        ("ArgumentRunsOnce", r#"Tdd Target: procedure "Log"(Arg1: Integer)"#),
        (
            "RecordIsEmpty",
            r#"Tdd Target: procedure "GetItem"(): Record "Tdd Item""#,
        ),
    ];
    for (test, stub) in cases {
        let r = row(&doc, test);
        assert_eq!(r["status"], "pass", "{test}: {r:#}");
        assert_eq!(r["stubbed"], true, "{test}: {r:#}");
        assert_eq!(stubs(r), vec![stub.to_string()], "{test}: {r:#}");
    }
    let r = row(&doc, "Condition");
    assert_eq!(r["status"], "pass", "{r:#}");
    assert_eq!(
        stubs(r),
        vec![
            r#"Tdd Target: procedure "IsDone"(): Boolean"#.to_string(),
            r#"Tdd Target: procedure "IsReady"(): Boolean"#.to_string()
        ],
        "{r:#}"
    );
}

#[test]
fn a_stub_in_a_branch_that_does_not_run_does_not_flag_the_test() {
    let (doc, _) = json_run(&fixture("procs"), &["--tdd"]);
    let r = row(&doc, "BranchNotRun");
    assert_eq!(r["status"], "pass", "{r:#}");
    assert!(r.get("stubbed").is_none(), "{r:#}");
}

#[test]
fn a_site_whose_type_cannot_be_inferred_refuses_its_test_and_the_others_run() {
    let (doc, _) = json_run(&fixture("procs"), &["--tdd"]);
    let r = row(&doc, "BothSidesUnknown");
    assert_eq!(r["notRun"], true, "{r:#}");
    assert!(
        r["message"]
            .as_str()
            .unwrap()
            .contains("--tdd: cannot infer the return type of Target.ExpectedOf (both sides are unknown)"),
        "{r:#}"
    );
    let r = row(&doc, "ResultUsedAsReceiver");
    assert_eq!(r["notRun"], true, "{r:#}");
    assert!(
        r["message"]
            .as_str()
            .unwrap()
            .contains("--tdd: cannot infer the return type of Target.GetRec (its result is used as a receiver)"),
        "{r:#}"
    );
    assert_eq!(row(&doc, "StillRuns")["status"], "pass");
}

#[test]
fn the_text_output_names_the_stubs_and_counts_stubbed_tests_apart() {
    let o = alrunner(&["test", &fixture("procs"), "--no-download", "--no-test-data", "--tdd"]);
    let t = text(&o);
    assert!(
        t.contains(r#"reaches generated stub(s): Tdd Target: procedure "DoubleIt"(Arg1: Integer): Integer"#),
        "{t}"
    );
    assert!(
        t.contains("Tests: 18 total, 15 passed (12 stubbed), 1 failed (1 stubbed), 2 not run"),
        "{t}"
    );
    assert!(t.contains("--tdd: generated 15 member(s) this run:"), "{t}");
    assert!(t.contains("--tdd: 13 test(s) reach generated stubs this run:"), "{t}");
    assert!(
        t.contains(
            r#"Tdd Proc Tests.VariantSiblingFailsRed (fail): Tdd Target: procedure "DoubleIt"(Arg1: Integer): Integer"#
        ),
        "{t}"
    );
}

#[test]
fn junit_has_a_stubbed_property_on_each_stubbed_test() {
    let d = tempfile::tempdir().unwrap();
    let junit = d.path().join("r.xml");
    let o = alrunner(&[
        "test",
        &fixture("procs"),
        "--no-download",
        "--no-test-data",
        "--tdd",
        "--junit",
        junit.to_str().unwrap(),
    ]);
    let x = std::fs::read_to_string(&junit).unwrap_or_else(|e| panic!("{e}: {}", text(&o)));
    assert!(
        x.contains(
            r#"<property name="stubbed" value="Tdd Target: procedure &quot;DoubleIt&quot;(Arg1: Integer): Integer" />"#
        ),
        "{x}"
    );
    let healthy = x.split("<testcase").find(|c| c.contains(r#"name="Healthy""#)).unwrap();
    assert!(!healthy.contains("stubbed"), "{healthy}");
}

#[test]
fn a_run_whose_only_failures_are_stubbed_exits_1_and_one_of_stubbed_passes_exits_0() {
    let o = alrunner(&[
        "test",
        &fixture("procs"),
        "--no-download",
        "--no-test-data",
        "--tdd",
        "--codeunit",
        "50110",
    ]);
    assert_eq!(code(&o), 1, "{}", text(&o));
    let o = alrunner(&[
        "test",
        &fixture("procs"),
        "--no-download",
        "--no-test-data",
        "--tdd",
        "--test",
        "DefaultHoldsAndPasses",
    ]);
    assert_eq!(code(&o), 0, "{}", text(&o));
}

#[test]
fn a_stub_that_runs_in_an_event_subscriber_flags_the_test_that_raised_the_event() {
    let (doc, _) = json_run(&fixture("events"), &["--tdd"]);
    let r = row(&doc, "RaisesTheEvent");
    assert_eq!(r["status"], "pass", "{r:#}");
    assert_eq!(stubs(r), vec![r#"Tdd Ping Target: procedure "Pinged"()"#], "{r:#}");
    assert!(row(&doc, "DoesNotRaiseIt").get("stubbed").is_none(), "{doc:#}");
}

#[test]
fn a_stub_may_stand_for_a_procedure_of_another_app_but_the_app_under_test_keeps_its_errors() {
    let tests = fixture("deps/tests");
    let target = fixture("deps/under-test");
    for (label, args) in [
        (
            "the target app as a dependency",
            vec![tests.as_str(), "--dep", target.as_str()],
        ),
        ("both apps given", vec![target.as_str(), tests.as_str()]),
    ] {
        let mut extra = args[1..].to_vec();
        extra.push("--tdd");
        let (doc, o) = json_run(args[0], &extra);
        let r = row(&doc, "StubOnTheOtherApp");
        assert_eq!(r["status"], "pass", "{label}: {r:#}\n{}", text(&o));
        assert_eq!(
            stubs(r),
            vec![r#"Tdd App Target: procedure "Twice"(Arg1: Integer): Integer"#],
            "{label}: {r:#}"
        );
        // The call in the app under test (no test codeunit) is not a stub site.
        let r = row(&doc, "AppUnderTestStaysAnError");
        assert_eq!(r["notRun"], true, "{label}: {r:#}");
        assert!(
            r["message"].as_str().unwrap().contains("Target.NotYet()"),
            "{label}: {r:#}"
        );
        // A stub field and a stub enum value on objects of the other app (ADR-051 decision 10).
        for (method, want) in [
            (
                "StubFieldOnATableOfTheOtherApp",
                r#"Tdd App Item: field "Weight": Integer"#,
            ),
            ("StubValueOnAnEnumOfTheOtherApp", r#"Tdd App Kind: value "Medium""#),
        ] {
            let r = row(&doc, method);
            assert_eq!(r["status"], "pass", "{label} {method}: {r:#}\n{}", text(&o));
            assert_eq!(stubs(r), vec![want], "{label} {method}: {r:#}");
        }
        let all = doc["generatedStubs"].as_array().unwrap();
        assert_eq!(all.len(), 3, "{label}: {all:?}");
    }
}

#[test]
fn stubs_stand_for_procedures_of_records_rec_pages_reports_bare_calls_and_new_overloads() {
    let cases = [
        ("OnARecord", r#"Tdd Rec Item: procedure "OnRecord"(): Integer"#),
        ("OnRecInTheTable", r#"Tdd Rec Item: procedure "ViaRec"(): Integer"#),
        ("OnAPage", r#"Tdd Rec Card: procedure "OnPage"(): Boolean"#),
        ("OnAReport", r#"Tdd Rec Report: procedure "OnReport"(Arg1: Integer)"#),
        (
            "BareLocalCall",
            r#"Tdd Receiver Tests: procedure "NotWrittenYet"(Arg1: Integer): Text"#,
        ),
        (
            "NewOverload",
            r#"Tdd Rec Target: procedure "Existing"(Arg1: Integer): Integer"#,
        ),
    ];
    let (doc, o) = json_run(&fixture("receivers"), &["--tdd"]);
    for (method, stub) in cases {
        let r = row(&doc, method);
        assert_eq!(r["status"], "pass", "{method}: {r:#}\n{}", text(&o));
        assert_eq!(r["stubbed"], true, "{method}: {r:#}");
        assert_eq!(stubs(r), vec![stub], "{method}: {r:#}");
    }
    // Without --tdd the app does not compile, as before: the AL compiler's errors for the record member and the bare
    // name, and no test runs.
    let (doc, o) = json_run(&fixture("receivers"), &[]);
    assert_eq!(code(&o), 3, "{}", text(&o));
    assert_eq!(doc["tests"].as_array().map(Vec::len), Some(0), "{doc:#}");
    let errors = doc["compilationErrors"].to_string();
    for want in ["AL0132", "'OnRecord'", "'ViaRec'", "AL0118", "'NotWrittenYet'"] {
        assert!(errors.contains(want), "{want}: {errors}");
    }
    assert!(doc.get("generatedStubs").is_none(), "{doc:#}");
}

#[test]
fn stub_fields_enum_values_and_objects_are_declared_and_flag_the_tests_that_touch_them() {
    let cases: &[(&str, &[&str])] = &[
        (
            "FieldWrittenStoredAndReadBack",
            &[r#"Tdd Decl Item: field "Weight": Integer"#],
        ),
        (
            "TextFieldTypeFromTheAssignment",
            &[r#"Tdd Decl Item: field "Note": Text"#],
        ),
        ("FieldOnlyRead", &[r#"Tdd Decl Item: field "Flag": Boolean"#]),
        ("EnumValueOnAWorkspaceEnum", &[r#"Tdd Decl Color: value "Blue""#]),
        (
            "NewCodeunit",
            &[
                r#"Tdd New Calc: procedure "Compute"(): Integer"#,
                r#"codeunit "Tdd New Calc" (id 50220)"#,
            ],
        ),
        (
            "NewTable",
            &[
                r#"Tdd New Entry: field "Amount": Integer"#,
                r#"table "Tdd New Entry" (id 50221)"#,
            ],
        ),
        (
            "NewEnum",
            &[r#"Tdd New Kind: value "First""#, r#"enum "Tdd New Kind" (id 50220)"#],
        ),
    ];
    let (doc, o) = json_run(&fixture("decls"), &["--tdd"]);
    for (method, want) in cases {
        let r = row(&doc, method);
        assert_eq!(r["status"], "pass", "{method}: {r:#}\n{}", text(&o));
        assert_eq!(r["stubbed"], true, "{method}: {r:#}");
        assert_eq!(stubs(r), want.to_vec(), "{method}: {r:#}");
    }
    let r = row(&doc, "Untouched");
    assert_eq!(r["status"], "pass", "{r:#}");
    assert!(r.get("stubbed").is_none(), "{r:#}");
    // A write longer than any length a Text field takes is refused: the real length is not known yet.
    for method in ["TextFieldLongerThanTheStubLength", "ValidateLongerThanTheStubLength"] {
        let r = row(&doc, method);
        assert_ne!(r["status"], "pass", "{method}: {r:#}");
        assert_eq!(r["stubbed"], true, "{method}: {r:#}");
        assert!(
            r["message"]
                .as_str()
                .unwrap_or_default()
                .contains("--tdd: the length of the stub field Note is unknown"),
            "{method}: {r:#}"
        );
    }
    // The summary lists every stub of the program, declarations too.
    let all: Vec<&str> = doc["generatedStubs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap())
        .collect();
    for (_, want) in cases {
        for w in *want {
            assert!(all.contains(w), "{w}: {all:?}");
        }
    }
    // Without --tdd none of it compiles, and nothing is declared.
    let (doc, o) = json_run(&fixture("decls"), &[]);
    assert!(doc.get("generatedStubs").is_none(), "{doc:#}");
    let passed = doc["tests"]
        .as_array()
        .map(|t| t.iter().filter(|r| r["status"] == "pass").count())
        .unwrap_or(0);
    assert!(passed <= 1, "{}", text(&o));
}

#[test]
fn globals_of_stub_objects_bare_fields_and_dynamic_writes_flag_or_refuse() {
    let (doc, o) = json_run(&fixture("decls"), &["--tdd"]);
    // A global of a stub object flags every test that touches it; such a test is never a clean pass.
    let cases: &[(&str, &[&str])] = &[
        ("GlobalOfAStubTable", &[r#"table "Tdd New Global Entry" (id 50222)"#]),
        (
            "GlobalOfAStubCodeunit",
            &[r#"codeunit "Tdd New Global Calc" (id 50221)"#],
        ),
        // A bare field name in the table's own code (and in its extension's) is a stub site.
        (
            "BareFieldInTheTablesOwnCode",
            &[r#"Tdd Decl Item: field "Size": Integer"#],
        ),
        (
            "BareFieldInATableExtension",
            &[r#"Tdd Decl Item: field "Depth": Decimal"#],
        ),
    ];
    for (method, want) in cases {
        let r = row(&doc, method);
        assert_eq!(r["status"], "pass", "{method}: {r:#}\n{}", text(&o));
        assert_eq!(r["stubbed"], true, "{method}: {r:#}");
        assert_eq!(stubs(r), want.to_vec(), "{method}: {r:#}");
    }
    // A FieldRef or TransferFields write longer than the stub length is refused, as assignment and Validate are.
    for (method, field) in [
        ("FieldRefWriteLongerThanTheStubLength", "Note"),
        ("FieldRefValidateLongerThanTheStubLength", "Note"),
        ("TransferFieldsLongerThanTheStubLength", "Memo"),
    ] {
        let r = row(&doc, method);
        assert_ne!(r["status"], "pass", "{method}: {r:#}");
        assert_eq!(r["stubbed"], true, "{method}: {r:#}");
        let want = format!("--tdd: the length of the stub field {field} is unknown");
        assert!(
            r["message"].as_str().unwrap_or_default().contains(&want),
            "{method}: {r:#}"
        );
    }
}
