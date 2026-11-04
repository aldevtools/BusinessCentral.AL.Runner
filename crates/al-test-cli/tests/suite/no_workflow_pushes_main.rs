//! ADR-065, Conformance: no workflow pushes to main. Main changes only through merged pull requests, so each merge
//! moves it once (the regen bot that pushed generated files to main is gone).

use std::path::Path;

#[test]
fn no_workflow_pushes_to_main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.github/workflows");
    let mut seen = 0;
    for e in std::fs::read_dir(&dir).unwrap() {
        let p = e.unwrap().path();
        if !p.extension().is_some_and(|x| x == "yml" || x == "yaml") {
            continue;
        }
        seen += 1;
        let text = std::fs::read_to_string(&p).unwrap();
        for (n, line) in text.lines().enumerate() {
            let code = line.split('#').next().unwrap_or_default();
            if code.contains("git push") {
                assert!(
                    !code.contains("main") && code.contains("refs/"),
                    "{}:{}: a push must name a ref that is not main: {line}",
                    p.display(),
                    n + 1
                );
            }
        }
        assert!(!p.ends_with("regen-bot.yml"), "the regen bot is retired (ADR-065)");
    }
    assert!(seen > 3, "no workflows found in {}", dir.display());
}
