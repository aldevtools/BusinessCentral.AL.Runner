//! This crate's list of refusal keys (ADR-071; one list per crate that refuses, owner decision 2026-11-10): one file
//! per key in `src/refusals/<key>.rs`, collected by `build.rs`. `al-test-cli` checks every list and the generated
//! table of `docs/probes-needed.md` (`al-test-cli --root . --refusals-doc`).

use al_builtins::refusal_keys;

include!(concat!(env!("OUT_DIR"), "/refusal_registry.rs"));
