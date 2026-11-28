//! Decision 2.8 (review MED 2): every environment variable the crates of the compile and of the runner read is named,
//! either in `COMPILE_ENV` (keyed, off, or not an output) or below as a variable of the run phase or of the tools, with
//! the reason. The scan reads every source file that is not a test of al-testrun, al-version, al-deps and the compile
//! crates, and takes as a name:
//! - every string literal that looks like one of the runner's variables (`AL_`, `ALRUNNER_`, `ALRT_`), wherever it is
//!   (a read through a constant, a helper or a closure names it somewhere as a literal);
//! - every literal an environment read takes directly (`env::var`, `env::var_os`, `env::vars`);
//! - in a file whose reads take a value that is not a literal (a helper, a closure, a constant), every literal in upper
//!   case and underscores, which is how the helper is given its names.

use super::{COMPILE_ENV, EnvUse};
use std::path::{Path, PathBuf};

/// Variables of the run phase, of the tools and of the host, which the compile does not read: (name, why).
const NOT_COMPILE: &[(&str, &str)] = &[
    ("ALRT_DOTNET_SIDECAR", "the .NET sidecar of the run phase"),
    (
        "ALRUNNER_ARTIFACT_URL",
        "where Microsoft's artifacts are downloaded from (the apps, in the layers' key)",
    ),
    ("ALRUNNER_CACHE", "the cache folder"),
    ("AL_RUNNER_CACHE_ROOT", "the cache folder"),
    ("AL_RUNTIME_CACHE", "the cache folder"),
    (
        "ALRUNNER_CHECK_INCREMENTAL",
        "a check of serve's incremental compile against a complete one",
    ),
    (
        "ALRUNNER_NO_INCREMENTAL",
        "serve compiles all of it instead of the incremental compile",
    ),
    ("ALRUNNER_DEBUG_PANIC_IN", "a test of the debugger"),
    ("ALRUNNER_DOTNET_FRESH", "the .NET sidecar of the run phase"),
    ("ALRUNNER_DOTNET_REUSE", "the .NET sidecar of the run phase"),
    (
        "ALRUNNER_LOGIN_REPLAY_MAX_IMAGES",
        "the run phase: the most login starts an engine keeps",
    ),
    (
        "ALRUNNER_LOGIN_REPLAY_MAX_MB",
        "the run phase: the most bytes the kept login starts hold",
    ),
    ("ALRUNNER_NO_CACHE_PRUNE", "the cleanup of the cache folder"),
    ("ALRUNNER_REAL_CLOCK", "the run phase's clock"),
    (
        "ALRUNNER_TEST_SLOW_CACHE_WRITE_MS",
        "a test of serve's shutdown: the writes of the workspace layer wait",
    ),
    ("AL_BLOCK_PATH", "a test of the run phase"),
    (
        "AL_CHECK_BASELINE_RULE",
        "the run phase: the install baseline rule check",
    ),
    ("AL_CRASH_CHILD", "the crash report child process"),
    (
        "AL_FRESH_PASS_COMPILE",
        "the corpus harness's passes (ADR-063): a growing compile turns the layer off",
    ),
    ("AL_INSTALL_FACTS", "the run phase's install code"),
    ("AL_PROFILE", "the profiler of the run phase"),
    ("AL_PROFILE_US", "the profiler of the run phase"),
    ("AL_TEST_THREADS", "the run phase's test threads"),
    ("DOTNET_ROOT", "the .NET sidecar of the run phase"),
    ("HOME", "the cache folder"),
    ("PATH", "the .NET sidecar of the run phase"),
    ("XDG_CACHE_HOME", "the cache folder"),
    ("USERPROFILE", "the cache folder"),
    ("TMPDIR", "the cache folder"),
    ("DOTNET_CLI_TELEMETRY_OPTOUT", "the .NET sidecar of the run phase"),
    ("DOTNET_NOLOGO", "the .NET sidecar of the run phase"),
    ("TEMP", "the .NET sidecar of the run phase"),
    ("TMP", "the .NET sidecar of the run phase"),
    (
        "UTC",
        "the .NET sidecar of the run phase (a value of its environment, not a variable)",
    ),
];

/// The crates the scan reads.
const CRATES: &[&str] = &[
    "al-testrun",
    "al-version",
    "al-deps",
    "al-sema",
    "al-syntax",
    "al-ir",
    "al-link",
    "al-builtins",
    "al-cache",
];

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        if p.is_dir() {
            if name != "tests" {
                collect(&p, out);
            }
        } else if name.ends_with(".rs")
            && name != "tests.rs"
            && !name.ends_with("_tests.rs")
            // The list itself, whose reads take the names of the list.
            && name != "workspace_cache.rs"
        {
            out.push(p);
        }
    }
}

