//! `al-runner test-microsoft` (ADR-052 section 2) on a fake extracted artifact: small fixture test apps under the
//! paths the real artifact uses, a fake BCApps recipe, no network. Nothing here is Microsoft's code.

use std::path::Path;
use std::process::{Command, Output};

const BUILD: &str = "29.0.1.2";

fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// A ZIP with stored entries.
fn zip(entries: &[(&str, String)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, text) in entries {
        let data = text.as_bytes();
        let crc = crc32(data);
        let offset = out.len() as u32;
        out.extend(0x0403_4b50u32.to_le_bytes());
        out.extend([20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        out.extend(crc.to_le_bytes());
        out.extend((data.len() as u32).to_le_bytes());
        out.extend((data.len() as u32).to_le_bytes());
        out.extend((name.len() as u16).to_le_bytes());
        out.extend([0, 0]);
        out.extend(name.as_bytes());
        out.extend(data);
        central.extend(0x0201_4b50u32.to_le_bytes());
        central.extend([20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        central.extend(crc.to_le_bytes());
        central.extend((data.len() as u32).to_le_bytes());
        central.extend((data.len() as u32).to_le_bytes());
        central.extend((name.len() as u16).to_le_bytes());
        central.extend([0; 12]);
        central.extend(offset.to_le_bytes());
        central.extend(name.as_bytes());
    }
    let cd_offset = out.len() as u32;
    let n = entries.len() as u16;
    let cd_len = central.len() as u32;
    out.extend(central);
    out.extend(0x0605_4b50u32.to_le_bytes());
    out.extend([0, 0, 0, 0]);
    out.extend(n.to_le_bytes());
    out.extend(n.to_le_bytes());
    out.extend(cd_len.to_le_bytes());
    out.extend(cd_offset.to_le_bytes());
    out.extend([0, 0]);
    out
}

fn app(id: &str, name: &str, al: &str) -> Vec<u8> {
    let json = format!(
        r#"{{"id":"{id}","name":"{name}","publisher":"Fixture","version":"1.0.0.0","idRanges":[{{"from":50000,"to":59999}}]}}"#
    );
    zip(&[("app.json", json), ("src/Code.al", al.to_string())])
}

fn test_codeunit(id: u32, name: &str, tests: &[(&str, bool)]) -> String {
    let mut body = String::new();
    for (t, pass) in tests {
        let stmt = if *pass { "" } else { "Error('boom');" };
        body.push_str(&format!(
            "    [Test]\n    procedure {t}()\n    begin\n        {stmt}\n    end;\n\n"
        ));
    }
    format!("codeunit {id} \"{name}\"\n{{\n    Subtype = Test;\n    TestType = UnitTest;\n\n{body}}}\n")
}

/// The fake artifact under `home/.bcartifacts.cache/sandbox/<BUILD>/platform` and a fake BCApps checkout.
fn fixture(root: &Path, fail_b: bool) {
    let test = root.join(format!(
        "home/.bcartifacts.cache/sandbox/{BUILD}/platform/Applications/Fake/Test"
    ));
    std::fs::create_dir_all(&test).unwrap();
    let write = |file: &str, bytes: Vec<u8>| std::fs::write(test.join(file), bytes).unwrap();
    write(
        "Tests-Fake-A.Source.zip",
        app(
            "a0000000-0000-0000-0000-000000000001",
            "Tests-Fake-A",
            &test_codeunit(50100, "Fake A Tests", &[("APasses", true), ("ADisabled", false)]),
        ),
    );
    write(
        "Tests-Fake-B.Source.zip",
        app(
            "a0000000-0000-0000-0000-000000000002",
            "Tests-Fake-B",
            &test_codeunit(50200, "Fake B Tests", &[("BRuns", !fail_b)]),
        ),
    );
    write(
        "Fake Test Library.Source.zip",
        app(
            "a0000000-0000-0000-0000-000000000003",
            "Fake Test Library",
            "codeunit 50300 \"Fake Library\"\n{\n    procedure Help()\n    begin\n    end;\n}\n",
        ),
    );
    write(
        "_Exclude_Fake Tests.Source.zip",
        app(
            "a0000000-0000-0000-0000-000000000004",
            "_Exclude_Fake Tests",
            &test_codeunit(50400, "Fake Excluded Tests", &[("XFails", false)]),
        ),
    );
    // Not a test app: no test in its path.
    let source = test.parent().unwrap().join("Source");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(source.join("Fake App.Source.zip"), b"never read").unwrap();
    // BCApps: the W1 test project (default lane UnitTest, an empty company), no Legacy bucket for the fakes, and one
    // disabled test.
    let bcapps = root.join("bcapps");
    let settings = bcapps.join("build/projects/Test Apps W1/.AL-Go");
    std::fs::create_dir_all(&settings).unwrap();
    std::fs::write(
        settings.join("settings.json"),
        r#"{"testType":"UnitTest","companyName":"Empty Company","buildModes":["L"],
            "ConditionalSettings":[{"buildModes":["L"],"settings":{"testType":"Legacy","bucketNumber":1,"companyName":"CRONUS International Ltd."}}]}"#,
    )
    .unwrap();
    let scripts = bcapps.join("build/scripts");
    std::fs::create_dir_all(&scripts).unwrap();
    std::fs::write(
        scripts.join("TestConfiguration.json"),
        r#"{"LegacyTests-Bucket1":["Tests-Other"]}"#,
    )
    .unwrap();
    let disabled = bcapps.join("src/DisabledTests");
    std::fs::create_dir_all(&disabled).unwrap();
    std::fs::write(
        disabled.join("Fake.json"),
        r#"[{"codeunitId":50100,"codeunitName":"Fake A Tests","method":"ADisabled"}]"#,
    )
    .unwrap();
}

fn run(root: &Path, extra: &[&str]) -> Output {
    let cache = root.join("cache");
    let bcapps = root.join("bcapps");
    let mut args: Vec<&str> = vec![
        "test-microsoft",
        "--bc-version",
        BUILD,
        "--no-download",
        "--cache-dir",
        cache.to_str().unwrap(),
        "--bcapps",
        bcapps.to_str().unwrap(),
    ];
    args.extend_from_slice(extra);
    Command::new(env!("CARGO_BIN_EXE_al-runner"))
        .env("HOME", root.join("home"))
        .env("AL_CHECK_THREADS", "2")
        .env("AL_LOWER_THREADS", "2")
        .env("AL_TEST_THREADS", "2")
        .env("ALRUNNER_NO_PATH_NOTE", "1")
        .args(&args)
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

/// The numbers of a row of the summary table.
fn row(out: &str, name: &str) -> String {
    let line = out
        .lines()
        .find(|l| l.trim_start().starts_with(&format!("{name} ")))
        .unwrap_or_else(|| panic!("no row {name} in:\n{out}"));
    line.trim_start()[name.len()..]
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn every_test_app_of_the_build_runs_by_the_recipe_and_libraries_are_left_out() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path(), false);
    let o = run(dir.path(), &[]);
    let out = text(&o);
    assert!(
        out.contains(&format!("Microsoft tests: BC {BUILD} w1, 2 test app(s)")),
        "{out}"
    );
    assert!(out.contains("left out: Fake Test Library: no test codeunit"), "{out}");
    assert!(out.contains("left out: _Exclude_Fake Tests"), "{out}");
    assert!(!out.contains("== test app Fake Test Library"), "{out}");
    // passed failed not-run total codeunits-left exit. The disabled test is skipped by the recipe, not run.
    assert_eq!(row(&out, "Tests-Fake-A"), "1 0 0 1 0 0", "{out}");
    assert_eq!(row(&out, "Tests-Fake-B"), "1 0 0 1 0 0", "{out}");
    assert_eq!(row(&out, "Total"), "2 0 0 2 0 0", "{out}");
    assert_eq!(o.status.code(), Some(0), "{out}");
}

#[test]
fn app_narrows_to_the_named_test_apps_and_the_worst_app_sets_the_exit_code() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path(), true);
    let junit = dir.path().join("out/r.xml");
    let o = run(
        dir.path(),
        &["--app", "tests-fake-b", "--junit", junit.to_str().unwrap()],
    );
    let out = text(&o);
    assert!(out.contains("1 test app(s)"), "{out}");
    assert!(!out.contains("Tests-Fake-A"), "{out}");
    assert_eq!(row(&out, "Tests-Fake-B"), "0 1 0 1 0 1", "{out}");
    assert_eq!(o.status.code(), Some(1), "{out}");
    assert!(dir.path().join("out/r.Tests-Fake-B.unittest.xml").is_file(), "{out}");
}

#[test]
fn an_unknown_app_name_or_a_library_exits_2_before_any_test_runs() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path(), false);
    let o = run(dir.path(), &["--app", "Tests-Nope"]);
    let out = text(&o);
    assert_eq!(o.status.code(), Some(2), "{out}");
    assert!(
        out.contains("Tests-Nope") && out.contains("Tests-Fake-A, Tests-Fake-B"),
        "{out}"
    );
    let o = run(dir.path(), &["--app", "Fake Test Library"]);
    let out = text(&o);
    assert_eq!(o.status.code(), Some(2), "{out}");
    assert!(out.contains("holds no test codeunit"), "{out}");
    // An excluded app runs when named.
    let o = run(dir.path(), &["--app", "_exclude_fake tests"]);
    let out = text(&o);
    assert_eq!(row(&out, "_Exclude_Fake"), "Tests 0 1 0 1 0 1", "{out}");
}
