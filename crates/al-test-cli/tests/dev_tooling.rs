//! The developer tooling of `al-test-cli`: `--help`, test selection, `--explain`,
//! `--top-refusals` and `--baseline-diff`, against the real pinned corpus and oracle (BC 29),
//! without the Microsoft apps so that each run stays short in the unoptimised test build.

use al_test_cli::oracle::{self, CaseId};
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

struct Out {
    code: i32,
    out: String,
    err: String,
}

fn cli(args: &[&str]) -> Out {
    assert!(
        root().join("corpus/tests/al-language/app.json").exists(),
        "corpus submodule not checked out: run `git submodule update --init`"
    );
    let o = Command::new(env!("CARGO_BIN_EXE_al-test-cli"))
        .arg("--root")
        .arg(root())
        .args(["--no-deps", "--bc-version", "29"])
        .args(args)
        .output()
        .unwrap();
    Out {
        code: o.status.code().unwrap_or(-1),
        out: String::from_utf8_lossy(&o.stdout).into(),
        err: String::from_utf8_lossy(&o.stderr).into(),
    }
}

/// The oracle's cases of BC 29 that real BC passes: what the pass rate counts.
fn oracle_cases() -> Vec<CaseId> {
    let xml = std::fs::read_to_string(root().join("oracle/29/junit.xml")).unwrap();
    oracle::parse_junit(&xml)
        .unwrap()
        .into_iter()
        .filter(|c| !c.bc_failed)
        .map(|c| c.id)
        .collect()
}

fn total_of(args: &[&str]) -> u64 {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("r.json");
    let mut a = vec!["--no-ratchet", "--json", f.to_str().unwrap()];
    a.extend_from_slice(args);
    let r = cli(&a);
    assert_eq!(r.code, 0, "{}", r.err);
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap();
    v["overall"]["total"].as_u64().unwrap()
}

/// A test codeunit with a handful of tests that does not need the Microsoft apps.
const SMALL_CU: u32 = 71035;

#[test]
fn help_lists_every_flag_in_groups() {
    let r = Command::new(env!("CARGO_BIN_EXE_al-test-cli"))
        .arg("--help")
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&r.stdout);
    assert!(r.status.success());
    for group in [
        "INPUT",
        "SELECTION",
        "DIAGNOSE",
        "BASELINE AND REPORTS",
        "DOTNET",
        "EXIT CODES",
    ] {
        assert!(text.contains(group), "no group {group}:\n{text}");
    }
    for flag in [
        "--root",
        "--oracle",
        "--baseline",
        "--suite-baseline",
        "--json",
        "--markdown",
        "--junit",
        "--update-baseline",
        "--no-ratchet",
        "--deps",
        "--no-deps",
        "--bc-version",
        "--all-versions",
        "--dotnet-sidecar",
        "--dotnet-assembly-dir",
        "--dotnet-allow-roll-forward",
        "--dotnet-timeout-secs",
        "--require-dotnet",
        "--files-dir",
        "--no-download",
        "--cache-dir",
        "--fetch-deps",
        "--print-builds",
        "--project",
        "--codeunit",
        "--test",
        "--area",
        "--file",
        "--explain",
        "--top-refusals",
        "--top-refusals-area",
        "--baseline-diff",
        "--timings",
        "--timings-table",
        "--help",
    ] {
        assert!(text.contains(flag), "{flag} is not in --help");
    }
    // One line per flag, no wrapping of the flag column: every flag line starts with the flag.
    assert!(text.lines().any(|l| l.starts_with("  --codeunit ID ")), "{text}");
}

#[test]
fn an_unknown_flag_is_a_setup_error_that_points_at_help() {
    let r = Command::new(env!("CARGO_BIN_EXE_al-test-cli"))
        .arg("--frobnicate")
        .output()
        .unwrap();
    assert_eq!(r.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&r.stderr).contains("--help"));
}

