//! The note about another `al-runner` on PATH (ADR-016, amendment of 2026-10-21).
//!
//! AL Runner v2's dotnet tool installs a command of the same name into `~/.dotnet/tools`. Whichever comes
//! first on PATH runs. This binary can only notice the clash when it runs itself, so:
//! - each command except `serve`, `--help` and `--version` prints one line to stderr when it finds another
//!   copy, at most once a day (a stamp file in the cache folder);
//! - `--version` lists every other copy with the first line of its own `--version` output.
//!
//! `ALRUNNER_NO_PATH_NOTE=1` turns both off. Nothing here writes to stdout except the `--version` list, so
//! JUnit, JSON and `serve` output do not change. Tooling only: nothing AL code can see (ADR-009).

use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime};

/// Turns the note and the `--version` list off.
pub const OFF_VAR: &str = "ALRUNNER_NO_PATH_NOTE";
const NAMES: [&str; 2] = ["al-runner", "al-runner.exe"];
const ONCE_PER: Duration = Duration::from_secs(24 * 60 * 60);
const VERSION_WAIT: Duration = Duration::from_secs(2);

fn off() -> bool {
    std::env::var(OFF_VAR).is_ok_and(|v| !v.trim().is_empty() && v.trim() != "0")
}

fn home() -> Option<PathBuf> {
    ["HOME", "USERPROFILE"]
        .iter()
        .filter_map(std::env::var_os)
        .find(|v| !v.is_empty())
        .map(PathBuf::from)
}

/// The other `al-runner` executables: the folders of `path` in order, then `<home>/.dotnet/tools`. Leaves out
/// `me` (compared by canonical path) and links to a program of another name (a version manager's shim), and
/// names each file once.
pub fn others(path: Option<OsString>, home: Option<PathBuf>, me: Option<&Path>) -> Vec<PathBuf> {
    let me = me.and_then(|m| m.canonicalize().ok());
    let mut dirs: Vec<PathBuf> = path.map(|p| std::env::split_paths(&p).collect()).unwrap_or_default();
    dirs.extend(home.map(|h| h.join(".dotnet").join("tools")));
    let mut seen: Vec<PathBuf> = Vec::new();
    let mut out = Vec::new();
    for dir in dirs.iter().filter(|d| !d.as_os_str().is_empty()) {
        for name in NAMES {
            let p = dir.join(name);
            if !p.is_file() {
                continue;
            }
            let Ok(real) = p.canonicalize() else { continue };
            let real_name = real.file_name().and_then(|f| f.to_str()).map(str::to_ascii_lowercase);
            if me.as_ref() == Some(&real)
                || seen.contains(&real)
                || !NAMES.contains(&real_name.as_deref().unwrap_or(""))
            {
                continue;
            }
            seen.push(real);
            out.push(p);
        }
    }
    out
}

fn others_from_env() -> Vec<PathBuf> {
    let me = std::env::current_exe().ok();
    others(std::env::var_os("PATH"), home(), me.as_deref())
}

/// True when the stamp is missing, unreadable, or older than a day (or dated in the future).
pub fn due(stamp: &Path, now: SystemTime) -> bool {
    match std::fs::metadata(stamp).and_then(|m| m.modified()) {
        // A stamp newer than `now` (written after `now` was read, or a clock that went back) is not old.
        Ok(t) => now.duration_since(t).is_ok_and(|age| age >= ONCE_PER),
        Err(_) => true,
    }
}

fn in_dotnet_tools(p: &Path) -> bool {
    let mut parts = p.parent().into_iter().flat_map(|d| d.iter().rev().take(2));
    parts.next().is_some_and(|t| t == "tools") && parts.next().is_some_and(|d| d == ".dotnet")
}

/// The note's one line.
pub fn note(other: &Path, more: usize, version: &str, me: Option<&Path>) -> String {
    let (what, remove) = if in_dotnet_tools(other) {
        (
            " (AL Runner v2, a dotnet tool)",
            "Remove v2 with \"dotnet tool uninstall -g MSDyn365BC.AL.Runner\"",
        )
    } else {
        ("", "If it is AL Runner v2, remove it")
    };
    let first = match me.and_then(Path::parent) {
        Some(d) => format!("put {} first on PATH", d.display()),
        None => "put this al-runner first on PATH".into(),
    };
    let more = match more {
        0 => String::new(),
        n => format!(" ({n} more: see al-runner --version)"),
    };
    format!(
        "note: another al-runner is on PATH at {}{what}{more}. This is AL Runner v3 {version}. {remove}, or \
         {first}. ALRUNNER_NO_PATH_NOTE=1 turns this note off.",
        other.display()
    )
}

/// Prints the note to stderr when another copy is on PATH and the last note is a day old.
pub fn maybe_note() {
    if off() {
        return;
    }
    let others = others_from_env();
    let Some(first) = others.first() else { return };
    let stamp = al_deps::cache::cache_root_from_env(None).join("path-note.stamp");
    if !due(&stamp, SystemTime::now()) {
        return;
    }
    let me = std::env::current_exe().ok();
    eprintln!(
        "{}",
        note(first, others.len() - 1, env!("ALRUNNER_VERSION"), me.as_deref())
    );
    if let Some(dir) = stamp.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(&stamp, b"the note about another al-runner on PATH was printed\n");
}

/// The first line of `p --version`, waiting at most two seconds.
fn version_of(p: &Path) -> String {
    let child = Command::new(p)
        .arg("--version")
        // A v3 copy would list the others again, and so on.
        .env(OFF_VAR, "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => return format!("(could not start it: {e})"),
    };
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() < VERSION_WAIT => std::thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return "(no answer within 2 s)".into();
            }
        }
    }
    let mut out = String::new();
    if let Some(mut s) = child.stdout.take() {
        let _ = s.read_to_string(&mut out);
    }
    out.lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("(no version line)")
        .to_string()
}

/// The lines `--version` prints after its own: one per other copy.
pub fn version_lines() -> Vec<String> {
    if off() {
        return Vec::new();
    }
    others_from_env()
        .iter()
        .map(|p| format!("also on PATH: {}: {}", p.display(), version_of(p)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_note_is_due_without_a_stamp_and_a_day_after_the_last_one() {
        let d = tempfile::tempdir().unwrap();
        let stamp = d.path().join("path-note.stamp");
        let now = SystemTime::now();
        assert!(due(&stamp, now));
        std::fs::write(&stamp, b"x").unwrap();
        assert!(!due(&stamp, now));
        assert!(!due(&stamp, now + Duration::from_secs(23 * 3600)));
        assert!(due(&stamp, now + Duration::from_secs(25 * 3600)));
        let f = std::fs::File::options().write(true).open(&stamp).unwrap();
        f.set_modified(now - Duration::from_secs(25 * 3600)).unwrap();
        assert!(due(&stamp, now));
    }

    #[test]
    fn the_note_names_v2_only_for_the_dotnet_tools_folder() {
        let v2 = Path::new("/h/.dotnet/tools/al-runner");
        let other = Path::new("/opt/x/al-runner");
        let me = Path::new("/home/u/.local/bin/al-runner");
        assert!(note(v2, 0, "3.0.0", Some(me)).contains("(AL Runner v2, a dotnet tool)"));
        let n = note(other, 2, "3.0.0", Some(me));
        assert!(!n.contains("AL Runner v2, a dotnet tool"), "{n}");
        assert!(n.contains("If it is AL Runner v2, remove it, or put"), "{n}");
        assert!(n.contains("(2 more: see al-runner --version)"), "{n}");
        assert!(n.contains("put /home/u/.local/bin first on PATH"), "{n}");
    }
}
