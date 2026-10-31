//! Two branches that each add an assumption, or one that removes an entry while the other adds one, merge
//! without a conflict in the registry (`tools/assumption_merge_check.sh`). Before the registry kept one file
//! per entry, every pull request appended to the end of `assumptions.rs` and every pair of them conflicted.

use std::path::Path;
use std::process::Command;

#[test]
fn two_branches_that_change_the_registry_merge_cleanly() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let o = Command::new("bash")
        .arg(root.join("tools/assumption_merge_check.sh"))
        .arg(&root)
        .output()
        .expect("run tools/assumption_merge_check.sh");
    assert!(
        o.status.success(),
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
}
