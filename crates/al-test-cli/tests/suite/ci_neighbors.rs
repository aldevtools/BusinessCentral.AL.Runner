//! Owner decision 2026-11-21 (ci.yml header): the job `neighbors` runs `tools/neighbor-diff.sh` on every pull request,
//! drafts and fork pull requests included, checks the rule before it builds anything, and compares with the main
//! binary the `corpus` job uploads on a push to main.

use super::ci_bc_artifacts::{before, job, read};

#[test]
fn the_neighbor_job_runs_on_drafts_with_a_read_only_token_and_checks_the_rule_first() {
    let neighbors = job(&read(".github/workflows/ci.yml"), "neighbors");
    assert!(!neighbors.contains("draft"), "{neighbors}");
    assert!(!neighbors.contains("write"), "{neighbors}");
    assert!(!neighbors.contains("secrets."), "{neighbors}");
    before(
        &neighbors,
        "run: tools/neighbor-diff.sh --base HEAD^1 --plan",
        "uses: ./.github/actions/setup",
    );
    before(
        &neighbors,
        "uses: ./.github/actions/setup",
        "run: tools/neighbor-diff.sh --base HEAD^1 --changed",
    );
}

/// The corpus is checked out only when the rule step lists apps, and each run attempt names its own artifact.
#[test]
fn the_neighbor_job_checks_out_the_corpus_only_for_apps() {
    let neighbors = job(&read(".github/workflows/ci.yml"), "neighbors");
    assert!(!neighbors.contains("submodules: true"), "{neighbors}");
    before(
        &neighbors,
        "run: tools/neighbor-diff.sh --base HEAD^1 --plan",
        "git submodule update --init --depth 1 corpus",
    );
    let fetch = &neighbors[..neighbors.find("git submodule update --init --depth 1 corpus").unwrap()];
    let condition = fetch.lines().rev().find(|l| l.trim_start().starts_with("if:")).unwrap();
    assert!(condition.contains("steps.plan.outputs.apps == 'true'"), "{condition}");
    assert!(
        neighbors.contains("name: neighbors-${{ github.run_id }}-${{ github.run_attempt }}"),
        "{neighbors}"
    );
}

#[test]
fn a_push_to_main_uploads_the_main_binary_the_neighbor_job_fetches() {
    let corpus = job(&read(".github/workflows/ci.yml"), "corpus");
    before(
        &corpus,
        "cargo build --release -p alrunner-cli",
        "name: al-runner-main-${{ github.sha }}",
    );
    let upload = &corpus[corpus.find("name: al-runner-main-").unwrap()..];
    let step = &corpus[..corpus.find("name: al-runner-main-").unwrap()];
    let condition = step.lines().rev().find(|l| l.trim_start().starts_with("if:")).unwrap();
    assert!(
        condition.contains("github.event_name == 'push' && github.ref == 'refs/heads/main'"),
        "{condition}"
    );
    assert!(upload.contains("path: target/release/al-runner"));
    assert!(read("tools/neighbor-diff.sh").contains("artifacts?name=al-runner-main-$1"));
}
