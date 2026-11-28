//! The outcomes leg of the workspace layer (ADR-022 amendment 2026-11-15, "Equality requirement", Outcomes):
//! `tools/cache_vs_uncached.sh workspace` runs the corpus with the compile cache three times per BC version: without the
//! workspace layer, with it (each pass builds its entry), and with it again (each pass reads its entry). The test
//! outcomes of the three must be equal. A corpus run uses the layer only under [`SWITCH`], an internal switch of that
//! script, with `--compile-cache` and `AL_FRESH_PASS_COMPILE=1`: a pass that grows the main pass's compile (ADR-063)
//! does not use the layer (decision 7).

use al_testrun::workspace_cache::{WorkspaceCacheOptions, WorkspaceReport};

/// The switch of the leg: `AL_CORPUS_WORKSPACE_CACHE=1`.
pub const SWITCH: &str = "AL_CORPUS_WORKSPACE_CACHE";

/// The options whose value names a file the run writes its reports to. They change no compile, so two runs of the leg
/// that differ only in them share their entries.
const REPORT_FILES: [&str; 4] = ["--junit", "--json", "--markdown", "--cell-report"];

/// The workspace layer of a corpus run: off unless the leg's switch is on and the run uses the compile cache.
pub fn options(compile_cache: bool) -> Option<WorkspaceCacheOptions> {
    let on = compile_cache && al_testrun::switches::on(SWITCH);
    on.then(|| WorkspaceCacheOptions {
        compile_options: compile_text(std::env::args().skip(1)),
        app_json: Vec::new(),
        write_now: true,
        verbose: true,
    })
}

/// The compile options in the key of a corpus run's entries: every word of its command line but the files it writes
/// its reports to. A run with any other option is another key: the leg never decides which options leave a compile
/// unchanged.
fn compile_text(mut args: impl Iterator<Item = String>) -> String {
    let mut words = Vec::new();
    while let Some(a) = args.next() {
        if REPORT_FILES.contains(&a.as_str()) {
            args.next();
            continue;
        }
        words.push(a);
    }
    // Each word after its length, so that no two command lines give the same text.
    let words: Vec<String> = words.iter().map(|w| format!("{}:{w}", w.len())).collect();
    format!("al-test-cli {}", words.join(" "))
}

/// Under the switch, prints [`pass_line`] for a pass (the script counts the lines).
pub fn say(bc: &str, pass: &str, report: Option<&WorkspaceReport>) {
    if let Some(line) = pass_line(bc, pass, report) {
        println!("{line}");
    }
}

/// The line a pass prints under the switch: what the workspace layer did for its compile.
fn pass_line(bc: &str, pass: &str, report: Option<&WorkspaceReport>) -> Option<String> {
    if !al_testrun::switches::on(SWITCH) {
        return None;
    }
    Some(match report {
        Some(r) => format!(
            "BC {bc} ({pass}): workspace layer {}{}",
            r.state.as_str(),
            if r.notes.is_empty() {
                String::new()
            } else {
                format!(" ({})", r.notes.join("; "))
            }
        ),
        None => format!("BC {bc} ({pass}): workspace layer none"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(words: &[&str]) -> String {
        compile_text(words.iter().map(|w| w.to_string()))
    }

    #[test]
    fn two_runs_that_differ_only_in_their_report_files_share_a_key() {
        let built = text(&["--bc-version", "29", "--compile-cache", "--junit", "wsbuilt-junit.xml"]);
        let read = text(&["--bc-version", "29", "--compile-cache", "--junit", "wsread-junit.xml"]);
        assert_eq!(built, read);
        let json = text(&[
            "--bc-version",
            "29",
            "--compile-cache",
            "--json",
            "a.json",
            "--junit",
            "b.xml",
        ]);
        assert_eq!(built, json);
    }

    #[test]
    fn any_other_option_is_another_key() {
        let base = text(&["--bc-version", "29", "--compile-cache"]);
        for other in [
            &["--bc-version", "28", "--compile-cache"][..],
            &["--bc-version", "29", "--compile-cache", "--test-runner", "builtin"][..],
            &["--bc-version", "29", "--compile-cache", "--file", "a.al"][..],
        ] {
            assert_ne!(base, text(other), "{other:?}");
        }
    }
}
