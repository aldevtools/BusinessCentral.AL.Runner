//! The trademark notice is in the README, the NOTICE file and the README.txt of the release archive,
//! and no help text of al-runner makes a licensing claim (`tools/legal_guard.sh`
//! checks the tracked files; this checks what the binary prints).

use std::path::{Path, PathBuf};
use std::process::Command;

const NOTICE: &str = "AL Runner is an independent project. It is not affiliated with, endorsed by or sponsored by \
Microsoft. Microsoft, Dynamics 365 and Business Central are trademarks of Microsoft Corporation.";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Line breaks and runs of blanks count as one blank, so a wrapped paragraph still matches.
fn flat(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn readme_notice_and_release_readme_carry_the_trademark_notice() {
    for file in ["README.md", "NOTICE", "release/README.txt"] {
        let text = std::fs::read_to_string(root().join(file)).unwrap_or_default();
        assert!(flat(&text).contains(NOTICE), "{file} lacks the notice");
    }
}

#[test]
fn no_help_text_claims_a_run_without_a_license() {
    let mut helps = vec![vec!["--help"]];
    for sub in ["test", "debug", "cache", "serve", "tdd", "affected", "impact", "matrix"] {
        helps.push(vec![sub, "--help"]);
    }
    let claims = [
        ["without a BC lic", "ense"].concat(),
        ["no BC lic", "ense"].concat(),
        ["without a lic", "ense"].concat(),
        ["no lic", "ense"].concat(),
    ];
    for args in helps {
        let o = Command::new(env!("CARGO_BIN_EXE_al-runner"))
            .args(&args)
            .output()
            .unwrap();
        let out = flat(&format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        ))
        .to_lowercase();
        for c in &claims {
            assert!(!out.contains(&c.to_lowercase()), "{args:?} prints {c:?}: {out}");
        }
    }
}
