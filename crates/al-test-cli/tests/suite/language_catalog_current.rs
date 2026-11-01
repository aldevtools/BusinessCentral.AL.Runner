//! `docs/language-catalog.json` is what the AL language servers of BC 27, 28 and 29 offer today.
//! Ignored: it needs Python, .NET and the AL Language extensions (BC artifacts or VS Code, see
//! `tools/language-catalog/generate.sh`). Without an extension it says so and passes.

use std::process::Command;

#[test]
#[ignore]
fn language_catalog_is_current() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = Command::new("bash")
        .arg(root.join("tools/language-catalog/generate.sh"))
        .arg("--check")
        .output()
        .expect("bash");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    match out.status.code() {
        Some(0) => {}
        Some(2) => println!("skipped: {text}"),
        _ => panic!("{text}"),
    }
}
