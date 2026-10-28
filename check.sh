#!/usr/bin/env bash
# The pre-commit gate: the same checks CI runs, failing on the first error.
#
# The tests are split as CI splits them (.github/workflows/ci.yml): every test in a debug build
# without the `cli-e2e` feature (CI's test job), then the two test files that run `al-test-cli`
# over the whole pinned corpus, `corpus_harness` and `dev_tooling`, against the release build
# (CI's release-tests job). In the debug build those two took 67 of the 117 s the tests ran; in
# release they take about 5 s. `--workspace` gives the release test build the features of
# `cargo build --release`, so it reuses the release build the workflow has already made
# (`-p al-test-cli` would select other features and build al-vm and the crates below it again).
# Issue #45 has the measurements.
set -euo pipefail
tools/legal_guard.sh
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace --no-default-features -q
cargo test --release --workspace --test corpus_harness --test dev_tooling -q