/// The JSON and Markdown reports always say how long the run took, by phase, with no flag.
/// The numbers vary, so this checks the structure only.
#[test]
fn reports_carry_the_phase_timings_without_a_flag() {
    let dir = tempfile::tempdir().unwrap();
    let json = dir.path().join("r.json");
    let md = dir.path().join("r.md");
    let r = cli(&[
        "--no-ratchet",
        "--codeunit",
        &SMALL_CU.to_string(),
        "--json",
        json.to_str().unwrap(),
        "--markdown",
        md.to_str().unwrap(),
    ]);
    assert_eq!(r.code, 0, "{}", r.err);
    assert!(
        !r.err.contains("timings:"),
        "--timings stays opt-in on the console: {}",
        r.err
    );
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&json).unwrap()).unwrap();
    let t = &v["timings"];
    assert!(t["bc_version"].as_str().is_some_and(|v| v.starts_with("29")), "{t}");
    for key in ["load_s", "compile_s", "baseline_s", "tests_s", "total_s"] {
        assert!(t[key].as_f64().is_some_and(|x| x >= 0.0), "{key} in {t}");
    }
    assert!(t["suites"].as_array().is_some_and(|a| !a.is_empty()), "{t}");
    let text = std::fs::read_to_string(&md).unwrap();
    // The markdown is one line: counts, rate and the total time.
    assert_eq!(text.lines().count(), 1, "{text}");
    assert!(
        text.starts_with("- BC 29") && text.contains(" passed, ") && text.trim_end().ends_with(" s"),
        "{text}"
    );
    // The same table comes back from the JSON file.
    let table = cli(&["--timings-table", json.to_str().unwrap()]);
    assert_eq!(table.code, 0, "{}", table.err);
    assert!(table.out.contains("| Version | Load |"), "{}", table.out);
    assert_eq!(table.out.lines().count(), 3, "{}", table.out);
}

#[test]
fn a_codeunit_selection_reports_only_that_codeunit() {
    let expected = oracle_cases().iter().filter(|c| c.codeunit == SMALL_CU).count() as u64;
    assert!(expected > 1);
    assert_eq!(total_of(&["--codeunit", &SMALL_CU.to_string()]), expected);
}

#[test]
fn selection_flags_repeat_and_make_a_union() {
    let cases = oracle_cases();
    let in_cu = |cu: u32| cases.iter().filter(|c| c.codeunit == cu).count() as u64;
    let other = cases
        .iter()
        .map(|c| c.codeunit)
        .find(|&c| c != SMALL_CU && in_cu(c) > 0)
        .unwrap();
    assert_eq!(
        total_of(&["--codeunit", &SMALL_CU.to_string(), "--codeunit", &other.to_string()]),
        in_cu(SMALL_CU) + in_cu(other)
    );
    // --test: a substring of the name, any case. One name picks every codeunit that has it.
    let name = &cases.iter().find(|c| c.codeunit == SMALL_CU).unwrap().name;
    let by_name = cases
        .iter()
        .filter(|c| c.name.to_lowercase().contains(&name.to_lowercase()))
        .count() as u64;
    assert_eq!(total_of(&["--test", &name.to_uppercase()]), by_name);
    // --area and --codeunit together: the area, plus the codeunit when it is outside it.
    assert_eq!(total_of(&["--area", "json"]), 465); // corpus bump 29: 22 more json cases on BC 29; bump 52: one more
    assert_eq!(
        total_of(&["--area", "json", "--codeunit", &SMALL_CU.to_string()]),
        465 + in_cu(SMALL_CU)
    );
}

#[test]
fn a_file_selection_matches_the_end_of_the_path() {
    let n = total_of(&["--file", "types/TestDurationContracts.al"]);
    assert!(n > 0);
    assert_eq!(total_of(&["--file", "TestDurationContracts.al"]), n);
    assert_eq!(
        total_of(&[
            "--file",
            "types/TestDurationContracts.al",
            "--file",
            "TestDurationContracts.al"
        ]),
        n
    );
}

#[test]
fn a_selection_that_matches_nothing_is_a_setup_error() {
    let r = cli(&["--no-ratchet", "--test", "NoSuchTestAnywhere_zzz"]);
    assert_eq!(r.code, 2, "{}", r.err);
    assert!(r.err.contains("matches no test"), "{}", r.err);
}

#[test]
fn update_baseline_refuses_a_selection_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("passing.txt");
    std::fs::write(&base, "1 Keep\n").unwrap();
    let r = cli(&[
        "--update-baseline",
        "--baseline",
        base.to_str().unwrap(),
        "--codeunit",
        "60184",
    ]);
    assert_eq!(r.code, 2);
    assert!(
        r.err.contains("--update-baseline") && r.err.contains("selection"),
        "{}",
        r.err
    );
    assert_eq!(std::fs::read_to_string(&base).unwrap(), "1 Keep\n");
}

#[test]
fn the_ratchet_checks_only_the_selected_baseline_cases() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("passing.txt");
    // A case of a codeunit the selection leaves out: it did not run, so it cannot regress.
    std::fs::write(&base, "60002 NoSuchProcedure_OutsideTheSelection\n").unwrap();
    let b = base.to_str().unwrap();
    let ok = cli(&["--baseline", b, "--codeunit", &SMALL_CU.to_string()]);
    assert_eq!(ok.code, 0, "{}", ok.err);
    // The same case inside the selection is checked and fails the run.
    let bad = cli(&["--baseline", b, "--codeunit", "60002"]);
    assert_eq!(bad.code, 1, "{}", bad.err);
    assert!(bad.err.contains("NoSuchProcedure_OutsideTheSelection"), "{}", bad.err);
}

