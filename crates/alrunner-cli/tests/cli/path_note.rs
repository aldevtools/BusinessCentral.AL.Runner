//! The note about another al-runner on PATH.

use super::*;

// The note about another `al-runner` on PATH (ADR-016, amendment of 2026-10-21). AL Runner v2's dotnet
// tool installs a command of the same name; whichever comes first on PATH runs.

/// A fake other `al-runner`: a shell script that runs `body`.
#[cfg(unix)]
fn fake_al_runner(dir: &Path, body: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(dir).unwrap();
    let p = dir.join("al-runner");
    std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    p
}

/// Runs this al-runner with a PATH, a home folder and a cache folder of the test's own, the note switched on.
#[cfg(unix)]
fn alrunner_env(path: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_al-runner"))
        .env_remove("ALRUNNER_NO_PATH_NOTE")
        .env("PATH", path)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("ALRUNNER_CACHE", home.join("cache"))
        .args(args)
        .output()
        .expect("run al-runner")
}

#[cfg(unix)]
const NOTE: &str = "note: another al-runner is on PATH at ";

#[cfg(unix)]
#[test]
fn another_al_runner_on_path_gets_one_note_on_stderr_at_most_once_a_day() {
    let d = tempfile::tempdir().unwrap();
    let bin = d.path().join("v2bin");
    let other = fake_al_runner(&bin, "echo 'AL Runner 1.0.99'");
    let home = d.path().join("home");
    let cache_dir = d.path().join("c");
    let args = ["cache", "info", "--cache-dir", cache_dir.to_str().unwrap()];
    let first = alrunner_env(&bin, &home, &args);
    assert_eq!(code(&first), 0, "{}", text(&first));
    let err = String::from_utf8_lossy(&first.stderr);
    assert!(err.contains(&format!("{NOTE}{}", other.display())), "{err}");
    assert!(
        err.contains("ALRUNNER_NO_PATH_NOTE=1"),
        "the note says how to turn it off: {err}"
    );
    assert_eq!(err.matches(NOTE).count(), 1, "one line: {err}");
    assert!(
        !String::from_utf8_lossy(&first.stdout).contains("al-runner is on PATH"),
        "never on stdout"
    );
    let second = alrunner_env(&bin, &home, &args);
    assert!(!text(&second).contains(NOTE), "once a day: {}", text(&second));
    assert_eq!(first.stdout, second.stdout, "the note does not change stdout");
}