/// The code of a file without its `#[cfg(test)] mod ... { }` blocks and without comments.
fn code_of(text: &str) -> String {
    let mut out = String::new();
    for line in text.lines() {
        let t = line.trim_start();
        if t.starts_with("//") {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    // Each `#[cfg(test)]` item, up to the end of its braces (or its `;`).
    while let Some(at) = out.find("#[cfg(test)]") {
        let b = out.as_bytes();
        let mut j = at + "#[cfg(test)]".len();
        while j < b.len() && b[j] != b'{' && b[j] != b';' {
            j += 1;
        }
        let mut depth = 0usize;
        while j < b.len() {
            match b[j] {
                b'{' => depth += 1,
                b'}' => depth = depth.saturating_sub(1),
                _ => {}
            }
            j += 1;
            if depth == 0 {
                break;
            }
        }
        out.replace_range(at..j.min(out.len()), "");
    }
    out
}

/// The string literals of `code`, with where each starts.
fn literals(code: &str) -> Vec<(usize, String)> {
    let b = code.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\'' && i + 2 < b.len() && b[i + 2] == b'\'' {
            i += 3; // a char literal such as '"'
            continue;
        }
        if b[i] == b'"' {
            let start = i + 1;
            let mut j = start;
            while j < b.len() && b[j] != b'"' {
                if b[j] == b'\\' {
                    j += 1;
                }
                j += 1;
            }
            out.push((i, code[start..j.min(b.len())].to_string()));
            i = j + 1;
            continue;
        }
        i += 1;
    }
    out
}

fn upper_case(s: &str) -> bool {
    s.len() >= 3
        && s.bytes()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == b'_')
        && s.bytes().next().is_some_and(|c| c.is_ascii_uppercase())
}

fn runner_name(s: &str) -> bool {
    upper_case(s) && ["AL_", "ALRUNNER_", "ALRT_"].iter().any(|p| s.starts_with(p))
}

const READS: [&str; 3] = ["env::var(", "env::var_os(", "env::vars("];

#[test]
fn every_environment_variable_of_the_compile_and_the_runner_is_named() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    for c in CRATES {
        collect(&root.join("crates").join(c).join("src"), &mut files);
    }
    assert!(files.len() > 200, "the scan reads the crates ({} files)", files.len());
    let named = |n: &str| COMPILE_ENV.iter().any(|(c, _)| *c == n) || NOT_COMPILE.iter().any(|(c, _)| *c == n);
    let mut unnamed = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for path in &files {
        let code = code_of(&std::fs::read_to_string(path).unwrap());
        let lits = literals(&code);
        let mut names: Vec<String> = lits
            .iter()
            .filter(|(_, l)| runner_name(l))
            .map(|(_, l)| l.clone())
            .collect();
        let mut through_a_value = false;
        for read in READS {
            for (at, _) in code.match_indices(read) {
                let arg = code[at + read.len()..].trim_start();
                if let Some(rest) = arg.strip_prefix('"') {
                    names.push(rest.chars().take_while(|c| *c != '"').collect());
                } else if !arg.starts_with(')') {
                    through_a_value = true;
                }
            }
        }
        if through_a_value {
            names.extend(lits.iter().filter(|(_, l)| upper_case(l)).map(|(_, l)| l.clone()));
        }
        for n in names {
            seen.insert(n.clone());
            if !named(&n) {
                unnamed.push(format!("{}: {n}", path.strip_prefix(&root).unwrap_or(path).display()));
            }
        }
    }
    unnamed.sort();
    unnamed.dedup();
    assert!(
        unnamed.is_empty(),
        "environment variables named neither in COMPILE_ENV nor in NOT_COMPILE: {unnamed:#?}"
    );
    let stale: Vec<&str> = NOT_COMPILE
        .iter()
        .map(|(n, _)| *n)
        .filter(|n| !seen.contains(*n))
        .collect();
    assert!(stale.is_empty(), "named in NOT_COMPILE but read nowhere: {stale:?}");
    let stale: Vec<&str> = COMPILE_ENV
        .iter()
        .map(|(n, _)| *n)
        .filter(|n| !seen.contains(*n))
        .collect();
    assert!(stale.is_empty(), "named in COMPILE_ENV but read nowhere: {stale:?}");
}

/// The list of the compile and the list of the rest do not overlap, and the reason of every `NotAnOutput` is given.
#[test]
fn the_lists_do_not_overlap() {
    for (n, u) in COMPILE_ENV {
        assert!(!NOT_COMPILE.iter().any(|(c, _)| c == n), "{n} is in both lists");
        if let EnvUse::NotAnOutput(why) = u {
            assert!(!why.is_empty(), "{n}");
        }
    }
}
