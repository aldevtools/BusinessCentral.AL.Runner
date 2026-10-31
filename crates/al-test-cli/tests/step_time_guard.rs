//! `tools/step_time_guard.py`: the loose time guard of the corpus job. The step fails only above 1.5 times the median
//! of main's earlier runs of the step, and with fewer than 5 of them it passes with a note.

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// (exit code, output) of the guard for a step of `seconds` against `samples`.
fn guard(seconds: &str, samples: &[f64]) -> (i32, String) {
    let dir = std::env::temp_dir().join(format!(
        "step-time-guard-{}-{seconds}-{}",
        std::process::id(),
        samples.len()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("samples.txt");
    std::fs::write(&file, samples.iter().map(|s| format!("{s}\n")).collect::<String>()).unwrap();
    let o = Command::new("python3")
        .arg("-I")
        .arg(root().join("tools/step_time_guard.py"))
        .args([seconds, "--step", "Corpus", "--samples"])
        .arg(&file)
        .env_remove("GITHUB_ACTIONS")
        .output()
        .expect("python3 runs");
    let _ = std::fs::remove_dir_all(&dir);
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).into_owned(),
    )
}

#[test]
fn a_step_up_to_one_and_a_half_times_the_median_passes() {
    let main = [40.0, 45.0, 50.0, 38.0, 44.0, 47.0, 52.0, 41.0, 43.0, 46.0];
    let (code, out) = guard("66", &main);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("median 44 s"), "{out}");
}

#[test]
fn a_step_over_one_and_a_half_times_the_median_fails() {
    let main = [40.0, 45.0, 50.0, 38.0, 44.0, 47.0, 52.0, 41.0, 43.0, 46.0];
    let (code, out) = guard("90", &main);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("90 s, more than 1.5x the median 44 s"), "{out}");
}

#[test]
fn fewer_than_five_earlier_runs_pass_with_a_note() {
    let (code, out) = guard("500", &[40.0, 41.0, 42.0, 43.0]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("only 4 earlier runs"), "{out}");
}