#[cfg(unix)]
#[test]
fn the_path_note_is_off_with_alrunner_no_path_note() {
    let d = tempfile::tempdir().unwrap();
    let bin = d.path().join("v2bin");
    fake_al_runner(&bin, "echo 'AL Runner 1.0.99'");
    let home = d.path().join("home");
    let o = Command::new(env!("CARGO_BIN_EXE_al-runner"))
        .env("ALRUNNER_NO_PATH_NOTE", "1")
        .env("PATH", &bin)
        .env("HOME", &home)
        .env("ALRUNNER_CACHE", home.join("cache"))
        .args(["cache", "info", "--cache-dir", d.path().join("c").to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(code(&o), 0, "{}", text(&o));
    assert!(!text(&o).contains(NOTE), "{}", text(&o));
}

#[cfg(unix)]
#[test]
fn the_dotnet_tools_folder_is_checked_even_when_it_is_not_on_path() {
    let d = tempfile::tempdir().unwrap();
    let home = d.path().join("home");
    let other = fake_al_runner(&home.join(".dotnet").join("tools"), "echo 'AL Runner 1.0.99'");
    let empty = d.path().join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    let o = alrunner_env(
        &empty,
        &home,
        &["cache", "info", "--cache-dir", d.path().join("c").to_str().unwrap()],
    );
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(err.contains(&format!("{NOTE}{}", other.display())), "{err}");
    assert!(err.contains("AL Runner v2, a dotnet tool"), "{err}");
    assert!(err.contains("dotnet tool uninstall -g MSDyn365BC.AL.Runner"), "{err}");
}

#[cfg(unix)]
#[test]
fn this_binary_itself_and_a_shim_of_another_tool_are_not_another_al_runner() {
    let d = tempfile::tempdir().unwrap();
    let home = d.path().join("home");
    // A link to this very binary.
    let me = d.path().join("me");
    std::fs::create_dir_all(&me).unwrap();
    std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_al-runner"), me.join("al-runner")).unwrap();
    // A version manager's shim: `al-runner` is a link to a program of another name.
    let shims = d.path().join("shims");
    std::fs::create_dir_all(&shims).unwrap();
    let mgr = fake_al_runner(&d.path().join("mgr"), "echo manager");
    let mise = mgr.with_file_name("mise");
    std::fs::rename(&mgr, &mise).unwrap();
    std::os::unix::fs::symlink(&mise, shims.join("al-runner")).unwrap();
    let path = std::env::join_paths([&me, &shims]).unwrap();
    let o = alrunner_env(
        Path::new(&path),
        &home,
        &["cache", "info", "--cache-dir", d.path().join("c").to_str().unwrap()],
    );
    assert_eq!(code(&o), 0, "{}", text(&o));
    assert!(!text(&o).contains(NOTE), "{}", text(&o));
}

#[cfg(unix)]
#[test]
fn version_lists_each_other_al_runner_with_the_first_line_of_its_version() {
    let d = tempfile::tempdir().unwrap();
    let bin = d.path().join("v2bin");
    let other = fake_al_runner(&bin, "echo 'AL Runner 1.0.99'; echo 'second line'");
    let home = d.path().join("home");
    let o = alrunner_env(&bin, &home, &["--version"]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    let out = String::from_utf8_lossy(&o.stdout);
    let lines: Vec<&str> = out.lines().collect();
    assert!(
        lines[0].starts_with(&format!("al-runner {} (", env!("CARGO_PKG_VERSION"))),
        "{out}"
    );
    assert_eq!(lines.len(), 2, "{out}");
    assert!(
        lines[1].contains(&other.display().to_string()) && lines[1].contains("AL Runner 1.0.99"),
        "{out}"
    );
    assert!(!lines[1].contains("second line"), "{out}");
    assert!(
        !String::from_utf8_lossy(&o.stderr).contains(NOTE),
        "--version lists instead of the note"
    );
}

#[cfg(unix)]
#[test]
fn version_waits_at_most_2_seconds_for_another_al_runner() {
    let d = tempfile::tempdir().unwrap();
    let bin = d.path().join("v2bin");
    fake_al_runner(&bin, "exec /bin/sleep 20");
    let home = d.path().join("home");
    let start = std::time::Instant::now();
    let o = alrunner_env(&bin, &home, &["--version"]);
    assert!(
        start.elapsed() < std::time::Duration::from_secs(10),
        "{:?}",
        start.elapsed()
    );
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(out.contains("no answer within 2 s"), "{out}");
}

#[cfg(unix)]
#[test]
fn version_read_by_a_tool_that_takes_only_the_first_line_still_exits_0() {
    use std::io::BufRead;
    let d = tempfile::tempdir().unwrap();
    let bin = d.path().join("v2bin");
    // The other copy answers after the reader has gone (`al-runner --version | head -1`).
    fake_al_runner(&bin, "/bin/sleep 1; echo 'AL Runner 1.0.99'");
    let home = d.path().join("home");
    let mut child = Command::new(env!("CARGO_BIN_EXE_al-runner"))
        .env_remove("ALRUNNER_NO_PATH_NOTE")
        .env("PATH", &bin)
        .env("HOME", &home)
        .env("ALRUNNER_CACHE", home.join("cache"))
        .arg("--version")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut first = String::new();
    std::io::BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut first)
        .unwrap();
    assert!(first.starts_with("al-runner "), "{first}");
    let o = child.wait_with_output().unwrap();
    assert_eq!(code(&o), 0, "{}", String::from_utf8_lossy(&o.stderr));
}