/// (passing names, refused names) of one codeunit, from the JUnit the runtime writes. Two tests
/// ask for `SMALL_CU`; the run is made once.
fn outcomes_of_codeunit(cu: u32) -> (Vec<String>, Vec<String>) {
    static SMALL: std::sync::OnceLock<(Vec<String>, Vec<String>)> = std::sync::OnceLock::new();
    if cu == SMALL_CU {
        return SMALL.get_or_init(|| run_outcomes_of_codeunit(cu)).clone();
    }
    run_outcomes_of_codeunit(cu)
}

fn run_outcomes_of_codeunit(cu: u32) -> (Vec<String>, Vec<String>) {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("j.xml");
    let r = cli(&[
        "--no-ratchet",
        "--junit",
        f.to_str().unwrap(),
        "--codeunit",
        &cu.to_string(),
    ]);
    assert_eq!(r.code, 0, "{}", r.err);
    let xml = std::fs::read_to_string(&f).unwrap();
    let (mut pass, mut refused) = (Vec::new(), Vec::new());
    for case in xml.split("<testcase ").skip(1) {
        let case = case.split("</testcase>").next().unwrap();
        let name = case
            .split(" name=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .to_string();
        if case.contains("<skipped") {
            refused.push(name);
        } else if !case.contains("<failure") {
            pass.push(name);
        }
    }
    (pass, refused)
}

#[test]
fn baseline_diff_lists_new_passes_and_lost_baseline_cases_by_area() {
    let (pass, refused) = outcomes_of_codeunit(SMALL_CU);
    assert!(pass.len() >= 2 && !refused.is_empty(), "{pass:?} {refused:?}");
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("passing.txt");
    // Baseline: every pass but one, and one case that is refused today.
    let mut lines: Vec<String> = pass[1..].iter().map(|n| format!("{SMALL_CU} {n}")).collect();
    lines.push(format!("{SMALL_CU} {}", refused[0]));
    std::fs::write(&base, lines.join("\n") + "\n").unwrap();
    let r = cli(&[
        "--baseline",
        base.to_str().unwrap(),
        "--baseline-diff",
        "--codeunit",
        &SMALL_CU.to_string(),
    ]);
    assert_eq!(r.code, 1, "a lost baseline case is still a regression: {}", r.err);
    let diff = &r.out[r.out.find("Baseline diff").expect(&r.out)..];
    assert!(diff.contains("1 newly pass"), "{diff}");
    assert!(diff.contains("1 baseline cases no longer pass"), "{diff}");
    let section = |title: &str| {
        let at = diff
            .find(title)
            .unwrap_or_else(|| panic!("no section {title}:\n{diff}"));
        diff[at..].split("\n\n").next().unwrap().to_string()
    };
    let new = section("Newly passing");
    assert!(
        new.contains("  types (1):") && new.contains(&format!("{SMALL_CU} {}", pass[0])),
        "{new}"
    );
    let lost = section("In the baseline but no longer passing");
    assert!(
        lost.contains(&format!("{SMALL_CU} {}", refused[0])) && lost.contains("not run:"),
        "{lost}"
    );
}

#[test]
fn top_refusals_ranks_folded_reasons_with_an_example_each() {
    let r = cli(&["--no-ratchet", "--top-refusals", "3", "--area", "types"]);
    assert_eq!(r.code, 0, "{}", r.err);
    let at = r.out.find("Top refusals").expect(&r.out);
    let text = &r.out[at..];
    let counts: Vec<u64> = text
        .lines()
        .skip(1)
        .filter(|l| !l.trim_start().starts_with("e.g.") && !l.trim_start().starts_with("..."))
        .map_while(|l| l.split_whitespace().next()?.parse().ok())
        .collect();
    assert_eq!(counts.len(), 3, "{text}");
    assert!(counts.windows(2).all(|w| w[0] >= w[1]), "{counts:?}");
    assert_eq!(text.matches("e.g. ").count(), 3, "an example per reason:\n{text}");
    assert!(text.contains("more reasons"), "{text}");
    // The header's count is the run's own count of cases not run.
    let not_run: u64 = r
        .out
        .lines()
        .find(|l| l.starts_with("BC 29.0:"))
        .unwrap()
        .split(", ")
        .nth(2)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert!(
        text.lines().next().unwrap().contains(&format!("{not_run} not run")),
        "{text}"
    );
}

#[test]
fn top_refusals_area_ranks_one_area_and_selects_it() {
    let a = cli(&["--no-ratchet", "--top-refusals-area", "types"]);
    let b = cli(&["--no-ratchet", "--area", "types", "--top-refusals"]);
    assert_eq!(a.code, 0, "{}", a.err);
    // The ranking of the main app: the first "Top refusals" section whose header does not name an extra suite
    // (the OnPrem suite has cases in the types folder since corpus bump 22 and prints its own section).
    let ranking = |o: &Out| {
        let start = o
            .out
            .match_indices("Top refusals")
            .map(|(i, _)| i)
            .find(|&i| !o.out[i..].lines().next().unwrap().contains("onprem"))
            .unwrap();
        // The section ends at the first blank line.
        o.out[start..].split("\n\n").next().unwrap().to_string()
    };
    assert!(
        ranking(&a).lines().next().unwrap().contains("area types"),
        "{}",
        ranking(&a)
    );
    assert_eq!(
        ranking(&a).lines().skip(1).collect::<Vec<_>>(),
        ranking(&b).lines().skip(1).collect::<Vec<_>>()
    );
    assert!(a.out.contains("selection:"), "{}", a.out);
}

#[test]
fn explain_a_passing_test_and_a_refused_test() {
    let (pass, refused) = outcomes_of_codeunit(SMALL_CU);
    let p = cli(&[
        "--no-ratchet",
        "--explain",
        &pass[0],
        "--codeunit",
        &SMALL_CU.to_string(),
    ]);
    assert_eq!(p.code, 0, "{}", p.err);
    assert!(
        p.out.contains("runtime: PASS") && p.out.contains("real BC (oracle): passes"),
        "{}",
        p.out
    );
    let r = cli(&["--no-ratchet", "--explain", &refused[0]]);
    assert_eq!(r.code, 0, "{}", r.err);
    assert!(
        r.out.contains("runtime: NOT RUN (refused)") && r.out.contains("reason: "),
        "{}",
        r.out
    );
    // Where it was refused: a routine and a source position.
    assert!(r.out.contains(".al:") && r.out.contains("codeunit "), "{}", r.out);
}

#[test]
fn explain_a_failure_prints_error_code_and_stack() {
    // Any test of the recordref area that fails today; the unit tests in explain_site.rs pin the
    // content, this one the wiring.
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("j.xml");
    let r = cli(&["--no-ratchet", "--junit", f.to_str().unwrap(), "--area", "recordref"]);
    assert_eq!(r.code, 0, "{}", r.err);
    let xml = std::fs::read_to_string(&f).unwrap();
    let Some(failing) = xml.split("<testcase ").skip(1).find(|c| c.contains("<failure")) else {
        return; // nothing fails in this area any more
    };
    let name = failing.split(" name=\"").nth(1).unwrap().split('"').next().unwrap();
    let r = cli(&["--no-ratchet", "--explain", name]);
    assert_eq!(r.code, 0, "{}", r.err);
    assert!(
        r.out.contains("runtime: FAIL") && r.out.contains("error: ") && r.out.contains("code: "),
        "{}",
        r.out
    );
    assert!(
        r.out.contains("AL call stack") && r.out.contains("raised in (innermost first):"),
        "{}",
        r.out
    );
}

#[test]
fn explain_needs_one_test() {
    let many = cli(&["--no-ratchet", "--explain", "Throws"]);
    assert_eq!(many.code, 2);
    assert!(
        many.err.contains("matches") && many.err.contains("--codeunit"),
        "{}",
        many.err
    );
    let none = cli(&["--no-ratchet", "--explain", "NoSuchTestAnywhere_zzz"]);
    assert_eq!(none.code, 2);
    assert!(none.err.contains("no test matches"), "{}", none.err);
}

#[test]
fn timings_names_the_slowest_test_codeunits() {
    let r = cli(&["--no-ratchet", "--timings", "--codeunit", &SMALL_CU.to_string()]);
    assert_eq!(r.code, 0, "{}", r.err);
    assert!(r.err.contains("timings: BC 29"), "{}", r.err);
    let line = r
        .err
        .lines()
        .find(|l| l.starts_with("slowest test codeunits:"))
        .unwrap_or_else(|| panic!("no slowest line:\n{}", r.err));
    assert!(line.contains(&format!("{SMALL_CU} ")), "{line}");
    // The compile is split into its parts.
    let detail = r
        .err
        .lines()
        .find(|l| l.starts_with("compile:"))
        .unwrap_or_else(|| panic!("no compile line:\n{}", r.err));
    for part in [
        "load",
        "members",
        "check workspace",
        "baseline roots",
        "demand",
        "lower",
        "engine",
    ] {
        assert!(detail.contains(part), "{part} missing in {detail}");
    }
}
