//! The run seed (ADR-061), the license file and duplicate app ids.

use super::*;

// ---------------------------------------------------------------- the run seed (ADR-061)

const SEEDED: &str = r#"
codeunit 50300 "Seeded Tests"
{
    Subtype = Test;

    [Test]
    procedure Draws()
    begin
        Error('%1 %2 %3', Random(1000000), Random(1000000), CreateGuid());
    end;

    [Test]
    procedure DrawsToo()
    begin
        Error('%1 %2', Random(1000000), CreateGuid());
    end;
}
"#;

fn seeded_app(root: &Path) -> PathBuf {
    let dir = root.join("seeded");
    write(
        &dir,
        "app.json",
        &app_json("11111111-0000-0000-0000-000000000003", "Seeded", &[]),
    );
    write(&dir, "src/Tests.al", SEEDED);
    dir
}

/// The failure messages of a run of the seeded app: the values each test drew.
fn drawn(app: &Path, extra: &[&str], env: &[(&str, &str)]) -> (Vec<String>, String) {
    let mut args = vec!["test", app.to_str().unwrap(), "--no-download"];
    args.extend_from_slice(extra);
    let o = alrunner_with(&args, env);
    let t = text(&o);
    assert_eq!(code(&o), 1, "{t}");
    let mut v: Vec<String> = t
        .lines()
        .filter(|l| l.starts_with("FAIL"))
        .map(|l| l.split_once("Seeded Tests.").map_or(l, |(_, r)| r).to_string())
        .collect();
    v.sort();
    (v, t)
}

#[test]
fn the_run_seed_is_printed_and_replays_the_values() {
    let d = tempfile::tempdir().unwrap();
    let app = seeded_app(d.path());
    let (a, t) = drawn(&app, &["--seed", "4271833"], &[]);
    assert_eq!(a.len(), 2, "{t}");
    assert_eq!(
        t.matches("Seed: 4271833").count(),
        2,
        "at the start and in the summary: {t}"
    );
    assert!(
        t.contains(&format!(
            "Replay: al-runner test {} --seed 4271833 --test Codeunit50300.Draws",
            app.display()
        )),
        "{t}"
    );
    // The same seed gives the same values, from the option or the environment; another seed other values.
    assert_eq!(drawn(&app, &["--seed", "4271833"], &[]).0, a);
    assert_eq!(drawn(&app, &[], &[("ALRUNNER_SEED", "4271833")]).0, a);
    assert_eq!(drawn(&app, &[], &[("AL_RUNNER_SEED", "4271833")]).0, a);
    let (b, _) = drawn(&app, &["--seed", "4271834"], &[]);
    for (x, y) in a.iter().zip(&b) {
        assert_ne!(x, y);
    }
    // `--test`: the test sees what it saw in the full run.
    let (alone, _) = drawn(&app, &["--seed", "4271833", "--test", "DrawsToo"], &[]);
    assert_eq!(alone, [a[1].clone()]);
    // Without a seed, a fresh one is printed, and it replays the run.
    let (fresh, t) = drawn(&app, &[], &[]);
    let seed = t
        .lines()
        .find_map(|l| l.strip_prefix("Seed: "))
        .expect("a fresh seed is printed")
        .trim()
        .to_string();
    assert_eq!(drawn(&app, &["--seed", &seed], &[]).0, fresh);
    // `--quiet` still prints the seed.
    let (_, t) = drawn(&app, &["--seed", "9", "--quiet"], &[]);
    assert!(t.contains("Seed: 9"), "{t}");
}

#[test]
fn the_run_seed_is_in_the_json_and_junit_reports() {
    let d = tempfile::tempdir().unwrap();
    let app = seeded_app(d.path());
    let junit = d.path().join("j.xml");
    let o = alrunner_with(
        &[
            "test",
            app.to_str().unwrap(),
            "--no-download",
            "--output-json",
            "--junit",
            junit.to_str().unwrap(),
        ],
        &[("ALRUNNER_SEED", "77")],
    );
    let doc: serde_json::Value = serde_json::from_slice(&o.stdout).expect("stdout is only the JSON document");
    assert_eq!(doc["seed"], 77);
    assert_eq!(doc["seedSource"], "env");
    assert!(String::from_utf8_lossy(&o.stderr).contains("Seed: 77"));
    let x = std::fs::read_to_string(&junit).unwrap();
    assert_eq!(
        x.matches("<property name=\"seed\" value=\"77\" />").count(),
        x.matches("<testsuite ").count(),
        "{x}"
    );
    let o = alrunner(&["test", app.to_str().unwrap(), "--no-download", "--output-json"]);
    let doc: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(doc["seedSource"], "fresh");
    assert!(doc["seed"].as_i64().is_some_and(|n| n >= 0));
}

