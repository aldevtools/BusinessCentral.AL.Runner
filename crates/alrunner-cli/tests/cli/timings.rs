//! `--timings` (al-runner issue 8205): the setup before the compile, split into its parts.

use super::*;

#[test]
fn timings_split_the_setup_into_its_parts() {
    let d = tempfile::tempdir().unwrap();
    let app = one_file_app(
        d.path(),
        "setup",
        "11111111-0000-0000-0000-000000008205",
        "Setup Parts",
        "src/Tests.al",
        "codeunit 50820 \"Setup Parts\" { Subtype = Test; [Test] procedure Passes() begin end; }\n",
    );
    let o = alrunner(&["test", app.to_str().unwrap(), "--no-download", "--timings"]);
    let t = text(&o);
    assert_eq!(code(&o), 0, "{t}");
    let line = t
        .lines()
        .find(|l| l.starts_with("setup: "))
        .unwrap_or_else(|| panic!("{t}"));
    let parts: Vec<&str> = line["setup: ".len()..]
        .split(", ")
        .map(|p| p.rsplitn(3, ' ').nth(2).unwrap_or_else(|| panic!("{line}")))
        .collect();
    assert_eq!(
        parts,
        [
            "options",
            "sources",
            "selection",
            "dependencies",
            "test data",
            "license",
            "dotnet",
            "affected",
            "entries",
            "prepare"
        ],
        "{line}"
    );
}