#[test]
fn a_bad_run_seed_is_a_bad_invocation() {
    let d = tempfile::tempdir().unwrap();
    let app = seeded_app(d.path());
    for bad in ["x", "-1", "2147483648"] {
        let o = alrunner(&["test", app.to_str().unwrap(), "--no-download", "--seed", bad]);
        assert_eq!(code(&o), 2, "{}", text(&o));
        assert!(text(&o).contains("--seed"), "{}", text(&o));
    }
    let o = alrunner_with(
        &["test", app.to_str().unwrap(), "--no-download"],
        &[("ALRUNNER_SEED", "abc")],
    );
    assert_eq!(code(&o), 2, "{}", text(&o));
    assert!(text(&o).contains("ALRUNNER_SEED"), "{}", text(&o));
}

/// ADR-060: `--license FILE` is the license of the run. The summary, the JUnit file and the JSON report name it
/// (licensee, expiry, SHA-256, signature not checked); a file that is no license stops the run before any test.
/// The file is synthetic (made-up values, no signature): no license file is ever a fixture.
#[test]
fn a_license_file_is_used_and_named_in_every_report() {
    let d = tempfile::tempdir().unwrap();
    let app = mixed_app(d.path());
    let lic = d.path().join("Dev.bclicense");
    std::fs::write(
        &lic,
        al_host::license::synthetic::file("Synthetic Partner", "1/2/2040", &[("Codeunit", &[(1, 99, 31)])]),
    )
    .unwrap();
    let junit = d.path().join("junit.xml");
    let json = d.path().join("out.json");
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--no-download",
        "--license",
        lic.to_str().unwrap(),
        "--junit",
        junit.to_str().unwrap(),
        "--output-json",
        json.to_str().unwrap(),
    ]);
    let t = text(&o);
    assert_eq!(code(&o), 1, "{t}");
    assert!(
        t.contains("license: Synthetic Partner, expires 1/2/2040, from --license ")
            && t.contains("signature not checked"),
        "{t}"
    );
    let x = std::fs::read_to_string(&junit).unwrap();
    assert!(x.contains("<property name=\"license.source\" value=\"flag\" />"), "{x}");
    assert!(
        x.contains("<property name=\"license.licensee\" value=\"Synthetic Partner\" />"),
        "{x}"
    );
    let j: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&json).unwrap()).unwrap();
    assert_eq!(j["license"]["source"], "flag");
    assert_eq!(j["license"]["expires"], "1/2/2040");
    assert_eq!(j["license"]["signatureChecked"], false);
    assert_eq!(j["license"]["sha256"].as_str().map(str::len), Some(64));

    std::fs::write(&lic, b"not a license").unwrap();
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--no-download",
        "--license",
        lic.to_str().unwrap(),
    ]);
    let t = text(&o);
    assert_eq!(code(&o), 2, "{t}");
    assert!(t.contains("--license") && t.contains("byte order mark"), "{t}");
    assert!(!t.contains("PASS"), "no test ran: {t}");
}

/// Two app folders with the same app id (a copy of an app kept in another folder) are refused before the compile, with
/// both folders and the id named: the link step cannot tell their objects apart, and a panic is not an answer.
#[test]
fn two_app_folders_with_one_app_id_are_refused_before_the_compile() {
    let d = tempfile::tempdir().unwrap();
    let id = "11111111-0000-0000-0000-0000000000d1";
    let mut dirs = Vec::new();
    for name in ["current", "archived"] {
        let dir = d.path().join(name);
        write(&dir, "app.json", &app_json(id, "Twice", &[]));
        write(
            &dir,
            "src/Perm.al",
            "permissionset 50100 \"Twice Perm\"\n{\n    Assignable = true;\n}\n",
        );
        write(&dir, "src/Tests.al", MIXED);
        dirs.push(dir);
    }
    let o = alrunner(&[
        "test",
        dirs[0].to_str().unwrap(),
        dirs[1].to_str().unwrap(),
        "--no-download",
    ]);
    let t = text(&o);
    assert_eq!(code(&o), 2, "{t}");
    assert!(!t.contains("panicked"), "{t}");
    assert!(t.contains(id), "{t}");
    for dir in &dirs {
        assert!(t.contains(dir.to_str().unwrap()), "{}: {t}", dir.display());
    }
}
